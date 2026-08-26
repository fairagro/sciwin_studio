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

<aside class="flex w-64 shrink-0 flex-col border-r border-rule bg-surface-2 select-none">
  <div class="flex items-center justify-between gap-2 border-b border-rule px-3 py-2.5">
    <span class="truncate text-sm font-medium text-ink" title={workspace.projectName ?? undefined}>
      {workspace.projectName ?? "No project"}
    </span>
    {#if workspace.projectPath}
      <button
        type="button"
        class="shrink-0 rounded p-0.5 text-ink-3 hover:bg-fairagro-red-light/20 hover:text-fairagro-red"
        title="Close project"
        onclick={() => workspace.closeProject()}
      >
        &times;
      </button>
    {/if}
  </div>

  <div class="flex-1 overflow-y-auto p-2">
    {#if workspace.projectPath}
      <p class="px-1 text-xs text-ink-3">File tree coming soon.</p>
    {:else}
      <div class="flex flex-col items-center gap-3 px-2 pt-12 text-center">
        <p class="text-xs text-ink-3">Open a folder to get started</p>
        <button
          type="button"
          class="rounded-md bg-fairagro-dark-500 px-3 py-1.5 text-xs font-medium text-white hover:bg-fairagro-dark-400"
          onclick={openProject}
        >
          Open Folder
        </button>
      </div>
    {/if}
  </div>
</aside>
