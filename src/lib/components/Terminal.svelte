<script lang="ts">
  import "@xterm/xterm/css/xterm.css";
  import { Terminal } from "@xterm/xterm";
  import { FitAddon } from "@xterm/addon-fit";
  import { invoke } from "@tauri-apps/api/core";
  import { listen } from "@tauri-apps/api/event";
  import { onMount, onDestroy } from "svelte";
  import { SquareTerminal, ScrollText, RotateCw, X } from "@lucide/svelte";
  import { workspace } from "$lib/state/workspace.svelte";
  import { execution } from "$lib/state/execution.svelte";

  let consolePanel = $state<"terminal" | "output">("terminal");
  let outputEl: HTMLDivElement;

  // Autoscroll only if the viewer was already at (or near) the bottom -- otherwise new
  // output while they've scrolled up to read an earlier step would keep yanking them back down.
  $effect(() => {
    void execution.output.length;
    if (consolePanel !== "output" || !outputEl) return;
    const nearBottom = outputEl.scrollHeight - outputEl.scrollTop - outputEl.clientHeight < 48;
    if (nearBottom) {
      requestAnimationFrame(() => (outputEl.scrollTop = outputEl.scrollHeight));
    }
  });

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
  <div class="flex h-8 shrink-0 items-center gap-1 border-b border-border-soft px-1.5">
    <button
      type="button"
      onclick={() => (consolePanel = "terminal")}
      class="flex items-center gap-1.5 rounded px-2 py-1 font-mono text-[11px] {consolePanel === 'terminal'
        ? 'bg-border-soft text-text'
        : 'text-text-2 hover:text-text'}"
    >
      <SquareTerminal size={13} strokeWidth={1.8} />
      Terminal
    </button>
    <button
      type="button"
      onclick={() => (consolePanel = "output")}
      class="flex items-center gap-1.5 rounded px-2 py-1 font-mono text-[11px] {consolePanel === 'output'
        ? 'bg-border-soft text-text'
        : 'text-text-2 hover:text-text'}"
    >
      <ScrollText size={13} strokeWidth={1.8} />
      Output
      {#if execution.isRunning}
        <span class="h-1.5 w-1.5 shrink-0 rounded-full bg-fairagro-mid-500"></span>
      {/if}
    </button>
    <div class="flex-1"></div>
    {#if consolePanel === "terminal"}
      <button
        type="button"
        class="rounded p-0.5 text-text-3 hover:bg-border-soft hover:text-text"
        title="Restart terminal"
        onclick={restartShell}
      >
        <RotateCw size={12} strokeWidth={1.8} />
      </button>
    {/if}
    <button
      type="button"
      class="rounded p-0.5 text-text-3 hover:bg-border-soft hover:text-text"
      title="Hide panel"
      onclick={() => workspace.toggleTerminal()}
    >
      <X size={12} strokeWidth={1.8} />
    </button>
  </div>
  <div bind:this={containerEl} class="min-h-0 flex-1 overflow-hidden bg-bg-well p-1.5 {consolePanel === 'terminal' ? '' : 'hidden'}"></div>
  <div
    bind:this={outputEl}
    class="mr-2 min-h-0 min-w-0 flex-1 overflow-y-auto overflow-x-hidden bg-bg-well p-2 font-mono text-[11px] {consolePanel ===
    'output'
      ? ''
      : 'hidden'}"
  >
    {#if execution.error}
      <pre class="mb-3 min-w-0 wrap-break-word whitespace-pre-wrap rounded-md border border-fairagro-red/40 bg-fairagro-red/10 p-2 text-fairagro-red-light">{execution.error}</pre>
    {/if}
    {#if execution.output.length === 0 && !execution.error}
      <p class="text-text-3">No step output yet -- appears here as each step finishes.</p>
    {:else if execution.output.length > 0}
      {#each execution.output as entry, i (i)}
        <div class="mb-2.5 min-w-0">
          <div class="mb-0.5 text-text-2">{entry.stepId}</div>
          {#if entry.stdout}
            <pre class="min-w-0 wrap-break-word whitespace-pre-wrap text-text">{entry.stdout}</pre>
          {/if}
          {#if entry.stderr}
            <pre class="min-w-0 wrap-break-word whitespace-pre-wrap text-fairagro-red-light">{entry.stderr}</pre>
          {/if}
        </div>
      {/each}
    {/if}
  </div>
</div>
