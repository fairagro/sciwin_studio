mod files;
mod lsp;
mod project;
mod session;
mod terminal;

use files::{cwl_doc_type, get_cwl_files, list_dir, path_exists, read_file, write_file};
use lsp::lsp_send;
use project::{has_workflow_config, init_sciwin_project};
use session::{load_session, save_session};
use terminal::{PtyState, check_s4n, pty_kill, pty_resize, pty_spawn, pty_write};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .manage(PtyState::default())
        .setup(|app| {
            lsp::init(app.handle());
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            pty_spawn,
            pty_write,
            pty_resize,
            pty_kill,
            check_s4n,
            list_dir,
            get_cwl_files,
            read_file,
            write_file,
            path_exists,
            has_workflow_config,
            init_sciwin_project,
            lsp_send,
            save_session,
            load_session,
            cwl_doc_type
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
