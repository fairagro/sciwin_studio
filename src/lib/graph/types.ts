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
  // Hash of the file's on-disk bytes at load time. Mutation commands take
  // this back and refuse if the file no longer matches.
  revision: string;
}

// Payload of the "workflow-changed" Tauri event, emitted by write_file and
// the mutation commands after a .cwl file changes on disk.
export interface WorkflowChanged {
  path: string;
  revision: string;
}

// One end of a connect/disconnect call -- mirrors ConnectionEndpoint in
// src-tauri/src/mutation.rs.
export interface ConnectionEndpoint {
  kind: NodeKind;
  id: string;
  port: string;
}

// Mirrors MutationError in src-tauri/src/mutation.rs.
export type MutationError =
  | { kind: "editorDirty" }
  | { kind: "staleRevision" }
  | { kind: "lossy" }
  | { kind: "incompatibleTypes"; reason: string }
  | { kind: "invalidConnection"; reason: string }
  | { kind: "notFound"; message: string }
  | { kind: "duplicateId"; id: string }
  | { kind: "io"; message: string };

export function mutationErrorMessage(error: MutationError): string {
  switch (error.kind) {
    case "editorDirty":
      return "Save your changes in the editor before editing the graph.";
    case "staleRevision":
      return "The file changed since the graph was loaded. Reloading.";
    case "lossy":
      return "This workflow uses $import or $graph, which the graph view can't edit yet.";
    case "incompatibleTypes":
    case "invalidConnection":
      return error.reason;
    case "notFound":
    case "io":
      return error.message;
    case "duplicateId":
      return `A node named "${error.id}" already exists.`;
  }
}
