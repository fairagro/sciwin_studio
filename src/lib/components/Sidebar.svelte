<script lang="ts">
  import { open } from "@tauri-apps/plugin-dialog";
  import { FolderOpen, X } from "@lucide/svelte";
  import { workspace } from "$lib/state/workspace.svelte";

  async function openProject() {
    const dir = await open({ directory: true, multiple: false });
    if (typeof dir === "string") {
      workspace.openProject(dir);
    }
  }

  const sectionLabels = {
    workflows: "Workflows",
    filesystem: "Filesystem",
    sourcecontrol: "Source Control",
  } as const;

  const comingSoon = {
    workflows: "File tree coming soon.",
    filesystem: "Filesystem browser coming soon.",
    sourcecontrol: "Source control panel coming soon.",
  } as const;
</script>

<aside
  class="flex shrink-0 flex-col border-r border-border bg-bg-panel select-none"
  style="width: {workspace.sidebarWidth}px"
>
  <div class="flex items-center gap-2 border-b border-border-soft px-3 py-2.5">
    <FolderOpen size={14} strokeWidth={1.6} class="shrink-0 text-text-2" />
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
        <X size={12} strokeWidth={1.8} />
      </button>
    {/if}
  </div>

  {#if workspace.projectPath}
    <div class="px-3 pt-2.5 pb-1">
      <span class="font-mono text-[10px] tracking-widest text-text-3 uppercase">{sectionLabels[workspace.sidebarView]}</span>
    </div>
  {/if}

  <div class="flex-1 overflow-y-auto p-2">
    {#if workspace.projectPath}
      <p class="px-1 font-mono text-[11px] text-text-3">{comingSoon[workspace.sidebarView]}</p>
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
