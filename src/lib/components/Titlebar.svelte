<script lang="ts">
  import { getCurrentWindow } from "@tauri-apps/api/window";

  const appWindow = getCurrentWindow();
</script>

<div class="flex h-10 shrink-0 items-center gap-2.5 border-b border-border bg-bg-panel px-3" data-tauri-drag-region>
  <img src="/icon.png" alt="" width="22" height="22" class="shrink-0" />
  <div class="flex flex-col leading-tight">
    <span class="font-mono text-[8px] font-semibold tracking-[0.12em] text-fairagro-dark-500">FAIRAGRO</span>
    <span class="font-display text-sm font-semibold text-text">SciWIn-Studio</span>
  </div>
  <div class="flex-1" data-tauri-drag-region></div>
  <div class="flex gap-1">
    <button
      type="button"
      class="flex h-6.5 w-6.5 items-center justify-center rounded text-text-2 hover:bg-border-soft hover:text-text"
      title="Minimize"
      onclick={() => appWindow.minimize()}
    >
      <svg width="12" height="12" viewBox="0 0 12 12"
        ><line x1="2" y1="9" x2="10" y2="9" stroke="currentColor" stroke-width="1.4" /></svg
      >
    </button>
    <button
      type="button"
      class="flex h-6.5 w-6.5 items-center justify-center rounded text-text-2 hover:bg-border-soft hover:text-text"
      title="Maximize"
      onclick={() => appWindow.toggleMaximize()}
    >
      <svg width="12" height="12" viewBox="0 0 12 12"
        ><rect x="2.2" y="2.2" width="7.6" height="7.6" fill="none" stroke="currentColor" stroke-width="1.3" /></svg
      >
    </button>
    <button
      type="button"
      class="flex h-6.5 w-6.5 items-center justify-center rounded text-text-2 hover:bg-fairagro-red hover:text-white"
      title="Close"
      onclick={() => appWindow.close()}
    >
      <svg width="12" height="12" viewBox="0 0 12 12"
        ><line x1="2" y1="2" x2="10" y2="10" stroke="currentColor" stroke-width="1.4" /><line
          x1="10"
          y1="2"
          x2="2"
          y2="10"
          stroke="currentColor"
          stroke-width="1.4"
        /></svg
      >
    </button>
  </div>
</div>
