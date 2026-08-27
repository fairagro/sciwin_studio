use std::{
    collections::{HashMap, HashSet},
    path::Path,
};

use commonwl::{
    OneOrMany,
    documents::{CWLDocument, ScatterMethod, StringOrDocument, Workflow},
    inputs::{InputSchema, InputType, WorkflowStepInput},
    load_cwl_file,
    outputs::{
        CommandOutputParameterType, CommandOutputSchema, CommandOutputType, LinkMergeMethod,
        PickValueMethod,
    },
};
use petgraph::{algo::tarjan_scc, graph::DiGraph};

use crate::graph_types::{
    FlowEdge, FlowNode, FlowNodeData, FlowPort, FlowPosition, NodeDiagnostic, NodeKind, NodeRef,
    RunRef, WorkflowView,
};

/// Loads and reads a workflow's graph.
#[tauri::command]
pub fn get_workflow_graph(path: String) -> Result<WorkflowView, String> {
    let doc = load_cwl_file(&path, true).map_err(|e| e.to_string())?;
    let CWLDocument::Workflow(workflow) = doc else {
        return Err(format!("{path} is not a Workflow document"));
    };
    let mut view = load_workflow_graph(&workflow, &path);
    let bytes = std::fs::read(&path).map_err(|e| e.to_string())?;
    view.revision = compute_revision(&bytes);
    Ok(view)
}

/// Hash of raw file bytes, not of the parsed/preprocessed document
pub fn compute_revision(bytes: &[u8]) -> String {
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    bytes.hash(&mut hasher);
    format!("{:016x}", hasher.finish())
}

