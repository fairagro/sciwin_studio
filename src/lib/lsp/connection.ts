import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import {
  AbstractMessageReader,
  AbstractMessageWriter,
  createMessageConnection,
  type DataCallback,
  type Disposable,
  type Message,
  type MessageConnection,
} from "vscode-jsonrpc";

// Minimal slice of the LSP types cwl-lsp actually uses (see
// commonwl/crates/lsp/src/backend.rs) - not pulling in
// vscode-languageserver-protocol since that would mean re-vetting another
// dependency's module graph for a stray `vscode` import.
export interface LspPosition {
  line: number;
  character: number;
}

export interface LspRange {
  start: LspPosition;
  end: LspPosition;
}

export interface LspDiagnostic {
  range: LspRange;
  severity?: 1 | 2 | 3 | 4;
  message: string;
}

interface PublishDiagnosticsParams {
  uri: string;
  diagnostics: LspDiagnostic[];
}

/** Reads complete LSP messages off the `lsp://message` Tauri event. Rust does
 * the Content-Length framing, so each event payload is already one full,
 * valid JSON-RPC message string. */
class TauriMessageReader extends AbstractMessageReader {
  private callback: DataCallback | undefined;
  private unlisten: (() => void) | undefined;

  listen(callback: DataCallback): Disposable {
    this.callback = callback;
    listen<string>("lsp://message", (event) => {
      try {
        this.callback?.(JSON.parse(event.payload) as Message);
      } catch (e) {
        this.fireError(e);
      }
    }).then((fn) => {
      this.unlisten = fn;
    });
    return {
      dispose: () => {
        this.unlisten?.();
        this.unlisten = undefined;
      },
    };
  }
}

/** Sends LSP messages via the `lsp_send` command, framing each one with the
 * Content-Length header the Rust-side duplex stream expects. */
class TauriMessageWriter extends AbstractMessageWriter {
  private encoder = new TextEncoder();

  async write(msg: Message): Promise<void> {
    const json = JSON.stringify(msg);
    const byteLength = this.encoder.encode(json).byteLength;
    const framed = `Content-Length: ${byteLength}\r\n\r\n${json}`;
    try {
      await invoke("lsp_send", { msg: framed });
    } catch (e) {
      this.fireError(e, msg, 1);
    }
  }

  end(): void { }
}

let connectionPromise: Promise<MessageConnection> | undefined;

function diagnosticsToMarkers(diagnostics: LspDiagnostic[]) {
  const severityMap: Record<number, number> = { 1: 8, 2: 4, 3: 2, 4: 1 }; // monaco.MarkerSeverity Error/Warning/Info/Hint
  return diagnostics.map((d) => ({
    severity: severityMap[d.severity ?? 1] ?? 8,
    message: d.message,
    startLineNumber: d.range.start.line + 1,
    startColumn: d.range.start.character + 1,
    endLineNumber: d.range.end.line + 1,
    endColumn: d.range.end.character + 1,
  }));
}

let onDiagnostics: ((uri: string, diagnostics: LspDiagnostic[]) => void) | undefined;

/** Registers the callback invoked with (uri, markers) whenever the server
 * publishes diagnostics for a document. Editor.svelte wires this into
 * monaco.editor.setModelMarkers. */
export function setDiagnosticsHandler(handler: (uri: string, markers: ReturnType<typeof diagnosticsToMarkers>) => void) {
  onDiagnostics = (uri, diagnostics) => handler(uri, diagnosticsToMarkers(diagnostics));
}

async function connect(): Promise<MessageConnection> {
  const connection = createMessageConnection(new TauriMessageReader(), new TauriMessageWriter());
  connection.onNotification("textDocument/publishDiagnostics", (params: PublishDiagnosticsParams) => {
    onDiagnostics?.(params.uri, params.diagnostics);
  });
  connection.listen();

  await connection.sendRequest("initialize", {
    processId: null,
    rootUri: null,
    capabilities: {},
  });
  connection.sendNotification("initialized", {});

  return connection;
}

function getConnection(): Promise<MessageConnection> {
  if (!connectionPromise) connectionPromise = connect();
  return connectionPromise;
}

// The editor must stay usable even if the in-process LSP never comes up (or
// hiccups on one message), so every notification here is fire-and-forget.
export async function notifyDidOpen(uri: string, languageId: string, text: string) {
  try {
    const connection = await getConnection();
    connection.sendNotification("textDocument/didOpen", {
      textDocument: { uri, languageId, version: 1, text },
    });
  } catch {
    // ignored 
  }
}

const versions = new Map<string, number>();

export async function notifyDidChange(uri: string, text: string) {
  try {
    const connection = await getConnection();
    const version = (versions.get(uri) ?? 1) + 1;
    versions.set(uri, version);
    connection.sendNotification("textDocument/didChange", {
      textDocument: { uri, version },
      contentChanges: [{ text }],
    });
  } catch {
    // ignored
  }
}

export async function notifyDidClose(uri: string) {
  try {
    const connection = await getConnection();
    versions.delete(uri);
    connection.sendNotification("textDocument/didClose", {
      textDocument: { uri },
    });
  } catch {
    // ignored
  }
}
