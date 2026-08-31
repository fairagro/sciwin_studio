<script lang="ts">
  import { X } from "@lucide/svelte";
  import { workspace } from "$lib/state/workspace.svelte";
  import { nodeHeaderClass } from "$lib/graph/styling";
  import type { NodeKind } from "$lib/graph/types";
  import { Switch } from "bits-ui";

  const data = $derived(workspace.selectedNodeData);

  const KIND_LABEL: Record<NodeKind, string> = {
    step: "step",
    input: "workflow input",
    output: "workflow output",
  };
</script>

<aside class="flex shrink-0 flex-col overflow-y-auto border-l border-border bg-bg-panel select-none" style="width: {workspace.inspectorWidth}px">
  <div class="flex items-start justify-between gap-2 border-b border-border-soft px-3.5 py-3">
    <div class="min-w-0 flex-1">
      <span class="font-mono text-[10px] tracking-widest text-text-3 uppercase">Inspector</span>
      {#if data}
        <div class="mt-2 flex items-center gap-2">
          <span class="h-2 w-2 shrink-0 rounded-sm {nodeHeaderClass(data.ref.kind)}"></span>
          <span class="truncate font-mono text-[13px] font-semibold text-text" title={data.label}>{data.label}</span>
        </div>
        <div class="mt-1 font-mono text-[10.5px] text-text-3">{KIND_LABEL[data.ref.kind]}</div>
      {/if}
    </div>
    <button type="button" class="shrink-0 rounded p-0.5 text-text-3 hover:bg-fairagro-red-light/20 hover:text-fairagro-red-light" title="Close inspector" onclick={() => workspace.closeInspector()}>
      <X size={13} strokeWidth={1.8} />
    </button>
  </div>

  {#if data}
    <div class="border-b border-border-soft px-3.5 py-3.5">
      <div class="mb-2 font-mono text-[10px] tracking-widest text-text-3 uppercase">General</div>
      <div class="flex flex-col gap-2">
        <div class="rounded-md border border-border-soft bg-bg-well px-2.5 py-2 font-mono text-xs text-text">{data.ref.id}</div>
        {#if data.run}
          <div class="rounded-md border border-border-soft bg-bg-well px-2.5 py-2 font-mono text-[11px] text-text-2">
            {data.run.kind === "file" ? data.run.path : "inline run"}
          </div>
        {/if}
      </div>
    </div>

    {#if data.when !== null}
      <div class="border-b border-border-soft px-3.5 py-3.5">
        <div class="mb-2 font-mono text-[10px] tracking-widest text-text-3 uppercase">Conditional &middot; when</div>
        <div class="rounded-md border border-border-soft bg-bg-well px-2.5 py-2 font-mono text-[11.5px] text-text">{data.when}</div>
      </div>
    {/if}

    {#if data.scatter.length > 0}
      <div class="border-b border-border-soft px-3.5 py-3.5">
        <div class="mb-2.5 font-mono text-[10px] tracking-widest text-text-3 uppercase">Scatter</div>
        <div class="mb-2.5 flex flex-wrap gap-1.5">
          {#each data.scatter as port (port)}
            <span class="rounded-md border border-green-hdr bg-fairagro-mid-500/16 px-2 py-1 font-mono text-[11px] text-fairagro-mid-500">{port}</span>
          {/each}
        </div>
        <div class="mb-1.5 font-mono text-[10px] tracking-widest text-text-3 uppercase">Scatter method</div>
        <div class="rounded-md border border-border-soft bg-bg-well px-2.5 py-2 font-mono text-xs text-text">{data.scatterMethod ?? "dotproduct"}</div>
      </div>
    {/if}

    {#if data.diagnostics.length > 0}
      <div class="px-3.5 py-3.5">
        <div class="mb-2 font-mono text-[10px] tracking-widest text-fairagro-red-light uppercase">Diagnostics</div>
        <div class="flex flex-col gap-1.5">
          {#each data.diagnostics as d, i (i)}
            <div class="rounded-md border border-fairagro-red/40 bg-fairagro-red/10 px-2.5 py-2 text-[11.5px] text-fairagro-red-light">{d.message}</div>
          {/each}
        </div>
      </div>
    {/if}
  {/if}
</aside>
