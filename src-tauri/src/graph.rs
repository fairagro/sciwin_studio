use std::{
    collections::{HashMap, HashSet, VecDeque},
    path::Path,
};

use commonwl::{
    OneOrMany, load_cwl_file,
    documents::{CWLDocument, StringOrDocument, Workflow},
    inputs::{InputSchema, InputType},
    outputs::{CommandOutputParameterType, CommandOutputSchema, CommandOutputType},
};

use crate::graph_types::{
    FlowEdge, FlowNode, FlowNodeData, FlowPort, FlowPosition, NodeDiagnostic, NodeKind, NodeRef,
    RunRef, WorkflowView,
};

pub fn load_workflow_graph(workflow: &Workflow, path: impl AsRef<Path>) -> WorkflowView {
    let path = path.as_ref();
    let mut view = WorkflowView {
        nodes: Vec::new(),
        edges: Vec::new(),
    };

    let input_ids: HashSet<&str> = workflow.inputs.iter().filter_map(|i| i.id.as_deref()).collect();
    let step_ids: HashSet<&str> = workflow.steps.iter().filter_map(|s| s.id.as_deref()).collect();
    let cyclic_steps = cyclic_step_ids(workflow);

    for input in &workflow.inputs {
        let Some(id) = input.id.clone() else { continue };
        let node_ref = NodeRef::new(NodeKind::Input, id.clone());
        view.nodes.push(FlowNode {
            id: node_ref.flat(),
            node_type: "input".into(),
            position: FlowPosition::default(),
            data: FlowNodeData {
                node_ref,
                label: id.clone(),
                inputs: vec![],
                outputs: vec![FlowPort {
                    id: id.clone(),
                    data_type: input_type_label(&input.r#type),
                }],
                run: None,
                diagnostics: vec![],
                status: None,
            },
        });
    }

    for step in &workflow.steps {
        let Some(step_id) = step.id.clone() else { continue };
        let node_ref = NodeRef::new(NodeKind::Step, step_id.clone());

        let mut diagnostics = vec![];
        let (run, doc) = match &step.run {
            StringOrDocument::String(rel) => {
                let step_file = path.parent().unwrap_or(path).join(rel);
                let doc = match load_cwl_file(&step_file, true) {
                    Ok(doc) => Some(doc),
                    Err(e) => {
                        diagnostics.push(NodeDiagnostic {
                            message: format!("could not load {}: {e}", step_file.display()),
                        });
                        None
                    }
                };
                (
                    RunRef::File {
                        path: step_file.to_string_lossy().into_owned(),
                    },
                    doc,
                )
            }
            StringOrDocument::Document(doc) => (RunRef::Inline, Some((**doc).clone())),
        };

        if cyclic_steps.contains(step_id.as_str()) {
            diagnostics.push(NodeDiagnostic {
                message: "step participates in a cycle".into(),
            });
        }

        let (inputs, outputs) = match &doc {
            Some(doc) => (
                doc.get_inputs()
                    .iter()
                    .map(|i| FlowPort {
                        id: i.id.clone().unwrap_or_default(),
                        data_type: input_type_label(&i.r#type),
                    })
                    .collect(),
                doc.get_output_ids()
                    .iter()
                    .map(|id| FlowPort {
                        id: id.clone(),
                        data_type: get_output_type(doc, id)
                            .map(|t| output_type_label(&t))
                            .unwrap_or_default(),
                    })
                    .collect(),
            ),
            None => (vec![], vec![]),
        };

        view.nodes.push(FlowNode {
            id: node_ref.flat(),
            node_type: "step".into(),
            position: FlowPosition::default(),
            data: FlowNodeData {
                node_ref: node_ref.clone(),
                label: step_id.clone(),
                inputs,
                outputs,
                run: Some(run),
                diagnostics,
                status: None,
            },
        });

        for wsip in &step.r#in {
            let Some(target_port) = wsip.id.clone() else { continue };
            let Some(source) = &wsip.source else { continue };
            // one edge per source
            for src in source.as_many() {
                if let Some((from_ref, from_port)) = resolve_source(&src, &input_ids, &step_ids) {
                    view.edges.push(make_edge(
                        from_ref,
                        &from_port,
                        node_ref.clone(),
                        &target_port,
                    ));
                }
            }
        }
    }

    for output in &workflow.outputs {
        let Some(id) = output.id.clone() else { continue };
        let node_ref = NodeRef::new(NodeKind::Output, id.clone());
        view.nodes.push(FlowNode {
            id: node_ref.flat(),
            node_type: "output".into(),
            position: FlowPosition::default(),
            data: FlowNodeData {
                node_ref: node_ref.clone(),
                label: id.clone(),
                inputs: vec![FlowPort {
                    id: id.clone(),
                    data_type: output_type_label(&output.r#type),
                }],
                outputs: vec![],
                run: None,
                diagnostics: vec![],
                status: None,
            },
        });

        if let Some(output_source) = &output.output_source {
            // `outputSource: some_input` (no slash) is a legal pass-through
            for src in output_source.as_many() {
                if let Some((from_ref, from_port)) = resolve_source(&src, &input_ids, &step_ids) {
                    view.edges.push(make_edge(from_ref, &from_port, node_ref.clone(), &id));
                }
            }
        }
    }

    view
}

fn resolve_source(
    source: &str,
    input_ids: &HashSet<&str>,
    step_ids: &HashSet<&str>,
) -> Option<(NodeRef, String)> {
    match source.split_once('/') {
        Some((step_id, port)) if step_ids.contains(step_id) => {
            Some((NodeRef::new(NodeKind::Step, step_id), port.to_string()))
        }
        None if input_ids.contains(source) => {
            Some((NodeRef::new(NodeKind::Input, source), source.to_string()))
        }
        _ => None,
    }
}

fn make_edge(from: NodeRef, from_port: &str, to: NodeRef, to_port: &str) -> FlowEdge {
    let from = from.flat();
    let to = to.flat();
    FlowEdge {
        id: format!("{from}:{from_port}->{to}:{to_port}"),
        source: from,
        target: to,
        source_handle: from_port.to_string(),
        target_handle: to_port.to_string(),
    }
}

/// Step ids that are part of, or depend on, a cycle. Cycles used to abort the
/// whole graph build (`sort_steps`); rendering a cyclic workflow is how you
/// find the cycle, so this only reports it as a per-node diagnostic now.
fn cyclic_step_ids(workflow: &Workflow) -> HashSet<String> {
    let step_ids: HashSet<String> = workflow.steps.iter().filter_map(|s| s.id.clone()).collect();

    let mut dependents: HashMap<String, Vec<String>> = HashMap::new();
    let mut in_degree: HashMap<String, usize> = step_ids.iter().cloned().map(|id| (id, 0)).collect();

    for step in &workflow.steps {
        let Some(step_id) = &step.id else { continue };
        for wsip in &step.r#in {
            let Some(source) = &wsip.source else { continue };
            for src in source.as_many() {
                let Some((dep, _)) = src.split_once('/') else { continue };
                if step_ids.contains(dep) {
                    dependents.entry(dep.to_string()).or_default().push(step_id.clone());
                    *in_degree.entry(step_id.clone()).or_insert(0) += 1;
                }
            }
        }
    }

    let mut queue: VecDeque<String> = in_degree
        .iter()
        .filter(|&(_, &degree)| degree == 0)
        .map(|(id, _)| id.clone())
        .collect();

    let mut resolved = HashSet::new();
    while let Some(step_id) = queue.pop_front() {
        resolved.insert(step_id.clone());
        if let Some(deps) = dependents.get(&step_id) {
            for dependent in deps {
                if let Some(degree) = in_degree.get_mut(dependent) {
                    *degree -= 1;
                    if *degree == 0 {
                        queue.push_back(dependent.clone());
                    }
                }
            }
        }
    }

    step_ids.difference(&resolved).cloned().collect()
}

fn get_output_type(doc: &CWLDocument, id: &str) -> Option<CommandOutputParameterType> {
    match doc {
        CWLDocument::ExpressionTool(et) => et
            .outputs
            .iter()
            .find(|o| o.id.as_deref() == Some(id))
            .map(|o| o.r#type.clone())
            .map(Into::into),
        CWLDocument::Operation(op) => op
            .outputs
            .iter()
            .find(|o| o.id.as_deref() == Some(id))
            .map(|o| o.r#type.clone())
            .map(Into::into),
        CWLDocument::Workflow(wf) => wf
            .outputs
            .iter()
            .find(|o| o.id.as_deref() == Some(id))
            .map(|o| o.r#type.clone()),
        CWLDocument::CommandLineTool(clt) => clt
            .outputs
            .iter()
            .find(|o| o.id.as_deref() == Some(id))
            .map(|o| o.r#type.clone()),
    }
}

// Full recursive type rendering (record fields, enum symbols, nested arrays)
// is Phase 6's type-palette work. This gives a clean top-level label instead
// of leaking `format!("{:?}", ...)` Rust Debug output into port labels.
fn input_type_label(t: &OneOrMany<InputType>) -> String {
    t.as_many().iter().map(single_input_type_label).collect::<Vec<_>>().join(" | ")
}

fn single_input_type_label(t: &InputType) -> String {
    match t {
        InputType::CWLType(cwl) => cwl.to_string(),
        InputType::InputSchema(schema) => match schema.as_ref() {
            InputSchema::Array(_) => "array".to_string(),
            InputSchema::Record(_) => "record".to_string(),
            InputSchema::Enum(_) => "enum".to_string(),
        },
        InputType::String(s) => s.clone(),
    }
}

fn output_type_label(t: &CommandOutputParameterType) -> String {
    match t {
        CommandOutputParameterType::Stdout => "stdout".to_string(),
        CommandOutputParameterType::Stderr => "stderr".to_string(),
        CommandOutputParameterType::CommandOutputType(types) => types
            .as_many()
            .iter()
            .map(single_output_type_label)
            .collect::<Vec<_>>()
            .join(" | "),
    }
}

fn single_output_type_label(t: &CommandOutputType) -> String {
    match t {
        CommandOutputType::CWLType(cwl) => cwl.to_string(),
        CommandOutputType::CommandOutputSchema(schema) => match schema.as_ref() {
            CommandOutputSchema::Array(_) => "array".to_string(),
            CommandOutputSchema::Record(_) => "record".to_string(),
            CommandOutputSchema::Enum(_) => "enum".to_string(),
        },
        CommandOutputType::String(s) => s.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use commonwl::documents::CWLDocument;

    fn load_workflow(path: &str) -> Workflow {
        let CWLDocument::Workflow(workflow) = load_cwl_file(path, true).unwrap() else {
            panic!("Expected a workflow document")
        };
        workflow
    }

    #[test]
    fn test_load_workflow_graph() {
        let path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../testdata/hello_world/workflows/main/main.cwl"
        );
        let workflow = load_workflow(path);
        let view = load_workflow_graph(&workflow, path);

        assert_eq!(view.nodes.len(), 5); // 2 inputs, 2 steps, 1 output
        assert_eq!(view.edges.len(), 4);
    }

    #[test]
    fn test_load_workflow_graph_mkdir() {
        let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../testdata/mkdir_wf.cwl");
        let workflow = load_workflow(path);
        let view = load_workflow_graph(&workflow, path);

        assert_eq!(view.nodes.len(), 3);
        assert_eq!(view.edges.len(), 2);
    }

    // A step id equal to an input id is legal CWL -- commonwl's own
    // `validate_unique_ids` only checks uniqueness within one list (inputs,
    // outputs, steps each validated separately), not across them. The old
    // `HashMap<String, NodeIndex>` node_map let the step silently overwrite
    // the input's entry; every edge then resolved to the step. NodeRef fixes
    // this by keying identity on (kind, id), not id alone.
    #[test]
    fn test_step_id_collides_with_input_id() {
        let yaml = r"
cwlVersion: v1.2
class: Workflow
inputs:
- id: x
  type: File
outputs:
- id: result
  type: File
  outputSource: x/out
steps:
- id: x
  in:
  - id: in_file
    source: x
  out:
  - out
  run:
    class: CommandLineTool
    inputs:
    - id: in_file
      type: File
    outputs:
    - id: out
      type: File
";
        let CWLDocument::Workflow(workflow) = commonwl::from_str(yaml).unwrap() else {
            panic!("Expected a workflow document")
        };
        let view = load_workflow_graph(&workflow, "workflow.cwl");

        assert_eq!(view.nodes.len(), 3);
        let ids: HashSet<&str> = view.nodes.iter().map(|n| n.id.as_str()).collect();
        assert!(ids.contains("input/x"));
        assert!(ids.contains("step/x"));
        assert!(ids.contains("output/result"));

        assert_eq!(view.edges.len(), 2);
        assert!(view.edges.iter().any(|e| e.source == "input/x" && e.target == "step/x"));
        assert!(view.edges.iter().any(|e| e.source == "step/x" && e.target == "output/result"));
    }

    #[test]
    fn test_cyclic_workflow_reports_diagnostic_without_failing() {
        let yaml = r"
cwlVersion: v1.2
class: Workflow
inputs: []
outputs: []
steps:
- id: a
  in:
  - id: in_val
    source: b/out
  out:
  - out
  run:
    class: CommandLineTool
    inputs:
    - id: in_val
      type: File
    outputs:
    - id: out
      type: File
- id: b
  in:
  - id: in_val
    source: a/out
  out:
  - out
  run:
    class: CommandLineTool
    inputs:
    - id: in_val
      type: File
    outputs:
    - id: out
      type: File
";
        let CWLDocument::Workflow(workflow) = commonwl::from_str(yaml).unwrap() else {
            panic!("Expected a workflow document")
        };
        let view = load_workflow_graph(&workflow, "workflow.cwl");

        assert_eq!(view.nodes.len(), 2);
        assert_eq!(view.edges.len(), 2);
        assert!(view.nodes.iter().all(|n| !n.data.diagnostics.is_empty()));
    }
}
