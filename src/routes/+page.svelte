<script lang="ts">
  import Titlebar from "$lib/components/Titlebar.svelte";
  import IconRail from "$lib/components/IconRail.svelte";
  import Sidebar from "$lib/components/Sidebar.svelte";
  import TabBar from "$lib/components/TabBar.svelte";
  import Editor from "$lib/components/Editor.svelte";
  import GraphView from "$lib/components/GraphView.svelte";
  import ViewModeToggle from "$lib/components/ViewModeToggle.svelte";
  import Terminal from "$lib/components/Terminal.svelte";
  import StatusBar from "$lib/components/StatusBar.svelte";
  import ResizeHandles from "$lib/components/ResizeHandles.svelte";
  import ResizeHandle from "$lib/components/ResizeHandle.svelte";
  import { workspace } from "$lib/state/workspace.svelte";
  import { restoreSession, scheduleSave } from "$lib/state/session";
  import { onMount } from "svelte";

  onMount(() => {
    restoreSession();
  });

  $effect(() => {
    scheduleSave();
  });
</script>

<ResizeHandles />
<div class="grid h-screen w-screen grid-rows-[auto_1fr_auto] overflow-hidden bg-bg text-text">
  <Titlebar />
  <div class="flex min-h-0 flex-1">
    <IconRail />
    {#if !workspace.sidebarCollapsed}
      <Sidebar />
      <ResizeHandle orientation="vertical" onResize={(d) => workspace.resizeSidebar(d)} />
    {/if}
    <div class="flex min-w-0 flex-1 flex-col">
      <TabBar />
      {#if workspace.activeTab?.name.toLowerCase().endsWith(".cwl")}
        <div class="flex items-center border-b border-border bg-bg-panel px-2.5 py-1.5">
          <ViewModeToggle tab={workspace.activeTab} />
        </div>
      {/if}
      <main class="relative flex-1 overflow-hidden">
        <div
          class="h-full w-full {workspace.activeTab && workspace.activeTab.viewMode === 'code' ? '' : 'hidden'}"
          style="background-image: radial-gradient(circle, var(--border-soft) 1px, transparent 1px); background-size: 22px 22px;"
        >
          <Editor />
        </div>
        <div class="h-full w-full {workspace.activeTab?.viewMode === 'graph' ? '' : 'hidden'}">
          <GraphView />
        </div>
        {#if !workspace.activeTab}
          <div class="flex h-full items-center justify-center font-mono text-sm text-text-3">
            {workspace.projectPath ? "Select a file to begin editing" : "Open a folder to get started"}
          </div>
        {/if}
      </main>
      {#if workspace.terminalOpen}
        <ResizeHandle orientation="horizontal" onResize={(d) => workspace.resizeTerminal(-d)} />
      {/if}
      <Terminal />
    </div>
  </div>
  <StatusBar />
</div>
