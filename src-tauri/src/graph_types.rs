use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum NodeKind {
    Input,
    Output,
    Step,
}

/// Identifies a node by kind *and* bare CWL id, never id alone -- a step named
/// the same as a workflow input/output must not collide with it. See graph.rs.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize)]
pub struct NodeRef {
    pub kind: NodeKind,
    pub id: String,
}

impl NodeRef {
    pub fn new(kind: NodeKind, id: impl Into<String>) -> Self {
        Self {
            kind,
            id: id.into(),
        }
    }

    /// The id Svelte Flow sees, e.g. "step/plot".
    pub fn flat(&self) -> String {
        let prefix = match self.kind {
            NodeKind::Input => "input",
            NodeKind::Output => "output",
            NodeKind::Step => "step",
        };
        format!("{prefix}/{}", self.id)
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkflowView {
    pub nodes: Vec<FlowNode>,
    pub edges: Vec<FlowEdge>,
    /// Hash of the file's on-disk bytes at load time. A mutation command
    /// passes this back; the backend refuses (does not corrupt the file)
    /// if the file no longer matches, whether from an external change or an
    /// unsaved Monaco edit the graph view hasn't seen yet.
    pub revision: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FlowNode {
    pub id: String,
    pub node_type: String,
    pub position: FlowPosition,
    pub data: FlowNodeData,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct FlowPosition {
    pub x: f32,
    pub y: f32,
}
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FlowNodeData {
    #[serde(rename = "ref")]
    pub node_ref: NodeRef,
    pub label: String,
    pub inputs: Vec<FlowPort>,
    pub outputs: Vec<FlowPort>,
    pub run: Option<RunRef>,
    pub diagnostics: Vec<NodeDiagnostic>,
    pub status: Option<String>,
    pub when: Option<String>,
    pub scatter: Vec<String>,
    pub scatter_method: Option<String>,
}

/// A step's `run:` is a path to a tool file or an inline document (legal CWL,
/// guaranteed after unpacking a `$graph`) -- the old builder bailed on inline
/// docs entirely ("Inline Document not supported"), taking the whole graph down.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum RunRef {
    File { path: String },
    Inline,
}

#[derive(Debug, Clone, Serialize)]
pub struct NodeDiagnostic {
    pub message: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FlowPort {
    pub id: String,
    pub data_type: String,
    /// "merge_nested" | "merge_flattened".
    pub link_merge: Option<String>,
    /// "first_non_null" | "the_only_non_null" | "all_non_null".
    pub pick_value: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FlowEdge {
    pub id: String,
    pub source: String,
    pub target: String,
    pub source_handle: String,
    pub target_handle: String,
}
