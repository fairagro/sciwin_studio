<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { ContextMenu } from "bits-ui";
  import { workspace } from "$lib/state/workspace.svelte";
  import ConfirmDialog from "../ConfirmDialog.svelte";
  import NewWorkflowDialog from "../NewWorkflowDialog.svelte";

  let { id, is_dir }: { id: string; is_dir: boolean } = $props();

  let deleteDialogOpen = $state(false);
  let deleteBusy = $state(false);
  let deleteError = $state<string | null>(null);

  let showNewWorkflow = $state(false);
  let newWorkflowBusy = $state(false);
  let newWorkflowError = $state<string | null>(null);

  let showNewTool = $state(false);
  let newToolBusy = $state(false);
  let newToolError = $state<string | null>(null);

  function openNewWorkflow() {
    newWorkflowError = null;
    showNewWorkflow = true;
  }

  async function createWorkflow(name: string) {
    newWorkflowBusy = true;
    try {
      const path = await invoke<string>("create_workflow", { dir: id, name });
      newWorkflowBusy = false;
      showNewWorkflow = false;
      workspace.notifyFilesystemChanged();
      const fileName = path.split(/[\\/]/).pop() ?? path;
      await workspace.openTab(path, fileName);
    } catch (err) {
      newWorkflowBusy = false;
      newWorkflowError = String(err);
    }
  }

  function cancelNewWorkflow() {
    showNewWorkflow = false;
    newWorkflowError = null;
  }

  function openNewTool() {
    newToolError = null;
    showNewTool = true;
  }

  async function createTool(name: string) {
    newToolBusy = true;
    try {
      const path = await invoke<string>("create_command_line_tool", { dir: id, name });
      newToolBusy = false;
      showNewTool = false;
      workspace.notifyFilesystemChanged();
      const fileName = path.split(/[\\/]/).pop() ?? path;
      await workspace.openTab(path, fileName);
    } catch (err) {
      newToolBusy = false;
      newToolError = String(err);
    }
  }

  function cancelNewTool() {
    showNewTool = false;
    newToolError = null;
  }

  function requestDelete() {
    deleteError = null;
    deleteDialogOpen = true;
  }

  async function confirmDelete() {
    deleteBusy = true;
    try {
      await invoke("delete_file", { path: id });
      deleteBusy = false;
      deleteDialogOpen = false;
      workspace.closeTabsUnder(id);
      workspace.notifyFilesystemChanged();
    } catch (err) {
      deleteBusy = false;
      deleteError = String(err);
    }
  }

  function cancelDelete() {
    deleteDialogOpen = false;
    deleteError = null;
  }
</script>

<ContextMenu.Content class="z-10 min-w-40 rounded-md border border-border bg-bg-surface p-1 shadow-lg">
  <p class="px-2 py-1 font-mono text-[10px] break-all text-text-3">
    {id}
  </p>
  {#if is_dir}
    <ContextMenu.Item
      class="w-full rounded px-2 py-1.5 text-left font-mono text-xs text-text-2 cursor-pointer select-none outline-none hover:bg-border-soft hover:text-text data-highlighted:bg-border-soft data-highlighted:text-text"
      onSelect={openNewWorkflow}>Create Workflow</ContextMenu.Item
    >
    <ContextMenu.Item
      class="w-full rounded px-2 py-1.5 text-left font-mono text-xs text-text-2 cursor-pointer select-none outline-none hover:bg-border-soft hover:text-text data-highlighted:bg-border-soft data-highlighted:text-text"
      onSelect={openNewTool}>Create CommandLineTool</ContextMenu.Item
    >
  {/if}
  <ContextMenu.Item
    onSelect={requestDelete}
    class="w-full rounded px-2 py-1.5 text-left font-mono text-xs text-text-2 cursor-pointer select-none outline-none hover:bg-border-soft hover:text-fairagro-red-light data-highlighted:bg-border-soft data-highlighted:text-fairagro-red-light"
  >
    Delete {is_dir ? "directory" : "file"}
  </ContextMenu.Item>
</ContextMenu.Content>

<ConfirmDialog
  bind:open={deleteDialogOpen}
  title="Delete {is_dir ? 'directory' : 'file'}"
  message={`Permanently delete "${id}"? This can't be undone.`}
  confirmLabel="Delete"
  busy={deleteBusy}
  error={deleteError}
  onConfirm={confirmDelete}
  onCancel={cancelDelete}
/>

<NewWorkflowDialog
  bind:open={showNewWorkflow}
  title="New Workflow"
  description="Creates an empty workflow file in this folder."
  placeholder="workflow name"
  busy={newWorkflowBusy}
  error={newWorkflowError}
  onCreate={createWorkflow}
  onCancel={cancelNewWorkflow}
/>

<NewWorkflowDialog
  bind:open={showNewTool}
  title="New CommandLineTool"
  description="Creates an empty command line tool file in this folder."
  placeholder="tool name"
  busy={newToolBusy}
  error={newToolError}
  onCreate={createTool}
  onCancel={cancelNewTool}
/>
