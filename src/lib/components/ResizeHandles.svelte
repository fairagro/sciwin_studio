<script lang="ts">
  import { getCurrentWindow } from "@tauri-apps/api/window";

  const appWindow = getCurrentWindow();

  type Direction = "North" | "South" | "East" | "West" | "NorthEast" | "NorthWest" | "SouthEast" | "SouthWest";

  function resize(e: MouseEvent, direction: Direction) {
    if (e.button !== 0) return;
    e.preventDefault();
    appWindow.startResizeDragging(direction);
  }
</script>

<div class="pointer-events-none fixed inset-0 z-50">
  <div
    class="pointer-events-auto fixed top-0 right-1.5 left-1.5 h-1.5 cursor-n-resize"
    aria-hidden="true"
    onmousedown={(e) => resize(e, "North")}
  ></div>
  <div
    class="pointer-events-auto fixed right-1.5 bottom-0 left-1.5 h-1.5 cursor-s-resize"
    aria-hidden="true"
    onmousedown={(e) => resize(e, "South")}
  ></div>
  <div
    class="pointer-events-auto fixed top-1.5 bottom-1.5 left-0 w-1.5 cursor-w-resize"
    aria-hidden="true"
    onmousedown={(e) => resize(e, "West")}
  ></div>
  <div
    class="pointer-events-auto fixed top-1.5 bottom-1.5 right-0 w-1.5 cursor-e-resize"
    aria-hidden="true"
    onmousedown={(e) => resize(e, "East")}
  ></div>
  <div
    class="pointer-events-auto fixed top-0 left-0 h-1.5 w-1.5 cursor-nw-resize"
    aria-hidden="true"
    onmousedown={(e) => resize(e, "NorthWest")}
  ></div>
  <div
    class="pointer-events-auto fixed top-0 right-0 h-1.5 w-1.5 cursor-ne-resize"
    aria-hidden="true"
    onmousedown={(e) => resize(e, "NorthEast")}
  ></div>
  <div
    class="pointer-events-auto fixed bottom-0 left-0 h-1.5 w-1.5 cursor-sw-resize"
    aria-hidden="true"
    onmousedown={(e) => resize(e, "SouthWest")}
  ></div>
  <div
    class="pointer-events-auto fixed right-0 bottom-0 h-1.5 w-1.5 cursor-se-resize"
    aria-hidden="true"
    onmousedown={(e) => resize(e, "SouthEast")}
  ></div>
</div>
