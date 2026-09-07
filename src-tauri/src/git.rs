use sciwin::repository::{
    Repository, checkout_branch, current_branch, get_modified_files_recursive, list_branches,
};
use serde::Serialize;
use std::path::{Path, PathBuf};
use tauri::{AppHandle, Emitter};

use crate::files::WorkflowChanged;
use crate::graph::compute_revision;

const LAYOUT_DIR_PREFIX: &str = ".sciwin/layout/";

/// If `relative_path` is a node-layout sidecar (`.sciwin/layout/<relative-cwl-path>.json`,
/// see `layout.rs`), returns the workflow file it positions nodes for.
fn workflow_for_layout_file(relative_path: &str) -> Option<&str> {
    relative_path
        .strip_prefix(LAYOUT_DIR_PREFIX)
        .and_then(|rest| rest.strip_suffix(".json"))
}

fn emit_workflow_changed(app: &AppHandle, path: &Path, contents: &str) {
    let revision = compute_revision(contents.as_bytes());
    let _ = app.emit(
        "workflow-changed",
        WorkflowChanged {
            path: path.to_string_lossy().into_owned(),
            revision,
        },
    );
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct BranchInfo {
    current_branch: Option<String>,
    available_branches: Vec<String>,
}

#[tauri::command]
pub fn git_branch_info(path: String) -> Result<BranchInfo, String> {
    let repo = Repository::discover(&path).map_err(|e| e.to_string())?;

    Ok(BranchInfo {
        current_branch: current_branch(&repo).map_err(|e| e.to_string())?,
        available_branches: list_branches(&repo).map_err(|e| e.to_string())?,
    })
}

#[tauri::command]
pub fn git_status(path: String) -> Result<Vec<String>, String> {
    let repo = Repository::discover(&path).map_err(|e| e.to_string())?;
    get_modified_files_recursive(&repo).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn git_checkout_branch(path: String, branch: String) -> Result<BranchInfo, String> {
    let repo = Repository::discover(&path).map_err(|e| e.to_string())?;
    checkout_branch(&repo, &branch).map_err(|e| e.to_string())?;

    Ok(BranchInfo {
        current_branch: current_branch(&repo).map_err(|e| e.to_string())?,
        available_branches: list_branches(&repo).map_err(|e| e.to_string())?,
    })
}

#[tauri::command]
pub fn git_stage_all(path: String) -> Result<(), String> {
    let repo = Repository::discover(&path).map_err(|e| e.to_string())?;
    sciwin::repository::stage_all(&repo).map_err(|e| e.to_string())
}

/// Discards uncommitted changes to `file` (a repo-relative path, as returned
/// by `git_status`). Returns the file's absolute path so the caller can check
/// whether it still exists (untracked files are deleted rather than reverted)
/// and close any open tab for it.
#[tauri::command]
pub fn git_discard_file(app: AppHandle, path: String, file: String) -> Result<String, String> {
    let repo = Repository::discover(&path).map_err(|e| e.to_string())?;
    let workdir: PathBuf = repo.workdir().ok_or("Repository has no working directory")?.to_path_buf();
    let full_path = workdir.join(&file);

    let still_exists = sciwin::repository::discard_file(&repo, &file).map_err(|e| e.to_string())?;

    if still_exists
        && file.to_lowercase().ends_with(".cwl")
        && let Ok(contents) = std::fs::read_to_string(&full_path)
    {
        emit_workflow_changed(&app, &full_path, &contents);
    } else if let Some(workflow_relative) = workflow_for_layout_file(&file) {
        // An open graph view keeps its dragged positions in memory even
        // after the layout sidecar is discarded, so it needs telling to
        // reload -- same as `handleResetLayout` -- to fall back to dagre.
        let workflow_path = workdir.join(workflow_relative);
        if let Ok(contents) = std::fs::read_to_string(&workflow_path) {
            emit_workflow_changed(&app, &workflow_path, &contents);
        }
    }

    Ok(full_path.to_string_lossy().into_owned())
}

#[tauri::command]
pub fn git_commit(path: String, message: String) -> Result<(), String> {
    let repo = Repository::discover(&path).map_err(|e| e.to_string())?;
    sciwin::repository::commit(&repo, &message).map_err(|e| e.to_string())?;
    Ok(())
}
