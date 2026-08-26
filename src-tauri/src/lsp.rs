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
        let mut pending: Vec<u8> = Vec::new();
        loop {
            match client_read.read(&mut buf).await {
                Ok(0) => break,
                Ok(n) => {
                    pending.extend_from_slice(&buf[..n]);
                    while let Some(message) = take_frame(&mut pending) {
                        if app_handle.emit("lsp://message", message).is_err() {
                            return;
                        }
                    }
                }
                Err(_) => break,
            }
        }
    });

    app.manage(LspState(Mutex::new(Some(client_write))));
}

/// Extracts one complete Content-Length framed JSON-RPC message from the
/// front of `pending`, if fully buffered, and drains the consumed bytes.
/// Buffering raw bytes (rather than decoding each read chunk as it arrives)
/// avoids splitting a multi-byte UTF-8 sequence across two reads.
fn take_frame(pending: &mut Vec<u8>) -> Option<String> {
    let header_end = pending
        .windows(4)
        .position(|w| w == b"\r\n\r\n")?;
    let header = std::str::from_utf8(&pending[..header_end]).ok()?;
    let content_length: usize = header
        .lines()
        .find_map(|line| line.strip_prefix("Content-Length:"))
        .and_then(|v| v.trim().parse().ok())?;

    let body_start = header_end + 4;
    let body_end = body_start + content_length;
    if pending.len() < body_end {
        return None;
    }

    let body = String::from_utf8(pending[body_start..body_end].to_vec()).ok()?;
    pending.drain(..body_end);
    Some(body)
}

#[tauri::command]
pub async fn lsp_send(state: State<'_, LspState>, msg: String) -> Result<(), String> {
    let mut guard = state.0.lock().await;
    let writer = guard.as_mut().ok_or("lsp not started")?;
    writer.write_all(msg.as_bytes()).await.map_err(|e| e.to_string())?;
    writer.flush().await.map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::take_frame;

    fn frame(body: &str) -> Vec<u8> {
        format!("Content-Length: {}\r\n\r\n{}", body.len(), body).into_bytes()
    }

    #[test]
    fn extracts_a_complete_message() {
        let mut pending = frame(r#"{"jsonrpc":"2.0","method":"ping"}"#);
        let message = take_frame(&mut pending).unwrap();
        assert_eq!(message, r#"{"jsonrpc":"2.0","method":"ping"}"#);
        assert!(pending.is_empty());
    }

    #[test]
    fn extracts_two_concatenated_messages_in_order() {
        let mut pending = frame(r#"{"id":1}"#);
        pending.extend(frame(r#"{"id":2}"#));
        assert_eq!(take_frame(&mut pending).unwrap(), r#"{"id":1}"#);
        assert_eq!(take_frame(&mut pending).unwrap(), r#"{"id":2}"#);
        assert!(pending.is_empty());
    }

    #[test]
    fn returns_none_until_the_body_is_fully_buffered() {
        let full = frame(r#"{"message":"hello"}"#);
        let (first, second) = full.split_at(full.len() - 3);
        let mut pending = first.to_vec();
        assert!(take_frame(&mut pending).is_none());
        pending.extend_from_slice(second);
        assert_eq!(take_frame(&mut pending).unwrap(), r#"{"message":"hello"}"#);
    }

    /// Regression test: the previous implementation decoded each raw read
    /// chunk with `String::from_utf8_lossy` independently, which corrupted a
    /// multi-byte UTF-8 character split across two reads. `take_frame` only
    /// decodes once the full byte length named by Content-Length has arrived,
    /// so a split multi-byte character must survive intact.
    #[test]
    fn survives_a_multibyte_utf8_character_split_across_two_reads() {
        // "日本語" is 9 bytes in UTF-8; split in the middle of the first
        // character's 3-byte encoding (e6 97 a5).
        let body = r#"{"message":"日本語"}"#;
        let full = frame(body);
        let split_at = full.len() - body.len() - 1;
        let (first, second) = full.split_at(split_at);

        let mut pending = first.to_vec();
        assert!(take_frame(&mut pending).is_none());
        pending.extend_from_slice(second);
        assert_eq!(take_frame(&mut pending).unwrap(), body);
    }
}
