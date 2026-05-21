<script>
  import { connectionStore } from '$lib/stores/connectionStore.svelte.js';
  import { authStore } from '$lib/stores/authStore.svelte.js';
  import { CheckCircle2, Circle, XCircle } from 'lucide-svelte';

  const steps = $derived(connectionStore.steps);
  const state = $derived(connectionStore.state);
  const error = $derived(connectionStore.error);
  const activeStep = $derived(steps.length > 0 ? steps.length - 1 : 0);
</script>

<div class="flex flex-col h-full px-6 py-7 gap-6">
  <!-- Header -->
  <div class="flex flex-col gap-1">
    <p class="text-np-text font-semibold text-sm">
      {state === 'TAILSCALE_UP' ? 'Joining mesh…' : 'Preparing connection…'}
    </p>
    <p class="text-np-muted text-xs">This usually takes 10–30 seconds</p>
  </div>

  <!-- Animated indicator -->
  <div class="flex justify-center py-2">
    {#if error}
      <div class="w-14 h-14 rounded-full flex items-center justify-center"
           style="background: var(--color-np-red-dim); border: 1px solid color-mix(in srgb, var(--color-np-red) 20%, transparent);">
        <XCircle size={24} class="text-np-red" />
      </div>
    {:else}
      <div class="relative w-14 h-14">
        <!-- Pulsing ring -->
        <div class="absolute inset-0 rounded-full animate-ping opacity-20"
             style="background: var(--color-np-indigo);"></div>
        <div class="absolute inset-1 rounded-full"
             style="background: var(--color-np-indigo-dim); border: 1px solid var(--color-np-indigo-ring);">
        </div>
        <!-- Spinner ring -->
        <svg class="absolute inset-0 animate-spin" viewBox="0 0 56 56" fill="none">
          <circle cx="28" cy="28" r="22" stroke="var(--color-np-indigo)" stroke-width="2"
                  stroke-dasharray="100" stroke-dashoffset="75" stroke-linecap="round"/>
        </svg>
        <!-- NP badge -->
        <div class="absolute inset-0 flex items-center justify-center">
          <span class="text-np-indigo-light font-bold text-xs">NP</span>
        </div>
      </div>
    {/if}
  </div>

  <!-- Step list -->
  <div class="flex flex-col gap-2.5 flex-1">
    {#each steps as step, i}
      <div class="flex items-center gap-2.5">
        {#if i < activeStep || (i === activeStep && !error && state === 'CONNECTED')}
          <CheckCircle2 size={15} class="shrink-0 text-np-green" />
          <span class="text-sm text-np-muted">{step}</span>
        {:else if i === activeStep && error}
          <XCircle size={15} class="shrink-0 text-np-red" />
          <span class="text-sm text-np-red">{step}</span>
        {:else if i === activeStep}
          <div class="w-[15px] h-[15px] shrink-0 rounded-full border-2 border-np-indigo border-t-transparent animate-spin"></div>
          <span class="text-sm text-np-text">{step}</span>
        {:else}
          <Circle size={15} class="shrink-0 text-np-subtle" />
          <span class="text-sm text-np-subtle">{step}</span>
        {/if}
      </div>
    {/each}

    {#if steps.length === 0}
      <div class="flex items-center gap-2.5 text-sm text-np-muted">
        <div class="w-[15px] h-[15px] shrink-0 rounded-full border-2 border-np-indigo border-t-transparent animate-spin"></div>
        Starting…
      </div>
    {/if}
  </div>

  <!-- Error + actions -->
  {#if error}
    <div class="flex flex-col gap-2">
      <div class="rounded-lg p-3 text-xs text-np-red leading-relaxed"
           style="background: var(--color-np-red-dim); border: 1px solid color-mix(in srgb, var(--color-np-red) 20%, transparent);">
        {error}
      </div>
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
    </div>
  {/if}
</div>
