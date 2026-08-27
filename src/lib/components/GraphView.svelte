<script lang="ts">
  import { SvelteFlow, Background, Controls, MiniMap, Position, type Node, type Edge } from "@xyflow/svelte";
  import "@xyflow/svelte/dist/style.css";
  import dagre from "@dagrejs/dagre";
  import { workspace } from "$lib/state/workspace.svelte";
    import { invoke } from "@tauri-apps/api/core";

  // Stand-in for container-registry/workflows/workflow.cwl's step graph -
  // proves SvelteFlow + dagre render together in the real app shell. Not yet
  // wired to the active tab's actual parsed CWL content.
  const rawNodes = [
    { id: "images", label: "images" },
    { id: "namespaces", label: "namespaces" },
    { id: "discover_images", label: "discover_images" },
    { id: "scan_image", label: "scan_image" },
    { id: "collect", label: "collect" },
    { id: "add_index", label: "add_index" },
    { id: "generate_api", label: "generate_api" },
  ];
  const rawEdges = [
    { id: "e1", source: "namespaces", target: "discover_images" },
    { id: "e2", source: "discover_images", target: "scan_image" },
    { id: "e3", source: "images", target: "scan_image" },
    { id: "e4", source: "scan_image", target: "collect" },
    { id: "e5", source: "collect", target: "add_index" },
    { id: "e6", source: "add_index", target: "generate_api" },
  ];

  const NODE_WIDTH = 160;
  const NODE_HEIGHT = 40;

  function layout(): { nodes: Node[]; edges: Edge[] } {
    const g = new dagre.graphlib.Graph();
    g.setGraph({ rankdir: "LR", nodesep: 32, ranksep: 64 });
    g.setDefaultEdgeLabel(() => ({}));

    for (const n of rawNodes) g.setNode(n.id, { width: NODE_WIDTH, height: NODE_HEIGHT });
    for (const e of rawEdges) g.setEdge(e.source, e.target);

    dagre.layout(g);

    const nodes: Node[] = rawNodes.map((n) => {
      const { x, y } = g.node(n.id);
      return {
        id: n.id,
        data: { label: n.label },
        position: { x: x - NODE_WIDTH / 2, y: y - NODE_HEIGHT / 2 },
        sourcePosition: Position.Right,
        targetPosition: Position.Left,
        style: `width: ${NODE_WIDTH}px;`,
      };
    });

    const edges: Edge[] = rawEdges.map((e) => ({ ...e }));

    return { nodes, edges };
  }

  const initial = layout();
  let nodes = $state.raw(initial.nodes);
  let edges = $state.raw(initial.edges);

  $effect(() => {
    const tab = workspace.activeTab;
    if (!tab) return;
    let workflow = invoke('get_workflow_graph', {path: tab.path})
  });
</script>

<div class="h-full w-full">
  <SvelteFlow bind:nodes bind:edges fitView colorMode="dark">
    <Background />
    <Controls />
    <MiniMap />
  </SvelteFlow>
</div>
