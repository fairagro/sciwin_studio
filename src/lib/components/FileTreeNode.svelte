<script lang="ts">
  import { untrack } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { ChevronRight, File, Folder, FolderOpen } from "@lucide/svelte";
  import { workspace } from "$lib/state/workspace.svelte";
  import FileTreeNode from "./FileTreeNode.svelte";

  export interface FsEntry {
    name: string;
    path: string;
    isDir: boolean;
    children?: FsEntry[];
  }

  interface Props {
    entry: FsEntry;
    depth: number;
    lazy: boolean;
  }

  let { entry, depth, lazy }: Props = $props();

  let expanded = $state(false);
  let children = $state<FsEntry[] | null>(untrack(() => entry.children) ?? null);
  let loading = $state(false);

  async function toggle() {
    if (!entry.isDir) {
      workspace.openTab(entry.path, entry.name);
      return;
    }
    expanded = !expanded;
    if (expanded && lazy && children === null) {
      loading = true;
      children = await invoke<FsEntry[]>("list_dir", { path: entry.path }).catch(() => []);
      loading = false;
    }
  }
</script>

<div>
  <button
    type="button"
    class="flex w-full items-center gap-1.5 rounded py-0.75 pr-1 text-left font-mono text-[11.5px] whitespace-nowrap text-text-2 hover:bg-border-soft hover:text-text {!entry.isDir &&
    entry.path === workspace.activePath
      ? 'bg-fairagro-mid-500/14 text-text'
      : ''}"
    style="padding-left: {depth * 14 + 4}px"
    onclick={toggle}
  >
    {#if entry.isDir}
      <ChevronRight size={11} class="shrink-0 transition-transform {expanded ? 'rotate-90' : ''}" />
      {#if expanded}
        <FolderOpen size={13} strokeWidth={1.8} class="shrink-0 text-fairagro-mid-500" />
      {:else}
        <Folder size={13} strokeWidth={1.8} class="shrink-0 text-fairagro-mid-500" />
      {/if}
    {:else}
      <span class="w-2.75 shrink-0"></span>
      <File size={13} strokeWidth={1.8} class="shrink-0 text-text-3" />
    {/if}
    <span class="truncate">{entry.name}</span>
  </button>

  {#if entry.isDir && expanded}
    {#if loading}
      <p class="font-mono text-[10.5px] text-text-3" style="padding-left: {(depth + 1) * 14 + 20}px">Loading...</p>
    {:else if children}
      {#each children as child (child.path)}
        <FileTreeNode entry={child} depth={depth + 1} {lazy} />
      {/each}
    {/if}
  {/if}
</div>
