<script lang="ts">
  import { workspace } from "$lib/state/workspace.svelte";

  function closeTab(e: MouseEvent, path: string) {
    e.stopPropagation();
    workspace.closeTab(path);
  }
</script>

{#if workspace.tabs.length > 0}
  <div class="flex h-9 shrink-0 items-stretch overflow-x-auto border-b border-rule bg-surface-2 select-none">
    {#each workspace.tabs as tab (tab.path)}
      {@const active = tab.path === workspace.activePath}
      <button
        type="button"
        class="group flex max-w-48 shrink-0 items-center gap-2 border-r border-rule px-3 text-xs text-ink-2 hover:bg-surface {active
          ? 'bg-surface text-ink'
          : ''}"
        onclick={() => (workspace.activePath = tab.path)}
      >
        <span class="truncate font-mono">{tab.name}</span>
        {#if tab.dirty}
          <span class="h-1.5 w-1.5 shrink-0 rounded-full bg-fairagro-dark-500"></span>
        {/if}
        <span
          role="button"
          tabindex="0"
          class="shrink-0 rounded px-1 text-ink-3 opacity-0 group-hover:opacity-100 hover:bg-rule hover:text-ink"
          onclick={(e) => closeTab(e, tab.path)}
          onkeydown={(e) => e.key === "Enter" && closeTab(e as unknown as MouseEvent, tab.path)}
        >
          &times;
        </span>
      </button>
    {/each}
  </div>
{/if}
