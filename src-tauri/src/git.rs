use sciwin::repository::{
    Repository, checkout_branch, current_branch, get_modified_files, list_branches,
};
use serde::Serialize;

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
    get_modified_files(&repo).map_err(|e| e.to_string())
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

#[tauri::command]
pub fn git_commit(path: String, message: String) -> Result<(), String> {
    let repo = Repository::discover(&path).map_err(|e| e.to_string())?;
    sciwin::repository::commit(&repo, &message).map_err(|e| e.to_string())?;
    Ok(())
}
