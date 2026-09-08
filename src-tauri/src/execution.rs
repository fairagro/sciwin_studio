//! Runs a workflow in-process against a `sciwin::execution::WorkflowRunner`, forwarding
//! step-level progress to the frontend as Tauri events -- the backend half of a live execution
//! overlay. `execute_workflow` runs with an empty job unless the frontend passes `input_file`,
//! a YAML/JSON job file the user picked (see `ExecutionControls.svelte`).

use commonwl::engine::{InputObject, StepEvent, load_input_file_from_file};
use futures::StreamExt;
use sciwin::{
    authoring::tool::auto_container_engine,
    execution::{RunId, RunStatus, TaskRunner, WorkflowRunner, local_backend},
};
use serde::Serialize;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use tauri::{AppHandle, Emitter, State};

/// One [`WorkflowRunner`] this session can submit to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BackendId {
    Local,
}

type DynamicRunner = Arc<dyn WorkflowRunner + Send + Sync>;

/// Runners this session has actually built, keyed by [`BackendId`]
/// `run_owner` records which runner a run actually went to
#[derive(Default)]
pub struct ExecutionState {
    runners: Mutex<HashMap<BackendId, DynamicRunner>>,
    run_owner: Mutex<HashMap<RunId, DynamicRunner>>,
}

impl ExecutionState {
    /// The runner for `id`, building and caching it on first use.
    ///
    /// # Errors
    /// `id`'s backend isn't usable right now (e.g. no container engine found for `Local`).
    fn runner(&self, id: BackendId) -> Result<DynamicRunner, String> {
        let mut runners = self.runners.lock().unwrap();
        if let Some(runner) = runners.get(&id) {
            return Ok(runner.clone());
        }
        let runner = build_runner(id)?;
        runners.insert(id, runner.clone());
        Ok(runner)
    }

    fn owner_of(&self, run_id: &str) -> Option<DynamicRunner> {
        self.run_owner.lock().unwrap().get(run_id).cloned()
    }

    fn record_owner(&self, run_id: RunId, runner: DynamicRunner) {
        self.run_owner.lock().unwrap().insert(run_id, runner);
    }
}

fn build_runner(id: BackendId) -> Result<DynamicRunner, String> {
    match id {
        BackendId::Local => {
            let engine = auto_container_engine()
                .ok_or("no container engine (docker/podman/apptainer/singularity) found on PATH")?;
            Ok(Arc::new(TaskRunner::new(local_backend(engine))))
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum StepEventPayload {
    #[serde(rename_all = "camelCase")]
    Started {
        run_id: String,
        step_id: String,
        at: String,
    },
    #[serde(rename_all = "camelCase")]
    Finished {
        run_id: String,
        step_id: String,
        at: String,
    },
    #[serde(rename_all = "camelCase")]
    Output {
        run_id: String,
        step_id: String,
        stdout: String,
        stderr: String,
    },
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RunStatusPayload {
    pub run_id: String,
    pub status: String,
}

/// Submits `cwlfile` to the local backend and returns its `RunId` immediately
///
/// `input_file`, if given, is a YAML/JSON job file
///
/// # Errors
/// The backend isn't available, the CWL file or job file fails to load, or the runner rejects
/// the submission (see `sciwin::execution::RunnerError`).
#[tauri::command]
pub async fn execute_workflow(
    app: AppHandle,
    state: State<'_, ExecutionState>,
    cwlfile: String,
    input_file: Option<String>,
    out_dir: Option<String>,
) -> Result<String, String> {
    let runner = state.runner(BackendId::Local)?;
    let cwlfile = PathBuf::from(cwlfile);
    let out_dir = out_dir.map(PathBuf::from);

    let inputs = match input_file {
        Some(path) => {
            let base_path = cwlfile.parent().unwrap_or_else(|| Path::new("."));
            load_input_file_from_file(PathBuf::from(path), base_path).map_err(|e| e.to_string())?
        }
        None => InputObject::default(),
    };

    let run_id = runner
        .submit(&cwlfile, inputs, out_dir.as_deref())
        .await
        .map_err(|e| e.to_string())?;
    state.record_owner(run_id.clone(), runner.clone());

    let _ = app.emit(
        "execution://status",
        RunStatusPayload {
            run_id: run_id.clone(),
            status: format_status(&RunStatus::Running),
        },
    );

    spawn_step_event_forwarder(app.clone(), runner.clone(), run_id.clone());
    spawn_status_forwarder(app, runner, run_id.clone());

    Ok(run_id)
}

/// Cancels a run started by [`execute_workflow`]. Cooperative where the engine supports it,
/// otherwise a runner may hard-abort after a short grace period 
///
/// # Errors
/// `run_id` is not a run this session started (or it already finished and was forgotten).
#[tauri::command]
pub async fn cancel_workflow(
    state: State<'_, ExecutionState>,
    run_id: String,
) -> Result<(), String> {
    let runner = state
        .owner_of(&run_id)
        .ok_or_else(|| "no run with that id".to_string())?;
    runner.cancel(&run_id).await.map_err(|e| e.to_string())
}

fn spawn_step_event_forwarder(app: AppHandle, runner: DynamicRunner, run_id: String) {
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
                StepEvent::Output {
                    step_id,
                    stdout,
                    stderr,
                } => StepEventPayload::Output {
                    run_id: run_id.clone(),
                    step_id,
                    stdout,
                    stderr,
                },
            };
            let _ = app.emit("execution://step-event", payload);
        }
    });
}

fn spawn_status_forwarder(app: AppHandle, runner: DynamicRunner, run_id: String) {
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
