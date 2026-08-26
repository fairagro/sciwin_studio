<script lang="ts">
  import Sidebar from "$lib/components/Sidebar.svelte";
  import TabBar from "$lib/components/TabBar.svelte";
  import StatusBar from "$lib/components/StatusBar.svelte";
  import { workspace } from "$lib/state/workspace.svelte";
</script>

<div class="grid h-screen w-screen grid-rows-[1fr_auto] overflow-hidden bg-ground text-ink">
  <div class="flex min-h-0 flex-1">
    <Sidebar />
    <div class="flex min-w-0 flex-1 flex-col">
      <TabBar />
      <main class="flex-1 overflow-auto">
        {#if workspace.activeTab}
          <div class="p-4 text-sm text-ink-2">Editor for {workspace.activeTab.path} goes here.</div>
        {:else}
          <div class="flex h-full items-center justify-center text-sm text-ink-3">
            {workspace.projectPath ? "Select a file to begin editing" : "Open a folder to get started"}
          </div>
        {/if}
      </main>
    </div>
  </div>
  <StatusBar />
</div>
