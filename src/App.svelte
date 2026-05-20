<script>
  import { invoke } from '@tauri-apps/api/core';
  import { authStore } from '$lib/stores/authStore.svelte.js';
  import { connectionStore } from '$lib/stores/connectionStore.svelte.js';
  import Setup from '$lib/components/Setup.svelte';
  import ClusterSelect from '$lib/components/ClusterSelect.svelte';
  import Connecting from '$lib/components/Connecting.svelte';
  import Connected from '$lib/components/Connected.svelte';
  import UpdateNotice from '$lib/components/UpdateNotice.svelte';
  import TailscaleSetup from '$lib/components/TailscaleSetup.svelte';

  const state = $derived(connectionStore.state);

  let updateDismissed = $state(false);
  let tailscaleReady = $state(true); // optimistic — set to false if binary absent

  $effect(() => {
    invoke('tailscale_is_ready').then((ready) => {
      tailscaleReady = ready;
    });
    authStore.load().then(() => {
      connectionStore.tryAutoReconnect();
    });
  });
</script>

<div class="h-screen bg-np-bg text-np-text flex flex-col overflow-hidden">
  {#if !tailscaleReady}
    <TailscaleSetup onReady={() => (tailscaleReady = true)} />
  {:else if state === 'IDLE' || state === 'AUTHENTICATING'}
    <Setup />
  {:else if state === 'CLUSTER_SELECT'}
    <ClusterSelect />
  {:else if state === 'GENERATING_KEY' || state === 'TAILSCALE_UP'}
    <Connecting />
  {:else if state === 'CONNECTED'}
    <Connected />
  {/if}

  {#if tailscaleReady && !updateDismissed}
    <UpdateNotice onDismiss={() => (updateDismissed = true)} />
  {/if}
</div>
