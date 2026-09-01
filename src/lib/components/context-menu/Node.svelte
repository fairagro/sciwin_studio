<script lang="ts">
  import { useSvelteFlow } from "@xyflow/svelte";
  import { ContextMenu } from "bits-ui";

  let { id }: { id: string } = $props();

  const { deleteElements } = useSvelteFlow();

  function deleteNode() {
    deleteElements({ nodes: [{ id }] });
  }
</script>

<ContextMenu.Content class="z-10 min-w-40 rounded-md border border-border bg-bg-surface p-1 shadow-lg">
  <p class="px-2 py-1 font-mono text-[10px] break-all text-text-3">
    {id}
  </p>
  <ContextMenu.Item
    onSelect={deleteNode}
    class="w-full rounded px-2 py-1.5 text-left font-mono text-xs text-text-2 outline-none hover:bg-border-soft hover:text-fairagro-red-light data-highlighted:bg-border-soft data-highlighted:text-fairagro-red-light"
  >
    Delete Node
  </ContextMenu.Item>
</ContextMenu.Content>
