<script lang="ts">
  interface Props {
    orientation: "vertical" | "horizontal";
    onResize: (deltaPx: number) => void;
  }

  let { orientation, onResize }: Props = $props();
  let dragging = $state(false);

  function onPointerDown(e: PointerEvent) {
    if (e.button !== 0) return;
    e.preventDefault();
    dragging = true;
    let last = orientation === "vertical" ? e.clientX : e.clientY;
    document.body.style.cursor = orientation === "vertical" ? "col-resize" : "row-resize";
    document.body.style.userSelect = "none";

    function onMove(ev: PointerEvent) {
      const pos = orientation === "vertical" ? ev.clientX : ev.clientY;
      onResize(pos - last);
      last = pos;
    }
    function onUp() {
      dragging = false;
      document.body.style.cursor = "";
      document.body.style.userSelect = "";
      window.removeEventListener("pointermove", onMove);
      window.removeEventListener("pointerup", onUp);
    }
    window.addEventListener("pointermove", onMove);
    window.addEventListener("pointerup", onUp);
  }
</script>

<div
  aria-hidden="true"
  class="shrink-0 {orientation === 'vertical' ? 'w-1 cursor-col-resize' : 'h-1 cursor-row-resize'} {dragging
    ? 'bg-fairagro-mid-500/50'
    : 'hover:bg-fairagro-mid-500/30'}"
  onpointerdown={onPointerDown}
></div>
