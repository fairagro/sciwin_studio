<script lang="ts">
  import { Handle, Position, type Node as XYNode, type NodeProps } from "@xyflow/svelte";
  import type { FlowNodeData } from "$lib/graph/types";
  import { nodeHeaderClass, portBg, portBorder, portGeometry } from "$lib/graph/styling";

  let { data: nodeData }: NodeProps<XYNode<FlowNodeData, "workflow">> = $props();
</script>

<div class="min-w-42.5 max-w-sm rounded-md border border-zinc-600 bg-zinc-800 text-white shadow-md">
  <div class="{nodeHeaderClass(nodeData.ref.kind)} flex items-center gap-1.5 overflow-hidden rounded-t-md px-2 py-1 text-xs font-medium">
    <span class="truncate">{nodeData.label}</span>
    {#if nodeData.diagnostics.length > 0}
      <span class="ml-auto shrink-0 text-amber-300" title={nodeData.diagnostics.map((d) => d.message).join("\n")}> ⚠ </span>
    {/if}
  </div>

  <div class="py-1">
    {#each nodeData.outputs as port (port.id)}
      <div class="flex items-center justify-end gap-1.5 px-2 py-0.5">
        <span class="truncate text-[11px]">{port.id}</span>
        <span class="shrink-0 text-[9px] text-zinc-400">{port.dataType}</span>
        <Handle type="source" position={Position.Right} id={port.id} class="static! h-2.5! w-2.5! shrink-0! transform-none! {portBg(port.dataType)} {portGeometry(port.dataType)} {portBorder(port.dataType)}" />
      </div>
    {/each}

    {#each nodeData.inputs as port (port.id)}
      <div class="flex items-center justify-start gap-1.5 px-2 py-0.5">
        <Handle type="target" position={Position.Left} id={port.id} class="static! h-2.5! w-2.5! shrink-0! transform-none! {portBg(port.dataType)} {portGeometry(port.dataType)} {portBorder(port.dataType)}" />
        <span class="shrink-0 text-[9px] text-zinc-400">{port.dataType}</span>
        <span class="truncate text-[11px]">{port.id}</span>
      </div>
    {/each}
  </div>
</div>
