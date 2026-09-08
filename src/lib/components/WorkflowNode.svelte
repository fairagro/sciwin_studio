<script lang="ts">
  import { Handle, Position, type Node as XYNode, type NodeProps } from "@xyflow/svelte";
  import { CircleCheck, LoaderCircle } from "@lucide/svelte";
  import type { FlowNodeData } from "$lib/graph/types";
  import { nodeHeaderClass, portArrayRing, portBg, portBorder, portGeometry, portRing } from "$lib/graph/styling";
  import { execution } from "$lib/state/execution.svelte";

  let { data: nodeData, selected }: NodeProps<XYNode<FlowNodeData, "workflow">> = $props();

  const isScattered = $derived(nodeData.scatter.length > 0);
  const isConditional = $derived(nodeData.when !== null);

  // Live run state, keyed by the bare CWL step id -- irrelevant for input/output nodes, which
  // never appear in a StepEvent.
  const liveStatus = $derived(nodeData.ref.kind === "step" ? execution.stepStatus[nodeData.ref.id] : undefined);

  // Selection ring, the scatter "stack of cards" effect, and the live-run glow are all plain
  // box-shadow layers so they combine on one element instead of fighting over a single
  // `shadow-*` class.
  const cardShadowClass = $derived.by(() => {
    const layers: string[] = [];
    if (selected) layers.push("0_0_0_1px_var(--color-fairagro-mid-500)");
    if (isScattered) layers.push("3px_3px_0_0_var(--color-border-soft)", "6px_6px_0_0_var(--color-bg)");
    if (liveStatus === "started") layers.push("0_0_0_2px_var(--color-fairagro-mid-500)");
    else if (liveStatus === "finished") layers.push("0_0_0_1.5px_var(--color-fairagro-mid-400)");
    layers.push(selected ? "0_10px_26px_rgba(106,191,92,0.28)" : "0_6px_18px_rgba(0,0,0,0.45)");
    return `shadow-[${layers.join(",")}]`;
  });

  // scatterMethod is only required by the CWL spec once >1 input is
  // scattered, but "dotproduct" is the de facto default assumed elsewhere
  // (e.g. by runners) when it's left unset -- show that instead of nothing.
  function scatterTitle(): string {
    return `Scattered · ${nodeData.scatterMethod ?? "dotproduct"}`;
  }
</script>

<div
  class="min-w-42.5 max-w-sm rounded-lg border bg-bg-surface text-text {cardShadowClass} {selected
    ? 'border-fairagro-mid-500'
    : 'border-border'} {isConditional ? 'border-dashed' : ''} {liveStatus === 'started' ? 'running-glow' : ''}"
>
  <div
    class="{nodeHeaderClass(
      nodeData.ref.kind
    )} flex h-7 items-center gap-1.5 overflow-hidden rounded-t-lg px-2.5 text-xs font-semibold text-white"
  >
    <span class="truncate">{nodeData.label}</span>
    {#if isConditional}
      <span class="shrink-0 font-mono text-[10px] font-normal opacity-80" title={nodeData.when ?? undefined}
        >◇ if</span
      >
    {/if}
    {#if isScattered}
      <span class="shrink-0 font-mono text-[10px] font-normal opacity-80" title={scatterTitle()}>⑃ scatter</span>
    {/if}
    <span class="ml-auto flex shrink-0 items-center gap-1.5">
      {#if liveStatus === "started"}
        <LoaderCircle size={11} strokeWidth={2.5} class="animate-spin text-fairagro-mid-300" />
      {:else if liveStatus === "finished"}
        <CircleCheck size={11} strokeWidth={2.5} class="text-fairagro-mid-300" />
      {/if}
      {#if nodeData.diagnostics.length > 0}
        <span class="text-amber-300" title={nodeData.diagnostics.map((d) => d.message).join("\n")}>⚠</span>
      {/if}
    </span>
  </div>

  <div class="py-1">
    {#each nodeData.outputs as port (port.id)}
      <div class="flex h-6.5 items-center justify-end gap-1.5 px-2">
        <span class="truncate text-[11px] text-text-2">{port.id}</span>
        <span class="shrink-0 text-[9px] text-text-3">{port.dataType}</span>
        <Handle type="source" position={Position.Right} id={port.id} class="static! -mr-1.5! h-2.5! w-2.5! shrink-0! transform-none! {portBg(port.dataType)} {portGeometry(port.dataType)} {portBorder(port.dataType)} {portRing} {portArrayRing(port.dataType)}" />
      </div>
    {/each}

    {#each nodeData.inputs as port (port.id)}
      {@const scattered = nodeData.scatter.includes(port.id)}
      <div class="flex h-6.5 items-center justify-start gap-1.5 px-2">
        <span class="-ml-1.5 inline-flex shrink-0 items-center justify-center rounded-full {scattered ? 'border border-dashed border-fairagro-mid-500 p-0.5' : ''}" title={scattered ? scatterTitle() : undefined}>
          <Handle type="target" position={Position.Left} id={port.id} class="static! h-2.5! w-2.5! shrink-0! transform-none! {portBg(port.dataType)} {portGeometry(port.dataType)} {portBorder(port.dataType)} {portRing} {portArrayRing(port.dataType)}" />
        </span>
        <span class="shrink-0 text-[9px] text-text-3">{port.dataType}</span>
        <span class="truncate text-[11px] text-text-2">{port.id}</span>
      </div>
    {/each}
  </div>
</div>

<style>
  /* Independent of cardShadowClass's box-shadow layers (a different CSS property), so it
     layers on top rather than fighting them for the same `shadow-*` composition. */
  @keyframes running-glow {
    0%,
    100% {
      filter: drop-shadow(0 0 2px var(--color-fairagro-mid-500));
    }
    50% {
      filter: drop-shadow(0 0 9px var(--color-fairagro-mid-500));
    }
  }
  .running-glow {
    animation: running-glow 1.4s ease-in-out infinite;
  }
</style>
