//! Runs a workflow in-process against a `sciwin::execution::WorkflowRunner`, forwarding
//! step-level progress to the frontend as Tauri events

use crate::settings::{
    ContainerEngineChoice, RemoteBackendKind, S3Settings, Settings, SettingsState,
};
use commonwl::engine::{ContainerEngine, InputObject, StepEvent, load_input_file_from_file};
use commonwl::storage::{S3Config, StorageBackend};
use futures::StreamExt;
use sciwin::{
    authoring::tool::auto_container_engine,
    execution::{
        ReanaRunner, RunId, RunStatus, TaskRunner, TesBackendConfig, WorkflowRunner,
        docker_backend_with_storage, local_backend_with_storage, tes_backend_from_config,
    },
    reana::{api::client::ReanaClient, auth::ReanaAccessToken},
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use tauri::{AppHandle, Emitter, State};
use url::Url;

/// One [`WorkflowRunner`] this session can submit to. `Local`/`Docker` need no Settings-sourced
/// credentials to exist at all (though `Local`'s container engine choice and the shared S3
/// config both come from there); `Remote` is one of `Settings::remote_backends`, addressed by
/// its persisted id.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum BackendId {
    Local,
    Docker,
    Remote { id: String },
}

type DynamicRunner = Arc<dyn WorkflowRunner + Send + Sync>;

/// Runners this session has actually built, keyed by [`BackendId`]
///
/// `run_owner` records which runner a run actually went to, so `cancel_workflow` (and any
/// future per-run command) only needs a `run_id`, not the backend it happened to run on --
/// the same shape `TaskRunner` itself uses one level down for its own `jobs` map.
#[derive(Default)]
pub struct ExecutionState {
    runners: Mutex<HashMap<BackendId, DynamicRunner>>,
    run_owner: Mutex<HashMap<RunId, DynamicRunner>>,
}

impl ExecutionState {
    /// The runner for `id`, building and caching it on first use.
    ///
    /// # Errors
    /// `id`'s backend isn't usable right now (e.g. no REANA instance with that id, or a TES
    /// instance configured without the process-wide S3 settings it needs).
    async fn runner(&self, id: BackendId, settings: &Settings) -> Result<DynamicRunner, String> {
        if let Some(runner) = self.runners.lock().unwrap().get(&id) {
            return Ok(runner.clone());
        }
        let runner = build_runner(&id, settings).await?;
        self.runners.lock().unwrap().insert(id, runner.clone());
        Ok(runner)
    }

    fn owner_of(&self, run_id: &str) -> Option<DynamicRunner> {
        self.run_owner.lock().unwrap().get(run_id).cloned()
    }

    fn record_owner(&self, run_id: RunId, runner: DynamicRunner) {
        self.run_owner.lock().unwrap().insert(run_id, runner);
    }
}

fn container_engine_from_choice(choice: ContainerEngineChoice) -> ContainerEngine {
    match choice {
        ContainerEngineChoice::Docker => ContainerEngine::Docker,
        ContainerEngineChoice::Podman => ContainerEngine::Podman,
        ContainerEngineChoice::Singularity => ContainerEngine::Singularity,
        ContainerEngineChoice::Apptainer => ContainerEngine::Apptainer,
    }
}

/// `Local` never fails to resolve an engine
fn resolve_local_engine(settings: &Settings) -> ContainerEngine {
    settings
        .local_container_engine
        .map(container_engine_from_choice)
        .or_else(auto_container_engine)
        .unwrap_or(ContainerEngine::Docker)
}

fn s3_config(s3: &S3Settings) -> S3Config {
    S3Config {
        endpoint_url: s3.endpoint_url.clone(),
        access_key_id: s3.access_key_id.clone(),
        secret_access_key: s3.secret_access_key.clone(),
        session_token: None,
        region: s3.region.clone(),
    }
}

/// The shared storage backend every `TaskBackend` built here uses
fn shared_storage(settings: &Settings) -> Arc<StorageBackend> {
    match &settings.s3 {
        Some(s3) => Arc::new(StorageBackend::with_s3_config(s3_config(s3))),
        None => Arc::new(StorageBackend::new()),
    }
}

