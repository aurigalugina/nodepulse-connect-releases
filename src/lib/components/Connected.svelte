<script>
  import { authStore } from '$lib/stores/authStore.svelte.js';
  import { connectionStore } from '$lib/stores/connectionStore.svelte.js';
  import { getClusterNodes } from '$lib/api/nodepulse.js';
  import { Wifi, WifiOff, LogOut, ArrowLeftRight, RefreshCw } from 'lucide-svelte';
  import { getVersion } from '@tauri-apps/api/app';

  let appVersion = $state('');
  getVersion().then(v => appVersion = v).catch(() => {});

  /** @type {{ id: string, hostname: string, status: string }[]} */
  let nodes = $state([]);
  let nodesLoading = $state(false);

  const cluster = $derived(connectionStore.cluster);
  const meshIp = $derived(connectionStore.meshIp);
  const deviceName = $derived(connectionStore.deviceName);

  $effect(() => {
    if (cluster?.id) loadNodes(cluster.id);
  });

  async function loadNodes(clusterId) {
    nodesLoading = true;
    try {
      const res = await getClusterNodes(authStore.url, authStore.token, clusterId);
      nodes = res?.data ?? res ?? [];
    } catch {
      nodes = [];
    } finally {
      nodesLoading = false;
    }
  }

  const onlineNodes = $derived(nodes.filter(n => n.status === 'online').length);
</script>

<div class="flex flex-col h-full">
  <!-- Connected banner -->
  <div class="px-6 pt-6 pb-4">
    <div class="flex items-center gap-3 rounded-xl p-4"
         style="background: var(--color-np-green-dim); border: 1px solid color-mix(in srgb, var(--color-np-green) 20%, transparent);">
      <!-- Pulsing dot -->
      <div class="relative w-3 h-3 shrink-0">
        <div class="absolute inset-0 rounded-full bg-np-green animate-ping opacity-50"></div>
        <div class="w-3 h-3 rounded-full bg-np-green"></div>
      </div>
      <div class="flex-1 min-w-0">
        <p class="text-np-green font-semibold text-sm leading-tight">Connected</p>
        <p class="text-np-muted text-xs leading-tight truncate">{cluster?.name ?? '—'}</p>
      </div>
      <button onclick={() => loadNodes(cluster?.id)}
              class="text-np-subtle hover:text-np-muted transition-colors p-1 rounded">
        <RefreshCw size={13} />
      </button>
    </div>
  </div>

  <!-- Info grid -->
  <div class="mx-6 rounded-xl overflow-hidden" style="border: 1px solid var(--color-np-border);">
    <div class="grid grid-cols-[100px_1fr]">
      <div class="px-3 py-2.5 text-xs text-np-muted font-medium"
           style="background: var(--color-np-surface); border-right: 1px solid var(--color-np-border); border-bottom: 1px solid var(--color-np-border);">
        Device
      </div>
      <div class="px-3 py-2.5 text-xs font-mono text-np-text"
           style="background: var(--color-np-elevated); border-bottom: 1px solid var(--color-np-border);">
        {deviceName ?? '—'}
      </div>

      <div class="px-3 py-2.5 text-xs text-np-muted font-medium"
           style="background: var(--color-np-surface); border-right: 1px solid var(--color-np-border); border-bottom: 1px solid var(--color-np-border);">
        Mesh IP
      </div>
      <div class="px-3 py-2.5 text-xs font-mono"
           style="background: var(--color-np-elevated); border-bottom: 1px solid var(--color-np-border); color: var(--color-np-indigo-light);">
        {meshIp ?? 'Acquiring…'}
      </div>

      <div class="px-3 py-2.5 text-xs text-np-muted font-medium"
           style="background: var(--color-np-surface); border-right: 1px solid var(--color-np-border);">
        Server
      </div>
      <div class="px-3 py-2.5 text-xs font-mono text-np-muted truncate"
           style="background: var(--color-np-elevated);">
        {authStore.url}
      </div>
    </div>
  </div>

  <!-- Nodes -->
  <div class="flex-1 flex flex-col px-6 pt-4 gap-2 overflow-hidden">
    <div class="flex items-center justify-between">
      <p class="text-xs text-np-muted font-medium">Cluster nodes</p>
      {#if !nodesLoading && nodes.length > 0}
        <span class="text-xs text-np-subtle">{onlineNodes}/{nodes.length} online</span>
      {/if}
    </div>

    <div class="flex-1 overflow-y-auto flex flex-col gap-1">
      {#if nodesLoading}
        <p class="text-xs text-np-subtle py-1">Loading…</p>
      {:else if nodes.length === 0}
        <p class="text-xs text-np-subtle py-1">No nodes found in this cluster.</p>
      {:else}
        {#each nodes as node}
          <div class="flex items-center gap-2 py-1.5 px-2.5 rounded-lg text-xs"
               style="background: {node.status === 'online' ? 'var(--color-np-surface)' : 'transparent'};">
            {#if node.status === 'online'}
              <Wifi size={12} class="text-np-green shrink-0" />
              <span class="font-mono text-np-text">{node.hostname}</span>
            {:else}
              <WifiOff size={12} class="text-np-subtle shrink-0" />
              <span class="font-mono text-np-subtle">{node.hostname}</span>
            {/if}
          </div>
        {/each}
      {/if}
    </div>
  </div>

  <!-- Actions -->
  <div class="px-6 py-4 flex gap-2" style="border-top: 1px solid var(--color-np-border-dim);">
    <button onclick={() => connectionStore.disconnect()} class="np-btn-danger flex-1 flex items-center justify-center gap-1.5">
      <LogOut size={13} />
      Disconnect
    </button>
    <button onclick={() => connectionStore.transition('CLUSTER_SELECT')} class="np-btn-ghost flex items-center justify-center gap-1.5 px-4">
      <ArrowLeftRight size={13} />
      Switch
    </button>
  </div>

  <!-- Version footer -->
  {#if appVersion}
    <p class="text-center text-[10px] pb-3" style="color: var(--color-np-subtle);">
      v{appVersion}
    </p>
  {/if}
</div>
