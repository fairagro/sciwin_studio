<script lang="ts">
  import { Play, Square, Loader2, TriangleAlert } from "@lucide/svelte";
  import { execution } from "$lib/state/execution.svelte";
  import type { Tab } from "$lib/state/workspace.svelte";

  let { tab }: { tab: Tab } = $props();

  // Another tab's run shouldn't make this one look busy or show its status.
  const isThisFile = $derived(execution.cwlfile === tab.path);
  const running = $derived(isThisFile && execution.isRunning);

  const STATUS_LABEL: Record<string, string> = {
    created: "Created…",
    queued: "Queued…",
    running: "Running…",
    finished: "Finished",
    failed: "Failed",
    cancelled: "Cancelled",
    stopped: "Stopped",
  };

  function handleClick() {
    if (running) {
      execution.cancel();
    } else {
      execution.run(tab.path);
    }
  }
</script>

<div class="flex items-center gap-2">
  {#if isThisFile && execution.error}
    <span class="flex items-center gap-1 font-mono text-[11px] text-fairagro-red-light" title={execution.error}>
      <TriangleAlert size={12} strokeWidth={2} />
      Failed to start
    </span>
  {:else if isThisFile && execution.status}
    <span class="font-mono text-[11px] text-text-2">{STATUS_LABEL[execution.status]}</span>
  {/if}

  <button
    type="button"
    onclick={handleClick}
    class="flex items-center gap-1.5 rounded-full px-3 py-1 font-mono text-xs font-semibold shadow-[0_0_0_1px_rgba(0,0,0,0.25)] transition-colors {running
      ? 'bg-fairagro-red text-white hover:bg-fairagro-red/85'
      : 'bg-fairagro-mid-500 text-bg-well hover:bg-fairagro-mid-400'}"
  >
    {#if running}
      <Square size={11} strokeWidth={2.5} fill="currentColor" />
      Stop
    {:else if isThisFile && execution.status === "queued"}
      <Loader2 size={12} strokeWidth={2.5} class="animate-spin" />
      Run
    {:else}
      <Play size={11} strokeWidth={2.5} fill="currentColor" />
      Run
    {/if}
  </button>
</div>
