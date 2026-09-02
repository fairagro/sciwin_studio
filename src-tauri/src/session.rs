use serde_json::Value;
use std::fs;
use std::path::PathBuf;
use tauri::{AppHandle, Manager};

/// The persisted shape is owned entirely by the frontend (workspace.svelte.ts) -
/// Rust just stores and returns whatever JSON it's given, so the two sides
/// never need to keep a duplicated struct in sync.
fn session_path(app: &AppHandle) -> Result<PathBuf, String> {
    let dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    Ok(dir.join("session.json"))
}

#[tauri::command]
pub fn save_session(app: AppHandle, session: Value) -> Result<(), String> {
    let path = session_path(&app)?;
    fs::write(path, session.to_string()).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn load_session(app: AppHandle) -> Option<Value> {
    let path = session_path(&app).ok()?;
    let content = fs::read_to_string(path).ok()?;
    serde_json::from_str(&content).ok()
}
