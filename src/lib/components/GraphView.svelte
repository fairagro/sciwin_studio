<script lang="ts">
  import { SvelteFlow, Background, Controls, MiniMap, type Node, type Edge, type NodeTypes } from "@xyflow/svelte";
  import "@xyflow/svelte/dist/style.css";
  import { invoke } from "@tauri-apps/api/core";
  import { workspace } from "$lib/state/workspace.svelte";
  import { toSvelteFlow } from "$lib/graph/transform";
  import type { WorkflowView } from "$lib/graph/types";
  import WorkflowNode from "./WorkflowNode.svelte";

  const nodeTypes: NodeTypes = { workflow: WorkflowNode };

  let nodes = $state.raw<Node[]>([]);
  let edges = $state.raw<Edge[]>([]);
  let loadError = $state<string | null>(null);

  async function loadGraph(path: string) {
    try {
      const view = await invoke<WorkflowView>("get_workflow_graph", { path });
      ({ nodes, edges } = toSvelteFlow(view));
      loadError = null;
    } catch (error) {
      console.error("Failed to load workflow graph:", error);
      nodes = [];
      edges = [];
      loadError = String(error);
    }
  }

  // Re-fetches on tab switch and on the code/graph view-mode toggle (both
  // change what `workspace.activeTab` reads). Not yet re-fetched on save --
  // that needs the `workflow-changed` event Phase 5 introduces alongside
  // write commands; until then the graph can go stale under an open tab.
  $effect(() => {
    const tab = workspace.activeTab;
    if (!tab || tab.viewMode !== "graph") {
      nodes = [];
      edges = [];
      loadError = null;
      return;
    }
    loadGraph(tab.path);
  });
</script>

<div class="relative h-full w-full">
  <SvelteFlow bind:nodes bind:edges {nodeTypes} nodesConnectable={false} fitView colorMode="dark">
    <Background bgColor="var(--color-bg)" patternColor="var(--color-border-soft)" gap={22} size={1} />
    <Controls />
    <MiniMap bgColor="var(--color-bg-panel)" maskColor="rgba(18, 19, 22, 0.65)" />
  </SvelteFlow>
  {#if loadError}
    <div
      class="absolute top-2 left-2 rounded border border-fairagro-red bg-bg-panel px-3 py-1.5 font-mono text-xs text-fairagro-red-light"
    >
      Failed to load graph: {loadError}
    </div>
  {/if}
</div>
