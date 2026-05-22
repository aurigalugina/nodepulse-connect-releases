import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { authStore } from './authStore.svelte.js';

/**
 * Connection states:
 * IDLE → AUTHENTICATING → CLUSTER_SELECT → GENERATING_KEY → TAILSCALE_UP → CONNECTED
 */

/** @type {'IDLE'|'AUTHENTICATING'|'CLUSTER_SELECT'|'GENERATING_KEY'|'TAILSCALE_UP'|'CONNECTED'} */
let _state = $state('IDLE');
/** @type {string|null} */
let _error = $state(null);
/** @type {string|null} */
let _meshIp = $state(null);
/** @type {{ id: string, name: string }|null} */
let _cluster = $state(null);
/** @type {string|null} */
let _deviceName = $state(null);
/** @type {string[]} */
let _steps = $state([]);

let _pollInterval = null;
let _pollWasEverOnline = false;

export const connectionStore = {
  get state() { return _state; },
  get error() { return _error; },
  get meshIp() { return _meshIp; },
  get cluster() { return _cluster; },
  get deviceName() { return _deviceName; },
  get steps() { return _steps; },

  /** Called from App.svelte after authStore.load() to attempt auto-reconnect. */
  async tryAutoReconnect() {
    const { auth_token, last_cluster_id, device_name } = authStore.config;
    if (!auth_token || !last_cluster_id) return;

    try {
      const status = await invoke('tailscale_status');
      if (status.online) {
        _meshIp = status.mesh_ip;
        _deviceName = device_name;
        // Cluster info will be re-fetched by Connected.svelte
        _cluster = { id: last_cluster_id, name: '...' };
        _state = 'CONNECTED';
        startPolling(10_000); // reconnect: node should already be online, short grace
        await invoke('set_tray_connected', { connected: true });
      }
    } catch {
      // Not connected — stay at IDLE
    }
  },

  setError(msg) {
    _error = msg;
  },

  clearError() {
    _error = null;
  },

  transition(newState) {
    _state = newState;
    _error = null;
  },

  addStep(msg) {
    _steps = [..._steps, msg];
  },

  resetSteps() {
    _steps = [];
  },

  setMeshIp(ip) {
    _meshIp = ip;
  },

  setCluster(cluster) {
    _cluster = cluster;
    authStore.save({ last_cluster_id: cluster.id });
  },

  setDeviceName(name) {
    _deviceName = name;
    authStore.save({ device_name: name });
  },

  async disconnect() {
    stopPolling();
    try {
      await invoke('tailscale_down');
    } catch (e) {
      console.warn('tailscale_down error (ignoring):', e);
    }
    await invoke('set_tray_connected', { connected: false });
    await authStore.clearAuth();
    _state = 'IDLE';
    _meshIp = null;
    _cluster = null;
    _steps = [];
  },
};

// graceMs: how long after polling starts to wait before treating offline as a disconnect.
// After a fresh tailscale_up, the DERP relay + WireGuard key exchange can take 30-60s
// on Windows userspace networking. Without a grace period, the first poll (5s) fires
// before the mesh is established and falsely disconnects the session.
function startPolling(graceMs = 45_000) {
  if (_pollInterval) return;

  _pollWasEverOnline = false;
  const graceEnd = Date.now() + graceMs;
  let consecutiveOffline = 0;

  _pollInterval = setInterval(async () => {
    try {
      const status = await invoke('tailscale_status');
      if (status.online) {
        consecutiveOffline = 0;
        _pollWasEverOnline = true;
        _meshIp = status.mesh_ip;
      } else if (_state === 'CONNECTED') {
        if (_pollWasEverOnline) {
          // Was connected — require 3 consecutive offline readings before declaring lost
          consecutiveOffline++;
          if (consecutiveOffline >= 3) {
            stopPolling();
            _meshIp = null;
            _steps = [];
            _error = 'Mesh connection lost. Please reconnect.';
            _state = 'TAILSCALE_UP'; // show Connecting.svelte with error + Try again button
            await invoke('set_tray_connected', { connected: false });
          }
        } else if (Date.now() > graceEnd) {
          // Never came online within grace window — give up
          stopPolling();
          _meshIp = null;
          _steps = [];
          _error = 'Mesh connection timed out. Try again, or check if the DERP relay is reachable.';
          _state = 'TAILSCALE_UP'; // show Connecting.svelte with error + Try again button
          await invoke('set_tray_connected', { connected: false });
        }
        // else: still within grace period — keep waiting, don't disconnect
      }
    } catch {
      // Ignore transient errors during polling
    }
  }, 5000);
}

function stopPolling() {
  if (_pollInterval) {
    clearInterval(_pollInterval);
    _pollInterval = null;
  }
}

// Listen for tray "disconnect" menu item
// Guard: listen() only works inside Tauri window, not plain browser
if (typeof window !== 'undefined' && window.__TAURI_INTERNALS__) {
  listen('tray-disconnect', async () => {
    if (_state === 'CONNECTED') {
      await connectionStore.disconnect();
    }
  }).catch((e) => console.warn('tray-disconnect listen error:', e));
}

// Export startPolling so Connected.svelte can call it after TAILSCALE_UP
export { startPolling };
