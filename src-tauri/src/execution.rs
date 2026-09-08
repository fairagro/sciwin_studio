//! Runs a workflow in-process via `sciwin::execution::TaskRunner`, forwarding step-level
//! progress to the frontend as Tauri events -- the backend half of a live execution overlay.
//! No inputs-collection or run-cancellation UI yet; `execute_workflow` runs with an empty job.

use commonwl::engine::{ContainerEngine, InputObject, StepEvent};
use futures::StreamExt;
use sciwin::execution::{RunStatus, TaskRunner, WorkflowRunner, local_backend};
use serde::Serialize;
use std::path::PathBuf;
use std::sync::Arc;
use tauri::{AppHandle, Emitter, State};

/// One `TaskRunner` (and its Docker-backed local engine) shared across every run this session
/// starts, so a run's `RunId` stays resolvable for as long as the app is open.
pub struct ExecutionState(Arc<TaskRunner>);

impl Default for ExecutionState {
    fn default() -> Self {
        Self(Arc::new(TaskRunner::new(local_backend(ContainerEngine::Docker))))
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum StepEventPayload {
    Started {
        run_id: String,
        step_id: String,
        at: String,
    },
    Finished {
        run_id: String,
        step_id: String,
        at: String,
    },
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RunStatusPayload {
    pub run_id: String,
    pub status: String,
}

/// Submits `cwlfile` to the shared local runner and returns its `RunId` immediately; progress
/// arrives afterward as `execution://step-event` and `execution://status` events carrying that
/// same `run_id`, so a caller with multiple runs going can tell them apart.
///
/// # Errors
/// The file fails to load, or the runner rejects the submission (see
/// `sciwin::execution::RunnerError`).
#[tauri::command]
pub async fn execute_workflow(
    app: AppHandle,
    state: State<'_, ExecutionState>,
    cwlfile: String,
    out_dir: Option<String>,
) -> Result<String, String> {
    let runner = state.0.clone();
    let cwlfile = PathBuf::from(cwlfile);
    let out_dir = out_dir.map(PathBuf::from);

    let run_id = runner
        .submit(&cwlfile, InputObject::default(), out_dir.as_deref())
        .await
        .map_err(|e| e.to_string())?;

    spawn_step_event_forwarder(app.clone(), runner.clone(), run_id.clone());
    spawn_status_forwarder(app, runner, run_id.clone());

    Ok(run_id)
}

fn spawn_step_event_forwarder(app: AppHandle, runner: Arc<TaskRunner>, run_id: String) {
    tokio::spawn(async move {
        let Ok(mut events) = runner.step_events(&run_id).await else {
            return;
        };
        while let Some(event) = events.next().await {
            let Ok(event) = event else { break };
            let payload = match event {
                StepEvent::Started {
                    step_id,
                    started_at,
                } => StepEventPayload::Started {
                    run_id: run_id.clone(),
                    step_id,
                    at: started_at.to_string(),
                },
                StepEvent::Finished {
                    step_id,
                    finished_at,
                } => StepEventPayload::Finished {
                    run_id: run_id.clone(),
                    step_id,
                    at: finished_at.to_string(),
                },
            };
            let _ = app.emit("execution://step-event", payload);
        }
    });
}

fn spawn_status_forwarder(app: AppHandle, runner: Arc<TaskRunner>, run_id: String) {
    tokio::spawn(async move {
        if let Ok(status) = runner.wait_for_completion(&run_id).await {
            let _ = app.emit(
                "execution://status",
                RunStatusPayload {
                    run_id,
                    status: format_status(&status),
                },
            );
        }
    });
}

fn format_status(status: &RunStatus) -> String {
    match status {
        RunStatus::Created => "created",
        RunStatus::Running => "running",
        RunStatus::Finished => "finished",
        RunStatus::Failed => "failed",
        RunStatus::Stopped => "stopped",
        RunStatus::Cancelled => "cancelled",
        RunStatus::Queued => "queued",
    }
    .to_string()
}
