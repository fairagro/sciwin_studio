<script lang="ts">
    import { GitBranch } from "@lucide/svelte";
    import { invoke } from "@tauri-apps/api/core";
    import { ContextMenu } from "bits-ui";
    import { workspace } from "$lib/state/workspace.svelte";
    import SourceControlFileContextMenu from "./context-menu/SourceControlFile.svelte";

    let switching = $state(false);
    let branchError = $state<string | null>(null);

    async function onSelect(e: Event) {
        const branch = (e.currentTarget as HTMLSelectElement).value;
        if (branch === workspace.currentBranch) return;
        branchError = null;
        switching = true;
        try {
            await workspace.checkoutBranch(branch);
        } catch (err) {
            branchError = String(err);
        } finally {
            switching = false;
        }
    }

    async function onCommit() {
        if (!workspace.projectPath) return;
        if (!commitMessage || commitMessage.trim() === "") {
            commitError = "Commit message cannot be empty.";
            return;
        }
        commitError = null;
        try {
            await invoke("git_stage_all", { path: workspace.projectPath });
            await invoke("git_commit", { path: workspace.projectPath, message: commitMessage });
            commitMessage = "";
            workspace.refreshGitStatus();
        } catch (err) {
            commitError = String(err);
        }
    }

    let commitMessage = $state<string | undefined>(undefined);
    let commitError = $state<string | null>(null);
</script>

<div class="px-3 py-1">
    {#if workspace.currentBranch}
        <div class="flex items-center gap-1.5 font-mono text-xs text-text">
            <GitBranch size={12} strokeWidth={1.8} class="shrink-0 text-text-2" />
            <select
                class="min-w-0 flex-1 truncate rounded border border-border-soft bg-bg-panel px-1 py-0.5 font-mono text-xs text-text disabled:opacity-60"
                value={workspace.currentBranch}
                disabled={switching || workspace.availableBranches.length <= 1}
                onchange={onSelect}
            >
                {#each workspace.availableBranches as branch (branch)}
                    <option value={branch}>{branch}</option>
                {/each}
            </select>
        </div>
        {#if branchError}
            <p class="mt-1.5 font-mono text-xs text-fairagro-red-light">{branchError}</p>
        {/if}
        <textarea
            rows="3"
            placeholder="Commit message for '{workspace.currentBranch}'"
            bind:value={commitMessage}
            onkeydown={(e) => {
                if (e.key === "Enter" && (e.ctrlKey || e.metaKey)) {
                    e.preventDefault();
                    onCommit();
                }
            }}
            class="my-2 w-full resize-none rounded-md border border-border bg-bg-panel px-2.5 py-1.5 font-mono text-xs text-text outline-none focus:border-fairagro-mid-500"
        ></textarea>
        <button type="button" class="w-full rounded-md bg-fairagro-mid-500 px-2.5 py-1.5 font-mono text-xs text-white hover:bg-fairagro-mid-600" onclick={onCommit}>Commit</button>
        {#if commitError}
            <p class="mt-1.5 font-mono text-xs text-fairagro-red-light">{commitError}</p>
        {/if}
        <p class="mt-3 mb-1 flex items-center gap-1.5 font-mono text-xs tracking-widest text-text-3 uppercase">
            Changes
            {#if workspace.uncommittedFiles.length > 0}
                <span class="rounded-full bg-border-soft px-1.5 py-0.5 text-xs text-text-2 normal-case">{workspace.uncommittedFiles.length}</span>
            {/if}
        </p>
        {#if !workspace.gitStatusLoaded}
            <p class="px-0.5 font-mono text-xs text-text-3">Loading...</p>
        {:else if workspace.uncommittedFiles.length > 0}
            <ul>
                {#each workspace.uncommittedFiles as file (file)}
                    <li class="hover:bg-bg-surface cursor-pointer">
                        <ContextMenu.Root>
                            <ContextMenu.Trigger>
                                {#snippet child({ props })}
                                    <div {...props} class="truncate rounded px-0.5 py-1 font-mono text-xs text-text-2" title={file}>
                                        {file}
                                    </div>
                                {/snippet}
                            </ContextMenu.Trigger>
                            <ContextMenu.Portal>
                                <SourceControlFileContextMenu {file} />
                            </ContextMenu.Portal>
                        </ContextMenu.Root>
                    </li>
                {/each}
            </ul>
        {:else}
            <p class="px-0.5 font-mono text-xs text-text-3">No uncommitted changes.</p>
        {/if}
    {:else}
        <p class="font-mono text-xs text-text-3">Not a git repository.</p>
    {/if}
</div>
