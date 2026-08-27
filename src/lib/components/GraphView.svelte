<script lang="ts">
  import { SvelteFlow, Background, Controls, MiniMap, type Node, type Edge, type NodeTypes } from "@xyflow/svelte";
  import "@xyflow/svelte/dist/style.css";
  import { invoke } from "@tauri-apps/api/core";
  import { listen } from "@tauri-apps/api/event";
  import { onMount } from "svelte";
  import { workspace } from "$lib/state/workspace.svelte";
  import { toSvelteFlow } from "$lib/graph/transform";
  import type { WorkflowChanged, WorkflowView } from "$lib/graph/types";
  import WorkflowNode from "./WorkflowNode.svelte";

  const nodeTypes: NodeTypes = { workflow: WorkflowNode };

  let nodes = $state.raw<Node[]>([]);
  let edges = $state.raw<Edge[]>([]);
  let loadError = $state<string | null>(null);
  // Not consumed yet -- lands with the first mutation command, which passes
  // this back so the backend can refuse a write against a stale file instead
  // of silently clobbering it.
  let revision = $state<string | null>(null);

  // Read-only for now (Phase 5 mutations aren't wired up), but the buffer
  // being dirty is still worth surfacing: the graph only ever reflects what's
  // on disk, so while Monaco holds unsaved edits for this file, the canvas is
  // showing the last save, not what's in the editor.
  const isEditorDirty = $derived(
    workspace.activeTab?.viewMode === "graph" && workspace.activeTab?.dirty === true
  );

  async function loadGraph(path: string) {
    try {
      const view = await invoke<WorkflowView>("get_workflow_graph", { path });
      ({ nodes, edges } = toSvelteFlow(view));
      revision = view.revision;
      loadError = null;
    } catch (error) {
      console.error("Failed to load workflow graph:", error);
      nodes = [];
      edges = [];
      revision = null;
      loadError = String(error);
    }
  }

  // Re-fetches on tab switch and on the code/graph view-mode toggle (both
  // change what `workspace.activeTab` reads).
  $effect(() => {
    const tab = workspace.activeTab;
    if (!tab || tab.viewMode !== "graph") {
      nodes = [];
      edges = [];
      revision = null;
      loadError = null;
      return;
    }
    loadGraph(tab.path);
  });

  // Also re-fetches when the file changes on disk out from under an already
  // open graph tab -- a Monaco save today, a mutation command once Phase 5's
  // write commands land (both emit this event).
  onMount(() => {
    const unlisten = listen<WorkflowChanged>("workflow-changed", (event) => {
      const tab = workspace.activeTab;
      if (tab && tab.viewMode === "graph" && tab.path === event.payload.path) {
        loadGraph(tab.path);
      }
    });
    return () => {
      unlisten.then((fn) => fn());
    };
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
  {:else if isEditorDirty}
    <div
      class="absolute top-2 left-2 rounded border border-border bg-bg-panel px-3 py-1.5 font-mono text-xs text-text-2"
    >
      Editor has unsaved changes &middot; showing the last saved version
    </div>
  {/if}
</div>
