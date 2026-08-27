use std::path::{Path, PathBuf};

use commonwl::{
    OneOrMany,
    documents::{CWLDocument, StringOrDocument, Workflow},
    format::format_cwl,
    inputs::InputType,
    load_cwl_file,
    outputs::CommandOutputParameterType,
};
use sciwin::authoring::workflow::{self, WorkflowSlot, check_slot_compatibility};
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter};

use crate::files::WorkflowChanged;
use crate::graph::{compute_revision, get_output_type};
use crate::graph_types::{NodeKind, NodeRef};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConnectionEndpoint {
    pub kind: NodeKind,
    pub id: String,
    pub port: String,
}

/// Everything a mutation command can refuse with, typed so the frontend can
/// react to each case
#[derive(Debug, Serialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum MutationError {
    /// The Monaco buffer for this file has unsaved edits; refuse rather than
    /// read stale text and clobber them on write.
    EditorDirty,
    /// The file on disk no longer matches the revision the caller loaded.
    StaleRevision,
    /// `$import`/`$graph` would be inlined/flattened permanently by writing
    /// this file back out, so mutation is refused rather than silently
    /// destroying it. See the Clean/Lossy gate in the migration doc.
    Lossy,
    IncompatibleTypes {
        reason: String,
    },
    InvalidConnection {
        reason: String,
    },
    NotFound {
        message: String,
    },
    Io {
        message: String,
    },
}

impl From<sciwin::authoring::AuthoringError> for MutationError {
    fn from(e: sciwin::authoring::AuthoringError) -> Self {
        match e {
            sciwin::authoring::AuthoringError::IncompatibleType { message } => {
                MutationError::IncompatibleTypes { reason: message }
            }
            e => MutationError::InvalidConnection {
                reason: e.to_string(),
            },
        }
    }
}

fn io_err(e: std::io::Error) -> MutationError {
    MutationError::Io {
        message: e.to_string(),
    }
}

/// Checked on raw bytes, before anything parses them
fn is_lossy(bytes: &[u8]) -> bool {
    let text = String::from_utf8_lossy(bytes);
    text.contains("$import") || text.contains("$graph")
}

fn load_for_mutation(path: &Path, revision: &str, dirty: bool) -> Result<Workflow, MutationError> {
    if dirty {
        return Err(MutationError::EditorDirty);
    }
    let bytes = std::fs::read(path).map_err(io_err)?;
    if compute_revision(&bytes) != revision {
        return Err(MutationError::StaleRevision);
    }
    if is_lossy(&bytes) {
        return Err(MutationError::Lossy);
    }
    let text = String::from_utf8(bytes).map_err(|_| MutationError::Io {
        message: "file is not valid UTF-8".into(),
    })?;
    let doc = commonwl::from_str(&text).map_err(|e| MutationError::InvalidConnection {
        reason: e.to_string(),
    })?;
    let CWLDocument::Workflow(workflow) = doc else {
        return Err(MutationError::InvalidConnection {
            reason: "not a Workflow document".into(),
        });
    };
    Ok(workflow)
}

fn save_workflow(workflow: &Workflow, path: &Path) -> Result<String, MutationError> {
    let doc = CWLDocument::Workflow(workflow.clone());
    let raw = serde_saphyr::to_string(&doc).map_err(|e| MutationError::Io {
        message: e.to_string(),
    })?;
    let formatted = format_cwl(&raw).map_err(|e| MutationError::Io {
        message: e.to_string(),
    })?;
    std::fs::write(path, &formatted).map_err(io_err)?;
    Ok(formatted)
}

fn resolve_step_tool_path(
    workflow: &Workflow,
    workflow_path: &Path,
    step_id: &str,
) -> Result<PathBuf, MutationError> {
    let step = workflow
        .steps
        .iter()
        .find(|s| s.id.as_deref() == Some(step_id))
        .ok_or_else(|| MutationError::NotFound {
            message: format!("step {step_id} not found"),
        })?;
    match &step.run {
        StringOrDocument::String(rel) => {
            Ok(workflow_path.parent().unwrap_or(workflow_path).join(rel))
        }
        StringOrDocument::Document(_) => Err(MutationError::InvalidConnection {
            reason: format!(
                "step {step_id} has an inline run: document, which isn't connectable yet"
            ),
        }),
    }
}

