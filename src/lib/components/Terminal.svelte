<script lang="ts">
  import "@xterm/xterm/css/xterm.css";
  import { Terminal } from "@xterm/xterm";
  import { FitAddon } from "@xterm/addon-fit";
  import { invoke } from "@tauri-apps/api/core";
  import { listen } from "@tauri-apps/api/event";
  import { onMount, onDestroy } from "svelte";
  import { SquareTerminal, RotateCw, X } from "@lucide/svelte";
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
  let started = false;

  function fitAndResize() {
    if (!term.element) return;
    fitAddon.fit();
    invoke("pty_resize", { cols: term.cols, rows: term.rows }).catch(() => {});
  }

  async function startShell() {
    if (started) return;
    started = true;

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

    unlistenOutput = await listen<string>("pty-output", (event) => term.write(event.payload));
  }

  async function restartShell() {
    await invoke("pty_kill").catch(() => {});
    unlistenOutput?.();
    unlistenOutput = undefined;
    term.reset();
    started = false;
    await startShell();
  }

  onMount(() => {
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
      startShell();
      requestAnimationFrame(() => {
        fitAndResize();
        term.focus();
      });
    }
  });
</script>

<div
  class="flex shrink-0 flex-col border-t border-border bg-bg-panel {workspace.terminalOpen ? '' : 'hidden'}"
  style="height: {workspace.terminalHeight}px"
>
  <div class="flex h-8 shrink-0 items-center gap-2 border-b border-border-soft px-2.5">
    <SquareTerminal size={13} strokeWidth={1.8} class="text-text-2" />
    <span class="font-mono text-[11px] text-text">Console</span>
    <div class="flex-1"></div>
    <button
      type="button"
      class="rounded p-0.5 text-text-3 hover:bg-border-soft hover:text-text"
      title="Restart terminal"
      onclick={restartShell}
    >
      <RotateCw size={12} strokeWidth={1.8} />
    </button>
    <button
      type="button"
      class="rounded p-0.5 text-text-3 hover:bg-border-soft hover:text-text"
      title="Hide panel"
      onclick={() => workspace.toggleTerminal()}
    >
      <X size={12} strokeWidth={1.8} />
    </button>
  </div>
  <div bind:this={containerEl} class="min-h-0 flex-1 overflow-hidden bg-bg-well p-1.5"></div>
</div>
