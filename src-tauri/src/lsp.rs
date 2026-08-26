use cwl_lsp::backend::Backend;
use tauri::{AppHandle, Emitter, Manager, State};
use tokio::io::{AsyncReadExt, AsyncWriteExt, DuplexStream, WriteHalf};
use tokio::sync::Mutex;
use tower_lsp_server::{LspService, Server};

pub struct LspState(pub Mutex<Option<WriteHalf<DuplexStream>>>);

/// Starts cwl-lsp in-process, wired to the app over an in-memory duplex pipe
/// instead of stdio. Call once from `.setup()`; the frontend never needs to
/// start or stop this itself.
pub fn init(app: &AppHandle) {
    let (server_stream, client_stream) = tokio::io::duplex(4096);
    let (server_read, server_write) = tokio::io::split(server_stream);
    let (mut client_read, client_write) = tokio::io::split(client_stream);

    let (service, socket) = LspService::new(Backend::new);
    tauri::async_runtime::spawn(async move {
        Server::new(server_read, server_write, socket).serve(service).await;
    });

    let app_handle = app.clone();
    tauri::async_runtime::spawn(async move {
        let mut buf = [0u8; 4096];
        loop {
            match client_read.read(&mut buf).await {
                Ok(0) => break,
                Ok(n) => {
                    let chunk = String::from_utf8_lossy(&buf[..n]).into_owned();
                    if app_handle.emit("lsp://message", chunk).is_err() {
                        break;
                    }
                }
                Err(_) => break,
            }
        }
    });

    app.manage(LspState(Mutex::new(Some(client_write))));
}

#[tauri::command]
pub async fn lsp_send(state: State<'_, LspState>, msg: String) -> Result<(), String> {
    let mut guard = state.0.lock().await;
    let writer = guard.as_mut().ok_or("lsp not started")?;
    writer.write_all(msg.as_bytes()).await.map_err(|e| e.to_string())?;
    writer.flush().await.map_err(|e| e.to_string())
}
