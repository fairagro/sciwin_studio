<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { ContextMenu } from "bits-ui";
  import { workspace } from "$lib/state/workspace.svelte";
  import ConfirmDialog from "../ConfirmDialog.svelte";

  let { file }: { file: string } = $props();

  let revertDialogOpen = $state(false);
  let revertBusy = $state(false);
  let revertError = $state<string | null>(null);

  function requestRevert() {
    revertError = null;
    revertDialogOpen = true;
  }

  async function confirmRevert() {
    if (!workspace.projectPath) return;
    revertBusy = true;
    try {
      const absolutePath = await invoke<string>("git_discard_file", { path: workspace.projectPath, file });
      const stillExists = await invoke<boolean>("path_exists", { path: absolutePath });
      if (!stillExists) workspace.closeTabsUnder(absolutePath);
      workspace.notifyFilesystemChanged();
      workspace.refreshGitStatus();
      revertBusy = false;
      revertDialogOpen = false;
    } catch (err) {
      revertBusy = false;
      revertError = String(err);
    }
  }

  function cancelRevert() {
    revertDialogOpen = false;
    revertError = null;
  }
</script>

<ContextMenu.Content class="z-10 min-w-40 rounded-md border border-border bg-bg-surface p-1 shadow-lg">
  <p class="px-2 py-1 font-mono text-[10px] break-all text-text-3">
    {file}
  </p>
  <ContextMenu.Item
    onSelect={requestRevert}
    class="w-full rounded px-2 py-1.5 text-left font-mono text-xs text-text-2 cursor-pointer select-none outline-none hover:bg-border-soft hover:text-fairagro-red-light data-highlighted:bg-border-soft data-highlighted:text-fairagro-red-light"
  >
    Revert changes
  </ContextMenu.Item>
</ContextMenu.Content>

<ConfirmDialog
  bind:open={revertDialogOpen}
  title="Revert changes"
  message={`Discard uncommitted changes to "${file}"? This can't be undone.`}
  confirmLabel="Revert"
  busy={revertBusy}
  error={revertError}
  onConfirm={confirmRevert}
  onCancel={cancelRevert}
/>
