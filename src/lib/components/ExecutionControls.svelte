<script lang="ts">
  import { onMount } from "svelte";
  import { Select } from "bits-ui";
  import { Play, Square, LoaderCircle, TriangleAlert, FileInput, X, ChevronDown, Check } from "@lucide/svelte";
  import { open } from "@tauri-apps/plugin-dialog";
  import { execution, backendKey } from "$lib/state/execution.svelte";
  import type { Tab } from "$lib/state/workspace.svelte";

  let { tab }: { tab: Tab } = $props();

  onMount(() => {
    execution.refreshBackends();
  });

  // Another tab's run shouldn't make this one look busy or show its status.
  const isThisFile = $derived(execution.cwlfile === tab.path);
  const running = $derived(isThisFile && execution.isRunning);
  const jobFile = $derived(execution.jobFileFor(tab.path));
  const jobFileName = $derived(jobFile?.split(/[\\/]/).pop() ?? null);

  const selectedBackend = $derived(execution.backendFor(tab.path));
  const selectedBackendKey = $derived(backendKey(selectedBackend));
  const selectedBackendSummary = $derived(execution.backends.find((b) => backendKey(b.id) === selectedBackendKey));

  function handleBackendChange(key: string) {
    const found = execution.backends.find((b) => backendKey(b.id) === key);
    if (found) execution.setBackend(tab.path, found.id);
  }

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

  async function pickJobFile() {
    const dir = tab.path.slice(0, Math.max(0, tab.path.replace(/\\/g, "/").lastIndexOf("/")));
    const selected = await open({
      multiple: false,
      defaultPath: dir || undefined,
      filters: [{ name: "Job file", extensions: ["yml", "yaml", "json"] }],
    });
    if (typeof selected === "string") {
      execution.setJobFile(tab.path, selected);
    }
  }
</script>

<div class="flex items-center gap-2">
  {#if isThisFile && execution.error}
    <span
      class="flex max-w-70 items-center gap-1 font-mono text-[11px] text-fairagro-red-light"
      title={execution.error}
    >
      <TriangleAlert size={12} strokeWidth={2} class="shrink-0" />
      <span class="truncate">{execution.error}</span>
    </span>
  {:else if isThisFile && execution.status}
    <span class="font-mono text-[11px] text-text-2">{STATUS_LABEL[execution.status]}</span>
  {/if}

  <Select.Root
    type="single"
    value={selectedBackendKey}
    onValueChange={handleBackendChange}
    onOpenChange={(isOpen) => {
      if (isOpen) execution.refreshBackends();
    }}
  >
    <Select.Trigger
      disabled={running}
      title={selectedBackendSummary?.message ?? undefined}
      class="flex items-center gap-1.5 rounded-md border px-2 py-1 font-mono text-[11px] disabled:opacity-50 {selectedBackendSummary?.available ===
      false
        ? 'border-amber-500/40 text-amber-400'
        : 'border-border text-text-2 hover:bg-border-soft hover:text-text'}"
    >
      {#if selectedBackendSummary?.available === false}
        <TriangleAlert size={11} strokeWidth={1.8} />
      {/if}
      <span>{selectedBackendSummary?.name ?? "Local"}</span>
      <ChevronDown size={11} strokeWidth={2} class="text-text-3" />
    </Select.Trigger>
    <Select.Portal>
      <Select.Content class="z-50 min-w-40 rounded-md border border-border bg-bg-surface p-1 shadow-lg">
        <Select.Viewport>
          {#each execution.backends as backend (backendKey(backend.id))}
            <Select.Item
              value={backendKey(backend.id)}
              label={backend.name}
              disabled={!backend.available}
              title={backend.message ?? undefined}
              class="flex cursor-pointer items-center gap-2 rounded px-2 py-1.5 font-mono text-xs outline-none hover:bg-border-soft data-disabled:cursor-not-allowed data-disabled:opacity-40 data-highlighted:bg-border-soft {backend.available
                ? 'text-text-2'
                : 'text-text-3'}"
            >
              <span class="flex-1">{backend.name}</span>
              {#if backendKey(backend.id) === selectedBackendKey}
                <Check size={12} strokeWidth={2} />
              {/if}
            </Select.Item>
          {/each}
        </Select.Viewport>
      </Select.Content>
    </Select.Portal>
  </Select.Root>

  <div class="flex max-w-40 items-center gap-1 rounded-md border border-border bg-bg-surface pr-1 text-[11px] text-text-2">
    <button
      type="button"
      onclick={pickJobFile}
      disabled={running}
      title={jobFile ?? "Run with an empty job -- click to pick a job file (YAML/JSON)"}
      class="flex min-w-0 flex-1 items-center gap-1.5 px-2 py-1 font-mono hover:text-text disabled:opacity-50"
    >
      <FileInput size={12} strokeWidth={1.8} class="shrink-0" />
      <span class="min-w-0 truncate">{jobFileName ?? "No job file"}</span>
    </button>
    {#if jobFile && !running}
      <button
        type="button"
        title="Clear job file"
        class="shrink-0 rounded p-0.5 hover:bg-border hover:text-text"
        onclick={() => execution.setJobFile(tab.path, null)}
      >
        <X size={10} strokeWidth={2} />
      </button>
    {/if}
  </div>

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
      <LoaderCircle size={12} strokeWidth={2.5} class="animate-spin" />
      Run
    {:else}
      <Play size={11} strokeWidth={2.5} fill="currentColor" />
      Run
    {/if}
  </button>
</div>
