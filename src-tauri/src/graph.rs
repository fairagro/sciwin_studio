use std::path::Path;

use commonwl::documents::Workflow;

use crate::graph_types::{FlowNode, FlowNodeData, FlowPort, FlowPosition, WorkflowView};

pub fn load_workflow_graph(workflow: &Workflow, path: impl AsRef<Path>) -> WorkflowView {
    let mut graph = WorkflowView {
        nodes: Vec::new(),
        edges: Vec::new(),
    };

    for input in &workflow.inputs {
        let id = input.id.clone().unwrap();

        graph.nodes.push(FlowNode {
            id: id.clone(),
            node_type: "input".into(),
            position: FlowPosition { x: 0.0, y: 0.0 },
            data: FlowNodeData {
                label: id.clone(),
                inputs: vec![],
                outputs: vec![FlowPort {
                    id,
                    data_type: format!("{:?}", input.r#type),
                }],
            },
        });
    }

    for output in &workflow.outputs {
        let id = output.id.clone().unwrap();

        graph.nodes.push(FlowNode {
            id: id.clone(),
            node_type: "output".into(),
            position: FlowPosition { x: 0.0, y: 0.0 },
            data: FlowNodeData {
                label: id.clone(),
                inputs: vec![FlowPort {
                    id,
                    data_type: format!("{:?}", output.r#type),
                }],
                outputs: vec![],
            },
        });
    }

    graph
}
