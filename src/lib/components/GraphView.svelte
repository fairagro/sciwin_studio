<script lang="ts">
  import { SvelteFlow, Background, Controls, MiniMap, type Node, type Edge, type NodeTypes, type Connection, type IsValidConnection, type OnConnectEnd } from "@xyflow/svelte";
  import "@xyflow/svelte/dist/style.css";
  import { invoke } from "@tauri-apps/api/core";
  import { listen } from "@tauri-apps/api/event";
  import { onMount } from "svelte";
  import { workspace } from "$lib/state/workspace.svelte";
  import { toSvelteFlow } from "$lib/graph/transform";
  import { addWorkflowStepNode, connectWorkflowNodes, deleteWorkflowNode, disconnectWorkflowNodes } from "$lib/graph/mutation";
  import { getNodeLayout, saveNodeLayout } from "$lib/graph/layout";
  import { mutationErrorMessage, type ConnectionEndpoint, type FlowNodeData, type LayoutPosition, type MutationError, type WorkflowChanged, type WorkflowView } from "$lib/graph/types";
  import WorkflowNode from "./WorkflowNode.svelte";
  import ContextMenu from "./context-menu/Edge.svelte";
  import ConfirmDialog from "./ConfirmDialog.svelte";

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

  // Loaded alongside the graph and kept in sync on every drag, so a save
  // only ever has to write the full current map, never merge against disk.
  let layoutPositions: Record<string, LayoutPosition> = {};

  async function loadGraph(path: string) {
    try {
      const view = await invoke<WorkflowView>("get_workflow_graph", { path });
      const projectRoot = workspace.projectPath;
      layoutPositions = projectRoot ? await getNodeLayout(projectRoot, path).catch(() => ({})) : {};
      ({ nodes, edges } = toSvelteFlow(view, layoutPositions));
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

  // Best-effort: a node's position is a convenience, not workflow content,
  // so a save failure here shouldn't surface as a MutationError or block
  // editing the way a failed connect/delete would.
  async function handleNodeDragStop({ nodes: dragged }: { nodes: Node[] }) {
    const tab = workspace.activeTab;
    const projectRoot = workspace.projectPath;
    if (!tab || !projectRoot || dragged.length === 0) return;

    for (const n of dragged) {
      layoutPositions[n.id] = { x: n.position.x, y: n.position.y };
    }
    try {
      await saveNodeLayout(projectRoot, tab.path, layoutPositions);
    } catch (error) {
      console.error("Failed to save node layout:", error);
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

  const CWL_DRAG_MIME = "application/x-sciwin-cwl-path";

  function handleDragOver(event: DragEvent) {
    if (!event.dataTransfer?.types.includes(CWL_DRAG_MIME)) return;
    event.preventDefault();
    event.dataTransfer.dropEffect = "copy";
  }

  // A dropped tool named e.g. "plot" becomes step id "plot", or "plot_2",
  // "plot_3", ... if that id is already on the canvas -- add_workflow_step_node
  // refuses outright on a collision rather than silently no-op'ing. Also used
  // for the input/output nodes spawned by dragging a step port out to empty
  // canvas, keyed by their own "input/"/"output/" namespace.
  function uniqueNodeId(kind: "step" | "input" | "output", base: string): string {
    const taken = new Set(nodes.map((n) => n.id));
    if (!taken.has(`${kind}/${base}`)) return base;
    for (let i = 2; ; i++) {
      if (!taken.has(`${kind}/${base}_${i}`)) return `${base}_${i}`;
    }
  }

  async function handleDrop(event: DragEvent) {
    const toolPath = event.dataTransfer?.getData(CWL_DRAG_MIME);
    if (!toolPath) return;
    event.preventDefault();

    const tab = workspace.activeTab;
    if (!tab || revision === null) return;
    if (toolPath === tab.path) {
      mutationError = { kind: "invalidConnection", reason: "A workflow can't be a step of itself." };
      return;
    }

    const base =
      toolPath
        .split(/[\\/]/)
        .pop()
        ?.replace(/\.cwl$/i, "") || "step";
    const name = uniqueNodeId("step", base);

    try {
      await addWorkflowStepNode({ path: tab.path, revision, dirty: tab.dirty, toolPath, name });
      mutationError = null;
    } catch (error) {
      mutationError = error as MutationError;
    }
  }

  const isValidConnection: IsValidConnection = (candidate) => {
    const from = endpointOf(candidate.source, candidate.sourceHandle);
    const to = endpointOf(candidate.target, candidate.targetHandle);
    if (!from || !to) return false;
    return (from.kind === "input" && to.kind === "step") || (from.kind === "step" && to.kind === "step") || (from.kind === "step" && to.kind === "output");
  };

  async function performConnect(from: ConnectionEndpoint, to: ConnectionEndpoint): Promise<boolean> {
    const tab = workspace.activeTab;
    if (!tab || revision === null) return false;

    try {
      await connectWorkflowNodes({ path: tab.path, revision, dirty: tab.dirty, from, to });
      mutationError = null;
      return true;
      // the resulting workflow-changed event reloads the graph with the
      // real edge (correct id, styling, dagre layout), so any optimistic
      // one gets overwritten wholesale rather than reconciled.
    } catch (error) {
      mutationError = error as MutationError;
      return false;
    }
  }

  async function handleConnect(connection: Connection) {
    const from = endpointOf(connection.source, connection.sourceHandle);
    const to = endpointOf(connection.target, connection.targetHandle);
    if (!from || !to) return;

    const ok = await performConnect(from, to);
    if (!ok) {
      // Svelte Flow's Handle component adds this edge to the bound array
      // the instant the drag completes, before onconnect even runs -- undo
      // that on refusal, or it sits there rendered until the next reload.
      edges = edges.filter((e) => !(e.source === connection.source && e.target === connection.target && e.sourceHandle === connection.sourceHandle && e.targetHandle === connection.targetHandle));
    }
  }

  const handleConnectEnd: OnConnectEnd = async (event, state) => {
    if (state.toHandle) return; // landed on a real handle; onconnect handles that
    const target = event.target as HTMLElement | null;
    if (!target?.classList.contains("svelte-flow__pane")) return; // e.g. dropped on a node body

    const fromNode = state.fromNode;
    const fromHandle = state.fromHandle;
    if (!fromNode || !fromHandle?.id) return;
    const from = endpointOf(fromNode.id, fromHandle.id);
    if (!from || from.kind !== "step") return;

    if (fromHandle.type === "source") {
      const name = uniqueNodeId("output", from.port);
      await performConnect(from, { kind: "output", id: name, port: name });
    } else {
      const name = uniqueNodeId("input", from.port);
      await performConnect({ kind: "input", id: name, port: name }, from);
    }
  };

  // Deleting a node that still has connections needs the user's confirmation
  // before the backend cascades. handleBeforeDelete resolves this promise
  // once the dialog below is answered, which keeps Svelte Flow's own delete
  // pipeline (Delete/Backspace, the edge context menu's deleteElements) as
  // the single path -- nothing here removes the node itself.
  let deleteConfirm: { node: Node; count: number } | null = $state(null);
  let deleteDialogOpen = $state(false);
  let deleteBusy = $state(false);
  let deleteError = $state<string | null>(null);
  let resolveDeleteConfirm: ((ok: boolean) => void) | null = null;

  async function deleteNode(target: Node): Promise<boolean> {
    const tab = workspace.activeTab;
    const ref = (target.data as FlowNodeData).ref;
    if (!tab || revision === null) return false;

    try {
      await deleteWorkflowNode({ path: tab.path, revision, dirty: tab.dirty, node: ref });
      mutationError = null;
      return true;
    } catch (error) {
      mutationError = error as MutationError;
      return false;
    }
  }

  function requestNodeDeletion(target: Node, connectionCount: number): Promise<boolean> {
    if (connectionCount === 0) return deleteNode(target);
    return new Promise<boolean>((resolve) => {
      resolveDeleteConfirm = resolve;
      deleteError = null;
      deleteConfirm = { node: target, count: connectionCount };
      deleteDialogOpen = true;
    });
  }

  async function confirmNodeDeletion() {
    if (!deleteConfirm) return;
    deleteBusy = true;
    const ok = await deleteNode(deleteConfirm.node);
    deleteBusy = false;
    if (ok) {
      resolveDeleteConfirm?.(true);
      resolveDeleteConfirm = null;
      deleteConfirm = null;
      deleteDialogOpen = false;
    } else {
      deleteError = mutationError ? mutationErrorMessage(mutationError) : "Failed to delete.";
    }
  }

  function cancelNodeDeletion() {
    resolveDeleteConfirm?.(false);
    resolveDeleteConfirm = null;
    deleteConfirm = null;
    deleteDialogOpen = false;
    deleteError = null;
  }

  async function handleBeforeDelete({ nodes: nodesToDelete, edges: edgesToDelete }: { nodes: Node[]; edges: Edge[] }): Promise<boolean> {
    if (nodesToDelete.length > 0) {
      if (nodesToDelete.length > 1) {
        mutationError = { kind: "invalidConnection", reason: "Delete one node at a time." };
        return false;
      }
      const target = nodesToDelete[0];
      // Svelte Flow bundles a node's own edges into edgesToDelete alongside
      // it. An edge unrelated to the node landing in there too means a
      // mixed selection, which we don't try to reconcile.
      const stray = edgesToDelete.filter((e) => e.source !== target.id && e.target !== target.id);
      if (stray.length > 0) {
        mutationError = { kind: "invalidConnection", reason: "Delete one thing at a time." };
        return false;
      }
      return requestNodeDeletion(target, edgesToDelete.length);
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

<!-- svelte-ignore a11y_no_static_element_interactions -- drop zone for dragging a tool from the Sidebar onto the graph; the canvas itself is SvelteFlow's, this div only relays drops -->
<div class="relative h-full w-full" bind:this={containerEl} bind:clientWidth bind:clientHeight ondragover={handleDragOver} ondrop={handleDrop}>
  <SvelteFlow
    bind:nodes
    bind:edges
    {nodeTypes}
    {isValidConnection}
    onconnect={handleConnect}
    onconnectend={handleConnectEnd}
    onnodedragstop={handleNodeDragStop}
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

<ConfirmDialog
  bind:open={deleteDialogOpen}
  title="Delete node"
  message={deleteConfirm ? `"${(deleteConfirm.node.data as FlowNodeData).label}" is still connected. Deleting it will also remove ${deleteConfirm.count} connection${deleteConfirm.count === 1 ? "" : "s"}.` : ""}
  confirmLabel="Delete"
  busy={deleteBusy}
  error={deleteError}
  onConfirm={confirmNodeDeletion}
  onCancel={cancelNodeDeletion}
/>
