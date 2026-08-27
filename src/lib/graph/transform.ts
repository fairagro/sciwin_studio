import dagre from "@dagrejs/dagre";
import { Position, type Node, type Edge } from "@xyflow/svelte";
import type { FlowNode, FlowPort, WorkflowView } from "./types";

// Sized from label/port text length
const CHAR_WIDTH = 6.5;
const ROW_HEIGHT = 20;
const HEADER_HEIGHT = 26;
const MIN_WIDTH = 170;
const MAX_WIDTH = 320;

function portLabelLength(port: FlowPort): number {
  return port.id.length + port.dataType.length + 2;
}

function estimateSize(node: FlowNode): { width: number; height: number } {
  const longest = Math.max(
    node.data.label.length,
    ...node.data.inputs.map(portLabelLength),
    ...node.data.outputs.map(portLabelLength),
    1
  );
  const width = Math.min(MAX_WIDTH, Math.max(MIN_WIDTH, longest * CHAR_WIDTH + 44));
  const rows = Math.max(node.data.inputs.length + node.data.outputs.length, 1);
  const height = HEADER_HEIGHT + rows * ROW_HEIGHT + 8;
  return { width, height };
}

export function toSvelteFlow(view: WorkflowView): { nodes: Node[]; edges: Edge[] } {
  const g = new dagre.graphlib.Graph();
  g.setGraph({ rankdir: "LR", nodesep: 24, ranksep: 72 });
  g.setDefaultEdgeLabel(() => ({}));

  const sizes = new Map(view.nodes.map((n) => [n.id, estimateSize(n)]));
  for (const n of view.nodes) {
    const { width, height } = sizes.get(n.id)!;
    g.setNode(n.id, { width, height });
  }
  for (const e of view.edges) {
    g.setEdge(e.source, e.target);
  }

  dagre.layout(g);

  const nodes: Node[] = view.nodes.map((n) => {
    const { x, y } = g.node(n.id);
    const { width, height } = sizes.get(n.id)!;
    return {
      id: n.id,
      type: "workflow",
      data: n.data,
      position: { x: x - width / 2, y: y - height / 2 },
      sourcePosition: Position.Right,
      targetPosition: Position.Left,
      style: `width: ${width}px;`,
    };
  });

  const edges: Edge[] = view.edges.map((e) => ({
    id: e.id,
    source: e.source,
    target: e.target,
    sourceHandle: e.sourceHandle,
    targetHandle: e.targetHandle,
  }));

  return { nodes, edges };
}
