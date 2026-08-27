// Mirrors src-tauri/src/graph_types.rs  keep in sync by hand 

export type NodeKind = "input" | "output" | "step";

export interface NodeRef {
  kind: NodeKind;
  id: string;
}

export type LinkMerge = "merge_nested" | "merge_flattened";
export type PickValue = "first_non_null" | "the_only_non_null" | "all_non_null";

export interface FlowPort {
  id: string;
  dataType: string;
  // Set only on a step's input ports -- null on workflow inputs/outputs and
  // on step outputs, which have no WorkflowStepInput to carry these.
  linkMerge: LinkMerge | null;
  pickValue: PickValue | null;
}

export type RunRef = { kind: "file"; path: string } | { kind: "inline" };

export interface NodeDiagnostic {
  message: string;
}


export interface FlowNodeData extends Record<string, unknown> {
  ref: NodeRef;
  label: string;
  inputs: FlowPort[];
  outputs: FlowPort[];
  run: RunRef | null;
  diagnostics: NodeDiagnostic[];
  status: string | null;
  when: string | null;
  scatter: string[];
  scatterMethod: "dotproduct" | "nested_crossproduct" | "flat_crossproduct" | null;
}

export interface FlowNode {
  id: string;
  nodeType: string;
  position: { x: number; y: number };
  data: FlowNodeData;
}

export interface FlowEdge {
  id: string;
  source: string;
  target: string;
  sourceHandle: string;
  targetHandle: string;
}

export interface WorkflowView {
  nodes: FlowNode[];
  edges: FlowEdge[];
}
