<script lang="ts">
  import Titlebar from "$lib/components/Titlebar.svelte";
  import IconRail from "$lib/components/IconRail.svelte";
  import Sidebar from "$lib/components/Sidebar.svelte";
  import TabBar from "$lib/components/TabBar.svelte";
  import Terminal from "$lib/components/Terminal.svelte";
  import StatusBar from "$lib/components/StatusBar.svelte";
  import ResizeHandles from "$lib/components/ResizeHandles.svelte";
  import { workspace } from "$lib/state/workspace.svelte";
</script>

<ResizeHandles />
<div class="grid h-screen w-screen grid-rows-[auto_1fr_auto] overflow-hidden bg-bg text-text">
  <Titlebar />
  <div class="flex min-h-0 flex-1">
    <IconRail />
    {#if !workspace.sidebarCollapsed}
      <Sidebar />
    {/if}
    <div class="flex min-w-0 flex-1 flex-col">
      <TabBar />
      <main
        class="flex-1 overflow-auto"
        style="background-image: radial-gradient(circle, var(--border-soft) 1px, transparent 1px); background-size: 22px 22px;"
      >
        {#if workspace.activeTab}
          <div class="p-4 font-mono text-sm text-text-2">Editor for {workspace.activeTab.path} goes here.</div>
        {:else}
          <div class="flex h-full items-center justify-center font-mono text-sm text-text-3">
            {workspace.projectPath ? "Select a file to begin editing" : "Open a folder to get started"}
          </div>
        {/if}
      </main>
      <Terminal />
    </div>
  </div>
  <StatusBar />
</div>
