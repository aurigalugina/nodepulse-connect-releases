<script>
  import { invoke } from '@tauri-apps/api/core';
  import { authStore } from '$lib/stores/authStore.svelte.js';
  import { connectionStore, startPolling } from '$lib/stores/connectionStore.svelte.js';
  import { getClusters, getNetworkConfig, generateNetworkKey, registerDevice, detectPlatform } from '$lib/api/nodepulse.js';
  import { getVersion } from '@tauri-apps/api/app';
  import { ChevronLeft, Loader, Server } from 'lucide-svelte';

  const isClient = $derived(authStore.role === 'client');
  const isAdmin  = $derived(authStore.role === 'admin' || authStore.role === 'operator');

  /** @type {{ id: string, name: string, health: string }[]} */
  let clusters = $state([]);
  let loading = $state(true);
  let error = $state('');
  let submitting = $state(false);

  let selectedClusterId = $state('');
  let deviceName = $state(authStore.config.device_name ?? '');

  $effect(() => {
    if (isClient) {
      loadClusters().then(() => {
        if (deviceName.trim()) autoJoinClient();
      });
    }
    // admin/operator: no cluster fetch needed
  });

  async function loadClusters() {
    loading = true;
    error = '';
    try {
      const res = await getClusters(authStore.url, authStore.token);
      const raw = res?.data ?? res ?? [];
      clusters = raw.map(c => ({
        id:     c.id     ?? c.ID,
        name:   c.name   ?? c.Name   ?? '',
        health: c.health ?? c.Health ?? '',
      }));

      if (isClient && authStore.clusterId) {
        // Pre-select the assigned cluster
        selectedClusterId = authStore.clusterId;
      }
    } catch (e) {
      error = e.message || 'Failed to load clusters.';
    } finally {
      loading = false;
    }
  }

  function slugify(name) {
    return name.toLowerCase()
      .normalize('NFD').replace(/[̀-ͯ]/g, '')
      .replace(/[^a-z0-9\s-]/g, '').trim()
      .replace(/\s+/g, '-').replace(/-+/g, '-').replace(/^-|-$/g, '');
  }

  /** For admin/operator: always join nodepulse-core namespace */
  async function autoJoinAdmin() {
    await startJoin({ id: 'core', name: 'NodePulse Core' }, 'nodepulse-core');
  }

  /** For client role: derive headscale namespace as <cluster-slug>-connect */
  async function autoJoinClient() {
    if (!authStore.clusterId) {
      error = 'No cluster assigned to your account. Contact your administrator.';
      return;
    }
    const cluster = clusters.find((c) => c.id === authStore.clusterId);
    if (!cluster) {
      error = 'Assigned cluster not found. Contact your administrator.';
      return;
    }
    const headscaleUser = slugify(cluster.name) + '-connect';
    await startJoin(cluster, headscaleUser);
  }

  async function handleJoin() {
    if (!selectedClusterId || !deviceName.trim()) {
      error = 'Select a cluster and enter a device name.';
      return;
    }
    const cluster = clusters.find((c) => c.id === selectedClusterId);
    if (!cluster) return;
    const headscaleUser = slugify(cluster.name);
    await startJoin(cluster, headscaleUser);
  }

  async function startJoin(cluster, headscaleUser) {
    submitting = true;
    error = '';
    connectionStore.transition('GENERATING_KEY');
    connectionStore.resetSteps();
    connectionStore.setCluster(cluster);
    connectionStore.setDeviceName(deviceName.trim() || authStore.config.device_name);

    const dbg = (msg) => connectionStore.addDebugLog(msg);
    const hn = deviceName.trim() || authStore.config.device_name;

    try {
      dbg(`connect → cluster="${cluster.name}" user="${headscaleUser}" device="${hn}"`);

      dbg(`GET /api/v1/network/config`);
      const netConfig = await getNetworkConfig(authStore.url, authStore.token);
      const headscaleUrl = netConfig?.headscale_url ?? netConfig?.data?.headscale_url;
      if (!headscaleUrl) throw new Error('Could not retrieve Headscale URL from NodePulse.');
      dbg(`headscale_url = ${headscaleUrl}`);
      connectionStore.addStep('Network config retrieved');

      dbg(`POST /api/v1/network/generate_key (user=${headscaleUser})`);
      const keyRes = await generateNetworkKey(authStore.url, authStore.token, headscaleUser);
      const preAuthKey = keyRes?.data?.pre_auth_key ?? keyRes?.pre_auth_key;
      if (!preAuthKey) throw new Error('Failed to generate pre-auth key.');
      dbg(`pre_auth_key generated (${preAuthKey.slice(0, 8)}...)`);
      connectionStore.addStep('Pre-auth key generated');

      connectionStore.transition('TAILSCALE_UP');
      connectionStore.addStep('Joining mesh network…');
      dbg(`invoke tailscale_up hostname="${hn}"`);
      await invoke('tailscale_up', {
        loginServer: headscaleUrl,
        authkey: preAuthKey,
        hostname: hn,
      });
      dbg(`tailscale_up returned OK`);
      connectionStore.addStep('Mesh connection established');

      const status = await invoke('tailscale_status');
      dbg(`tailscale_status: state=${status.backend_state} online=${status.online} ip=${status.mesh_ip}`);
      connectionStore.setMeshIp(status.mesh_ip ?? null);
      connectionStore.addStep('IP address assigned');

      connectionStore.transition('CONNECTED');
      startPolling();
      await invoke('set_tray_connected', { connected: true });

      // Fire-and-forget: register device so web panel can track it
      Promise.all([
        getVersion(),
        invoke('get_device_identity').catch(() => ({ machine_id: '', mac_address: '' })),
      ]).then(([appVersion, identity]) =>
        registerDevice(authStore.url, authStore.token, {
          hostname:    (deviceName.trim() || authStore.config.device_name),
          platform:    detectPlatform(),
          app_version: appVersion,
          mesh_ip:     status.mesh_ip ?? '',
          machine_id:  identity.machine_id ?? '',
          mac_address: identity.mac_address ?? '',
        })
      ).catch(() => { /* non-fatal */ });
    } catch (e) {
      const msg = typeof e === 'string' ? e : (e?.message || String(e) || 'Connection failed.');
      dbg(`ERROR: ${msg}`);
      connectionStore.setError(msg);
    } finally {
      submitting = false;
    }
  }

  function handleBack() {
    authStore.clearAuth();
    connectionStore.transition('IDLE');
  }
