<script>
  import { invoke } from '@tauri-apps/api/core';
  import { open } from '@tauri-apps/plugin-shell';
  import { authStore } from '$lib/stores/authStore.svelte.js';
  import { connectionStore } from '$lib/stores/connectionStore.svelte.js';
  import { login } from '$lib/api/nodepulse.js';
  import { AlertTriangle, Download, RefreshCw } from 'lucide-svelte';

  let url = $state(authStore.config.nodepulse_url ?? '');
  let username = $state(authStore.config.username ?? '');
  let password = $state('');
  let error = $state('');
  let loading = $state(false);
  let tailscaleFound = $state(null);
  let checking = $state(true);

  const downloadLinks = [
    { label: 'Windows', url: 'https://tailscale.com/download/windows' },
    { label: 'macOS',   url: 'https://tailscale.com/download/mac' },
    { label: 'Linux',   url: 'https://tailscale.com/download/linux' },
  ];

  function detectPrimaryPlatform() {
    const p = navigator.platform?.toLowerCase() ?? '';
    if (p.startsWith('win')) return 'Windows';
    if (p.startsWith('mac')) return 'macOS';
    return 'Linux';
  }

  const primaryPlatform = detectPrimaryPlatform();

  async function checkTailscale() {
    checking = true;
    try {
      await invoke('detect_tailscale');
      tailscaleFound = true;
    } catch {
      tailscaleFound = false;
    } finally {
      checking = false;
    }
  }

  $effect(() => { checkTailscale(); });

  async function handleSubmit() {
    error = '';
    if (!url || !username || !password) {
      error = 'All fields are required.';
      return;
    }
    const normalizedUrl = url.replace(/\/$/, '');
    loading = true;
    connectionStore.transition('AUTHENTICATING');
    try {
      const result = await login(normalizedUrl, username, password);
      await authStore.setAuth(normalizedUrl, username, result.token, result.role ?? null, result.cluster_id ?? null);
      connectionStore.transition('CLUSTER_SELECT');
    } catch (e) {
      error = e.message || 'Login failed. Check URL and credentials.';
      connectionStore.transition('IDLE');
    } finally {
      loading = false;
    }
  }
</script>

<div class="flex flex-col h-full">
  <!-- Brand header -->
  <div class="px-6 pt-7 pb-5 flex items-center gap-3">
    <div class="w-8 h-8 rounded-lg flex items-center justify-center shrink-0"
         style="background: var(--color-np-indigo-dim); border: 1px solid var(--color-np-indigo-ring);">
      <span class="text-np-indigo-light font-bold text-xs">NP</span>
    </div>
    <div>
      <p class="text-np-text font-semibold text-sm leading-tight">NodePulse Connect</p>
      <p class="text-np-muted text-xs leading-tight">Join your infrastructure mesh</p>
    </div>
  </div>

  <!-- Divider -->
  <div class="mx-6 h-px" style="background: var(--color-np-border-dim);"></div>

  {#if tailscaleFound === false}
    <!-- Tailscale not found — download prompt -->
    <div class="flex flex-col gap-4 px-6 pt-5 pb-6 flex-1">
      <div class="flex items-start gap-2.5 rounded-lg p-3"
           style="background: var(--color-np-amber-dim); border: 1px solid color-mix(in srgb, var(--color-np-amber) 20%, transparent);">
        <AlertTriangle size={13} class="shrink-0 mt-0.5 text-np-amber" />
        <div class="min-w-0">
          <p class="text-xs font-medium text-np-amber leading-tight">Tailscale not detected</p>
          <p class="text-xs text-np-muted leading-relaxed mt-0.5">
            Tailscale must be installed before you can join the mesh network.
          </p>
        </div>
      </div>

      <div class="flex flex-col gap-2">
        <p class="text-xs text-np-muted font-medium">Download Tailscale</p>

        <!-- Primary platform — prominent -->
        {@const primary = downloadLinks.find(l => l.label === primaryPlatform)}
        {#if primary}
          <button
            onclick={() => open(primary.url)}
            class="np-btn-primary flex items-center justify-center gap-2"
          >
            <Download size={13} />
            Download for {primary.label}
          </button>
        {/if}

        <!-- Other platforms — subtle links -->
        <div class="flex gap-2 justify-center">
          {#each downloadLinks.filter(l => l.label !== primaryPlatform) as link}
            <button
              onclick={() => open(link.url)}
              class="text-xs text-np-muted hover:text-np-text underline underline-offset-2 transition-colors"
            >
              {link.label}
            </button>
          {/each}
        </div>
      </div>

      <button
        onclick={checkTailscale}
        disabled={checking}
        class="np-btn-ghost flex items-center justify-center gap-2 mt-auto"
      >
        <RefreshCw size={12} class={checking ? 'animate-spin' : ''} />
        {checking ? 'Checking…' : "I've installed Tailscale"}
      </button>

      <p class="text-center text-xs text-np-subtle">NodePulse IDP by Ussi</p>
    </div>

  {:else}
    <!-- Login form -->
    <form
      onsubmit={(e) => { e.preventDefault(); handleSubmit(); }}
      class="flex flex-col gap-4 px-6 pt-5 pb-6 flex-1"
    >
      <div class="flex flex-col gap-1.5">
        <label class="text-xs text-np-muted font-medium" for="url">NodePulse URL</label>
        <input
          id="url"
          type="url"
          class="np-input"
          bind:value={url}
          placeholder="http://192.168.1.100:18080"
          disabled={loading}
          autocomplete="off"
          spellcheck="false"
        />
      </div>

      <div class="flex flex-col gap-1.5">
        <label class="text-xs text-np-muted font-medium" for="username">Username</label>
        <input
          id="username"
          type="text"
          class="np-input"
          bind:value={username}
          placeholder="admin"
          disabled={loading}
          autocomplete="username"
        />
      </div>

      <div class="flex flex-col gap-1.5">
        <label class="text-xs text-np-muted font-medium" for="password">Password</label>
        <input
          id="password"
          type="password"
          class="np-input"
          bind:value={password}
          placeholder="••••••••"
          disabled={loading}
          autocomplete="current-password"
        />
      </div>

      {#if error}
        <p class="text-xs text-np-red -mt-1">{error}</p>
      {/if}

      <button
        type="submit"
        class="np-btn-primary mt-auto"
        disabled={loading}
      >
        {loading ? 'Authenticating…' : 'Sign In'}
      </button>

      <p class="text-center text-xs text-np-subtle">NodePulse IDP by Ussi</p>
    </form>
  {/if}
</div>
