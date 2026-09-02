import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import {
  AbstractMessageReader,
  AbstractMessageWriter,
  createMessageConnection,
  DidChangeTextDocumentNotification,
  DidCloseTextDocumentNotification,
  DidOpenTextDocumentNotification,
  DocumentFormattingRequest,
  DocumentSymbolRequest,
  InitializeRequest,
  InitializedNotification,
  PublishDiagnosticsNotification,
  SemanticTokensRequest,
  type DataCallback,
  type Diagnostic,
  type Disposable,
  type Message,
  type MessageConnection,
  type Range,
  type SemanticTokensLegend,
  type SymbolInformation,
  type TextEdit,
} from "vscode-languageserver-protocol/browser";

/** LSP positions/ranges are 0-indexed; Monaco's are 1-indexed. */
export function toMonacoRange(range: Range) {
  return {
    startLineNumber: range.start.line + 1,
    startColumn: range.start.character + 1,
    endLineNumber: range.end.line + 1,
    endColumn: range.end.character + 1,
  };
}

/** Reads complete LSP messages off the `lsp://message` Tauri event. Rust does
 * the Content-Length framing, so each event payload is already one full,
 * valid JSON-RPC message string. */
class TauriMessageReader extends AbstractMessageReader {
  private callback: DataCallback | undefined;
  private unlisten: (() => void) | undefined;

  // Tauri's listen() is itself an async IPC round trip to register the
  // subscription. Without waiting for it, connect() could send `initialize`
  // before the listener actually exists, and the response would be dropped
  // on the floor with zero visible symptoms - a permanently-pending
  // getConnection() promise, no error, no diagnostics, nothing.
  readonly ready: Promise<void>;

  constructor() {
    super();
    this.ready = listen<string>("lsp://message", (event) => {
      try {
        this.callback?.(JSON.parse(event.payload) as Message);
      } catch (e) {
        console.error("[lsp] failed to parse message from server", e, event.payload);
        this.fireError(e);
      }
    }).then((fn) => {
      this.unlisten = fn;
    });
  }

  listen(callback: DataCallback): Disposable {
    this.callback = callback;
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
      console.error("[lsp] failed to send message", e, msg);
      this.fireError(e, msg, 1);
    }
  }

  end(): void { }
}

let connectionPromise: Promise<MessageConnection> | undefined;

function diagnosticsToMarkers(diagnostics: Diagnostic[]) {
  const severityMap: Record<number, number> = { 1: 8, 2: 4, 3: 2, 4: 1 }; // monaco.MarkerSeverity Error/Warning/Info/Hint
  return diagnostics.map((d) => ({
    severity: severityMap[d.severity ?? 1] ?? 8,
    // cwl-lsp always sends a plain string (Rust's Diagnostic.message is a
    // String, not the MarkupContent variant), but the LSP spec allows both.
    message: typeof d.message === "string" ? d.message : d.message.value,
    ...toMonacoRange(d.range),
  }));
}

let onDiagnostics: ((uri: string, diagnostics: Diagnostic[]) => void) | undefined;

/** Registers the callback invoked with (uri, markers) whenever the server
 * publishes diagnostics for a document. Editor.svelte wires this into
 * monaco.editor.setModelMarkers. */
export function setDiagnosticsHandler(handler: (uri: string, markers: ReturnType<typeof diagnosticsToMarkers>) => void) {
  onDiagnostics = (uri, diagnostics) => handler(uri, diagnosticsToMarkers(diagnostics));
}

let semanticTokensLegend: SemanticTokensLegend | null = null;

