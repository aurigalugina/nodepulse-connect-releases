<script>
  import { authStore } from '$lib/stores/authStore.svelte.js';
  import { connectionStore } from '$lib/stores/connectionStore.svelte.js';
  import Setup from '$lib/components/Setup.svelte';
  import ClusterSelect from '$lib/components/ClusterSelect.svelte';
  import Connecting from '$lib/components/Connecting.svelte';
  import Connected from '$lib/components/Connected.svelte';
  import UpdateNotice from '$lib/components/UpdateNotice.svelte';

  const state = $derived(connectionStore.state);

  let updateDismissed = $state(false);

  $effect(() => {
    authStore.load().then(() => {
      connectionStore.tryAutoReconnect();
    });
  });
</script>

<div class="h-screen bg-np-bg text-np-text flex flex-col overflow-hidden">
  {#if state === 'IDLE' || state === 'AUTHENTICATING'}
    <Setup />
  {:else if state === 'CLUSTER_SELECT'}
    <ClusterSelect />
  {:else if state === 'GENERATING_KEY' || state === 'TAILSCALE_UP'}
    <Connecting />
  {:else if state === 'CONNECTED'}
    <Connected />
  {/if}

  {#if !updateDismissed}
    <UpdateNotice onDismiss={() => (updateDismissed = true)} />
  {/if}
</div>
