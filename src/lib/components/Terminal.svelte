<script lang="ts">
  import "@xterm/xterm/css/xterm.css";
  import { Terminal } from "@xterm/xterm";
  import { FitAddon } from "@xterm/addon-fit";
  import { invoke } from "@tauri-apps/api/core";
  import { listen } from "@tauri-apps/api/event";
  import { onMount, onDestroy } from "svelte";
  import { workspace } from "$lib/state/workspace.svelte";

  interface S4nStatus {
    installed: boolean;
    version: string | null;
    installHint: string;
  }

  let containerEl: HTMLDivElement;
  let term: Terminal;
  let fitAddon: FitAddon;
  let resizeObserver: ResizeObserver;
  let unlistenOutput: (() => void) | undefined;

  function fitAndResize() {
    if (!term.element) return;
    fitAddon.fit();
    invoke("pty_resize", { cols: term.cols, rows: term.rows }).catch(() => {});
  }

  onMount(async () => {
    term = new Terminal({
      fontFamily: "'IBM Plex Mono', ui-monospace, monospace",
      fontSize: 12,
      theme: {
        background: "#0d0e10",
        foreground: "#eef0f2",
        cursor: "#6abf5c",
        cursorAccent: "#0d0e10",
        selectionBackground: "#2b2e33",
      },
    });
    fitAddon = new FitAddon();
    term.loadAddon(fitAddon);
    term.open(containerEl);
    fitAddon.fit();

    const s4n = await invoke<S4nStatus>("check_s4n").catch(() => null);
    if (s4n && !s4n.installed) {
      term.writeln("\x1b[33;1mWarning: s4n was not found on PATH.\x1b[0m");
      term.writeln("SciWIn-Studio works best with the s4n CLI to create, run, and manage CWL workflows.");
      term.writeln("Install it with:");
      term.writeln("");
      term.writeln(`  ${s4n.installHint}`);
      term.writeln("");
    }

    invoke("pty_spawn", { cwd: workspace.projectPath, cols: term.cols, rows: term.rows }).catch((err) => {
      term.write(`\r\nfailed to start terminal: ${err}\r\n`);
    });

    listen<string>("pty-output", (event) => term.write(event.payload)).then((fn) => {
      unlistenOutput = fn;
    });

    term.onData((data) => {
      invoke("pty_write", { data }).catch(() => {});
    });

    resizeObserver = new ResizeObserver(() => fitAndResize());
    resizeObserver.observe(containerEl);
  });

  onDestroy(() => {
    resizeObserver?.disconnect();
    unlistenOutput?.();
    term?.dispose();
  });

  $effect(() => {
    if (workspace.terminalOpen && term) {
      requestAnimationFrame(() => {
        fitAndResize();
        term.focus();
      });
    }
  });
</script>

<div class="flex h-44 shrink-0 flex-col border-t border-border bg-bg-panel {workspace.terminalOpen ? '' : 'hidden'}">
  <div class="flex h-8 shrink-0 items-center gap-2 border-b border-border-soft px-2.5">
    <svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="var(--text-2)" stroke-width="2">
      <path d="M4 5h16v14H4z" />
      <path d="M8 9l3 3-3 3" />
      <path d="M13 15h4" />
    </svg>
    <span class="font-mono text-[11px] text-text">Console</span>
    <div class="flex-1"></div>
    <button type="button" class="rounded p-0.5 text-text-3 hover:bg-border-soft hover:text-text" title="Close panel" onclick={() => workspace.toggleTerminal()}> &times; </button>
  </div>
  <div bind:this={containerEl} class="min-h-0 flex-1 overflow-hidden bg-bg-well p-1.5"></div>
</div>
