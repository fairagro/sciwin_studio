<script lang="ts">
  import { workspace, type SidebarView } from "$lib/state/workspace.svelte";

  function isActiveView(view: SidebarView) {
    return workspace.sidebarView === view && !workspace.sidebarCollapsed;
  }
</script>

<div class="flex w-14 shrink-0 flex-col items-center gap-0.75 border-r border-border bg-bg-panel py-2 select-none">
  <button
    type="button"
    class="flex w-12 flex-col items-center justify-center gap-1 rounded-md py-1.75 text-text-2 hover:bg-border-soft hover:text-text {isActiveView('workflows')
      ? 'bg-fairagro-mid-500/14 text-fairagro-mid-500 shadow-[inset_-2px_0_0_0_var(--color-fairagro-mid-500)]'
      : ''}"
    title="Workflows"
    onclick={() => workspace.selectSidebarView("workflows")}
  >
    <svg width="19" height="19" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round">
      <circle cx="7" cy="7" r="2.4" />
      <circle cx="17" cy="7" r="2.4" />
      <circle cx="12" cy="18" r="2.4" />
      <path d="M9 8.4 12 15.6" />
      <path d="M15 8.4 12 15.6" />
    </svg>
    <span class="font-mono text-[8.5px] font-semibold whitespace-nowrap">Flows</span>
  </button>

  <button
    type="button"
    class="flex w-12 flex-col items-center justify-center gap-1 rounded-md py-1.75 text-text-2 hover:bg-border-soft hover:text-text {isActiveView('filesystem')
      ? 'bg-fairagro-mid-500/14 text-fairagro-mid-500 shadow-[inset_-2px_0_0_0_var(--color-fairagro-mid-500)]'
      : ''}"
    title="Filesystem"
    onclick={() => workspace.selectSidebarView("filesystem")}
  >
    <svg width="19" height="19" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8">
      <path d="M3 7a1 1 0 0 1 1-1h4l2 2h10a1 1 0 0 1 1 1v9a1 1 0 0 1-1 1H4a1 1 0 0 1-1-1V7z" />
    </svg>
    <span class="font-mono text-[8.5px] font-semibold whitespace-nowrap">Files</span>
  </button>

  <button
    type="button"
    class="flex w-12 flex-col items-center justify-center gap-1 rounded-md py-1.75 text-text-2 hover:bg-border-soft hover:text-text {isActiveView('sourcecontrol')
      ? 'bg-fairagro-mid-500/14 text-fairagro-mid-500 shadow-[inset_-2px_0_0_0_var(--color-fairagro-mid-500)]'
      : ''}"
    title="Source Control"
    onclick={() => workspace.selectSidebarView("sourcecontrol")}
  >
    <svg width="19" height="19" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.1">
      <circle cx="6" cy="6" r="2.3" />
      <circle cx="6" cy="18" r="2.3" />
      <path d="M6 8.3V18" />
      <path d="M6 8.3a8 8 0 0 0 8 8" />
    </svg>
    <span class="font-mono text-[8.5px] font-semibold whitespace-nowrap">Git</span>
  </button>

  <div class="my-1.5 h-px w-5.5 bg-border-soft"></div>

  <button
    type="button"
    class="flex w-12 flex-col items-center justify-center gap-1 rounded-md py-1.75 text-text-2 hover:bg-border-soft hover:text-text {workspace.terminalOpen
      ? 'bg-fairagro-mid-500/14 text-fairagro-mid-500 shadow-[inset_-2px_0_0_0_var(--color-fairagro-mid-500)]'
      : ''}"
    title="Console"
    onclick={() => workspace.toggleTerminal()}
  >
    <svg width="19" height="19" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.9">
      <path d="M4 5h16v14H4z" />
      <path d="M8 9l3 3-3 3" />
      <path d="M13 15h4" />
    </svg>
    <span class="font-mono text-[8.5px] font-semibold whitespace-nowrap">Console</span>
  </button>

  <div class="flex-1"></div>

  <button type="button" class="mb-0.5 flex w-12 flex-col items-center justify-center gap-1 rounded-md py-1.75 text-text-2 hover:bg-border-soft hover:text-text" title="Settings">
    <svg width="19" height="19" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8">
      <circle cx="12" cy="12" r="3.4" />
      <path d="M12 3v2.4M12 18.6V21M21 12h-2.4M5.4 12H3M18.4 5.6l-1.7 1.7M7.3 16.7l-1.7 1.7M18.4 18.4l-1.7-1.7M7.3 7.3 5.6 5.6" />
    </svg>
    <span class="font-mono text-[8.5px] font-semibold whitespace-nowrap">Settings</span>
  </button>
</div>
