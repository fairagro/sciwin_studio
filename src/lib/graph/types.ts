// Mirrors src-tauri/src/graph_types.rs  keep in sync by hand 

export type NodeKind = "input" | "output" | "step";

export interface NodeRef {
  kind: NodeKind;
  id: string;
}

export interface FlowPort {
  id: string;
  dataType: string;
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
