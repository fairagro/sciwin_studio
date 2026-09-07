<script lang="ts">
  import { workspace } from "$lib/state/workspace.svelte";
  import { GitBranch } from "@lucide/svelte";
</script>

<footer class="flex h-6.5 shrink-0 items-center justify-between gap-4 bg-fairagro-dark-500 px-3.5 font-mono text-[11px] text-white select-none">
  <span class="truncate">
    {workspace.activeTab?.path ?? workspace.projectPath ?? "No project open"}
  </span>
  <button class="flex items-center gap-1.5 font-mono text-[11px] text-text px-1 py-0.5 hover:bg-fairagro-dark-400" onclick={() => workspace.selectSidebarView("sourcecontrol")} title="Source Control">
    <GitBranch size={12} strokeWidth={1.8} class="shrink-0 text-text-2" />
    <span class="truncate">{workspace.currentBranch}</span>
  </button>
</footer>