async fn build_runner(id: &BackendId, settings: &Settings) -> Result<DynamicRunner, String> {
    match id {
        BackendId::Local => {
            let engine = resolve_local_engine(settings);
            let backend = local_backend_with_storage(engine, shared_storage(settings));
            Ok(Arc::new(TaskRunner::new(backend)))
        }
        BackendId::Docker => {
            let backend = docker_backend_with_storage(shared_storage(settings))
                .await
                .map_err(|e| e.to_string())?;
            Ok(Arc::new(TaskRunner::new(backend)))
        }
        BackendId::Remote { id } => {
            let remote = settings
                .remote_backends
                .iter()
                .find(|b| &b.id == id)
                .ok_or_else(|| "no remote backend with that id".to_string())?;
            match &remote.kind {
                RemoteBackendKind::Tes { url, bucket, token } => {
                    let s3 = settings.s3.as_ref().ok_or_else(|| {
                        "TES needs the process-wide S3 storage settings configured".to_string()
                    })?;
                    let storage = Arc::new(StorageBackend::with_s3_config(s3_config(s3)));
                    let config = TesBackendConfig {
                        url: Url::parse(url).map_err(|e| e.to_string())?,
                        storage_url: Url::parse(&format!("s3://{bucket}"))
                            .map_err(|e| e.to_string())?,
                        token: token.clone(),
                    };
                    let backend = tes_backend_from_config(config, storage)
                        .await
                        .map_err(|e| e.to_string())?;
                    Ok(Arc::new(TaskRunner::new(backend)))
                }
                RemoteBackendKind::Reana { url, token } => {
                    let base_url = Url::parse(url).map_err(|e| e.to_string())?;
                    let client =
                        ReanaClient::new(base_url, Arc::new(ReanaAccessToken::new(token.clone())));
                    Ok(Arc::new(ReanaRunner::new(client)))
                }
            }
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BackendSummary {
    pub id: BackendId,
    pub name: String,
    pub available: bool,
    /// Why `available` is false, or a non-fatal heads-up even when it's true (e.g. `Local`
    /// with no container engine found -- still usable, just not for `DockerRequirement` steps).
    pub message: Option<String>,
}

#[tauri::command]
pub async fn list_backends(
    execution: State<'_, ExecutionState>,
    settings: State<'_, SettingsState>,
) -> Result<Vec<BackendSummary>, String> {
    let settings = settings.get();
    let mut backends = Vec::new();

    let local_warning = (settings.local_container_engine.is_none() && auto_container_engine().is_none())
        .then(|| "No container engine (docker/podman/apptainer/singularity) found on PATH -- steps with a DockerRequirement will fail".to_string());
    backends.push(BackendSummary {
        id: BackendId::Local,
        name: "Local".to_string(),
        available: true,
        message: local_warning,
    });

    let docker_result = execution.runner(BackendId::Docker, &settings).await;
    backends.push(BackendSummary {
        id: BackendId::Docker,
        name: "Docker".to_string(),
        available: docker_result.is_ok(),
        message: docker_result.err(),
    });

    for remote in &settings.remote_backends {
        let (kind_label, configured) = match &remote.kind {
            RemoteBackendKind::Tes { url, bucket, .. } => (
                "TES",
                if url.trim().is_empty() || bucket.trim().is_empty() {
                    Err("needs a URL and a bucket".to_string())
                } else if settings.s3.is_none() {
                    Err("needs the process-wide S3 storage settings configured".to_string())
                } else {
                    Ok(())
                },
            ),
            RemoteBackendKind::Reana { url, token } => (
                "REANA",
                if url.trim().is_empty() || token.trim().is_empty() {
                    Err("needs a URL and an access token".to_string())
                } else {
                    Ok(())
                },
            ),
        };
        backends.push(BackendSummary {
            id: BackendId::Remote {
                id: remote.id.clone(),
            },
            name: format!("{} ({kind_label})", remote.name),
            available: configured.is_ok(),
            message: configured.err(),
        });
    }

    Ok(backends)
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
    /// Why the run ended in a non-`Finished` terminal state -- e.g. REANA's `failure_detail`
    /// walks the failed job's own logs for this. `None` for a non-terminal status, or a
    /// terminal one the runner has nothing more specific to say about.
    pub message: Option<String>,
}

/// Submits `cwlfile` to `backend` and returns its `RunId` immediately; progress arrives
/// afterward as `execution://step-event` and `execution://status` events carrying that same
/// `run_id`, so a caller with multiple runs going can tell them apart.
///
/// `input_file`, if given, is a YAML/JSON job file, resolved relative to `cwlfile`'s own
/// directory (matching how `s4n execute`'s `--input-file` resolves paths inside the job file).
///
/// # Errors
/// The backend isn't available, the CWL file or job file fails to load, or the runner rejects
/// the submission (see `sciwin::execution::RunnerError`).
#[tauri::command]
pub async fn execute_workflow(
    app: AppHandle,
    execution: State<'_, ExecutionState>,
    settings: State<'_, SettingsState>,
    backend: BackendId,
    cwlfile: String,
    input_file: Option<String>,
    out_dir: Option<String>,
) -> Result<String, String> {
    let runner = execution.runner(backend, &settings.get()).await?;
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
    execution.record_owner(run_id.clone(), runner.clone());

    let _ = app.emit(
        "execution://status",
        RunStatusPayload {
            run_id: run_id.clone(),
            status: format_status(&RunStatus::Running),
            message: None,
        },
    );

    spawn_step_event_forwarder(app.clone(), runner.clone(), run_id.clone());
    spawn_status_forwarder(app, runner, run_id.clone());

    Ok(run_id)
}

/// Cancels a run started by [`execute_workflow`]. Cooperative where the engine supports it,
/// otherwise a runner may hard-abort after a short grace period -- see e.g.
/// `TaskRunner::cancel`'s own doc.
///
/// # Errors
/// `run_id` is not a run this session started (or it already finished and was forgotten).
#[tauri::command]
pub async fn cancel_workflow(
    execution: State<'_, ExecutionState>,
    run_id: String,
) -> Result<(), String> {
    let runner = execution
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
            // `wait_for_completion` only returns once `status` is terminal, so anything other
            // than `Finished` here is a run that didn't complete cleanly -- worth asking the
            // runner if it has more to say than the bare status (e.g. ReanaRunner's
            // `failure_detail` walks the failed job's own logs for this).
            let message = if status == RunStatus::Finished {
                None
            } else {
                runner.failure_detail(&run_id).await.ok().flatten()
            };
            let _ = app.emit(
                "execution://status",
                RunStatusPayload {
                    run_id,
                    status: format_status(&status),
                    message,
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