fn step_input_type(
    workflow: &Workflow,
    workflow_path: &Path,
    step_id: &str,
    port: &str,
) -> Result<OneOrMany<InputType>, MutationError> {
    let tool_path = resolve_step_tool_path(workflow, workflow_path, step_id)?;
    let doc = load_cwl_file(&tool_path, true).map_err(|e| MutationError::Io {
        message: e.to_string(),
    })?;
    doc.get_inputs()
        .iter()
        .find(|i| i.id.as_deref() == Some(port))
        .map(|i| i.r#type.clone())
        .ok_or_else(|| MutationError::NotFound {
            message: format!("{step_id}/{port} not found"),
        })
}

fn step_output_type(
    workflow: &Workflow,
    workflow_path: &Path,
    step_id: &str,
    port: &str,
) -> Result<CommandOutputParameterType, MutationError> {
    let tool_path = resolve_step_tool_path(workflow, workflow_path, step_id)?;
    let doc = load_cwl_file(&tool_path, true).map_err(|e| MutationError::Io {
        message: e.to_string(),
    })?;
    get_output_type(&doc, port).ok_or_else(|| MutationError::NotFound {
        message: format!("{step_id}/{port} not found"),
    })
}

fn emit_workflow_changed(app: &AppHandle, path: String, contents: &str) {
    let revision = compute_revision(contents.as_bytes());
    let _ = app.emit("workflow-changed", WorkflowChanged { path, revision });
}

/// Connects `from`/`to`, dispatching on node kind: workflow input -> step
/// input, step output -> step input, or step output -> workflow output. Any
/// other pairing is refused. Type compatibility is only checked step-to-step.
/// Returns the freshly written file contents, so the caller can hash them
/// for `workflow-changed` without a second disk read. Split from the
/// `#[tauri::command]` shim below so tests can call it without a live
/// `AppHandle`, same as `lsp::take_frame` vs `lsp_send`.
fn connect_workflow_nodes_impl(
    path: &Path,
    revision: &str,
    dirty: bool,
    from: &ConnectionEndpoint,
    to: &ConnectionEndpoint,
) -> Result<String, MutationError> {
    let mut wf = load_for_mutation(path, revision, dirty)?;

    match (from.kind, to.kind) {
        (NodeKind::Input, NodeKind::Step) => {
            let to_tool = resolve_step_tool_path(&wf, path, &to.id)?;
            let to_slot = WorkflowSlot::new(&to_tool, &to.id, &to.port);
            workflow::add_workflow_input_connection(&mut wf, path, &from.id, to_slot)?;
        }
        (NodeKind::Step, NodeKind::Step) => {
            let to_type = step_input_type(&wf, path, &to.id, &to.port)?;
            let from_type = step_output_type(&wf, path, &from.id, &from.port)?;
            if !check_slot_compatibility(&to_type, &from_type) {
                return Err(MutationError::IncompatibleTypes {
                    reason: format!(
                        "{}/{} does not accept {}/{}",
                        to.id, to.port, from.id, from.port
                    ),
                });
            }
            let from_tool = resolve_step_tool_path(&wf, path, &from.id)?;
            let to_tool = resolve_step_tool_path(&wf, path, &to.id)?;
            let from_slot = WorkflowSlot::new(&from_tool, &from.id, &from.port);
            let to_slot = WorkflowSlot::new(&to_tool, &to.id, &to.port);
            workflow::add_workflow_step_connection(&mut wf, path, from_slot, to_slot)?;
        }
        (NodeKind::Step, NodeKind::Output) => {
            let from_tool = resolve_step_tool_path(&wf, path, &from.id)?;
            let from_slot = WorkflowSlot::new(&from_tool, &from.id, &from.port);
            workflow::add_workflow_output_connection(&mut wf, path, from_slot, &to.id)?;
        }
        (from_kind, to_kind) => {
            return Err(MutationError::InvalidConnection {
                reason: format!("cannot connect {from_kind:?} to {to_kind:?}"),
            });
        }
    }

    save_workflow(&wf, path)
}

#[tauri::command]
pub fn connect_workflow_nodes(
    app: AppHandle,
    path: String,
    revision: String,
    dirty: bool,
    from: ConnectionEndpoint,
    to: ConnectionEndpoint,
) -> Result<(), MutationError> {
    let written = connect_workflow_nodes_impl(Path::new(&path), &revision, dirty, &from, &to)?;
    emit_workflow_changed(&app, path, &written);
    Ok(())
}

/// Removes the connection `from -> to`. Only that one wire, other sources
/// on a multi-source `to`, and the nodes themselves, are left alone.
fn disconnect_workflow_nodes_impl(
    path: &Path,
    revision: &str,
    dirty: bool,
    from: &ConnectionEndpoint,
    to: &ConnectionEndpoint,
) -> Result<String, MutationError> {
    let mut wf = load_for_mutation(path, revision, dirty)?;

    match (from.kind, to.kind) {
        (NodeKind::Input, NodeKind::Step) => {
            workflow::remove_workflow_input_connection(&mut wf, &from.id, &to.id, &to.port, false)?;
        }
        (NodeKind::Step, NodeKind::Step) => {
            workflow::remove_workflow_step_connection(
                &mut wf, &from.id, &from.port, &to.id, &to.port,
            )?;
        }
        (NodeKind::Step, NodeKind::Output) => {
            workflow::remove_workflow_output_connection(
                &mut wf, &from.id, &from.port, &to.id, false,
            )?;
        }
        (from_kind, to_kind) => {
            return Err(MutationError::InvalidConnection {
                reason: format!("cannot disconnect {from_kind:?} from {to_kind:?}"),
            });
        }
    }

    save_workflow(&wf, path)
}

#[tauri::command]
pub fn disconnect_workflow_nodes(
    app: AppHandle,
    path: String,
    revision: String,
    dirty: bool,
    from: ConnectionEndpoint,
    to: ConnectionEndpoint,
) -> Result<(), MutationError> {
    let written = disconnect_workflow_nodes_impl(Path::new(&path), &revision, dirty, &from, &to)?;
    emit_workflow_changed(&app, path, &written);
    Ok(())
}

/// Removes every source string anywhere in the workflow, a step's `in:`
/// source or a workflow output's `outputSource`, that `references` accepts,
/// collapsing a slot to `None`/`One` the same way
/// remove_workflow_step_input_source_mut does. Used to cascade-delete a
/// node's connections as part of removing the node itself. The frontend
/// confirms that cascade with the user; nothing here re-decides it.
fn strip_sources_everywhere(workflow: &mut Workflow, references: impl Fn(&str) -> bool) {
    let collapse = |remaining: Vec<String>| match remaining.len() {
        0 => None,
        1 => Some(OneOrMany::One(remaining.into_iter().next().unwrap())),
        _ => Some(OneOrMany::Many(remaining)),
    };

    for step in &mut workflow.steps {
        for wsip in &mut step.r#in {
            let Some(current) = wsip.source.take() else {
                continue;
            };
            let remaining: Vec<String> = current
                .into_many()
                .into_iter()
                .filter(|s| !references(s))
                .collect();
            wsip.source = collapse(remaining);
        }
    }
    for output in &mut workflow.outputs {
        let Some(current) = output.output_source.take() else {
            continue;
        };
        let remaining: Vec<String> = current
            .into_many()
            .into_iter()
            .filter(|s| !references(s))
            .collect();
        output.output_source = collapse(remaining);
    }
}

