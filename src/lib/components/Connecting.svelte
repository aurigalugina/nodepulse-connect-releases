<script>
  import { connectionStore } from '$lib/stores/connectionStore.svelte.js';
  import { authStore } from '$lib/stores/authStore.svelte.js';
  import { XCircle, Copy, Check } from 'lucide-svelte';

  const state = $derived(connectionStore.state);
  const error = $derived(connectionStore.error);
  const debugLog = $derived(connectionStore.debugLog);

  let copied = $state(false);
  async function copyLog() {
    const full = debugLog.map(e => `${e.time} ${e.msg}`).join('\n')
      + (error ? '\n\nERROR:\n' + error : '');
    await navigator.clipboard.writeText(full);
    copied = true;
    setTimeout(() => (copied = false), 2000);
  }

  // Auto-scroll log to bottom when new entries arrive
  let logEl = $state(null);
  $effect(() => {
    debugLog.length; // track length as dependency
    if (logEl) logEl.scrollTop = logEl.scrollHeight;
  });
</script>

<div class="flex flex-col h-full px-6 py-5 gap-4">
  <!-- Header -->
  <div class="flex items-center gap-3">
    {#if error}
      <div class="w-8 h-8 rounded-full flex items-center justify-center shrink-0"
           style="background: var(--color-np-red-dim); border: 1px solid color-mix(in srgb, var(--color-np-red) 20%, transparent);">
        <XCircle size={16} class="text-np-red" />
      </div>
    {:else}
      <div class="relative w-8 h-8 shrink-0">
        <div class="absolute inset-0 rounded-full animate-ping opacity-20"
             style="background: var(--color-np-indigo);"></div>
        <div class="absolute inset-0.5 rounded-full"
             style="background: var(--color-np-indigo-dim); border: 1px solid var(--color-np-indigo-ring);">
        </div>
        <svg class="absolute inset-0 animate-spin" viewBox="0 0 32 32" fill="none">
          <circle cx="16" cy="16" r="12" stroke="var(--color-np-indigo)" stroke-width="1.5"
                  stroke-dasharray="60" stroke-dashoffset="45" stroke-linecap="round"/>
        </svg>
        <div class="absolute inset-0 flex items-center justify-center">
          <span class="text-np-indigo-light font-bold" style="font-size: 7px;">NP</span>
        </div>
      </div>
    {/if}
    <div class="min-w-0">
      <p class="text-np-text font-semibold text-sm leading-tight">
        {state === 'TAILSCALE_UP' ? 'Joining mesh…' : error ? 'Connection failed' : 'Preparing…'}
      </p>
      <p class="text-np-muted text-xs leading-tight truncate">
        {error ? 'See log below' : 'This usually takes 10–30 seconds'}
      </p>
    </div>
    <button
      onclick={copyLog}
      title="Copy full log"
      class="ml-auto p-1.5 rounded opacity-50 hover:opacity-100 transition-opacity shrink-0"
      style="color: var(--color-np-muted);"
    >
      {#if copied}
        <Check size={13} />
      {:else}
        <Copy size={13} />
      {/if}
    </button>
  </div>

  <!-- Debug log — always visible, fills remaining space -->
  <div
    bind:this={logEl}
    class="flex-1 overflow-y-auto rounded-lg p-2.5 font-mono text-xs leading-5 break-all"
    style="background: var(--color-np-bg); border: 1px solid var(--color-np-border);"
  >
    {#if debugLog.length === 0}
      <span class="text-np-subtle">Waiting for connection…</span>
    {:else}
      {#each debugLog as entry}
        <div class={entry.msg.startsWith('ERROR') ? 'text-np-red' : entry.msg.startsWith('[rust]') ? 'text-np-indigo-light' : 'text-np-muted'}>
          <span class="text-np-subtle select-none">{entry.time} </span>{entry.msg}
        </div>
      {/each}
    {/if}
    {#if !error && debugLog.length > 0}
      <span class="text-np-indigo animate-pulse">▌</span>
    {/if}
  </div>

  <!-- Error message (concise, below log) -->
  {#if error}
    <div class="rounded-lg px-3 py-2 text-xs text-np-red leading-relaxed"
         style="background: var(--color-np-red-dim); border: 1px solid color-mix(in srgb, var(--color-np-red) 20%, transparent);">
      {error.split('\n\nDaemon log:')[0].trim()}
    </div>
  {/if}

  <!-- Actions -->
  {#if error}
    <div class="flex gap-2">
      <button
        onclick={() => { authStore.clearAuth(); connectionStore.transition('IDLE'); }}
        class="np-btn-ghost flex-1 text-xs"
      >
        Sign out
      </button>
      <button
        onclick={() => connectionStore.transition('CLUSTER_SELECT')}
        class="np-btn-primary flex-1 text-xs"
      >
        Try again
      </button>
    </div>
  {/if}
</div>
