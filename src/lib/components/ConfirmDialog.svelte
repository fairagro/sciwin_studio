<script lang="ts">
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
    open,
    title,
    message,
    confirmLabel = "Confirm",
    cancelLabel = "Cancel",
    busy = false,
    error = null,
    onConfirm,
    onCancel,
  }: Props = $props();

  let dialogEl: HTMLDialogElement;

  $effect(() => {
    if (!dialogEl) return;
    if (open && !dialogEl.open) dialogEl.showModal();
    if (!open && dialogEl.open) dialogEl.close();
  });
</script>

<dialog
  bind:this={dialogEl}
  class="m-auto w-95 rounded-xl border border-border bg-bg-surface p-0 text-text shadow-2xl backdrop:bg-black/60"
  oncancel={onCancel}
>
  <div class="flex items-center border-b border-border-soft px-4 py-3.5">
    <span class="font-display flex-1 text-[15px] font-semibold text-text">{title}</span>
  </div>
  <div class="px-4 py-4">
    <p class="text-[13px] leading-relaxed text-text-2">{message}</p>
    {#if error}
      <p class="mt-3 font-mono text-[11px] text-fairagro-red-light">{error}</p>
    {/if}
  </div>
  <div class="flex gap-2.5 border-t border-border-soft px-4 py-3.5">
    <button
      type="button"
      class="flex-1 rounded-md border border-border-soft py-2 text-[13px] text-text-2 hover:bg-border-soft hover:text-text disabled:opacity-60"
      onclick={onCancel}
      disabled={busy}
    >
      {cancelLabel}
    </button>
    <button
      type="button"
      class="flex-1 rounded-md bg-fairagro-mid-500 py-2 text-[13px] font-semibold text-bg hover:bg-fairagro-mid-400 disabled:opacity-60"
      onclick={onConfirm}
      disabled={busy}
    >
      {busy ? "Working..." : confirmLabel}
    </button>
  </div>
</dialog>