/// Deletes a workflow input, output or step, first stripping every
/// connection touching it (a workflow output is never itself a source, so
/// removing its own declaration is the whole cascade for that case). The
/// frontend confirms this cascade with the user before calling this command
/// at all; nothing here re-checks or re-asks.
fn delete_workflow_node_impl(
    path: &Path,
    revision: &str,
    dirty: bool,
    node: &NodeRef,
) -> Result<String, MutationError> {
    let mut wf = load_for_mutation(path, revision, dirty)?;

    match node.kind {
        NodeKind::Input => {
            strip_sources_everywhere(&mut wf, |s| s == node.id);
            wf.remove_workflow_input_mut(&node.id)
                .map_err(|e| MutationError::NotFound {
                    message: e.to_string(),
                })?;
        }
        NodeKind::Output => {
            wf.remove_workflow_output_mut(&node.id)
                .map_err(|e| MutationError::NotFound {
                    message: e.to_string(),
                })?;
        }
        NodeKind::Step => {
            let prefix = format!("{}/", node.id);
            strip_sources_everywhere(&mut wf, |s| s.starts_with(&prefix));
            wf.remove_workflow_step_mut(&node.id)
                .map_err(|e| MutationError::NotFound {
                    message: e.to_string(),
                })?;
        }
    }

    save_workflow(&wf, path)
}

