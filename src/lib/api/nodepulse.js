/**
 * NodePulse REST API client for the desktop app.
 * Uses Authorization: Bearer header (not cookies).
 */

/**
 * @param {string} url - NodePulse base URL, e.g. "http://192.168.1.100:18080"
 * @param {string} path
 * @param {RequestInit} [options]
 * @param {string} [token]
 */
async function apiFetch(url, path, options = {}, token = null) {
  const headers = { 'Content-Type': 'application/json', ...(options.headers || {}) };
  if (token) headers['Authorization'] = `Bearer ${token}`;

  const res = await fetch(`${url.replace(/\/$/, '')}${path}`, {
    ...options,
    headers,
  });

  if (!res.ok) {
    let errMsg = `HTTP ${res.status}`;
    try {
      const body = await res.json();
      errMsg = body.error || body.message || errMsg;
    } catch { /* ignore */ }
    throw new Error(errMsg);
  }

  return res.json();
}

/**
 * POST /api/v1/auth/login/token
 * Returns { token, expires_at, role, cluster_id }
 * role: 'admin' | 'operator' | 'client'
 * cluster_id: non-empty only when role === 'client'
 */
export async function login(nodepulseUrl, username, password) {
  const data = await apiFetch(nodepulseUrl, '/api/v1/auth/login/token', {
    method: 'POST',
    body: JSON.stringify({ username, password }),
  });
  return data; // { token, expires_at }
}

/**
 * GET /api/v1/auth/me
 * Returns { data: { id, username, role, org_id } }
 */
export async function getMe(nodepulseUrl, token) {
  return apiFetch(nodepulseUrl, '/api/v1/auth/me', {}, token);
}

/**
 * GET /api/v1/clusters
 * Returns { data: [{ id, name, ... }] }
 */
export async function getClusters(nodepulseUrl, token) {
  return apiFetch(nodepulseUrl, '/api/v1/clusters', {}, token);
}

/**
 * GET /api/v1/clusters/:id/nodes
 * Returns { data: [{ id, hostname, status, ... }] }
 */
export async function getClusterNodes(nodepulseUrl, token, clusterId) {
  return apiFetch(nodepulseUrl, `/api/v1/clusters/${clusterId}/nodes`, {}, token);
}

/**
 * GET /api/v1/network/config
 * Returns { headscale_url, core_grpc_address }
 */
export async function getNetworkConfig(nodepulseUrl, token) {
  return apiFetch(nodepulseUrl, '/api/v1/network/config', {}, token);
}

/**
 * POST /api/v1/network/provision/network-key
 * Generates a short-lived pre-auth key for a personal device (no runner).
 * headscaleUser is derived from the cluster name (slugified), same convention as web-panel.
 * Returns { data: { pre_auth_key, expires_at, headscale_user } }
 */
export async function generateNetworkKey(nodepulseUrl, token, headscaleUser) {
  return apiFetch(
    nodepulseUrl,
    '/api/v1/network/provision/network-key',
    {
      method: 'POST',
      body: JSON.stringify({ headscale_user: headscaleUser }),
    },
    token,
  );
}

/**
 * POST /api/v1/connect/register
 * Registers this device after joining the mesh. Fire-and-forget — caller should catch errors.
 */
export async function registerDevice(nodepulseUrl, token, payload) {
  return apiFetch(
    nodepulseUrl,
    '/api/v1/connect/register',
    { method: 'POST', body: JSON.stringify(payload) },
    token,
  );
}

/** Returns "windows" | "macos" | "linux" based on navigator.platform */
export function detectPlatform() {
  const p = navigator.platform.toLowerCase();
  if (p.startsWith('win')) return 'windows';
  if (p.startsWith('mac')) return 'macos';
  return 'linux';
}
