<script lang="ts">
  import { workspace, type SidebarView } from "$lib/state/workspace.svelte";
  import { Workflow, Folder, GitBranch, SquareTerminal, Settings } from "@lucide/svelte";

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
    <Workflow size={19} strokeWidth={1.8} />
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
    <Folder size={19} strokeWidth={1.8} />
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
    <GitBranch size={19} strokeWidth={1.8} />
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
    <SquareTerminal size={19} strokeWidth={1.8} />
    <span class="font-mono text-[8.5px] font-semibold whitespace-nowrap">Console</span>
  </button>

  <div class="flex-1"></div>

  <button type="button" class="mb-0.5 flex w-12 flex-col items-center justify-center gap-1 rounded-md py-1.75 text-text-2 hover:bg-border-soft hover:text-text" title="Settings">
    <Settings size={19} strokeWidth={1.8} />
    <span class="font-mono text-[8.5px] font-semibold whitespace-nowrap">Settings</span>
  </button>
</div>
