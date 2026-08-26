<script lang="ts">
  import { workspace } from "$lib/state/workspace.svelte";
</script>

<footer
  class="flex h-[26px] shrink-0 items-center justify-between gap-4 bg-fairagro-dark-500 px-3.5 font-mono text-[11px] text-white select-none"
>
  <span class="truncate">
    {workspace.activeTab?.path ?? workspace.projectPath ?? "No project open"}
  </span>
  <span class="shrink-0">SciWIn-Studio</span>
</footer>