async function connect(): Promise<MessageConnection> {
  const reader = new TauriMessageReader();
  const connection = createMessageConnection(reader, new TauriMessageWriter());
  connection.onNotification(PublishDiagnosticsNotification.type, (params) => {
    onDiagnostics?.(params.uri, params.diagnostics);
  });
  connection.onError(([error]) => console.error("[lsp] connection error", error));
  connection.onClose(() => console.error("[lsp] connection closed"));
  connection.onUnhandledNotification((n) => console.warn("[lsp] unhandled notification", n));
  connection.listen();
  await reader.ready;

  const result = await connection.sendRequest(InitializeRequest.type, {
    processId: null,
    rootUri: null,
    capabilities: {},
  });
  semanticTokensLegend = result.capabilities.semanticTokensProvider?.legend ?? null;
  connection.sendNotification(InitializedNotification.type, {});

  return connection;
}

function getConnection(): Promise<MessageConnection> {
  if (!connectionPromise) connectionPromise = connect();
  return connectionPromise;
}

/** Resolves once the handshake has completed and the server's semantic
 * token legend (token type/modifier names, positional) is known. */
export async function getSemanticTokensLegend(): Promise<SemanticTokensLegend | null> {
  await getConnection();
  return semanticTokensLegend;
}

// The editor must stay usable even if the in-process LSP never comes up (or
// hiccups on one message), so every notification here is fire-and-forget.
export async function notifyDidOpen(uri: string, languageId: string, text: string) {
  try {
    const connection = await getConnection();
    connection.sendNotification(DidOpenTextDocumentNotification.type, {
      textDocument: { uri, languageId, version: 1, text },
    });
  } catch (e) {
    console.error("[lsp] didOpen failed", e);
  }
}

const versions = new Map<string, number>();

export async function notifyDidChange(uri: string, text: string) {
  try {
    const connection = await getConnection();
    const version = (versions.get(uri) ?? 1) + 1;
    versions.set(uri, version);
    connection.sendNotification(DidChangeTextDocumentNotification.type, {
      textDocument: { uri, version },
      contentChanges: [{ text }],
    });
  } catch (e) {
    console.error("[lsp] didChange failed", e);
  }
}

export async function notifyDidClose(uri: string) {
  try {
    const connection = await getConnection();
    versions.delete(uri);
    connection.sendNotification(DidCloseTextDocumentNotification.type, {
      textDocument: { uri },
    });
  } catch (e) {
    console.error("[lsp] didClose failed", e);
  }
}

export async function requestFormatting(uri: string, tabSize: number, insertSpaces: boolean): Promise<TextEdit[]> {
  try {
    const connection = await getConnection();
    const edits = await connection.sendRequest(DocumentFormattingRequest.type, {
      textDocument: { uri },
      options: { tabSize, insertSpaces },
    });
    return edits ?? [];
  } catch (e) {
    console.error("[lsp] formatting request failed", e);
    return [];
  }
}

export async function requestDocumentSymbols(uri: string): Promise<SymbolInformation[]> {
  try {
    const connection = await getConnection();
    const symbols = await connection.sendRequest(DocumentSymbolRequest.type, {
      textDocument: { uri },
    });
    // cwl-lsp always returns the flat SymbolInformation[] shape (see
    // DocumentSymbolResponse::Flat in backend.rs), never the hierarchical
    // DocumentSymbol[] the response type also allows.
    return (symbols as SymbolInformation[] | null) ?? [];
  } catch (e) {
    console.error("[lsp] documentSymbol request failed", e);
    return [];
  }
}

/** Returns the flat, relative-encoded token data exactly as cwl-lsp sends it
 * (five uints per token: deltaLine, deltaStart, length, tokenType,
 * tokenModifiersBitset) - the same encoding Monaco's semantic tokens API
 * expects, so no reshaping is needed beyond `new Uint32Array(...)`. */
export async function requestSemanticTokens(uri: string): Promise<number[] | null> {
  try {
    const connection = await getConnection();
    const result = await connection.sendRequest(SemanticTokensRequest.type, {
      textDocument: { uri },
    });
    return result?.data ?? null;
  } catch (e) {
    console.error("[lsp] semanticTokens request failed", e);
    return null;
  }
}
