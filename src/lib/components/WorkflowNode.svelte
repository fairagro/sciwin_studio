<script lang="ts">
  import { Handle, Position, type Node as XYNode, type NodeProps } from "@xyflow/svelte";
  import type { FlowNodeData } from "$lib/graph/types";
  import { nodeHeaderClass, portBg, portBorder, portGeometry, portRing } from "$lib/graph/styling";

  let { data: nodeData, selected }: NodeProps<XYNode<FlowNodeData, "workflow">> = $props();

  const isScattered = $derived(nodeData.scatter.length > 0);
  const isConditional = $derived(nodeData.when !== null);

  // Selection ring and the scatter "stack of cards" effect are both plain
  // box-shadow layers so they combine on one element instead of fighting
  // over a single `shadow-*` class.
  const cardShadowClass = $derived.by(() => {
    const layers: string[] = [];
    if (selected) layers.push("0_0_0_1px_var(--color-fairagro-mid-500)");
    if (isScattered) layers.push("3px_3px_0_0_var(--color-border-soft)", "6px_6px_0_0_var(--color-bg)");
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
    : 'border-border'} {isConditional ? 'border-dashed' : ''}"
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
    {#if nodeData.diagnostics.length > 0}
      <span
        class="ml-auto shrink-0 text-amber-300"
        title={nodeData.diagnostics.map((d) => d.message).join("\n")}
      >
        ⚠
      </span>
    {/if}
  </div>

  <div class="py-1">
    {#each nodeData.outputs as port (port.id)}
      <div class="flex h-6.5 items-center justify-end gap-1.5 px-2">
        <span class="truncate text-[11px] text-text-2">{port.id}</span>
        <span class="shrink-0 text-[9px] text-text-3">{port.dataType}</span>
        <Handle type="source" position={Position.Right} id={port.id} class="static! -mr-1.5! h-2.5! w-2.5! shrink-0! transform-none! {portBg(port.dataType)} {portGeometry(port.dataType)} {portBorder(port.dataType)} {portRing}" />
      </div>
    {/each}

    {#each nodeData.inputs as port (port.id)}
      {@const scattered = nodeData.scatter.includes(port.id)}
      <div class="flex h-6.5 items-center justify-start gap-1.5 px-2">
        <span class="-ml-1.5 inline-flex shrink-0 items-center justify-center rounded-full {scattered ? 'border border-dashed border-fairagro-mid-500 p-0.5' : ''}" title={scattered ? scatterTitle() : undefined}>
          <Handle type="target" position={Position.Left} id={port.id} class="static! h-2.5! w-2.5! shrink-0! transform-none! {portBg(port.dataType)} {portGeometry(port.dataType)} {portBorder(port.dataType)} {portRing}" />
        </span>
        <span class="shrink-0 text-[9px] text-text-3">{port.dataType}</span>
        <span class="truncate text-[11px] text-text-2">{port.id}</span>
      </div>
    {/each}
  </div>
</div>
