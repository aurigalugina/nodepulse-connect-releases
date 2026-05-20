<script>
  import { invoke } from '@tauri-apps/api/core';
  import { listen } from '@tauri-apps/api/event';
  import { onMount } from 'svelte';
  import { getVersion } from '@tauri-apps/api/app';

  let { onReady } = $props();

  let step = $state('downloading');   // 'downloading' | 'extracting' | 'done' | 'error'
  let progress = $state(0);
  let errorMsg = $state('');
  let appVersion = $state('');

  getVersion().then(v => appVersion = v).catch(() => {});

  const stepLabel = $derived(
    step === 'downloading' ? 'Downloading network components…' :
    step === 'extracting'  ? 'Extracting…' :
    step === 'done'        ? 'Ready!' :
    'Setup failed'
  );

  onMount(async () => {
    const unlisten = await listen('tailscale-setup', (e) => {
      const payload = e.payload;
      step = payload.step;
      progress = payload.progress ?? 0;
    });

    try {
      await invoke('ensure_tailscale');
      // done event already emitted from Rust; wait a moment then hand off
      await new Promise(r => setTimeout(r, 600));
      onReady();
    } catch (e) {
      errorMsg = typeof e === 'string' ? e : 'Setup failed. Check your internet connection and try again.';
      step = 'error';
    }

    return () => unlisten();
  });

  async function retry() {
    errorMsg = '';
    step = 'downloading';
    progress = 0;
    try {
      await invoke('ensure_tailscale');
      await new Promise(r => setTimeout(r, 600));
      onReady();
    } catch (e) {
      errorMsg = typeof e === 'string' ? e : 'Setup failed.';
      step = 'error';
    }
  }
</script>

<div class="flex flex-col h-full px-6 pt-7 pb-6">
  <!-- Brand header -->
  <div class="flex items-center gap-3 mb-5">
    <div class="w-8 h-8 rounded-lg flex items-center justify-center shrink-0"
         style="background: var(--color-np-indigo-dim); border: 1px solid var(--color-np-indigo-ring);">
      <span class="text-np-indigo-light font-bold text-xs">NP</span>
    </div>
    <div>
      <p class="text-np-text font-semibold text-sm leading-tight">NodePulse Connect</p>
      <p class="text-np-muted text-xs leading-tight">First-time setup</p>
    </div>
  </div>

  <div class="mx-0 h-px mb-5" style="background: var(--color-np-border-dim);"></div>

  <div class="flex flex-col flex-1 justify-center gap-5">
    {#if step === 'error'}
      <!-- Error state -->
      <div class="rounded-lg p-3.5 flex items-start gap-2.5"
           style="background: var(--color-np-red-dim); border: 1px solid color-mix(in srgb, var(--color-np-red) 20%, transparent);">
        <svg class="w-4 h-4 shrink-0 mt-0.5 text-np-red" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
          <circle cx="12" cy="12" r="10"/><line x1="12" y1="8" x2="12" y2="12"/><line x1="12" y1="16" x2="12.01" y2="16"/>
        </svg>
        <div class="min-w-0">
          <p class="text-xs font-medium text-np-red leading-tight">Setup failed</p>
          <p class="text-xs text-np-muted leading-relaxed mt-0.5 break-all">{errorMsg}</p>
        </div>
      </div>

      <button onclick={retry} class="np-btn-primary">
        Try Again
      </button>

    {:else}
      <!-- Progress state -->
      <div class="flex flex-col gap-3">
        <div class="flex items-center gap-3">
          {#if step === 'done'}
            <div class="w-8 h-8 rounded-full flex items-center justify-center shrink-0"
                 style="background: color-mix(in srgb, var(--color-np-green) 15%, transparent);">
              <svg class="w-4 h-4 text-np-green" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5">
                <polyline points="20 6 9 17 4 12"/>
              </svg>
            </div>
          {:else}
            <!-- Spinner -->
            <div class="w-8 h-8 rounded-full border-2 shrink-0 animate-spin"
                 style="border-color: var(--color-np-border); border-top-color: var(--color-np-indigo);"></div>
          {/if}
          <div>
            <p class="text-sm font-medium text-np-text">{stepLabel}</p>
            <p class="text-xs text-np-muted mt-0.5">This only happens once</p>
          </div>
        </div>

        <!-- Progress bar -->
        <div class="h-1 rounded-full overflow-hidden" style="background: var(--color-np-border);">
          <div class="h-full rounded-full transition-all duration-300"
               style="width: {progress}%; background: {step === 'done' ? 'var(--color-np-green)' : 'var(--color-np-indigo)'};">
          </div>
        </div>

        <div class="flex flex-col gap-1.5 mt-1">
          {#each [
            ['downloading', 'Download Tailscale binary (~25 MB)'],
            ['extracting',  'Extract and verify'],
            ['done',        'Start network daemon'],
          ] as [s, label]}
            {@const done = progress >= (s === 'downloading' ? 75 : s === 'extracting' ? 99 : 100)}
            {@const active = step === s}
            <div class="flex items-center gap-2 text-xs"
                 style="color: {done ? 'var(--color-np-green)' : active ? 'var(--color-np-text)' : 'var(--color-np-muted)'}">
              {#if done}
                <svg class="w-3 h-3 shrink-0" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="3">
                  <polyline points="20 6 9 17 4 12"/>
                </svg>
              {:else if active}
                <div class="w-3 h-3 shrink-0 rounded-full border-2 animate-spin"
                     style="border-color: var(--color-np-border); border-top-color: var(--color-np-indigo);"></div>
              {:else}
                <div class="w-3 h-3 shrink-0 rounded-full border border-current opacity-30"></div>
              {/if}
              {label}
            </div>
          {/each}
        </div>
      </div>
    {/if}
  </div>

  <p class="text-center text-xs text-np-subtle mt-auto pt-4">
    NodePulse IDP by Ussi{#if appVersion} · v{appVersion}{/if}
  </p>
</div>
