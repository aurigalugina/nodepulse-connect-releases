import { invoke } from '@tauri-apps/api/core';

/** @type {{ nodepulse_url: string|null, username: string|null, auth_token: string|null, last_cluster_id: string|null, device_name: string|null, role: string|null, cluster_id: string|null }} */
let _config = $state({
  nodepulse_url: null,
  username: null,
  auth_token: null,
  last_cluster_id: null,
  device_name: null,
  role: null,
  cluster_id: null,
});

/** @type {boolean} */
let _loaded = $state(false);

export const authStore = {
  get config() { return _config; },
  get loaded() { return _loaded; },
  get token() { return _config.auth_token; },
  get url() { return _config.nodepulse_url; },
  get username() { return _config.username; },
  get role() { return _config.role; },
  get clusterId() { return _config.cluster_id; },

  /** Load persisted config from disk. Call once at app startup. */
  async load() {
    try {
      const saved = await invoke('read_config');
      _config = { ..._config, ...saved };
    } catch (e) {
      console.warn('Failed to load config:', e);
    }
    _loaded = true;
  },

  /** Persist a partial update to disk. */
  async save(patch) {
    _config = { ..._config, ...patch };
    await invoke('write_config', { config: _config });
  },

  /** Store credentials after successful login. */
  async setAuth(url, username, token, role = null, clusterId = null) {
    await authStore.save({ nodepulse_url: url, username, auth_token: token, role, cluster_id: clusterId });
  },

  /** Clear token on logout (keep url + username for re-login convenience). */
  async clearAuth() {
    await invoke('clear_auth_token');
    _config = { ..._config, auth_token: null };
  },
};