</script>

<div class="flex flex-col h-full">
  <!-- Header -->
  <div class="px-6 pt-6 pb-4 flex items-center gap-3">
    <button onclick={handleBack}
            class="w-7 h-7 rounded-md flex items-center justify-center text-np-muted hover:text-np-text transition-colors"
            style="background: var(--color-np-elevated); border: 1px solid var(--color-np-border);">
      <ChevronLeft size={15} />
    </button>
    <div>
      {#if isClient}
        <p class="text-np-text font-semibold text-sm leading-tight">Joining Your Cluster</p>
        <p class="text-np-muted text-xs leading-tight">Connecting to your assigned infrastructure</p>
      {:else}
        <p class="text-np-text font-semibold text-sm leading-tight">Connect to Core</p>
        <p class="text-np-muted text-xs leading-tight">Join the NodePulse core mesh network</p>
      {/if}
    </div>
  </div>

  <div class="mx-6 h-px" style="background: var(--color-np-border-dim);"></div>

  <div class="flex flex-col gap-4 px-6 pt-4 pb-6 flex-1 overflow-y-auto">

    {#if isClient}
      <!-- Client role: device name only, cluster is pre-assigned -->
      <div class="flex flex-col gap-1.5">
        <label class="text-xs text-np-muted font-medium" for="device-name">Device name</label>
        <input
          id="device-name"
          type="text"
          class="np-input"
          bind:value={deviceName}
          placeholder="my-home-pc"
          disabled={submitting}
          autocomplete="off"
          spellcheck="false"
        />
        <p class="text-xs text-np-subtle">Identifies this device in the mesh network</p>
      </div>

      {#if loading}
        <div class="flex items-center gap-2 py-3 text-xs text-np-muted">
          <Loader size={13} class="animate-spin" />
          Loading cluster info…
        </div>
      {:else}
        {@const assignedCluster = clusters.find((c) => c.id === authStore.clusterId)}
        {#if assignedCluster}
          <div class="flex items-center gap-3 rounded-lg px-3 py-2.5"
               style="background: var(--color-np-indigo-dim); border: 1px solid var(--color-np-indigo-ring);">
            <Server size={13} class="text-np-indigo-light shrink-0" />
            <span class="text-sm text-np-text flex-1 leading-tight">{assignedCluster.name}</span>
            <span class="text-xs text-np-muted">Assigned</span>
          </div>
        {/if}
      {/if}

      {#if error}
        <p class="text-xs text-np-red -mt-1">{error}</p>
      {/if}

      <button
        onclick={() => { if (deviceName.trim()) autoJoinClient(); else error = 'Enter a device name.'; }}
        class="np-btn-primary mt-auto"
        disabled={submitting || loading || !deviceName.trim()}
      >
        {submitting ? 'Connecting…' : 'Connect'}
      </button>

    {:else}
      <!-- Admin / operator role: join nodepulse-core directly -->
      <div class="flex flex-col gap-1.5">
        <label class="text-xs text-np-muted font-medium" for="device-name">Device name</label>
        <input
          id="device-name"
          type="text"
          class="np-input"
          bind:value={deviceName}
          placeholder="my-home-pc"
          disabled={submitting}
          autocomplete="off"
          spellcheck="false"
        />
        <p class="text-xs text-np-subtle">Identifies this device in the mesh network</p>
      </div>

      <div class="flex items-center gap-3 rounded-lg px-3 py-2.5"
           style="background: var(--color-np-indigo-dim); border: 1px solid var(--color-np-indigo-ring);">
        <Server size={13} class="text-np-indigo-light shrink-0" />
        <div class="flex-1 min-w-0">
          <p class="text-sm text-np-text leading-tight">NodePulse Core</p>
          <p class="text-xs text-np-muted leading-tight">nodepulse-core namespace</p>
        </div>
        <span class="text-xs text-np-muted">{authStore.role}</span>
      </div>

      {#if error}
        <p class="text-xs text-np-red -mt-1">{error}</p>
      {/if}

      <button
        onclick={() => { if (deviceName.trim()) autoJoinAdmin(); else error = 'Enter a device name.'; }}
        class="np-btn-primary mt-auto"
        disabled={submitting || !deviceName.trim()}
      >
        {submitting ? 'Connecting…' : 'Connect to Core'}
      </button>
    {/if}
  </div>
</div>
