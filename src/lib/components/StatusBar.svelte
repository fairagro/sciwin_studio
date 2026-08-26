<script lang="ts">
  import { workspace } from "$lib/state/workspace.svelte";
</script>

<footer
  class="flex h-6 shrink-0 items-center justify-between gap-4 bg-fairagro-dark-500 px-3 text-[11px] text-white select-none"
>
  <span class="truncate font-mono">
    {workspace.activeTab?.path ?? workspace.projectPath ?? "No project open"}
  </span>
  <span class="shrink-0">SciWIn-Studio</span>
</footer>
