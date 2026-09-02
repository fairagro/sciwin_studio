<script lang="ts">
  import { untrack } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { ContextMenu } from "bits-ui";
  import { ChevronRight, File, Folder, FolderOpen } from "@lucide/svelte";
  import { workspace } from "$lib/state/workspace.svelte";
  import FileTreeNode from "./FileTreeNode.svelte";
  import FileTreeNodeContextMenu from "./context-menu/FileTreeNode.svelte";

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

  // The Workflows view (lazy=false) hands down its whole subtree upfront and
  // refetches it wholesale on every change; sync that in so a file created
  // inside an already-expanded folder shows up without collapsing it.
  // Filesystem nodes (lazy=true) never get an entry.children from their
  // parent, so this never overwrites their own on-demand fetch below.
  $effect(() => {
    if (entry.children) children = entry.children;
  });

  // Filesystem nodes fetch their children on demand instead, so a fsVersion
  // bump (e.g. a file created via this node's own context menu) needs an
  // explicit refetch to pick up the change -- expanded/children state is
  // otherwise untouched by a reload, so a stale cache would go unnoticed.
  let lastFsVersion = untrack(() => workspace.fsVersion);
  $effect(() => {
    const version = workspace.fsVersion;
    if (version === lastFsVersion) return;
    lastFsVersion = version;
    if (lazy && expanded && children !== null) {
      invoke<FsEntry[]>("list_dir", { path: entry.path })
        .then((r) => (children = r))
        .catch(() => {});
    }
  });

  // Only .cwl files make sense as a graph step; dropping anything else onto
  // the canvas would just fail add_workflow_step_node's load_cwl_file call.
  const isDraggableTool = $derived(!entry.isDir && entry.name.toLowerCase().endsWith(".cwl"));

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

  // Custom MIME type keeps this from being picked up by unrelated drop
  // targets (e.g. the Monaco editor's own file-open drop handling).
  function onDragStart(event: DragEvent) {
    if (!isDraggableTool || !event.dataTransfer) return;
    event.dataTransfer.setData("application/x-sciwin-cwl-path", entry.path);
    event.dataTransfer.effectAllowed = "copy";
  }
</script>

<div>
  <ContextMenu.Root>
    <ContextMenu.Trigger>
      {#snippet child({ props })}
        <button
          {...props}
          type="button"
          class="flex w-full items-center gap-1.5 rounded py-0.75 pr-1 text-left font-mono text-[11.5px] whitespace-nowrap text-text-2 hover:bg-border-soft hover:text-text {!entry.isDir && entry.path === workspace.activePath
            ? 'bg-fairagro-mid-500/14 text-text'
            : ''}"
          style="padding-left: {depth * 14 + 4}px"
          draggable={isDraggableTool}
          ondragstart={onDragStart}
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
      {/snippet}
    </ContextMenu.Trigger>
    <ContextMenu.Portal>
      <FileTreeNodeContextMenu id={entry.path} is_dir={entry.isDir} />
    </ContextMenu.Portal>
  </ContextMenu.Root>

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
