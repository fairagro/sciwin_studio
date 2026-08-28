<script lang="ts">
  import { AlertDialog } from "bits-ui";
  import type { PickValue } from "$lib/graph/types";
  import { pickValueLabel } from "$lib/graph/styling";

  interface Props {
    open: boolean;
    port: string;
    busy?: boolean;
    error?: string | null;
    onChoose: (value: PickValue) => void;
    onCancel: () => void;
  }

  let { open = $bindable(false), port, busy = false, error = null, onChoose, onCancel }: Props = $props();

  const CHOICES: PickValue[] = ["first_non_null", "the_only_non_null", "all_non_null"];
</script>

<AlertDialog.Root
  bind:open
  onOpenChange={(next) => {
    if (!next) onCancel();
  }}
>
  <AlertDialog.Portal>
    <AlertDialog.Overlay class="fixed inset-0 z-40 bg-black/60" />
    <AlertDialog.Content
      class="fixed top-1/2 left-1/2 z-50 w-95 -translate-x-1/2 -translate-y-1/2 rounded-xl border border-border bg-bg-surface text-text shadow-2xl"
    >
      <div class="flex items-center border-b border-border-soft px-4 py-3.5">
        <AlertDialog.Title class="font-display flex-1 text-[15px] font-semibold text-text">Resolve multiple sources</AlertDialog.Title>
      </div>
      <div class="px-4 py-4">
        <AlertDialog.Description class="text-[13px] leading-relaxed text-text-2">
          "{port}" already has a source. Choose how to pick a value among the multiple sources feeding it.
        </AlertDialog.Description>
        {#if error}
          <p class="mt-3 font-mono text-[11px] text-fairagro-red-light">{error}</p>
        {/if}
        <div class="mt-3.5 flex flex-col gap-1.5">
          {#each CHOICES as choice (choice)}
            <button
              type="button"
              class="rounded-md border border-border-soft py-2 text-[13px] text-text hover:border-fairagro-mid-500 hover:bg-border-soft disabled:opacity-60"
              onclick={() => onChoose(choice)}
              disabled={busy}
            >
              {pickValueLabel(choice)}
            </button>
          {/each}
        </div>
      </div>
      <div class="flex border-t border-border-soft px-4 py-3.5">
        <AlertDialog.Cancel
          class="flex-1 rounded-md border border-border-soft py-2 text-[13px] text-text-2 hover:bg-border-soft hover:text-text disabled:opacity-60"
          disabled={busy}
        >
          Cancel
        </AlertDialog.Cancel>
      </div>
    </AlertDialog.Content>
  </AlertDialog.Portal>
</AlertDialog.Root>
