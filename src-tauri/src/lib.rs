mod files;
mod layout;
mod lsp;
mod mutation;
mod project;
mod session;
mod terminal;
mod graph;
mod graph_types;

use files::{cwl_doc_type, get_cwl_files, list_dir, path_exists, read_file, write_file};
use graph::get_workflow_graph;
use layout::{get_node_layout, reset_node_layout, save_node_layout};
use lsp::lsp_send;
use mutation::{
    add_step_input_slot, add_workflow_step_node, connect_workflow_nodes, delete_workflow_node,
    disconnect_workflow_nodes, rename_workflow_step, set_output_link_merge, set_output_pick_value,
    set_step_input_link_merge, set_step_input_value_from, set_step_pick_value,
    set_step_scatter_method, set_step_scattered, set_step_when,
};
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
            cwl_doc_type,
            get_workflow_graph,
            connect_workflow_nodes,
            disconnect_workflow_nodes,
            delete_workflow_node,
            add_workflow_step_node,
            rename_workflow_step,
            set_step_when,
            set_step_scatter_method,
            set_step_scattered,
            set_step_pick_value,
            set_step_input_value_from,
            set_step_input_link_merge,
            set_output_pick_value,
            set_output_link_merge,
            add_step_input_slot,
            get_node_layout,
            save_node_layout,
            reset_node_layout
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
