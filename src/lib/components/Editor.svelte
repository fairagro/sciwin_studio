<script lang="ts">
  import * as monaco from "monaco-editor";
  import editorWorker from "monaco-editor/editor/editor.worker?worker";
  import { invoke } from "@tauri-apps/api/core";
  import { onMount, onDestroy } from "svelte";
  import { workspace } from "$lib/state/workspace.svelte";

  let containerEl: HTMLDivElement;
  let editor: monaco.editor.IStandaloneCodeEditor;
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

  async function modelFor(path: string, name: string) {
    let model = models.get(path);
    if (model) return model;

    const content = await invoke<string>("read_file", { path }).catch(() => "");
    model = monaco.editor.createModel(content, languageFor(name), monaco.Uri.file(path));
    model.onDidChangeContent(() => {
      const tab = workspace.tabs.find((t) => t.path === path);
      if (tab) tab.dirty = true;
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
  });

  onDestroy(() => {
    editor?.dispose();
    for (const model of models.values()) model.dispose();
  });

  $effect(() => {
    const tab = workspace.activeTab;
    if (!tab || !editor) return;
    modelFor(tab.path, tab.name).then((model) => {
      if (editor.getModel() !== model) editor.setModel(model);
      requestAnimationFrame(() => editor.layout());
    });
  });

  $effect(() => {
    const openPaths = new Set(workspace.tabs.map((t) => t.path));
    for (const [path, model] of models) {
      if (!openPaths.has(path)) {
        model.dispose();
        models.delete(path);
      }
    }
  });
</script>

<div bind:this={containerEl} class="h-full w-full"></div>
