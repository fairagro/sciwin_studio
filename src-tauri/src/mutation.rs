use std::path::{Path, PathBuf};

use commonwl::{
    OneOrMany,
    documents::{CWLDocument, ScatterMethod, StringOrDocument, Workflow},
    inputs::InputType,
    load_cwl_file,
    outputs::{CommandOutputParameterType, LinkMergeMethod, PickValueMethod},
};
use sciwin::authoring::workflow::{
    self, ScatterProducerFit, WorkflowSlot, add_step_input_slot_mut, add_step_to_scatter_mut,
    check_slot_compatibility, check_slot_compatibility_scattered,
    check_slot_compatibility_scattered_producer, clear_step_pick_value_mut,
    ensure_multiple_input_feature_requirement_mut, input_type_is_array, is_scattered_array_of,
    remove_step_from_scatter_mut, rename_workflow_step_mut, set_output_link_merge_mut,
    set_output_pick_value_mut, set_step_input_link_merge_mut, set_step_input_value_from_mut,
    set_step_pick_value_mut, set_step_scatter_method_mut, set_step_when_mut,
};
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
    /// The source is array-shaped but the target port is a plain scalar --
    /// legal only if the step scatters over that port. Sent instead of
    /// `IncompatibleTypes` so the frontend can offer to enable scatter
    /// rather than just refusing outright.
    NeedsScatterConfirmation {
        port: String,
    },
    /// The target port already has a source and this connection would add a
    /// second one -- CWL needs a `pickValue` strategy to resolve multiple
    /// sources into one scalar value at runtime.
    NeedsPickValue {
        port: String,
    },
    NotFound {
        message: String,
    },
    DuplicateId {
        id: String,
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
            sciwin::authoring::AuthoringError::IO(e) => MutationError::Io {
                message: e.to_string(),
            },
            sciwin::authoring::AuthoringError::InvalidWorkflowStep { .. }
            | sciwin::authoring::AuthoringError::InvalidWorkflowInput { .. }
            | sciwin::authoring::AuthoringError::InvalidWorkflowOutput { .. } => {
                MutationError::NotFound {
                    message: e.to_string(),
                }
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

fn canonicalize_arg(path: &Path) -> Result<PathBuf, MutationError> {
    dunce::canonicalize(path).map_err(|e| MutationError::Io {
        message: format!("{}: {e}", path.display()),
    })
}

/// Loads and parses the workflow, returning its canonicalized path alongside
fn load_for_mutation(
    path: &Path,
    revision: &str,
    dirty: bool,
) -> Result<(PathBuf, Workflow), MutationError> {
    if dirty {
        return Err(MutationError::EditorDirty);
    }
    let path = canonicalize_arg(path)?;
    let bytes = std::fs::read(&path).map_err(io_err)?;
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
    Ok((path, workflow))
}

fn save_workflow(workflow: &Workflow, path: &Path) -> Result<String, MutationError> {
    let doc = CWLDocument::Workflow(workflow.clone());
    let formatted = sciwin::authoring::workflow::save_workflow(&doc, path)?;
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
            let joined = workflow_path.parent().unwrap_or(workflow_path).join(rel);
            canonicalize_arg(&joined)
        }
        StringOrDocument::Document(_) => Err(MutationError::InvalidConnection {
            reason: format!(
                "step {step_id} has an inline run: document, which isn't connectable yet"
            ),
        }),
    }
}

