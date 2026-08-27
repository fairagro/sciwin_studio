use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct WorkflowView {
    pub nodes: Vec<FlowNode>,
    pub edges: Vec<FlowEdge>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FlowNode {
    pub id: String,
    pub node_type: String,
    pub position: FlowPosition,
    pub data: FlowNodeData,
}

#[derive(Debug, Clone, Serialize)]
pub struct FlowPosition {
    pub x: f32,
    pub y: f32,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FlowNodeData {
    pub label: String,
    pub inputs: Vec<FlowPort>,
    pub outputs: Vec<FlowPort>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FlowPort {
    pub id: String,
    pub data_type: String,
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