pub fn load_workflow_graph(workflow: &Workflow, path: impl AsRef<Path>) -> WorkflowView {
    let path = path.as_ref();
    let mut view = WorkflowView {
        nodes: Vec::new(),
        edges: Vec::new(),
        revision: String::new(),
    };

    let input_ids: HashSet<&str> = workflow
        .inputs
        .iter()
        .filter_map(|i| i.id.as_deref())
        .collect();
    let step_ids: HashSet<&str> = workflow
        .steps
        .iter()
        .filter_map(|s| s.id.as_deref())
        .collect();
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
                    link_merge: None,
                    pick_value: None,
                }],
                run: None,
                diagnostics: vec![],
                status: None,
                when: None,
                scatter: vec![],
                scatter_method: None,
            },
        });
    }

    for step in &workflow.steps {
        let Some(step_id) = step.id.clone() else {
            continue;
        };
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

        // The tool's own parameter declarations (doc.get_inputs()) carry no
        // link_merge/pick_value -- those live on the *workflow's* per-step
        // wiring (step.r#in), keyed by the same port id.
        let step_inputs: HashMap<&str, &WorkflowStepInput> = step
            .r#in
            .iter()
            .filter_map(|wsip| Some((wsip.id.as_deref()?, wsip)))
            .collect();

        let (inputs, outputs) = match &doc {
            Some(doc) => {
                let mut inputs: Vec<FlowPort> = doc
                    .get_inputs()
                    .iter()
                    .map(|i| {
                        let id = i.id.clone().unwrap_or_default();
                        let wsip = step_inputs.get(id.as_str());
                        FlowPort {
                            data_type: input_type_label(&i.r#type),
                            link_merge: wsip
                                .and_then(|w| w.link_merge)
                                .map(link_merge_label)
                                .map(str::to_string),
                            pick_value: wsip
                                .and_then(|w| w.pick_value)
                                .map(pick_value_label)
                                .map(str::to_string),
                            id,
                        }
                    })
                    .collect();

                // step.r#in can bind a name the tool itself never declares --
                // e.g. a value only read by this step's own `when:` or
                // `valueFrom:` expression, legal CWL under
                // StepInputExpressionRequirement. Render those too, or the
                // edge feeding them has no port to land on and silently
                // fails to draw.
                let declared: HashSet<String> = inputs.iter().map(|p| p.id.clone()).collect();
                for (&id, &wsip) in &step_inputs {
                    if declared.contains(id) {
                        continue;
                    }
                    inputs.push(FlowPort {
                        id: id.to_string(),
                        data_type: "Any".to_string(),
                        link_merge: wsip.link_merge.map(link_merge_label).map(str::to_string),
                        pick_value: wsip.pick_value.map(pick_value_label).map(str::to_string),
                    });
                }

                let outputs = doc
                    .get_output_ids()
                    .iter()
                    .map(|id| FlowPort {
                        id: id.clone(),
                        data_type: get_output_type(doc, id)
                            .map(|t| output_type_label(&t))
                            .unwrap_or_default(),
                        link_merge: None,
                        pick_value: None,
                    })
                    .collect();

                (inputs, outputs)
            }
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
                when: step.when.clone(),
                scatter: step
                    .scatter
                    .as_ref()
                    .map(OneOrMany::as_many)
                    .unwrap_or_default(),
                scatter_method: step
                    .scatter_method
                    .map(scatter_method_label)
                    .map(str::to_string),
            },
        });

        for wsip in &step.r#in {
            let Some(target_port) = wsip.id.clone() else {
                continue;
            };
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
        let Some(id) = output.id.clone() else {
            continue;
        };
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
                    link_merge: None,
                    pick_value: None,
                }],
                outputs: vec![],
                run: None,
                diagnostics: vec![],
                status: None,
                when: None,
                scatter: vec![],
                scatter_method: None,
            },
        });

        if let Some(output_source) = &output.output_source {
            // `outputSource: some_input` (no slash) is a legal pass-through
            for src in output_source.as_many() {
                if let Some((from_ref, from_port)) = resolve_source(&src, &input_ids, &step_ids) {
                    view.edges
                        .push(make_edge(from_ref, &from_port, node_ref.clone(), &id));
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

/// Step ids that are themselves part of a cycle using petgraph
fn cyclic_step_ids(workflow: &Workflow) -> HashSet<String> {
    let mut graph = DiGraph::<&str, ()>::new();
    let mut index_of = HashMap::new();
    for step in &workflow.steps {
        if let Some(id) = step.id.as_deref() {
            index_of.entry(id).or_insert_with(|| graph.add_node(id));
        }
    }

    for step in &workflow.steps {
        let Some(step_id) = step.id.as_deref() else {
            continue;
        };
        let Some(&to) = index_of.get(step_id) else {
            continue;
        };
        for wsip in &step.r#in {
            let Some(source) = &wsip.source else { continue };
            for src in source.as_many() {
                let Some((dep, _)) = src.split_once('/') else {
                    continue;
                };
                if let Some(&from) = index_of.get(dep) {
                    graph.add_edge(from, to, ());
                }
            }
        }
    }

    tarjan_scc(&graph)
        .into_iter()
        .filter(|scc| scc.len() > 1 || graph.contains_edge(scc[0], scc[0]))
        .flat_map(|scc| scc.into_iter().map(|idx| graph[idx].to_string()))
        .collect()
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
fn input_type_label(t: &OneOrMany<InputType>) -> String {
    t.as_many()
        .iter()
        .map(single_input_type_label)
        .collect::<Vec<_>>()
        .join(" | ")
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

fn link_merge_label(m: LinkMergeMethod) -> &'static str {
    match m {
        LinkMergeMethod::MergeNested => "merge_nested",
        LinkMergeMethod::MergeFlattened => "merge_flattened",
    }
}

fn pick_value_label(m: PickValueMethod) -> &'static str {
    match m {
        PickValueMethod::FirstNonNull => "first_non_null",
        PickValueMethod::TheOnlyNonNull => "the_only_non_null",
        PickValueMethod::AllNonNull => "all_non_null",
    }
}

fn scatter_method_label(m: ScatterMethod) -> &'static str {
    match m {
        ScatterMethod::Dotproduct => "dotproduct",
        ScatterMethod::NestedCrossproduct => "nested_crossproduct",
        ScatterMethod::FlatCrossproduct => "flat_crossproduct",
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
        assert!(
            view.edges
                .iter()
                .any(|e| e.source == "input/x" && e.target == "step/x")
        );
        assert!(
            view.edges
                .iter()
                .any(|e| e.source == "step/x" && e.target == "output/result")
        );
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

    #[test]
    fn test_step_downstream_of_cycle_is_not_itself_flagged() {
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
- id: c
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
";
        let CWLDocument::Workflow(workflow) = commonwl::from_str(yaml).unwrap() else {
            panic!("Expected a workflow document")
        };
        let view = load_workflow_graph(&workflow, "workflow.cwl");

        let diagnostics_of = |id: &str| {
            view.nodes
                .iter()
                .find(|n| n.id == id)
                .map(|n| n.data.diagnostics.len())
                .unwrap()
        };
        assert!(diagnostics_of("step/a") > 0);
        assert!(diagnostics_of("step/b") > 0);
        assert_eq!(diagnostics_of("step/c"), 0);
    }

    #[test]
    fn test_scatter_when_and_step_input_wiring() {
        let yaml = r"
cwlVersion: v1.2
class: Workflow
inputs:
- id: a
  type: File[]
- id: b
  type: File[]
outputs: []
steps:
- id: process
  when: $(inputs.a != null)
  scatter:
  - a
  - b
  scatterMethod: dotproduct
  in:
  - id: a
    source: a
    linkMerge: merge_flattened
    pickValue: first_non_null
  - id: b
    source: b
  out:
  - out
  run:
    class: CommandLineTool
    inputs:
    - id: a
      type: File
    - id: b
      type: File
    outputs:
    - id: out
      type: File
";
        let CWLDocument::Workflow(workflow) = commonwl::from_str(yaml).unwrap() else {
            panic!("Expected a workflow document")
        };
        let view = load_workflow_graph(&workflow, "workflow.cwl");

        let step = view.nodes.iter().find(|n| n.id == "step/process").unwrap();
        assert_eq!(step.data.when.as_deref(), Some("$(inputs.a != null)"));
        assert_eq!(step.data.scatter, vec!["a".to_string(), "b".to_string()]);
        assert_eq!(step.data.scatter_method.as_deref(), Some("dotproduct"));

        let port_a = step.data.inputs.iter().find(|p| p.id == "a").unwrap();
        assert_eq!(port_a.link_merge.as_deref(), Some("merge_flattened"));
        assert_eq!(port_a.pick_value.as_deref(), Some("first_non_null"));

        let port_b = step.data.inputs.iter().find(|p| p.id == "b").unwrap();
        assert_eq!(port_b.link_merge, None);
        assert_eq!(port_b.pick_value, None);
    }

    // A step's `in:` can bind a name the underlying tool never declares --
    // e.g. a boolean only read by this step's own `when:` expression, legal
    // under StepInputExpressionRequirement (this is exactly the pattern in
    // container-registry/workflows/scan-image/workflow.cwl's `skip` input).
    // Building ports only from doc.get_inputs() drops that binding entirely,
    // so the edge feeding it has no port to land on and never renders.
    #[test]
    fn test_step_input_not_declared_by_tool_still_gets_a_port_and_edge() {
        let yaml = r"
cwlVersion: v1.2
class: Workflow
requirements:
- class: StepInputExpressionRequirement
inputs:
- id: image
  type: string
outputs: []
steps:
- id: check_index
  in:
  - id: image
    source: image
  out:
  - exists
  run:
    class: CommandLineTool
    inputs:
    - id: image
      type: string
    outputs:
    - id: exists
      type: boolean
- id: syft
  in:
  - id: image
    source: image
  - id: skip
    source: check_index/exists
  when: $(!inputs.skip)
  out:
  - output
  run:
    class: CommandLineTool
    inputs:
    - id: image
      type: string
    outputs:
    - id: output
      type: File
";
        let CWLDocument::Workflow(workflow) = commonwl::from_str(yaml).unwrap() else {
            panic!("Expected a workflow document")
        };
        let view = load_workflow_graph(&workflow, "workflow.cwl");

        let syft = view.nodes.iter().find(|n| n.id == "step/syft").unwrap();
        let skip_port = syft.data.inputs.iter().find(|p| p.id == "skip").unwrap();
        assert_eq!(skip_port.data_type, "Any");

        assert!(view.edges.iter().any(|e| e.source == "step/check_index"
            && e.source_handle == "exists"
            && e.target == "step/syft"
            && e.target_handle == "skip"));
    }
}
