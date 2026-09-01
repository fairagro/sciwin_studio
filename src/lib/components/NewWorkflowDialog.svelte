<script lang="ts">
  import { AlertDialog } from "bits-ui";

  interface Props {
    open: boolean;
    busy?: boolean;
    error?: string | null;
    onCreate: (name: string) => void;
    onCancel: () => void;
  }

  let { open = $bindable(false), busy = false, error = null, onCreate, onCancel }: Props = $props();

  let name = $state("");
  let inputEl: HTMLInputElement | undefined = $state();

  $effect(() => {
    if (open) {
      name = "";
      requestAnimationFrame(() => inputEl?.focus());
    }
  });

  function submit() {
    if (busy || !name.trim()) return;
    onCreate(name.trim());
  }
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
        <AlertDialog.Title class="font-display flex-1 text-[15px] font-semibold text-text">New Workflow</AlertDialog.Title>
      </div>
      <div class="px-4 py-4">
        <AlertDialog.Description class="text-[13px] leading-relaxed text-text-2">Creates an empty workflow file in the project's workflows folder.</AlertDialog.Description>
        <input
          bind:this={inputEl}
          bind:value={name}
          type="text"
          placeholder="workflow name"
          class="mt-3.5 w-full rounded-md border border-border-soft bg-bg px-2.5 py-1.5 font-mono text-[13px] text-text outline-none focus:border-fairagro-mid-500"
          onkeydown={(e) => e.key === "Enter" && submit()}
          disabled={busy}
        />
        {#if error}
          <p class="mt-2 font-mono text-[11px] text-fairagro-red-light">{error}</p>
        {/if}
      </div>
      <div class="flex gap-2.5 border-t border-border-soft px-4 py-3.5">
        <AlertDialog.Cancel
          class="flex-1 rounded-md border border-border-soft py-2 text-[13px] text-text-2 hover:bg-border-soft hover:text-text disabled:opacity-60"
          disabled={busy}
        >
          Cancel
        </AlertDialog.Cancel>
        <button
          type="button"
          class="flex-1 rounded-md bg-fairagro-mid-500 py-2 text-[13px] font-semibold text-bg hover:bg-fairagro-mid-400 disabled:opacity-60"
          onclick={submit}
          disabled={busy || !name.trim()}
        >
          {busy ? "Creating..." : "Create"}
        </button>
      </div>
    </AlertDialog.Content>
  </AlertDialog.Portal>
</AlertDialog.Root>
