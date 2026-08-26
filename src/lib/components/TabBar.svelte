<script lang="ts">
  import { workspace } from "$lib/state/workspace.svelte";

  function closeTab(e: MouseEvent, path: string) {
    e.stopPropagation();
    workspace.closeTab(path);
  }
</script>

{#if workspace.tabs.length > 0}
  <div class="flex h-10.5 shrink-0 items-center gap-2 overflow-x-auto border-b border-border bg-bg-panel px-2.5">
    {#each workspace.tabs as tab (tab.path)}
      {@const active = tab.path === workspace.activePath}
      <button
        type="button"
        class="group flex max-w-48 shrink-0 items-center gap-2 rounded-t-md border-t-2 px-2.5 py-1.5 font-mono text-xs {active
          ? 'border-fairagro-mid-500 bg-bg-surface text-text'
          : 'border-transparent text-text-2 hover:bg-bg-surface/60'}"
        onclick={() => (workspace.activePath = tab.path)}
      >
        <span class="truncate">{tab.name}</span>
        {#if tab.dirty}
          <span class="h-1.5 w-1.5 shrink-0 rounded-full bg-fairagro-mid-500"></span>
        {/if}
        <span
          role="button"
          tabindex="0"
          class="shrink-0 rounded px-1 text-text-3 opacity-0 group-hover:opacity-100 hover:bg-border-soft hover:text-text"
          onclick={(e) => closeTab(e, tab.path)}
          onkeydown={(e) => e.key === "Enter" && closeTab(e as unknown as MouseEvent, tab.path)}
        >
          &times;
        </span>
      </button>
    {/each}
  </div>
{/if}
