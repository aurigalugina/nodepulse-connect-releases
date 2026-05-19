<script>
  import { onMount } from 'svelte';

  let { onDismiss = () => {} } = $props();

  let update     = $state(null);
  let installing = $state(false);
  let progress   = $state(0);

  const isTauri = () => typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;

  onMount(() => {
    if (!isTauri()) return;
    const t = setTimeout(async () => {
      try {
        const { check } = await import(/* @vite-ignore */ '@tauri-apps/plugin-updater');
        const result = await check();
        if (result?.available) update = result;
      } catch {
        // Network unavailable or no update endpoint — silent
      }
    }, 3000);
    return () => clearTimeout(t);
  });

  async function install() {
    if (!update) return;
    installing = true;
    try {
      const { relaunch } = await import(/* @vite-ignore */ '@tauri-apps/plugin-process');
      let downloaded = 0;
      let total = 0;
      await update.downloadAndInstall((event) => {
        if (event.event === 'Started') {
          total = event.data.contentLength ?? 0;
        } else if (event.event === 'Progress') {
          downloaded += event.data.chunkLength;
          progress = total > 0 ? Math.round((downloaded / total) * 100) : 0;
        } else if (event.event === 'Finished') {
          progress = 100;
        }
      });
      await relaunch();
    } catch {
      installing = false;
    }
  }
</script>

{#if update}
  <div class="fixed bottom-0 left-0 right-0 z-50 px-3 pb-3">
    <div class="rounded-xl border border-np-border bg-np-surface p-3 shadow-lg">
      <div class="flex items-start gap-2.5">
        <div class="mt-0.5 h-2 w-2 flex-shrink-0 rounded-full bg-np-indigo animate-pulse"></div>
        <div class="flex-1 min-w-0">
          <p class="text-xs font-medium text-np-text">
            Update v{update.version} available
          </p>
          {#if update.body}
            <p class="mt-0.5 text-[10px] text-np-muted truncate">{update.body}</p>
          {/if}

          {#if installing}
            <div class="mt-2 space-y-1">
              <div class="h-1 w-full rounded-full bg-np-elevated overflow-hidden">
                <div
                  class="h-full rounded-full bg-np-indigo transition-all duration-200"
                  style="width: {progress}%"
                ></div>
              </div>
              <p class="text-[10px] text-np-muted">
                {progress < 100 ? `Downloading… ${progress}%` : 'Installing…'}
              </p>
            </div>
          {:else}
            <div class="mt-2 flex gap-2">
              <button onclick={install} class="np-btn-primary text-[11px] px-2.5 py-1">
                Update Now
              </button>
              <button onclick={onDismiss} class="np-btn-ghost text-[11px] px-2.5 py-1">
                Later
              </button>
            </div>
          {/if}
        </div>
      </div>
    </div>
  </div>
{/if}
