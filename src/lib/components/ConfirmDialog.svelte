<script lang="ts">
  import { AlertDialog } from "bits-ui";

  interface Props {
    open: boolean;
    title: string;
    message: string;
    confirmLabel?: string;
    cancelLabel?: string;
    busy?: boolean;
    error?: string | null;
    onConfirm: () => void;
    onCancel: () => void;
  }

  let {
    open = $bindable(false),
    title,
    message,
    confirmLabel = "Confirm",
    cancelLabel = "Cancel",
    busy = false,
    error = null,
    onConfirm,
    onCancel,
  }: Props = $props();
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
        <AlertDialog.Title class="font-display flex-1 text-[15px] font-semibold text-text">{title}</AlertDialog.Title>
      </div>
      <div class="px-4 py-4">
        <AlertDialog.Description class="text-[13px] leading-relaxed text-text-2">{message}</AlertDialog.Description>
        {#if error}
          <p class="mt-3 font-mono text-[11px] text-fairagro-red-light">{error}</p>
        {/if}
      </div>
      <div class="flex gap-2.5 border-t border-border-soft px-4 py-3.5">
        <AlertDialog.Cancel
          class="flex-1 rounded-md border border-border-soft py-2 text-[13px] text-text-2 hover:bg-border-soft hover:text-text disabled:opacity-60"
          disabled={busy}
        >
          {cancelLabel}
        </AlertDialog.Cancel>
        <button
          type="button"
          class="flex-1 rounded-md bg-fairagro-mid-500 py-2 text-[13px] font-semibold text-bg hover:bg-fairagro-mid-400 disabled:opacity-60"
          onclick={onConfirm}
          disabled={busy}
        >
          {busy ? "Working..." : confirmLabel}
        </button>
      </div>
    </AlertDialog.Content>
  </AlertDialog.Portal>
</AlertDialog.Root>
