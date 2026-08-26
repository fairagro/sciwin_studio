<script lang="ts">
  import { open } from "@tauri-apps/plugin-dialog";
  import { invoke } from "@tauri-apps/api/core";
  import { FolderOpen, RotateCw, X } from "@lucide/svelte";
  import { workspace } from "$lib/state/workspace.svelte";
  import FileTreeNode, { type FsEntry } from "./FileTreeNode.svelte";
  import ConfirmDialog from "./ConfirmDialog.svelte";

  let showInitPrompt = $state(false);
  let pendingProjectPath = $state<string | null>(null);
  let initBusy = $state(false);
  let initError = $state<string | null>(null);

  async function openProject() {
    const dir = await open({ directory: true, multiple: false });
    if (typeof dir !== "string") return;

    const hasConfig = await invoke<boolean>("has_workflow_config", { path: dir });
    if (hasConfig) {
      workspace.openProject(dir, true);
      return;
    }

    initError = null;
    pendingProjectPath = dir;
    showInitPrompt = true;
  }

  async function confirmInit() {
    if (!pendingProjectPath) return;
    initBusy = true;
    initError = null;
    try {
      await invoke("init_sciwin_project", { path: pendingProjectPath });
      workspace.openProject(pendingProjectPath, true);
      showInitPrompt = false;
      pendingProjectPath = null;
    } catch (err) {
      initError = String(err);
    } finally {
      initBusy = false;
    }
  }

  function declineInit() {
    if (pendingProjectPath) {
      workspace.openProject(pendingProjectPath, false);
    }
    showInitPrompt = false;
    pendingProjectPath = null;
  }

  const sectionLabels = {
    workflows: "Workflows",
    filesystem: "Filesystem",
    sourcecontrol: "Source Control",
  } as const;

  let workflowEntries = $state<FsEntry[] | null>(null);
  let filesystemEntries = $state<FsEntry[] | null>(null);
  let loadingWorkflows = $state(false);
  let loadingFilesystem = $state(false);
  let lastPath: string | null = null;

  function reloadCurrentView() {
    if (workspace.sidebarView === "workflows") workflowEntries = null;
    if (workspace.sidebarView === "filesystem") filesystemEntries = null;
  }

  $effect(() => {
    const path = workspace.projectPath;
    const view = workspace.sidebarView;

    if (path !== lastPath) {
      lastPath = path;
      workflowEntries = null;
      filesystemEntries = null;
    }

    if (!path) return;

    if (view === "workflows" && workflowEntries === null && !loadingWorkflows) {
      loadingWorkflows = true;
      invoke<FsEntry[]>("get_cwl_files", { root: path })
        .then((r) => (workflowEntries = r))
        .catch(() => (workflowEntries = []))
        .finally(() => (loadingWorkflows = false));
    } else if (view === "filesystem" && filesystemEntries === null && !loadingFilesystem) {
      loadingFilesystem = true;
      invoke<FsEntry[]>("list_dir", { path })
        .then((r) => (filesystemEntries = r))
        .catch(() => (filesystemEntries = []))
        .finally(() => (loadingFilesystem = false));
    }
  });
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

  {#if workspace.projectPath && workspace.sidebarView !== "sourcecontrol"}
    <div class="flex items-center gap-1 px-3 pt-2.5 pb-1">
      <span class="flex-1 font-mono text-[10px] tracking-widest text-text-3 uppercase">{sectionLabels[workspace.sidebarView]}</span>
      <button
        type="button"
        class="shrink-0 rounded p-0.5 text-text-3 hover:bg-border-soft hover:text-text"
        title="Reload"
        onclick={reloadCurrentView}
      >
        <RotateCw size={11} strokeWidth={1.8} />
      </button>
    </div>
  {/if}

  <div class="flex-1 overflow-y-auto py-1">
    {#if workspace.projectPath}
      {#if workspace.sidebarView === "workflows"}
        {#if loadingWorkflows}
          <p class="px-3 font-mono text-[11px] text-text-3">Loading...</p>
        {:else if workflowEntries && workflowEntries.length > 0}
          {#each workflowEntries as entry (entry.path)}
            <FileTreeNode {entry} depth={0} lazy={false} />
          {/each}
        {:else}
          <p class="px-3 font-mono text-[11px] text-text-3">No .cwl files found.</p>
        {/if}
      {:else if workspace.sidebarView === "filesystem"}
        {#if loadingFilesystem}
          <p class="px-3 font-mono text-[11px] text-text-3">Loading...</p>
        {:else if filesystemEntries && filesystemEntries.length > 0}
          {#each filesystemEntries as entry (entry.path)}
            <FileTreeNode {entry} depth={0} lazy={true} />
          {/each}
        {:else}
          <p class="px-3 font-mono text-[11px] text-text-3">Empty folder.</p>
        {/if}
      {:else}
        <p class="px-3 font-mono text-[11px] text-text-3">Source control panel coming soon.</p>
      {/if}
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

<ConfirmDialog
  bind:open={showInitPrompt}
  title="Initialize SciWIn project?"
  message="This folder doesn't have a workflow.toml yet. Initialize it as a SciWIn project (creates workflow.toml, a workflows/ folder, and a git repo if needed)? You can skip this, but some features will be unavailable until it's initialized."
  confirmLabel="Initialize"
  cancelLabel="Skip"
  busy={initBusy}
  error={initError}
  onConfirm={confirmInit}
  onCancel={declineInit}
/>
