mod files;
mod terminal;

use files::{get_cwl_files, list_dir};
use terminal::{check_s4n, pty_kill, pty_resize, pty_spawn, pty_write, PtyState};

// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .manage(PtyState::default())
        .invoke_handler(tauri::generate_handler![
            greet, pty_spawn, pty_write, pty_resize, pty_kill, check_s4n, list_dir, get_cwl_files
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