#[tauri::command]
pub fn delete_workflow_node(
    app: AppHandle,
    path: String,
    revision: String,
    dirty: bool,
    node: NodeRef,
) -> Result<(), MutationError> {
    let written = delete_workflow_node_impl(Path::new(&path), &revision, dirty, &node)?;
    emit_workflow_changed(&app, path, &written);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{fs, path::PathBuf};
    use tempfile::tempdir;

    const PRODUCER_TOOL: &str = r"
cwlVersion: v1.2
class: CommandLineTool
inputs:
- id: x
  type: string
outputs:
- id: out
  type: string
baseCommand: echo
";

    const CONSUMER_TOOL: &str = r"
cwlVersion: v1.2
class: CommandLineTool
inputs:
- id: y
  type: string
outputs:
- id: done
  type: string
baseCommand: echo
";

    const WORKFLOW: &str = r"
cwlVersion: v1.2
class: Workflow
inputs:
- id: image
  type: string
- id: extra
  type: string
outputs: []
steps:
- id: producer
  in:
  - id: x
    source: image
  out:
  - out
  run: producer.cwl
- id: consumer
  in: []
  out:
  - done
  run: consumer.cwl
";

    /// Writes the fixture workflow (`producer`, and `consumer` with its `y`
    /// input still unconnected) into a fresh temp dir.
    fn setup() -> (tempfile::TempDir, PathBuf, String) {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("producer.cwl"), PRODUCER_TOOL).unwrap();
        fs::write(dir.path().join("consumer.cwl"), CONSUMER_TOOL).unwrap();
        let workflow_path = dir.path().join("workflow.cwl");
        fs::write(&workflow_path, WORKFLOW).unwrap();
        let revision = compute_revision(WORKFLOW.as_bytes());
        (dir, workflow_path, revision)
    }

    fn consumer_y_source(cwl: &str) -> Option<OneOrMany<String>> {
        let CWLDocument::Workflow(wf) = commonwl::from_str(cwl).unwrap() else {
            panic!("expected a workflow")
        };
        wf.steps
            .iter()
            .find(|s| s.id.as_deref() == Some("consumer"))
            .and_then(|s| s.r#in.iter().find(|i| i.id.as_deref() == Some("y")))
            .and_then(|i| i.source.clone())
    }

    fn endpoint(kind: NodeKind, id: &str, port: &str) -> ConnectionEndpoint {
        ConnectionEndpoint {
            kind,
            id: id.to_string(),
            port: port.to_string(),
        }
    }

    #[test]
    fn connect_step_to_step_writes_source_entry() {
        let (_dir, path, revision) = setup();
        let from = endpoint(NodeKind::Step, "producer", "out");
        let to = endpoint(NodeKind::Step, "consumer", "y");

        let written = connect_workflow_nodes_impl(&path, &revision, false, &from, &to).unwrap();

        assert_eq!(
            consumer_y_source(&written),
            Some(OneOrMany::One("producer/out".to_string()))
        );
        assert_eq!(
            fs::read_to_string(&path).unwrap(),
            written,
            "must actually be on disk, not just returned"
        );
    }

    #[test]
    fn connect_refuses_when_editor_is_dirty() {
        let (_dir, path, revision) = setup();
        let before = fs::read_to_string(&path).unwrap();
        let from = endpoint(NodeKind::Step, "producer", "out");
        let to = endpoint(NodeKind::Step, "consumer", "y");

        let result = connect_workflow_nodes_impl(&path, &revision, true, &from, &to);

        assert!(matches!(result, Err(MutationError::EditorDirty)));
        assert_eq!(
            fs::read_to_string(&path).unwrap(),
            before,
            "must not touch the file"
        );
    }

    #[test]
    fn connect_refuses_on_stale_revision() {
        let (_dir, path, _revision) = setup();
        let before = fs::read_to_string(&path).unwrap();
        let from = endpoint(NodeKind::Step, "producer", "out");
        let to = endpoint(NodeKind::Step, "consumer", "y");

        let result = connect_workflow_nodes_impl(&path, "not-the-real-revision", false, &from, &to);

        assert!(matches!(result, Err(MutationError::StaleRevision)));
        assert_eq!(fs::read_to_string(&path).unwrap(), before);
    }

    #[test]
    fn connect_step_to_step_refuses_incompatible_types() {
        let dir = tempdir().unwrap();
        fs::write(
            dir.path().join("producer.cwl"),
            r"
cwlVersion: v1.2
class: CommandLineTool
inputs: []
outputs:
- id: out
  type: File
baseCommand: echo
",
        )
        .unwrap();
        fs::write(dir.path().join("consumer.cwl"), CONSUMER_TOOL).unwrap(); // y: string
        let workflow = r"
cwlVersion: v1.2
class: Workflow
inputs: []
outputs: []
steps:
- id: producer
  in: []
  out:
  - out
  run: producer.cwl
- id: consumer
  in: []
  out:
  - done
  run: consumer.cwl
";
        let path = dir.path().join("workflow.cwl");
        fs::write(&path, workflow).unwrap();
        let revision = compute_revision(workflow.as_bytes());

        let from = endpoint(NodeKind::Step, "producer", "out");
        let to = endpoint(NodeKind::Step, "consumer", "y");
        let result = connect_workflow_nodes_impl(&path, &revision, false, &from, &to);

        assert!(matches!(
            result,
            Err(MutationError::IncompatibleTypes { .. })
        ));
    }
    #[test]
    fn connect_input_to_step_refuses_type_mismatch_with_existing_input() {
        let dir = tempdir().unwrap();
        fs::write(
            dir.path().join("consumer.cwl"),
            r"
cwlVersion: v1.2
class: CommandLineTool
inputs:
- id: y
  type: File
outputs:
- id: done
  type: File
baseCommand: echo
",
        )
        .unwrap();
        let workflow = r"
cwlVersion: v1.2
class: Workflow
inputs:
- id: image
  type: string
outputs: []
steps:
- id: consumer
  in: []
  out:
  - done
  run: consumer.cwl
";
        let path = dir.path().join("workflow.cwl");
        fs::write(&path, workflow).unwrap();
        let revision = compute_revision(workflow.as_bytes());

        let result = connect_workflow_nodes_impl(
            &path,
            &revision,
            false,
            &endpoint(NodeKind::Input, "image", "image"),
            &endpoint(NodeKind::Step, "consumer", "y"),
        );

        assert!(matches!(
            result,
            Err(MutationError::IncompatibleTypes { .. })
        ));
    }

    fn node(kind: NodeKind, id: &str) -> NodeRef {
        NodeRef::new(kind, id)
    }

    #[test]
    fn delete_isolated_step_removes_it() {
        let (_dir, path, revision) = setup();

        let written =
            delete_workflow_node_impl(&path, &revision, false, &node(NodeKind::Step, "consumer"))
                .unwrap();

        let CWLDocument::Workflow(wf) = commonwl::from_str(&written).unwrap() else {
            panic!("expected a workflow")
        };
        assert!(wf.steps.iter().all(|s| s.id.as_deref() != Some("consumer")));
        assert!(
            wf.steps.iter().any(|s| s.id.as_deref() == Some("producer")),
            "unrelated step must survive"
        );
    }

    #[test]
    fn delete_connected_input_strips_source_from_step() {
        let (_dir, path, revision) = setup();

        let written =
            delete_workflow_node_impl(&path, &revision, false, &node(NodeKind::Input, "image"))
                .unwrap();

        let CWLDocument::Workflow(wf) = commonwl::from_str(&written).unwrap() else {
            panic!("expected a workflow")
        };
        assert!(wf.inputs.iter().all(|i| i.id.as_deref() != Some("image")));
        let x_source = wf
            .steps
            .iter()
            .find(|s| s.id.as_deref() == Some("producer"))
            .and_then(|s| s.r#in.iter().find(|i| i.id.as_deref() == Some("x")))
            .and_then(|i| i.source.clone());
        assert_eq!(
            x_source, None,
            "the only source on that slot is gone, so it must collapse to None"
        );
    }

    #[test]
    fn delete_connected_step_strips_prefixed_sources_from_other_steps_and_outputs() {
        let (_dir, path, revision) = setup();

        let written = connect_workflow_nodes_impl(
            &path,
            &revision,
            false,
            &endpoint(NodeKind::Step, "producer", "out"),
            &endpoint(NodeKind::Step, "consumer", "y"),
        )
        .unwrap();
        let revision = compute_revision(written.as_bytes());
        let written = connect_workflow_nodes_impl(
            &path,
            &revision,
            false,
            &endpoint(NodeKind::Step, "producer", "out"),
            &endpoint(NodeKind::Output, "result", "result"),
        )
        .unwrap();
        let revision = compute_revision(written.as_bytes());

        let written =
            delete_workflow_node_impl(&path, &revision, false, &node(NodeKind::Step, "producer"))
                .unwrap();

        let CWLDocument::Workflow(wf) = commonwl::from_str(&written).unwrap() else {
            panic!("expected a workflow")
        };
        assert!(wf.steps.iter().all(|s| s.id.as_deref() != Some("producer")));
        assert_eq!(
            consumer_y_source(&written),
            None,
            "producer/y source must be stripped, not left dangling"
        );
        let result_source = wf
            .outputs
            .iter()
            .find(|o| o.id.as_deref() == Some("result"))
            .and_then(|o| o.output_source.clone());
        assert_eq!(
            result_source, None,
            "outputSource referencing the deleted step must be stripped too"
        );
        assert!(
            wf.outputs.iter().any(|o| o.id.as_deref() == Some("result")),
            "the output declaration itself is untouched by deleting a step"
        );
    }

    #[test]
    fn delete_output_removes_declaration_without_touching_its_source_step() {
        let (_dir, path, revision) = setup();

        let written = connect_workflow_nodes_impl(
            &path,
            &revision,
            false,
            &endpoint(NodeKind::Step, "producer", "out"),
            &endpoint(NodeKind::Output, "result", "result"),
        )
        .unwrap();
        let revision = compute_revision(written.as_bytes());

        let written =
            delete_workflow_node_impl(&path, &revision, false, &node(NodeKind::Output, "result"))
                .unwrap();

        let CWLDocument::Workflow(wf) = commonwl::from_str(&written).unwrap() else {
            panic!("expected a workflow")
        };
        assert!(wf.outputs.iter().all(|o| o.id.as_deref() != Some("result")));
        assert!(
            wf.steps.iter().any(|s| s.id.as_deref() == Some("producer")),
            "the producer step must survive"
        );
    }

    #[test]
    fn delete_missing_node_errors() {
        let (_dir, path, revision) = setup();

        let result =
            delete_workflow_node_impl(&path, &revision, false, &node(NodeKind::Step, "nope"));

        assert!(matches!(result, Err(MutationError::NotFound { .. })));
    }

    #[test]
    fn delete_refuses_when_editor_is_dirty() {
        let (_dir, path, revision) = setup();
        let before = fs::read_to_string(&path).unwrap();

        let result =
            delete_workflow_node_impl(&path, &revision, true, &node(NodeKind::Step, "consumer"));

        assert!(matches!(result, Err(MutationError::EditorDirty)));
        assert_eq!(
            fs::read_to_string(&path).unwrap(),
            before,
            "must not touch the file"
        );
    }

    #[test]
    fn disconnect_removes_only_the_named_source_from_a_multi_source_input() {
        let (_dir, path, revision) = setup();

        let written = connect_workflow_nodes_impl(
            &path,
            &revision,
            false,
            &endpoint(NodeKind::Step, "producer", "out"),
            &endpoint(NodeKind::Step, "consumer", "y"),
        )
        .unwrap();
        let revision = compute_revision(written.as_bytes());

        let written = connect_workflow_nodes_impl(
            &path,
            &revision,
            false,
            &endpoint(NodeKind::Input, "extra", "extra"),
            &endpoint(NodeKind::Step, "consumer", "y"),
        )
        .unwrap();
        assert_eq!(
            consumer_y_source(&written),
            Some(OneOrMany::Many(vec![
                "producer/out".to_string(),
                "extra".to_string()
            ])),
            "both connections must land on the same multi-source slot, not overwrite each other"
        );
        let revision = compute_revision(written.as_bytes());

        let written = disconnect_workflow_nodes_impl(
            &path,
            &revision,
            false,
            &endpoint(NodeKind::Step, "producer", "out"),
            &endpoint(NodeKind::Step, "consumer", "y"),
        )
        .unwrap();

        assert_eq!(
            consumer_y_source(&written),
            Some(OneOrMany::One("extra".to_string())),
            "removing one source must leave the other, collapsed back to `One`"
        );
    }
}
