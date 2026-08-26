<script lang="ts">
  import { open } from "@tauri-apps/plugin-dialog";
  import { workspace } from "$lib/state/workspace.svelte";

  async function openProject() {
    const dir = await open({ directory: true, multiple: false });
    if (typeof dir === "string") {
      workspace.openProject(dir);
    }
  }
</script>

<aside class="flex w-[248px] shrink-0 flex-col border-r border-border bg-bg-panel select-none">
  <div class="flex items-center gap-2 border-b border-border-soft px-3 py-2.5">
    <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="var(--text-2)" stroke-width="1.6" class="shrink-0">
      <path d="M6 3h9l5 5v13a1 1 0 0 1-1 1H6a1 1 0 0 1-1-1V4a1 1 0 0 1 1-1z" />
      <path d="M14 3v6h6" />
    </svg>
    <span class="flex-1 truncate font-mono text-xs text-text" title={workspace.projectName ?? undefined}>
      {workspace.projectName ?? "No project"}
    </span>
    {#if workspace.projectPath}
      <button
        type="button"
        class="shrink-0 rounded p-0.5 text-text-3 hover:bg-fairagro-red-light/20 hover:text-fairagro-red-light"
        title="Close project"
        onclick={() => workspace.closeProject()}
      >
        &times;
      </button>
    {/if}
  </div>

  <div class="flex-1 overflow-y-auto p-2">
    {#if workspace.projectPath}
      <p class="px-1 font-mono text-[11px] text-text-3">File tree coming soon.</p>
    {:else}
      <div class="flex flex-col items-center gap-3 px-2 pt-12 text-center">
        <p class="font-mono text-[11px] text-text-3">Open a folder to get started</p>
        <button
          type="button"
          class="rounded-md bg-fairagro-mid-500 px-3 py-1.5 text-xs font-medium text-bg hover:bg-fairagro-mid-400"
          onclick={openProject}
        >
          Open Folder
        </button>
      </div>
    {/if}
  </div>
</aside>
