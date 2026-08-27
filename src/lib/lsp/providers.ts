import * as monaco from "monaco-editor";
import {
  getSemanticTokensLegend,
  requestDocumentSymbols,
  requestFormatting,
  requestSemanticTokens,
  toMonacoRange,
} from "./connection";

// cwl-lsp only ever has a document open for files we sent didOpen for -
// Editor.svelte gates that to .cwl tabs, so these providers must too, even
// though they're registered on the shared "yaml" language id.
function isCwlModel(model: monaco.editor.ITextModel): boolean {
  return model.uri.path.toLowerCase().endsWith(".cwl");
}

let registered = false;

/** Registers formatting, document symbol (outline) and semantic token
 * providers against cwl-lsp. Call once; safe to call more than once. */
export async function registerLspProviders() {
  if (registered) return;
  registered = true;

  monaco.languages.registerDocumentFormattingEditProvider("yaml", {
    async provideDocumentFormattingEdits(model, options) {
      if (!isCwlModel(model)) return [];
      const edits = await requestFormatting(model.uri.toString(), options.tabSize, options.insertSpaces);
      return edits.map((e) => ({ range: toMonacoRange(e.range), text: e.newText }));
    },
  });

  monaco.languages.registerDocumentSymbolProvider("yaml", {
    async provideDocumentSymbols(model) {
      if (!isCwlModel(model)) return [];
      const symbols = await requestDocumentSymbols(model.uri.toString());
      return symbols.map((s) => {
        const range = toMonacoRange(s.location.range);
        return {
          name: s.name,
          detail: "",
          // LSP SymbolKind is 1-indexed (File = 1); Monaco's is 0-indexed
          // (File = 0) over the same ordered list, so it's a plain offset.
          kind: (s.kind - 1) as monaco.languages.SymbolKind,
          tags: [],
          range,
          selectionRange: range,
        };
      });
    },
  });

  const legend = (await getSemanticTokensLegend()) ?? { tokenTypes: [], tokenModifiers: [] };
  monaco.languages.registerDocumentSemanticTokensProvider("yaml", {
    getLegend: () => legend,
    async provideDocumentSemanticTokens(model) {
      if (!isCwlModel(model)) return null;
      const data = await requestSemanticTokens(model.uri.toString());
      return data ? { data: new Uint32Array(data) } : null;
    },
    releaseDocumentSemanticTokens: () => {},
  });
}
