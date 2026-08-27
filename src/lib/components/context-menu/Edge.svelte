<script lang="ts">
  import { useSvelteFlow } from "@xyflow/svelte";

  let {
    id,
    top,
    left,
    right,
    bottom,
    onclick,
  }: {
    id: string;
    top: number | undefined;
    left: number | undefined;
    right: number | undefined;
    bottom: number | undefined;
    onclick: () => void;
  } = $props();

  const { deleteElements } = useSvelteFlow();

  function deleteEdge() {
    deleteElements({ edges: [{ id }] });
  }
</script>

<div
  style="top: {top}px; left: {left}px; right: {right}px; bottom: {bottom}px;"
  class="absolute z-10 min-w-40 rounded-md border border-border bg-bg-surface p-1 shadow-lg"
  role="menu"
  tabindex="-1"
  {onclick}
  onkeydown={(e) => {
    if (e.key === "Escape") onclick();
  }}
  onpointerdown={(e) => e.stopPropagation()}
>
  <p class="px-2 py-1 font-mono text-[10px] break-all text-text-3">
    {id}
  </p>
  <button
    onclick={deleteEdge}
    class="w-full rounded px-2 py-1.5 text-left font-mono text-xs text-text-2 hover:bg-border-soft hover:text-fairagro-red-light"
  >
    Delete connection
  </button>
</div>
