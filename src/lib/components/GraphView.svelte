<script lang="ts">
  import { SvelteFlow, Background, Controls, MiniMap, type Node, type Edge, type NodeTypes, type Connection, type IsValidConnection, type EdgeEvents } from "@xyflow/svelte";
  import "@xyflow/svelte/dist/style.css";
  import { invoke } from "@tauri-apps/api/core";
  import { listen } from "@tauri-apps/api/event";
  import { onMount } from "svelte";
  import { workspace } from "$lib/state/workspace.svelte";
  import { toSvelteFlow } from "$lib/graph/transform";
  import { connectWorkflowNodes, disconnectWorkflowNodes } from "$lib/graph/mutation";
  import { mutationErrorMessage, type ConnectionEndpoint, type FlowNodeData, type MutationError, type WorkflowChanged, type WorkflowView } from "$lib/graph/types";
  import WorkflowNode from "./WorkflowNode.svelte";
  import ContextMenu from "./context-menu/Edge.svelte";

  const nodeTypes: NodeTypes = { workflow: WorkflowNode };

  let nodes = $state.raw<Node[]>([]);
  let edges = $state.raw<Edge[]>([]);
  let loadError = $state<string | null>(null);
  // Passed back on every mutation call so the backend can refuse a write
  // against a stale file instead of silently clobbering it.
  let revision = $state<string | null>(null);
  let mutationError = $state<MutationError | null>(null);

  const isEditorDirty = $derived(workspace.activeTab?.viewMode === "graph" && workspace.activeTab?.dirty === true);

  let edge_menu: {
    id: string;
    top?: number;
    left?: number;
    right?: number;
    bottom?: number;
  } | null = $state(null);
  let clientWidth: number = $state(0);
  let clientHeight: number = $state(0);
  let containerEl: HTMLDivElement | undefined = $state();

  const handleEdgeContextMenu = ({ edge, event }: { edge: Edge; event: MouseEvent }) => {
    // Prevent native context menu from showing
    event.preventDefault();

    // event.clientX/Y are viewport-relative, but the menu is positioned
    // relative to this container (offset by the sidebar/titlebar) -- convert
    // before comparing against clientWidth/clientHeight, or the menu lands
    // wherever the container happens to be offset from the window origin.
    const rect = containerEl?.getBoundingClientRect();
    const x = event.clientX - (rect?.left ?? 0);
    const y = event.clientY - (rect?.top ?? 0);

    // Calculate position of the context menu. We want to make sure it
    // doesn't get positioned off-screen.
    edge_menu = {
      id: edge.id,
      top: y < clientHeight - 200 ? y : undefined,
      left: x < clientWidth - 200 ? x : undefined,
      right: x >= clientWidth - 200 ? clientWidth - x : undefined,
      bottom: y >= clientHeight - 200 ? clientHeight - y : undefined,
    };
  };

  function handlePaneClick() {
    edge_menu = null;
  }

  let mutationErrorTimer: ReturnType<typeof setTimeout> | undefined;
  $effect(() => {
    clearTimeout(mutationErrorTimer);
    if (mutationError) {
      mutationErrorTimer = setTimeout(() => (mutationError = null), 5000);
    }
  });

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
  // open graph tab -- a Monaco save, or a successful connect/disconnect
  // below (both emit this event).
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

  function endpointOf(nodeId: string, port: string | null | undefined): ConnectionEndpoint | null {
    if (!port) return null;
    const node = nodes.find((n) => n.id === nodeId);
    if (!node) return null;
    const ref = (node.data as FlowNodeData).ref;
    return { kind: ref.kind, id: ref.id, port };
  }

  const isValidConnection: IsValidConnection = (candidate) => {
    const from = endpointOf(candidate.source, candidate.sourceHandle);
    const to = endpointOf(candidate.target, candidate.targetHandle);
    if (!from || !to) return false;
    return (from.kind === "input" && to.kind === "step") || (from.kind === "step" && to.kind === "step") || (from.kind === "step" && to.kind === "output");
  };

  async function handleConnect(connection: Connection) {
    const tab = workspace.activeTab;
    const from = endpointOf(connection.source, connection.sourceHandle);
    const to = endpointOf(connection.target, connection.targetHandle);
    if (!tab || revision === null || !from || !to) return;

    try {
      await connectWorkflowNodes({ path: tab.path, revision, dirty: tab.dirty, from, to });
      mutationError = null;
      // the resulting workflow-changed event reloads the graph with the
      // real edge (correct id, styling, dagre layout), so the optimistic
      // one below gets overwritten wholesale rather than reconciled.
    } catch (error) {
      mutationError = error as MutationError;
      // Svelte Flow's Handle component adds this edge to the bound array
      // the instant the drag completes, before onconnect even runs -- undo
      // that on refusal, or it sits there rendered until the next reload.
      edges = edges.filter((e) => !(e.source === connection.source && e.target === connection.target && e.sourceHandle === connection.sourceHandle && e.targetHandle === connection.targetHandle));
    }
  }

  async function handleBeforeDelete({ nodes: nodesToDelete, edges: edgesToDelete }: { nodes: Node[]; edges: Edge[] }): Promise<boolean> {
    if (nodesToDelete.length > 0) {
      mutationError = { kind: "invalidConnection", reason: "Deleting nodes isn't supported yet." };
      return false;
    }
    if (edgesToDelete.length === 0) return true;
    if (edgesToDelete.length > 1) {
      mutationError = { kind: "invalidConnection", reason: "Delete one connection at a time." };
      return false;
    }

    const tab = workspace.activeTab;
    const edge = edgesToDelete[0];
    const from = endpointOf(edge.source, edge.sourceHandle);
    const to = endpointOf(edge.target, edge.targetHandle);
    if (!tab || revision === null || !from || !to) return false;

    try {
      await disconnectWorkflowNodes({ path: tab.path, revision, dirty: tab.dirty, from, to });
      mutationError = null;
      return true;
    } catch (error) {
      mutationError = error as MutationError;
      return false;
    }
  }
