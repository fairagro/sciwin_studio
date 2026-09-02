<script lang="ts">
  import * as monaco from "monaco-editor";
  import editorWorker from "monaco-editor/editor/editor.worker?worker";
  import { invoke } from "@tauri-apps/api/core";
  import { listen } from "@tauri-apps/api/event";
  import { onMount, onDestroy } from "svelte";
  import { workspace } from "$lib/state/workspace.svelte";
  import { notifyDidChange, notifyDidClose, notifyDidOpen, setDiagnosticsHandler } from "$lib/lsp/connection";
  import { registerLspProviders } from "$lib/lsp/providers";
  import type { WorkflowChanged } from "$lib/graph/types";

  function isCwl(name: string): boolean {
    return name.toLowerCase().endsWith(".cwl");
  }

  const BINARY_EXTENSIONS = new Set([
    "png", "jpg", "jpeg", "gif", "bmp", "ico", "webp", "tiff", "tif", "avif", "heic",
    "zip", "tar", "gz", "tgz", "bz2", "xz", "7z", "rar", "jar",
    "exe", "dll", "so", "dylib", "bin", "o", "a", "class", "wasm",
    "pdf", "doc", "docx", "xls", "xlsx", "ppt", "pptx",
    "mp3", "mp4", "wav", "ogg", "mov", "avi", "mkv", "flac", "webm",
    "ttf", "otf", "woff", "woff2", "eot",
    "db", "sqlite", "sqlite3", "pyc", "dat", "iso", "dmg",
  ]);

  function isLikelyBinary(name: string): boolean {
    const ext = name.split(".").pop()?.toLowerCase() ?? "";
    return BINARY_EXTENSIONS.has(ext);
  }

  let containerEl: HTMLDivElement;
  let editor: monaco.editor.IStandaloneCodeEditor;
  let unsupported = $state(false);
  const models = new Map<string, monaco.editor.ITextModel>();

  function languageFor(name: string): string {
    const ext = name.split(".").pop()?.toLowerCase() ?? "";
    switch (ext) {
      case "cwl":
      case "yml":
      case "yaml":
        return "yaml";
      case "json":
        return "json";
      case "py":
        return "python";
      case "js":
      case "mjs":
      case "cjs":
        return "javascript";
      case "ts":
        return "typescript";
      case "sh":
        return "shell";
      case "md":
        return "markdown";
      case "toml":
        return "ini";
      default:
        return "plaintext";
    }
  }

  const changeTimers = new Map<string, ReturnType<typeof setTimeout>>();
  // Paths whose model is being overwritten to match disk (a graph mutation
  // or another window's save), so the next onDidChangeContent for that path
  // shouldn't mark the tab dirty or be mistaken for a user edit.
  const syncingPaths = new Set<string>();

  async function modelFor(path: string, name: string) {
    let model = models.get(path);
    if (model) return model;

    let content: string;
    try {
      content = await invoke<string>("read_file", { path });
    } catch {
      // binary content, or the read genuinely failed - never fabricate an
      // empty writable model, since saving it would blank the real file
      return null;
    }

    model = monaco.editor.createModel(content, languageFor(name), monaco.Uri.file(path));
    const uri = model.uri.toString();
    const cwl = isCwl(name);

    if (cwl) notifyDidOpen(uri, "yaml", content);

    model.onDidChangeContent(() => {
      const tab = workspace.tabs.find((t) => t.path === path);
      if (!syncingPaths.delete(path) && tab) tab.dirty = true;

      if (!cwl) return;
      clearTimeout(changeTimers.get(path));
      changeTimers.set(
        path,
        setTimeout(() => notifyDidChange(uri, model!.getValue()), 300),
      );
    });
    models.set(path, model);
    return model;
  }

  async function save() {
    const tab = workspace.activeTab;
    const model = tab && models.get(tab.path);
    if (!tab || !model) return;
    await invoke("write_file", { path: tab.path, contents: model.getValue() }).catch(() => {});
    tab.dirty = false;
  }

  onMount(() => {
    (self as typeof self & { MonacoEnvironment: monaco.Environment }).MonacoEnvironment = {
      getWorker: () => new editorWorker(),
    };

    monaco.editor.defineTheme("sciwin-dark", {
      base: "vs-dark",
      inherit: true,
      rules: [],
      colors: {
        "editor.background": "#0d0e10",
        "editor.foreground": "#eef0f2",
        "editorLineNumber.foreground": "#6b7078",
        "editorLineNumber.activeForeground": "#9aa0a8",
        "editorCursor.foreground": "#6abf5c",
        "editor.selectionBackground": "#2b2e3399",
        "editorGutter.background": "#0d0e10",
        "editor.lineHighlightBackground": "#151719",
        "editorIndentGuide.background": "#24262a",
        "editorWhitespace.foreground": "#24262a",
      },
    });

    editor = monaco.editor.create(containerEl, {
      theme: "sciwin-dark",
      automaticLayout: true,
      fontFamily: "'IBM Plex Mono', ui-monospace, monospace",
      fontSize: 13,
      minimap: { enabled: false },
      scrollBeyondLastLine: false,
    });

    editor.addCommand(monaco.KeyMod.CtrlCmd | monaco.KeyCode.KeyS, save);

    setDiagnosticsHandler((uri, markers) => {
      const model = monaco.editor.getModel(monaco.Uri.parse(uri));
      if (model) monaco.editor.setModelMarkers(model, "cwl-lsp", markers);
    });
    registerLspProviders();
  });

  onDestroy(() => {
    editor?.dispose();
    for (const model of models.values()) model.dispose();
  });

  // Keeps an already-open code tab in sync with a graph mutation's write to
  // the same file (or another window's save). Skipped while the tab is
  // dirty, so it never clobbers unsaved edits.
  onMount(() => {
    const unlisten = listen<WorkflowChanged>("workflow-changed", async (event) => {
      const { path } = event.payload;
      const model = models.get(path);
      const tab = workspace.tabs.find((t) => t.path === path);
      if (!model || tab?.dirty) return;

      const content = await invoke<string>("read_file", { path }).catch(() => null);
      if (content === null || content === model.getValue()) return;

      syncingPaths.add(path);
      model.setValue(content);
    });
    return () => {
      unlisten.then((fn) => fn());
    };
  });

  $effect(() => {
    const tab = workspace.activeTab;
    if (!tab || !editor) return;

    if (isLikelyBinary(tab.name)) {
      unsupported = true;
      return;
    }

    modelFor(tab.path, tab.name).then((model) => {
      if (!model) {
        unsupported = true;
        return;
      }
      unsupported = false;
      if (editor.getModel() !== model) editor.setModel(model);
      requestAnimationFrame(() => editor.layout());
    });
  });

  $effect(() => {
    const openPaths = new Set(workspace.tabs.map((t) => t.path));
    for (const [path, model] of models) {
      if (!openPaths.has(path)) {
        clearTimeout(changeTimers.get(path));
        changeTimers.delete(path);
        if (isCwl(path)) notifyDidClose(model.uri.toString());
        model.dispose();
        models.delete(path);
      }
    }
  });
</script>

<div class="relative h-full w-full">
  <div bind:this={containerEl} class="h-full w-full"></div>
  {#if unsupported}
    <div class="absolute inset-0 flex items-center justify-center bg-bg font-mono text-sm text-text-3">
      This file type isn't supported for editing.
    </div>
  {/if}
</div>