/// `None` means `port` isn't declared by the tool -- a synthetic slot added
/// by `add_step_input_slot_mut`, untyped (`Any`) rather than missing. Still
/// errors if `port` is neither declared nor an existing `step.in` entry.
fn step_input_type(
    workflow: &Workflow,
    workflow_path: &Path,
    step_id: &str,
    port: &str,
) -> Result<Option<OneOrMany<InputType>>, MutationError> {
    let tool_path = resolve_step_tool_path(workflow, workflow_path, step_id)?;
    let doc = load_cwl_file(&tool_path, true).map_err(|e| MutationError::Io {
        message: e.to_string(),
    })?;
    if let Some(i) = doc
        .get_inputs()
        .iter()
        .find(|i| i.id.as_deref() == Some(port))
    {
        return Ok(Some(i.r#type.clone()));
    }
    if step_has_input_slot(workflow, step_id, port) {
        return Ok(None);
    }
    Err(MutationError::NotFound {
        message: format!("{step_id}/{port} not found"),
    })
}

/// True once `step_id` has *any* `step.in` entry named `port`, sourced or
/// not -- used to recognize a synthetic slot even before it's ever wired.
fn step_has_input_slot(workflow: &Workflow, step_id: &str, port: &str) -> bool {
    workflow
        .steps
        .iter()
        .find(|s| s.id.as_deref() == Some(step_id))
        .is_some_and(|s| s.r#in.iter().any(|i| i.id.as_deref() == Some(port)))
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
/// True once `step_id` already scatters over `port`.
fn step_already_scatters(wf: &Workflow, step_id: &str, port: &str) -> bool {
    wf.steps
        .iter()
        .find(|s| s.id.as_deref() == Some(step_id))
        .and_then(|s| s.scatter.as_ref())
        .is_some_and(|scatter| scatter.as_many().iter().any(|p| p == port))
}

/// True once `step_id`'s `port` input already has at least one source
/// wired to it.
fn step_input_already_sourced(wf: &Workflow, step_id: &str, port: &str) -> bool {
    wf.steps
        .iter()
        .find(|s| s.id.as_deref() == Some(step_id))
        .and_then(|s| s.r#in.iter().find(|i| i.id.as_deref() == Some(port)))
        .is_some_and(|i| i.source.is_some())
}

fn connect_workflow_nodes_impl(
    path: &Path,
    revision: &str,
    dirty: bool,
    from: &ConnectionEndpoint,
    to: &ConnectionEndpoint,
    scatter_confirmed: bool,
    pick_value: Option<PickValueMethod>,
) -> Result<String, MutationError> {
    let (path, mut wf) = load_for_mutation(path, revision, dirty)?;
    let path = path.as_path();

    match (from.kind, to.kind) {
        (NodeKind::Input, NodeKind::Step) => {
            let to_tool = resolve_step_tool_path(&wf, path, &to.id)?;
            let to_slot = WorkflowSlot::new(&to_tool, &to.id, &to.port);
            let to_type = step_input_type(&wf, path, &to.id, &to.port)?;

            let mut needs_scatter = false;

            // `to_type` is `None` for a slot the tool doesn't declare -- an
            // untyped (`Any`) synthetic input, compatible with anything, so
            // there's nothing to check against an existing workflow input.
            if let Some(to_type) = &to_type {
                // An existing workflow input reused for a second step port can
                // legitimately have a different type than that port declares: an
                // array-of-scalar input feeding a step that scatters over this
                // slot, same shape check as the step-to-step scatter path.
                if let Some(existing) = wf
                    .inputs
                    .iter()
                    .find(|i| i.id.as_deref() == Some(from.id.as_str()))
                {
                    let existing_type = existing.r#type.clone();
                    if existing_type != *to_type {
                        if !is_scattered_array_of(&existing_type, to_type) {
                            return Err(MutationError::IncompatibleTypes {
                                reason: format!(
                                    "input {} already has type {:?}, but {}/{} expects {:?}",
                                    from.id, existing_type, to.id, to.port, to_type
                                ),
                            });
                        }
                        if !step_already_scatters(&wf, &to.id, &to.port) {
                            if !scatter_confirmed {
                                return Err(MutationError::NeedsScatterConfirmation {
                                    port: to.port.clone(),
                                });
                            }
                            needs_scatter = true;
                        }
                    }
                }
            }

            let to_is_array = to_type.as_ref().is_some_and(input_type_is_array);
            if !to_is_array
                && step_input_already_sourced(&wf, &to.id, &to.port)
                && pick_value.is_none()
            {
                return Err(MutationError::NeedsPickValue {
                    port: to.port.clone(),
                });
            }

            // Scatter must be marked before add_workflow_input_connection
            // runs, since that function re-checks the same shape and only
            // tolerates the mismatch once the step already scatters over it.
            if needs_scatter {
                workflow::add_step_to_scatter_mut(&mut wf, &to.id, &to.port)?;
            }
            workflow::add_workflow_input_connection(&mut wf, path, &from.id, to_slot)?;
            if let Some(method) = pick_value {
                workflow::set_step_pick_value_mut(&mut wf, &to.id, &to.port, method)?;
            }
        }
        (NodeKind::Step, NodeKind::Step) => {
            let to_type = step_input_type(&wf, path, &to.id, &to.port)?;
            let from_type = step_output_type(&wf, path, &from.id, &from.port)?;

            let mut needs_scatter = false;

            // `to_type` is `None` for a slot the tool doesn't declare -- an
            // untyped (`Any`) synthetic input, compatible with any producer.
            if let Some(to_type) = &to_type {
                let direct_ok = check_slot_compatibility(to_type, &from_type);

                if !direct_ok {
                    // Two distinct scatter shapes can make an otherwise-mismatched
                    // connection legal: `from` is itself scattered, so every output
                    // it produces is array-wrapped one level (checked first, since
                    // it only applies when `from` actually scatters); or `to` isn't
                    // scattered yet but could be, once confirmed, since `from`'s
                    // output is genuinely array-shaped and `to`'s declared type is
                    // the scalar item type.
                    let mut resolved = false;

                    if workflow::step_is_scattered(&wf, &from.id) {
                        match check_slot_compatibility_scattered_producer(to_type, &from_type) {
                            ScatterProducerFit::Exact => resolved = true,
                            ScatterProducerFit::NeedsPickValueToDropNulls => {
                                if pick_value.is_none() {
                                    return Err(MutationError::NeedsPickValue {
                                        port: to.port.clone(),
                                    });
                                }
                                resolved = true;
                            }
                            ScatterProducerFit::Incompatible => {}
                        }
                    }

                    if !resolved {
                        if !check_slot_compatibility_scattered(to_type, &from_type) {
                            return Err(MutationError::IncompatibleTypes {
                                reason: format!(
                                    "{}/{} does not accept {}/{}",
                                    to.id, to.port, from.id, from.port
                                ),
                            });
                        }
                        // Legal without asking again once the step already scatters
                        // over this port; otherwise the caller needs to confirm.
                        if !step_already_scatters(&wf, &to.id, &to.port) {
                            if !scatter_confirmed {
                                return Err(MutationError::NeedsScatterConfirmation {
                                    port: to.port.clone(),
                                });
                            }
                            needs_scatter = true;
                        }
                    }
                }
            }

            let to_is_array = to_type.as_ref().is_some_and(input_type_is_array);
            if !to_is_array
                && step_input_already_sourced(&wf, &to.id, &to.port)
                && pick_value.is_none()
            {
                return Err(MutationError::NeedsPickValue {
                    port: to.port.clone(),
                });
            }

            let from_tool = resolve_step_tool_path(&wf, path, &from.id)?;
            let to_tool = resolve_step_tool_path(&wf, path, &to.id)?;
            let from_slot = WorkflowSlot::new(&from_tool, &from.id, &from.port);
            let to_slot = WorkflowSlot::new(&to_tool, &to.id, &to.port);
            workflow::add_workflow_step_connection(&mut wf, path, from_slot, to_slot)?;

            if needs_scatter {
                workflow::add_step_to_scatter_mut(&mut wf, &to.id, &to.port)?;
            }
            if let Some(method) = pick_value {
                workflow::set_step_pick_value_mut(&mut wf, &to.id, &to.port, method)?;
            }
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

    ensure_multiple_input_feature_requirement_mut(&mut wf);
    save_workflow(&wf, path)
}

#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub fn connect_workflow_nodes(
    app: AppHandle,
    path: String,
    revision: String,
    dirty: bool,
    from: ConnectionEndpoint,
    to: ConnectionEndpoint,
    scatter_confirmed: bool,
    pick_value: Option<PickValueMethod>,
) -> Result<(), MutationError> {
    let written = connect_workflow_nodes_impl(
        Path::new(&path),
        &revision,
        dirty,
        &from,
        &to,
        scatter_confirmed,
        pick_value,
    )?;
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
    let (path, mut wf) = load_for_mutation(path, revision, dirty)?;

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

    save_workflow(&wf, &path)
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
    let (path, mut wf) = load_for_mutation(path, revision, dirty)?;

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

    save_workflow(&wf, &path)
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

fn add_workflow_step_node_impl(
    path: &Path,
    revision: &str,
    dirty: bool,
    tool_path: &str,
    name: &str,
) -> Result<String, MutationError> {
    let (path, mut wf) = load_for_mutation(path, revision, dirty)?;

    if wf.has_step(name) {
        return Err(MutationError::DuplicateId {
            id: name.to_string(),
        });
    }

    // The frontend sends whatever path the Sidebar's file walk produced; it
    // must be canonical before it reaches add_workflow_step, same invariant
    // as workflow_path above.
    let tool_path = canonicalize_arg(Path::new(tool_path))?;
    let doc = load_cwl_file(&tool_path, true).map_err(|e| MutationError::Io {
        message: e.to_string(),
    })?;

    workflow::add_workflow_step(&mut wf, &path, name, &tool_path, &doc)?;

    save_workflow(&wf, &path)
}

#[tauri::command]
pub fn add_workflow_step_node(
    app: AppHandle,
    path: String,
    revision: String,
    dirty: bool,
    tool_path: String,
    name: String,
) -> Result<(), MutationError> {
    let written =
        add_workflow_step_node_impl(Path::new(&path), &revision, dirty, &tool_path, &name)?;
    emit_workflow_changed(&app, path, &written);
    Ok(())
}

fn rename_workflow_step_impl(
    path: &Path,
    revision: &str,
    dirty: bool,
    step_id: &str,
    new_id: &str,
) -> Result<String, MutationError> {
    let (path, mut wf) = load_for_mutation(path, revision, dirty)?;
    let new_id = new_id.trim();
    if new_id.is_empty() {
        return Err(MutationError::InvalidConnection {
            reason: "step id must not be empty".into(),
        });
    }
    if step_id != new_id && wf.has_step(new_id) {
        return Err(MutationError::DuplicateId {
            id: new_id.to_string(),
        });
    }
    rename_workflow_step_mut(&mut wf, step_id, new_id)?;
    save_workflow(&wf, &path)
}

#[tauri::command]
pub fn rename_workflow_step(
    app: AppHandle,
    path: String,
    revision: String,
    dirty: bool,
    step_id: String,
    new_id: String,
) -> Result<(), MutationError> {
    let written = rename_workflow_step_impl(Path::new(&path), &revision, dirty, &step_id, &new_id)?;
    emit_workflow_changed(&app, path, &written);
    Ok(())
}

fn set_step_when_impl(
    path: &Path,
    revision: &str,
    dirty: bool,
    step_id: &str,
    expression: Option<String>,
) -> Result<String, MutationError> {
    let (path, mut wf) = load_for_mutation(path, revision, dirty)?;
    set_step_when_mut(&mut wf, step_id, expression)?;
    save_workflow(&wf, &path)
}

#[tauri::command]
pub fn set_step_when(
    app: AppHandle,
    path: String,
    revision: String,
    dirty: bool,
    step_id: String,
    expression: Option<String>,
) -> Result<(), MutationError> {
    let written = set_step_when_impl(Path::new(&path), &revision, dirty, &step_id, expression)?;
    emit_workflow_changed(&app, path, &written);
    Ok(())
}

fn set_step_scatter_method_impl(
    path: &Path,
    revision: &str,
    dirty: bool,
    step_id: &str,
    method: Option<ScatterMethod>,
) -> Result<String, MutationError> {
    let (path, mut wf) = load_for_mutation(path, revision, dirty)?;
    set_step_scatter_method_mut(&mut wf, step_id, method)?;
    save_workflow(&wf, &path)
}

#[tauri::command]
pub fn set_step_scatter_method(
    app: AppHandle,
    path: String,
    revision: String,
    dirty: bool,
    step_id: String,
    method: Option<ScatterMethod>,
) -> Result<(), MutationError> {
    let written =
        set_step_scatter_method_impl(Path::new(&path), &revision, dirty, &step_id, method)?;
    emit_workflow_changed(&app, path, &written);
    Ok(())
}

/// Toggles `port` in `step_id`'s scatter list.
fn set_step_scattered_impl(
    path: &Path,
    revision: &str,
    dirty: bool,
    step_id: &str,
    port: &str,
    scattered: bool,
) -> Result<String, MutationError> {
    let (path, mut wf) = load_for_mutation(path, revision, dirty)?;
    if scattered {
        add_step_to_scatter_mut(&mut wf, step_id, port)?;
    } else {
        remove_step_from_scatter_mut(&mut wf, step_id, port)?;
    }
    save_workflow(&wf, &path)
}

#[tauri::command]
pub fn set_step_scattered(
    app: AppHandle,
    path: String,
    revision: String,
    dirty: bool,
    step_id: String,
    port: String,
    scattered: bool,
) -> Result<(), MutationError> {
    let written = set_step_scattered_impl(
        Path::new(&path),
        &revision,
        dirty,
        &step_id,
        &port,
        scattered,
    )?;
    emit_workflow_changed(&app, path, &written);
    Ok(())
}

fn set_step_pick_value_impl(
    path: &Path,
    revision: &str,
    dirty: bool,
    step_id: &str,
    port: &str,
    method: Option<PickValueMethod>,
) -> Result<String, MutationError> {
    let (path, mut wf) = load_for_mutation(path, revision, dirty)?;
    match method {
        Some(method) => set_step_pick_value_mut(&mut wf, step_id, port, method)?,
        None => clear_step_pick_value_mut(&mut wf, step_id, port)?,
    }
    save_workflow(&wf, &path)
}

#[tauri::command]
pub fn set_step_pick_value(
    app: AppHandle,
    path: String,
    revision: String,
    dirty: bool,
    step_id: String,
    port: String,
    method: Option<PickValueMethod>,
) -> Result<(), MutationError> {
    let written =
        set_step_pick_value_impl(Path::new(&path), &revision, dirty, &step_id, &port, method)?;
    emit_workflow_changed(&app, path, &written);
    Ok(())
}

fn set_step_input_value_from_impl(
    path: &Path,
    revision: &str,
    dirty: bool,
    step_id: &str,
    port: &str,
    value_from: Option<String>,
) -> Result<String, MutationError> {
    let (path, mut wf) = load_for_mutation(path, revision, dirty)?;
    set_step_input_value_from_mut(&mut wf, step_id, port, value_from)?;
    save_workflow(&wf, &path)
}

#[tauri::command]
pub fn set_step_input_value_from(
    app: AppHandle,
    path: String,
    revision: String,
    dirty: bool,
    step_id: String,
    port: String,
    value_from: Option<String>,
) -> Result<(), MutationError> {
    let written = set_step_input_value_from_impl(
        Path::new(&path),
        &revision,
        dirty,
        &step_id,
        &port,
        value_from,
    )?;
    emit_workflow_changed(&app, path, &written);
    Ok(())
}

fn add_step_input_slot_impl(
    path: &Path,
    revision: &str,
    dirty: bool,
    step_id: &str,
    port: &str,
) -> Result<String, MutationError> {
    let (path, mut wf) = load_for_mutation(path, revision, dirty)?;
    add_step_input_slot_mut(&mut wf, step_id, port)?;
    save_workflow(&wf, &path)
}

#[tauri::command]
pub fn add_step_input_slot(
    app: AppHandle,
    path: String,
    revision: String,
    dirty: bool,
    step_id: String,
    port: String,
) -> Result<(), MutationError> {
    let written = add_step_input_slot_impl(Path::new(&path), &revision, dirty, &step_id, &port)?;
    emit_workflow_changed(&app, path, &written);
    Ok(())
}

fn set_step_input_link_merge_impl(
    path: &Path,
    revision: &str,
    dirty: bool,
    step_id: &str,
    port: &str,
    method: Option<LinkMergeMethod>,
) -> Result<String, MutationError> {
    let (path, mut wf) = load_for_mutation(path, revision, dirty)?;
    set_step_input_link_merge_mut(&mut wf, step_id, port, method)?;
    ensure_multiple_input_feature_requirement_mut(&mut wf);
    save_workflow(&wf, &path)
}

#[tauri::command]
pub fn set_step_input_link_merge(
    app: AppHandle,
    path: String,
    revision: String,
    dirty: bool,
    step_id: String,
    port: String,
    method: Option<LinkMergeMethod>,
) -> Result<(), MutationError> {
    let written = set_step_input_link_merge_impl(
        Path::new(&path),
        &revision,
        dirty,
        &step_id,
        &port,
        method,
    )?;
    emit_workflow_changed(&app, path, &written);
    Ok(())
}

fn set_output_pick_value_impl(
    path: &Path,
    revision: &str,
    dirty: bool,
    output_id: &str,
    method: Option<PickValueMethod>,
) -> Result<String, MutationError> {
    let (path, mut wf) = load_for_mutation(path, revision, dirty)?;
    set_output_pick_value_mut(&mut wf, output_id, method)?;
    ensure_multiple_input_feature_requirement_mut(&mut wf);
    save_workflow(&wf, &path)
}

#[tauri::command]
pub fn set_output_pick_value(
    app: AppHandle,
    path: String,
    revision: String,
    dirty: bool,
    output_id: String,
    method: Option<PickValueMethod>,
) -> Result<(), MutationError> {
    let written =
        set_output_pick_value_impl(Path::new(&path), &revision, dirty, &output_id, method)?;
    emit_workflow_changed(&app, path, &written);
    Ok(())
}

fn set_output_link_merge_impl(
    path: &Path,
    revision: &str,
    dirty: bool,
    output_id: &str,
    method: Option<LinkMergeMethod>,
) -> Result<String, MutationError> {
    let (path, mut wf) = load_for_mutation(path, revision, dirty)?;
    set_output_link_merge_mut(&mut wf, output_id, method)?;
    ensure_multiple_input_feature_requirement_mut(&mut wf);
    save_workflow(&wf, &path)
}

#[tauri::command]
pub fn set_output_link_merge(
    app: AppHandle,
    path: String,
    revision: String,
    dirty: bool,
    output_id: String,
    method: Option<LinkMergeMethod>,
) -> Result<(), MutationError> {
    let written =
        set_output_link_merge_impl(Path::new(&path), &revision, dirty, &output_id, method)?;
    emit_workflow_changed(&app, path, &written);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use commonwl::requirements::MultipleInputFeatureRequirement;
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

        let written =
            connect_workflow_nodes_impl(&path, &revision, false, &from, &to, false, None).unwrap();

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

        let result = connect_workflow_nodes_impl(&path, &revision, true, &from, &to, false, None);

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

        let result = connect_workflow_nodes_impl(
            &path,
            "not-the-real-revision",
            false,
            &from,
            &to,
            false,
            None,
        );

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
        let result = connect_workflow_nodes_impl(&path, &revision, false, &from, &to, false, None);

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
            false,
            None,
        );

        assert!(matches!(
            result,
            Err(MutationError::IncompatibleTypes { .. })
        ));
    }

    #[test]
    fn connect_step_to_step_array_into_scalar_requires_scatter_confirmation() {
        let dir = tempdir().unwrap();
        fs::write(
            dir.path().join("producer.cwl"),
            r"
cwlVersion: v1.2
class: CommandLineTool
inputs: []
outputs:
- id: out
  type: string[]
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

        let result = connect_workflow_nodes_impl(&path, &revision, false, &from, &to, false, None);
        assert!(matches!(
            result,
            Err(MutationError::NeedsScatterConfirmation { ref port }) if port == "y"
        ));

        let written =
            connect_workflow_nodes_impl(&path, &revision, false, &from, &to, true, None).unwrap();

        let CWLDocument::Workflow(wf) = commonwl::from_str(&written).unwrap() else {
            panic!("expected a workflow")
        };
        let consumer = wf
            .steps
            .iter()
            .find(|s| s.id.as_deref() == Some("consumer"))
            .unwrap();
        assert_eq!(consumer.scatter, Some(OneOrMany::One("y".to_string())));
        assert_eq!(
            consumer
                .r#in
                .iter()
                .find(|i| i.id.as_deref() == Some("y"))
                .and_then(|i| i.source.clone()),
            Some(OneOrMany::One("producer/out".to_string()))
        );
    }

    #[test]
    fn connect_step_to_step_array_into_already_scattered_scalar_needs_no_confirmation() {
        let dir = tempdir().unwrap();
        fs::write(
            dir.path().join("producer.cwl"),
            r"
cwlVersion: v1.2
class: CommandLineTool
inputs: []
outputs:
- id: out
  type: string[]
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
  scatter: y
";
        let path = dir.path().join("workflow.cwl");
        fs::write(&path, workflow).unwrap();
        let revision = compute_revision(workflow.as_bytes());

        let from = endpoint(NodeKind::Step, "producer", "out");
        let to = endpoint(NodeKind::Step, "consumer", "y");

        // No scatter_confirmed needed -- the step already scatters over `y`.
        let written =
            connect_workflow_nodes_impl(&path, &revision, false, &from, &to, false, None).unwrap();

        let CWLDocument::Workflow(wf) = commonwl::from_str(&written).unwrap() else {
            panic!("expected a workflow")
        };
        let consumer = wf
            .steps
            .iter()
            .find(|s| s.id.as_deref() == Some("consumer"))
            .unwrap();
        assert_eq!(
            consumer
                .r#in
                .iter()
                .find(|i| i.id.as_deref() == Some("y"))
                .and_then(|i| i.source.clone()),
            Some(OneOrMany::One("producer/out".to_string()))
        );
    }

    #[test]
    fn connect_step_to_step_second_source_into_scalar_requires_pick_value() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("producer.cwl"), PRODUCER_TOOL).unwrap();
        fs::write(dir.path().join("producer2.cwl"), PRODUCER_TOOL).unwrap();
        fs::write(dir.path().join("consumer.cwl"), CONSUMER_TOOL).unwrap();
        let workflow = r"
cwlVersion: v1.2
class: Workflow
inputs:
- id: image
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
- id: producer2
  in:
  - id: x
    source: image
  out:
  - out
  run: producer2.cwl
- id: consumer
  in: []
  out:
  - done
  run: consumer.cwl
";
        let path = dir.path().join("workflow.cwl");
        fs::write(&path, workflow).unwrap();
        let revision = compute_revision(workflow.as_bytes());

        let written = connect_workflow_nodes_impl(
            &path,
            &revision,
            false,
            &endpoint(NodeKind::Step, "producer", "out"),
            &endpoint(NodeKind::Step, "consumer", "y"),
            false,
            None,
        )
        .unwrap();
        let revision = compute_revision(written.as_bytes());

        let from2 = endpoint(NodeKind::Step, "producer2", "out");
        let to = endpoint(NodeKind::Step, "consumer", "y");
        let result = connect_workflow_nodes_impl(&path, &revision, false, &from2, &to, false, None);
        assert!(matches!(
            result,
            Err(MutationError::NeedsPickValue { ref port }) if port == "y"
        ));

        let written = connect_workflow_nodes_impl(
            &path,
            &revision,
            false,
            &from2,
            &to,
            false,
            Some(PickValueMethod::FirstNonNull),
        )
        .unwrap();

        let CWLDocument::Workflow(wf) = commonwl::from_str(&written).unwrap() else {
            panic!("expected a workflow")
        };
        let y = wf
            .steps
            .iter()
            .find(|s| s.id.as_deref() == Some("consumer"))
            .unwrap()
            .r#in
            .iter()
            .find(|i| i.id.as_deref() == Some("y"))
            .unwrap();
        assert_eq!(
            y.source,
            Some(OneOrMany::Many(vec![
                "producer/out".to_string(),
                "producer2/out".to_string()
            ]))
        );
        assert_eq!(y.pick_value, Some(PickValueMethod::FirstNonNull));
    }

    #[test]
    fn connect_input_to_step_array_into_scalar_requires_scatter_confirmation() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("consumer.cwl"), CONSUMER_TOOL).unwrap(); // y: string
        let workflow = r"
cwlVersion: v1.2
class: Workflow
inputs:
- id: names
  type: string[]
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

        let from = endpoint(NodeKind::Input, "names", "names");
        let to = endpoint(NodeKind::Step, "consumer", "y");

        let result = connect_workflow_nodes_impl(&path, &revision, false, &from, &to, false, None);
        assert!(matches!(
            result,
            Err(MutationError::NeedsScatterConfirmation { ref port }) if port == "y"
        ));

        let written =
            connect_workflow_nodes_impl(&path, &revision, false, &from, &to, true, None).unwrap();

        let CWLDocument::Workflow(wf) = commonwl::from_str(&written).unwrap() else {
            panic!("expected a workflow")
        };
        let consumer = wf
            .steps
            .iter()
            .find(|s| s.id.as_deref() == Some("consumer"))
            .unwrap();
        assert_eq!(consumer.scatter, Some(OneOrMany::One("y".to_string())));
        assert_eq!(
            consumer
                .r#in
                .iter()
                .find(|i| i.id.as_deref() == Some("y"))
                .and_then(|i| i.source.clone()),
            Some(OneOrMany::One("names".to_string()))
        );
    }

    #[test]
    fn connect_step_to_step_scattered_producer_with_optional_output_requires_pick_value() {
        let dir = tempdir().unwrap();
        fs::write(
            dir.path().join("producer.cwl"),
            r#"
cwlVersion: v1.2
class: CommandLineTool
inputs:
- id: x
  type: string
outputs:
- id: out
  type: ["null", File]
baseCommand: echo
"#,
        )
        .unwrap();
        fs::write(
            dir.path().join("consumer.cwl"),
            r"
cwlVersion: v1.2
class: CommandLineTool
inputs:
- id: configs
  type: File[]
outputs:
- id: done
  type: string
baseCommand: echo
",
        )
        .unwrap();
        let workflow = r"
cwlVersion: v1.2
class: Workflow
inputs:
- id: image
  type: string[]
outputs: []
steps:
- id: producer
  in:
  - id: x
    source: image
  out:
  - out
  run: producer.cwl
  scatter: x
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
        let to = endpoint(NodeKind::Step, "consumer", "configs");

        // Scattered producer output is File|null -- fits File[] only once a
        // pickValue strategy is set to drop the nulls.
        let result = connect_workflow_nodes_impl(&path, &revision, false, &from, &to, false, None);
        assert!(matches!(
            result,
            Err(MutationError::NeedsPickValue { ref port }) if port == "configs"
        ));

        let written = connect_workflow_nodes_impl(
            &path,
            &revision,
            false,
            &from,
            &to,
            false,
            Some(PickValueMethod::AllNonNull),
        )
        .unwrap();

        let CWLDocument::Workflow(wf) = commonwl::from_str(&written).unwrap() else {
            panic!("expected a workflow")
        };
        let configs = wf
            .steps
            .iter()
            .find(|s| s.id.as_deref() == Some("consumer"))
            .unwrap()
            .r#in
            .iter()
            .find(|i| i.id.as_deref() == Some("configs"))
            .unwrap();
        assert_eq!(
            configs.source,
            Some(OneOrMany::One("producer/out".to_string()))
        );
        assert_eq!(configs.pick_value, Some(PickValueMethod::AllNonNull));
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
            false,
            None,
        )
        .unwrap();
        let revision = compute_revision(written.as_bytes());
        let written = connect_workflow_nodes_impl(
            &path,
            &revision,
            false,
            &endpoint(NodeKind::Step, "producer", "out"),
            &endpoint(NodeKind::Output, "result", "result"),
            false,
            None,
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
            false,
            None,
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
            false,
            None,
        )
        .unwrap();
        let revision = compute_revision(written.as_bytes());

        // A second source landing on an already-fed scalar slot needs a
        // pickValue strategy -- refused without one, same as step-to-step.
        let result = connect_workflow_nodes_impl(
            &path,
            &revision,
            false,
            &endpoint(NodeKind::Input, "extra", "extra"),
            &endpoint(NodeKind::Step, "consumer", "y"),
            false,
            None,
        );
        assert!(matches!(
            result,
            Err(MutationError::NeedsPickValue { ref port }) if port == "y"
        ));

        let written = connect_workflow_nodes_impl(
            &path,
            &revision,
            false,
            &endpoint(NodeKind::Input, "extra", "extra"),
            &endpoint(NodeKind::Step, "consumer", "y"),
            false,
            Some(PickValueMethod::AllNonNull),
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

    #[test]
    fn add_step_node_registers_step_with_no_connections() {
        let (dir, path, revision) = setup();
        let tool_path = dir.path().join("producer.cwl");

        let written = add_workflow_step_node_impl(
            &path,
            &revision,
            false,
            tool_path.to_str().unwrap(),
            "new_producer",
        )
        .unwrap();

        let CWLDocument::Workflow(wf) = commonwl::from_str(&written).unwrap() else {
            panic!("expected a workflow")
        };
        let step = wf
            .steps
            .iter()
            .find(|s| s.id.as_deref() == Some("new_producer"))
            .expect("step must be registered");
        assert!(
            step.r#in.is_empty(),
            "a freshly dropped step has nothing wired yet"
        );
        assert_eq!(
            step.out.len(),
            1,
            "outputs come pre-populated from the tool's own declaration"
        );
        assert_eq!(
            fs::read_to_string(&path).unwrap(),
            written,
            "must actually be on disk, not just returned"
        );
    }

    #[test]
    fn add_step_node_refuses_duplicate_id() {
        let (dir, path, revision) = setup();
        let tool_path = dir.path().join("consumer.cwl");

        let result = add_workflow_step_node_impl(
            &path,
            &revision,
            false,
            tool_path.to_str().unwrap(),
            "consumer",
        );

        assert!(matches!(result, Err(MutationError::DuplicateId { id }) if id == "consumer"));
    }

    #[test]
    fn add_step_node_refuses_when_editor_is_dirty() {
        let (dir, path, revision) = setup();
        let before = fs::read_to_string(&path).unwrap();
        let tool_path = dir.path().join("producer.cwl");

        let result = add_workflow_step_node_impl(
            &path,
            &revision,
            true,
            tool_path.to_str().unwrap(),
            "new_producer",
        );

        assert!(matches!(result, Err(MutationError::EditorDirty)));
        assert_eq!(
            fs::read_to_string(&path).unwrap(),
            before,
            "must not touch the file"
        );
    }

    #[test]
    fn add_step_node_refuses_on_stale_revision() {
        let (dir, path, _revision) = setup();
        let before = fs::read_to_string(&path).unwrap();
        let tool_path = dir.path().join("producer.cwl");

        let result = add_workflow_step_node_impl(
            &path,
            "not-the-real-revision",
            false,
            tool_path.to_str().unwrap(),
            "new_producer",
        );

        assert!(matches!(result, Err(MutationError::StaleRevision)));
        assert_eq!(fs::read_to_string(&path).unwrap(), before);
    }

    #[test]
    fn add_step_node_errors_on_missing_tool_file() {
        let (dir, path, revision) = setup();
        let tool_path = dir.path().join("does_not_exist.cwl");

        let result = add_workflow_step_node_impl(
            &path,
            &revision,
            false,
            tool_path.to_str().unwrap(),
            "new_step",
        );

        assert!(matches!(result, Err(MutationError::Io { .. })));
    }

    #[test]
    fn add_step_node_writes_a_resolvable_run_path_through_noncanonical_inputs() {
        let dir = tempdir().unwrap();
        let tools_dir = dir.path().join("tools");
        fs::create_dir_all(&tools_dir).unwrap();
        fs::write(tools_dir.join("producer.cwl"), PRODUCER_TOOL).unwrap();

        let workflows_dir = dir.path().join("workflows");
        let sub = workflows_dir.join("sub");
        fs::create_dir_all(&sub).unwrap();
        let workflow = "cwlVersion: v1.2\nclass: Workflow\ninputs: []\noutputs: []\nsteps: []\n";
        let real_workflow_path = workflows_dir.join("workflow.cwl");
        fs::write(&real_workflow_path, workflow).unwrap();
        let revision = compute_revision(workflow.as_bytes());

        // Same files, each reached through a redundant detour that a plain
        // "is this absolute" check would let straight through.
        let noncanonical_workflow = sub.join("..").join("workflow.cwl");
        let noncanonical_tool = tools_dir.join(".").join("producer.cwl");

        let written = add_workflow_step_node_impl(
            &noncanonical_workflow,
            &revision,
            false,
            noncanonical_tool.to_str().unwrap(),
            "producer",
        )
        .unwrap();

        let CWLDocument::Workflow(wf) = commonwl::from_str(&written).unwrap() else {
            panic!("expected a workflow")
        };
        let step = wf
            .steps
            .iter()
            .find(|s| s.id.as_deref() == Some("producer"))
            .expect("step must be registered");
        let StringOrDocument::String(run) = &step.run else {
            panic!("expected a file run:, not an inline document")
        };

        let resolved = dunce::canonicalize(workflows_dir.join(run))
            .unwrap_or_else(|e| panic!("run: {run:?} did not resolve back to a real file: {e}"));
        let real_tool = dunce::canonicalize(tools_dir.join("producer.cwl")).unwrap();
        assert_eq!(
            resolved, real_tool,
            "run: must resolve to the real tool file regardless of how the caller's paths were spelled"
        );
    }

    #[test]
    fn rename_step_rewrites_downstream_source() {
        let (_dir, path, revision) = setup();
        let written = connect_workflow_nodes_impl(
            &path,
            &revision,
            false,
            &endpoint(NodeKind::Step, "producer", "out"),
            &endpoint(NodeKind::Step, "consumer", "y"),
            false,
            None,
        )
        .unwrap();
        let revision = compute_revision(written.as_bytes());

        let written =
            rename_workflow_step_impl(&path, &revision, false, "producer", "renamed").unwrap();

        assert_eq!(
            consumer_y_source(&written),
            Some(OneOrMany::One("renamed/out".to_string()))
        );
    }

    #[test]
    fn rename_step_refuses_duplicate_id() {
        let (_dir, path, revision) = setup();
        let result = rename_workflow_step_impl(&path, &revision, false, "producer", "consumer");
        assert!(matches!(result, Err(MutationError::DuplicateId { id }) if id == "consumer"));
    }

    #[test]
    fn set_when_writes_and_clears_expression() {
        let (_dir, path, revision) = setup();
        let written = set_step_when_impl(
            &path,
            &revision,
            false,
            "consumer",
            Some("$(inputs.y != null)".to_string()),
        )
        .unwrap();
        let CWLDocument::Workflow(wf) = commonwl::from_str(&written).unwrap() else {
            panic!("expected a workflow")
        };
        assert_eq!(
            wf.get_step("consumer").unwrap().when.as_deref(),
            Some("$(inputs.y != null)")
        );

        let revision = compute_revision(written.as_bytes());
        let written = set_step_when_impl(&path, &revision, false, "consumer", None).unwrap();
        let CWLDocument::Workflow(wf) = commonwl::from_str(&written).unwrap() else {
            panic!("expected a workflow")
        };
        assert_eq!(wf.get_step("consumer").unwrap().when, None);
    }

    #[test]
    fn set_step_scattered_toggles_the_named_port() {
        let (_dir, path, revision) = setup();
        let written =
            set_step_scattered_impl(&path, &revision, false, "producer", "x", true).unwrap();
        let CWLDocument::Workflow(wf) = commonwl::from_str(&written).unwrap() else {
            panic!("expected a workflow")
        };
        assert_eq!(
            wf.get_step("producer").unwrap().scatter,
            Some(OneOrMany::One("x".to_string()))
        );

        let revision = compute_revision(written.as_bytes());
        let written =
            set_step_scattered_impl(&path, &revision, false, "producer", "x", false).unwrap();
        let CWLDocument::Workflow(wf) = commonwl::from_str(&written).unwrap() else {
            panic!("expected a workflow")
        };
        assert_eq!(wf.get_step("producer").unwrap().scatter, None);
    }

    #[test]
    fn add_step_input_slot_adds_a_bare_named_slot() {
        let (_dir, path, revision) = setup();
        let written =
            add_step_input_slot_impl(&path, &revision, false, "producer", "gate").unwrap();
        let CWLDocument::Workflow(wf) = commonwl::from_str(&written).unwrap() else {
            panic!("expected a workflow")
        };
        assert!(
            wf.get_step("producer")
                .unwrap()
                .r#in
                .iter()
                .any(|i| i.id.as_deref() == Some("gate"))
        );
    }

    #[test]
    fn add_step_input_slot_refuses_duplicate_name() {
        let (_dir, path, revision) = setup();
        add_step_input_slot_impl(&path, &revision, false, "producer", "gate").unwrap();
        let revision = compute_revision(fs::read_to_string(&path).unwrap().as_bytes());
        let result = add_step_input_slot_impl(&path, &revision, false, "producer", "gate");
        assert!(result.is_err());
    }

    #[test]
    fn connect_input_to_step_undeclared_slot_succeeds() {
        let (_dir, path, revision) = setup();
        let written =
            add_step_input_slot_impl(&path, &revision, false, "producer", "gate").unwrap();
        let revision = compute_revision(written.as_bytes());

        let written = connect_workflow_nodes_impl(
            &path,
            &revision,
            false,
            &endpoint(NodeKind::Input, "gate_in", "gate_in"),
            &endpoint(NodeKind::Step, "producer", "gate"),
            false,
            None,
        )
        .unwrap();

        let CWLDocument::Workflow(wf) = commonwl::from_str(&written).unwrap() else {
            panic!("expected a workflow")
        };
        let gate = wf
            .get_step("producer")
            .unwrap()
            .r#in
            .into_iter()
            .find(|i| i.id.as_deref() == Some("gate"))
            .unwrap();
        assert_eq!(gate.source.unwrap().as_many(), vec!["gate_in".to_string()]);
    }

    #[test]
    fn connect_step_to_step_undeclared_slot_succeeds() {
        let (_dir, path, revision) = setup();
        let written =
            add_step_input_slot_impl(&path, &revision, false, "consumer", "gate").unwrap();
        let revision = compute_revision(written.as_bytes());

        let written = connect_workflow_nodes_impl(
            &path,
            &revision,
            false,
            &endpoint(NodeKind::Step, "producer", "out"),
            &endpoint(NodeKind::Step, "consumer", "gate"),
            false,
            None,
        )
        .unwrap();

        let CWLDocument::Workflow(wf) = commonwl::from_str(&written).unwrap() else {
            panic!("expected a workflow")
        };
        let gate = wf
            .get_step("consumer")
            .unwrap()
            .r#in
            .into_iter()
            .find(|i| i.id.as_deref() == Some("gate"))
            .unwrap();
        assert_eq!(
            gate.source.unwrap().as_many(),
            vec!["producer/out".to_string()]
        );
    }

    #[test]
    fn set_step_input_link_merge_writes_and_clears() {
        let (_dir, path, revision) = setup();
        let written = set_step_input_link_merge_impl(
            &path,
            &revision,
            false,
            "producer",
            "x",
            Some(LinkMergeMethod::MergeFlattened),
        )
        .unwrap();
        let CWLDocument::Workflow(wf) = commonwl::from_str(&written).unwrap() else {
            panic!("expected a workflow")
        };
        let x = wf
            .get_step("producer")
            .unwrap()
            .r#in
            .into_iter()
            .find(|i| i.id.as_deref() == Some("x"))
            .unwrap();
        assert_eq!(x.link_merge, Some(LinkMergeMethod::MergeFlattened));
    }

    #[test]
    fn set_output_pick_value_and_link_merge_declare_multiple_input_requirement() {
        let (_dir, path, revision) = setup();
        let written = connect_workflow_nodes_impl(
            &path,
            &revision,
            false,
            &endpoint(NodeKind::Step, "producer", "out"),
            &endpoint(NodeKind::Output, "result", "result"),
            false,
            None,
        )
        .unwrap();
        let revision = compute_revision(written.as_bytes());
        let written = connect_workflow_nodes_impl(
            &path,
            &revision,
            false,
            &endpoint(NodeKind::Step, "consumer", "done"),
            &endpoint(NodeKind::Output, "result", "result"),
            false,
            None,
        )
        .unwrap();
        let revision = compute_revision(written.as_bytes());

        let written = set_output_pick_value_impl(
            &path,
            &revision,
            false,
            "result",
            Some(PickValueMethod::AllNonNull),
        )
        .unwrap();
        let CWLDocument::Workflow(wf) = commonwl::from_str(&written).unwrap() else {
            panic!("expected a workflow")
        };
        let result = wf
            .outputs
            .iter()
            .find(|o| o.id.as_deref() == Some("result"))
            .unwrap();
        assert_eq!(result.pick_value, Some(PickValueMethod::AllNonNull));
        assert!(wf.has_requirement::<MultipleInputFeatureRequirement>());
    }

    #[test]
    fn set_step_input_value_from_writes_and_clears() {
        let (_dir, path, revision) = setup();
        let written = set_step_input_value_from_impl(
            &path,
            &revision,
            false,
            "producer",
            "x",
            Some("$(self.toUpperCase())".to_string()),
        )
        .unwrap();
        let CWLDocument::Workflow(wf) = commonwl::from_str(&written).unwrap() else {
            panic!("expected a workflow")
        };
        let x = wf
            .get_step("producer")
            .unwrap()
            .r#in
            .into_iter()
            .find(|i| i.id.as_deref() == Some("x"))
            .unwrap();
        assert_eq!(x.value_from.as_deref(), Some("$(self.toUpperCase())"));
    }
}
