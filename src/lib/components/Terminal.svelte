<script lang="ts">
  import { workspace } from "$lib/state/workspace.svelte";
</script>

<div class="flex h-44 shrink-0 flex-col border-t border-border bg-bg-panel">
  <div class="flex h-8 shrink-0 items-center gap-2 border-b border-border-soft px-2.5">
    <svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="var(--text-2)" stroke-width="2">
      <path d="M4 5h16v14H4z" />
      <path d="M8 9l3 3-3 3" />
      <path d="M13 15h4" />
    </svg>
    <span class="font-mono text-[11px] text-text">Console</span>
    <div class="flex-1"></div>
    <button
      type="button"
      class="rounded p-0.5 text-text-3 hover:bg-border-soft hover:text-text"
      title="Close panel"
      onclick={() => workspace.toggleTerminal()}
    >
      &times;
    </button>
  </div>
  <div class="flex flex-1 items-center justify-center overflow-hidden bg-bg-well p-3">
    <p class="font-mono text-[11px] text-text-3">Terminal coming soon.</p>
  </div>
</div>
