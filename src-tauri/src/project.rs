use std::path::Path;

#[tauri::command]
pub fn has_workflow_config(path: String) -> bool {
    Path::new(&path).join("workflow.toml").exists()
}

#[tauri::command]
pub fn init_sciwin_project(path: String) -> Result<(), String> {
    sciwin::project::initialize_project(path).map_err(|e| e.to_string())
}