</script>

<div class="relative h-full w-full" bind:this={containerEl} bind:clientWidth bind:clientHeight>
  <SvelteFlow
    bind:nodes
    bind:edges
    {nodeTypes}
    {isValidConnection}
    onconnect={handleConnect}
    onpaneclick={handlePaneClick}
    onpointerdown={handlePaneClick}
    onedgecontextmenu={handleEdgeContextMenu}
    onbeforedelete={handleBeforeDelete}
    nodesConnectable={!isEditorDirty}
    deleteKey={["Backspace", "Delete"]}
    fitView
    colorMode="dark"
  >
    <Background bgColor="var(--color-bg)" patternColor="var(--color-border-soft)" gap={22} size={1} />
    {#if edge_menu}
      <ContextMenu onclick={handlePaneClick} id={edge_menu.id} top={edge_menu.top} left={edge_menu.left} right={edge_menu.right} bottom={edge_menu.bottom} />
    {/if}
    <Controls />
    <MiniMap bgColor="var(--color-bg-panel)" maskColor="rgba(18, 19, 22, 0.65)" />
  </SvelteFlow>
  {#if loadError}
    <div class="absolute top-2 left-2 rounded border border-fairagro-red bg-bg-panel px-3 py-1.5 font-mono text-xs text-fairagro-red-light">
      Failed to load graph: {loadError}
    </div>
  {:else if mutationError}
    <div class="absolute top-2 left-2 rounded border border-fairagro-red bg-bg-panel px-3 py-1.5 font-mono text-xs text-fairagro-red-light">
      {mutationErrorMessage(mutationError)}
    </div>
  {:else if isEditorDirty}
    <div class="absolute top-2 left-2 rounded border border-border bg-bg-panel px-3 py-1.5 font-mono text-xs text-text-2">Editor has unsaved changes &middot; showing the last saved version</div>
  {/if}
</div>
