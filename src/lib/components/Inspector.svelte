<script lang="ts">
  import { X, Plus, ChevronDown } from "@lucide/svelte";
  import { Switch } from "bits-ui";
  import { workspace } from "$lib/state/workspace.svelte";
  import { nodeHeaderClass, pickValueLabel, isArrayLike } from "$lib/graph/styling";
  import type { FlowPort, LinkMerge, MutationError, NodeKind, PickValue, ScatterMethod } from "$lib/graph/types";
  import { mutationErrorMessage } from "$lib/graph/types";
  import {
    renameWorkflowStep,
    setStepWhen,
    setStepScatterMethod,
    setStepScattered,
    setStepPickValue,
    setStepInputValueFrom,
    setStepInputLinkMerge,
    addStepInputSlot,
    setOutputPickValue,
    setOutputLinkMerge,
  } from "$lib/graph/mutation";

  const data = $derived(workspace.selectedNodeData);
  const isStep = $derived(data?.ref.kind === "step");
  const isOutput = $derived(data?.ref.kind === "output");

  const KIND_LABEL: Record<NodeKind, string> = {
    step: "step",
    input: "workflow input",
    output: "workflow output",
  };

  const SCATTER_METHODS: { value: ScatterMethod; label: string }[] = [
    { value: "dotproduct", label: "dotproduct" },
    { value: "nested_crossproduct", label: "nested crossproduct" },
    { value: "flat_crossproduct", label: "flat crossproduct" },
  ];
  const PICK_VALUES: PickValue[] = ["first_non_null", "the_only_non_null", "all_non_null"];
  const LINK_MERGES: LinkMerge[] = ["merge_nested", "merge_flattened"];
  const LINK_MERGE_LABEL: Record<LinkMerge, string> = {
    merge_nested: "merge nested",
    merge_flattened: "merge flattened",
  };

  // pickValue picks a value out of >1 candidate sources, so it's only
  // offerable once a port actually has more than one -- linkMerge additionally
  // only makes sense once the sink itself accepts an array to merge into.
  function showPickValue(port: FlowPort): boolean {
    return port.sourceCount > 1 || port.pickValue !== null;
  }
  function showLinkMerge(port: FlowPort): boolean {
    return (port.sourceCount > 1 && isArrayLike(port.dataType)) || port.linkMerge !== null;
  }

  let mutationError = $state<MutationError | null>(null);
  let mutationErrorTimer: ReturnType<typeof setTimeout> | undefined;
  $effect(() => {
    clearTimeout(mutationErrorTimer);
    if (mutationError) mutationErrorTimer = setTimeout(() => (mutationError = null), 5000);
  });

  function guardArgs() {
    const path = workspace.activeTab?.path;
    const revision = workspace.graphRevision;
    if (!path || revision === null || revision === undefined) return null;
    return { path, revision, dirty: workspace.activeTab?.dirty ?? false };
  }

  // Text drafts reset only when the selection itself changes, not on every
  // reload -- otherwise a keystroke would race a workflow-changed reload.
  let stepIdDraft = $state("");
  let whenDraft = $state("");
  let nodeKey: string | null = null;
  $effect(() => {
    const key = data ? `${data.ref.kind}/${data.ref.id}` : null;
    if (key === nodeKey) return;
    nodeKey = key;
    stepIdDraft = data?.ref.id ?? "";
    whenDraft = data?.when ?? "";
  });

  async function commitStepId() {
    if (!data || !isStep) return;
    const args = guardArgs();
    const newId = stepIdDraft.trim();
    if (!args || !newId || newId === data.ref.id) {
      stepIdDraft = data.ref.id;
      return;
    }
    const oldId = data.ref.id;
    // Flip the selection pointer before the request even goes out, so a
    // workflow-changed reload racing this call's own response still finds
    // the renamed node under its new id.
    workspace.selectedNodeId = `step/${newId}`;
    try {
      await renameWorkflowStep({ ...args, stepId: oldId, newId });
    } catch (e) {
      workspace.selectedNodeId = `step/${oldId}`;
      stepIdDraft = oldId;
      mutationError = e as MutationError;
    }
  }

  async function toggleWhen(checked: boolean) {
    const args = guardArgs();
    if (!args || !data) return;
    try {
      await setStepWhen({ ...args, stepId: data.ref.id, expression: checked ? "$(true)" : null });
    } catch (e) {
      mutationError = e as MutationError;
    }
  }

  async function commitWhen() {
    const args = guardArgs();
    if (!args || !data || data.when === null) return;
    const trimmed = whenDraft.trim();
    if (trimmed === "") {
      await toggleWhen(false);
      return;
    }
    if (trimmed === data.when) return;
    try {
      await setStepWhen({ ...args, stepId: data.ref.id, expression: trimmed });
    } catch (e) {
      whenDraft = data.when;
      mutationError = e as MutationError;
    }
  }

  async function toggleScattered(port: string, scattered: boolean) {
    const args = guardArgs();
    if (!args || !data) return;
    try {
      await setStepScattered({ ...args, stepId: data.ref.id, port, scattered });
    } catch (e) {
      mutationError = e as MutationError;
    }
  }

  async function commitScatterMethod(value: string) {
    const args = guardArgs();
    if (!args || !data) return;
    try {
      await setStepScatterMethod({ ...args, stepId: data.ref.id, method: value as ScatterMethod });
    } catch (e) {
      mutationError = e as MutationError;
    }
  }

  async function commitStepPickValue(port: string, value: string) {
    const args = guardArgs();
    if (!args || !data) return;
    const method = value === "" ? null : (value as PickValue);
    try {
      await setStepPickValue({ ...args, stepId: data.ref.id, port, method });
    } catch (e) {
      mutationError = e as MutationError;
    }
  }

  async function commitStepLinkMerge(port: string, value: string) {
    const args = guardArgs();
    if (!args || !data) return;
    const method = value === "" ? null : (value as LinkMerge);
    try {
      await setStepInputLinkMerge({ ...args, stepId: data.ref.id, port, method });
    } catch (e) {
      mutationError = e as MutationError;
    }
  }

  async function commitValueFrom(port: string, previous: string, value: string) {
    const trimmed = value.trim();
    if (trimmed === previous) return;
    const args = guardArgs();
    if (!args || !data) return;
    try {
      await setStepInputValueFrom({ ...args, stepId: data.ref.id, port, valueFrom: trimmed === "" ? null : trimmed });
    } catch (e) {
      mutationError = e as MutationError;
    }
  }

  async function commitOutputPickValue(value: string) {
    const args = guardArgs();
    if (!args || !data) return;
    const method = value === "" ? null : (value as PickValue);
    try {
      await setOutputPickValue({ ...args, outputId: data.ref.id, method });
    } catch (e) {
      mutationError = e as MutationError;
    }
  }

  async function commitOutputLinkMerge(value: string) {
    const args = guardArgs();
    if (!args || !data) return;
    const method = value === "" ? null : (value as LinkMerge);
    try {
      await setOutputLinkMerge({ ...args, outputId: data.ref.id, method });
    } catch (e) {
      mutationError = e as MutationError;
    }
  }

  let slotName = $state("");
  let addingSlot = $state(false);

  async function submitAddSlot() {
    const args = guardArgs();
    const name = slotName.trim();
    if (!args || !data || !name) return;
    addingSlot = true;
    try {
      await addStepInputSlot({ ...args, stepId: data.ref.id, port: name });
      slotName = "";
    } catch (e) {
      mutationError = e as MutationError;
    } finally {
      addingSlot = false;
    }
  }

  let inputsOpen = $state(true);
  let outputsOpen = $state(true);

  const fieldClass =
    "w-full rounded-md border border-border-soft bg-bg-well px-2.5 py-2 font-mono text-xs text-text outline-none focus:border-fairagro-mid-500";
  const labelClass = "mb-2 font-mono text-[10px] tracking-widest text-text-3 uppercase";
  const microSelectClass =
    "rounded border border-border-soft bg-bg-surface px-2 py-1 font-mono text-[11px] text-text outline-none";
</script>

{#snippet wiringFields(port: FlowPort, onPickValue: (v: string) => void, onLinkMerge: (v: string) => void)}
  {#if showPickValue(port)}
    <div class="mb-1.5 flex flex-col gap-1">
      <span class="font-mono text-[9px] tracking-widest text-text-3 uppercase">pickValue</span>
      <select class={microSelectClass} value={port.pickValue ?? ""} onchange={(e) => onPickValue(e.currentTarget.value)}>
        <option value="">none</option>
        {#each PICK_VALUES as v (v)}
          <option value={v}>{pickValueLabel(v)}</option>
        {/each}
      </select>
    </div>
  {/if}
  {#if showLinkMerge(port)}
    <div class="flex flex-col gap-1">
      <span class="font-mono text-[9px] tracking-widest text-text-3 uppercase">linkMerge</span>
      <select class={microSelectClass} value={port.linkMerge ?? ""} onchange={(e) => onLinkMerge(e.currentTarget.value)}>
        <option value="">default (merge_nested)</option>
        {#each LINK_MERGES as v (v)}
          <option value={v}>{LINK_MERGE_LABEL[v]}</option>
        {/each}
      </select>
    </div>
  {/if}
{/snippet}

<aside
  class="flex shrink-0 flex-col overflow-y-auto border-l border-border bg-bg-panel select-none"
  style="width: {workspace.inspectorWidth}px"
>
  <div class="flex items-start justify-between gap-2 border-b border-border-soft px-3.5 py-3">
    <div class="min-w-0 flex-1">
      <span class="font-mono text-[10px] tracking-widest text-text-3 uppercase">Inspector</span>
      {#if data}
        <div class="mt-2 flex items-center gap-2">
          <span class="h-2 w-2 shrink-0 rounded-sm {nodeHeaderClass(data.ref.kind)}"></span>
          <span class="truncate font-mono text-[13px] font-semibold text-text" title={data.label}>{data.label}</span>
        </div>
        <div class="mt-1 font-mono text-[10.5px] text-text-3">{KIND_LABEL[data.ref.kind]}</div>
      {/if}
    </div>
    <button
      type="button"
      class="shrink-0 rounded p-0.5 text-text-3 hover:bg-fairagro-red-light/20 hover:text-fairagro-red-light"
      title="Close inspector"
      onclick={() => workspace.closeInspector()}
    >
      <X size={13} strokeWidth={1.8} />
    </button>
  </div>

  {#if data}
    <div class="border-b border-border-soft px-3.5 py-3.5">
      <div class={labelClass}>General</div>
      <div class="flex flex-col gap-2">
        {#if isStep}
          <input
            class={fieldClass}
            bind:value={stepIdDraft}
            onblur={commitStepId}
            onkeydown={(e) => e.key === "Enter" && (e.currentTarget as HTMLInputElement).blur()}
            spellcheck="false"
          />
        {:else}
          <div class={fieldClass}>{data.ref.id}</div>
        {/if}
        {#if data.run}
          <div class="rounded-md border border-border-soft bg-bg-well px-2.5 py-2 font-mono text-[11px] text-text-2">
            {data.run.kind === "file" ? data.run.path : "inline run"}
          </div>
        {/if}
      </div>
    </div>

    {#if isOutput}
      {@const port = data.inputs[0]}
      {#if port && (showPickValue(port) || showLinkMerge(port))}
        <div class="border-b border-border-soft px-3.5 py-3.5">
          <div class={labelClass}>Wiring</div>
          {@render wiringFields(port, commitOutputPickValue, commitOutputLinkMerge)}
        </div>
      {/if}
    {/if}

    {#if isStep}
      <div class="border-b border-border-soft px-3.5 py-3.5">
        <div class="mb-2.5 flex items-center justify-between">
          <span class={labelClass + " mb-0"}>Conditional &middot; when</span>
          <Switch.Root
            checked={data.when !== null}
            onCheckedChange={toggleWhen}
            class="relative h-4.5 w-8 shrink-0 rounded-full bg-border transition-colors data-[state=checked]:bg-fairagro-mid-500"
          >
            <Switch.Thumb
              class="block h-3.5 w-3.5 translate-x-0.5 rounded-full bg-white transition-transform data-[state=checked]:translate-x-4"
            />
          </Switch.Root>
        </div>
        {#if data.when !== null}
          <input
            class={fieldClass}
            bind:value={whenDraft}
            onblur={commitWhen}
            onkeydown={(e) => e.key === "Enter" && (e.currentTarget as HTMLInputElement).blur()}
            placeholder="$(inputs.x != null)"
            spellcheck="false"
          />
          <p class="mt-1.5 text-[10.5px] leading-relaxed text-text-3">Step only runs when this expression evaluates truthy.</p>
        {/if}

        <div class="mt-3 flex items-center gap-1.5">
          <input
            class="min-w-0 flex-1 rounded-md border border-border-soft bg-bg-well px-2 py-1.5 font-mono text-[11px] text-text outline-none focus:border-fairagro-mid-500"
            bind:value={slotName}
            placeholder="slot name"
            spellcheck="false"
          />
          <button
            type="button"
            class="shrink-0 rounded-md border border-border-soft p-1.5 text-text-2 hover:border-fairagro-mid-500 hover:text-fairagro-mid-500 disabled:opacity-50"
            title="Add a slot not declared by the tool -- e.g. a gate for when"
            disabled={!slotName.trim() || addingSlot}
            onclick={submitAddSlot}
          >
            <Plus size={12} strokeWidth={2} />
          </button>
        </div>
      </div>

      <div class="border-b border-border-soft px-3.5 py-3.5">
        <div class={labelClass}>Scatter</div>
        <div class="mb-2.5 flex flex-wrap gap-1.5">
          {#each data.inputs as port (port.id)}
            {@const on = data.scatter.includes(port.id)}
            <button
              type="button"
              class="rounded-md border px-2 py-1 font-mono text-[11px] {on
                ? 'border-green-hdr bg-fairagro-mid-500/16 text-fairagro-mid-500'
                : 'border-border-soft text-text-3 hover:border-fairagro-mid-500/50 hover:text-text-2'}"
              onclick={() => toggleScattered(port.id, !on)}
            >
              {port.id}
            </button>
          {/each}
          {#if data.inputs.length === 0}
            <span class="font-mono text-[11px] text-text-3">no inputs on this step</span>
          {/if}
        </div>
        {#if data.scatter.length > 0}
          <div class={labelClass + " mb-1.5"}>Scatter method</div>
          <select
            class={fieldClass}
            value={data.scatterMethod ?? "dotproduct"}
            onchange={(e) => commitScatterMethod(e.currentTarget.value)}
          >
            {#each SCATTER_METHODS as m (m.value)}
              <option value={m.value}>{m.label}</option>
            {/each}
          </select>
        {/if}
      </div>

      {#if data.inputs.length > 0}
        <div class="border-b border-border-soft px-3.5 py-3.5">
          <button
            type="button"
            class="mb-2 flex w-full items-center justify-between font-mono text-[10px] tracking-widest text-text-3 uppercase"
            onclick={() => (inputsOpen = !inputsOpen)}
          >
            <span>Inputs</span>
            <ChevronDown size={12} strokeWidth={2} class="transition-transform {inputsOpen ? '' : '-rotate-90'}" />
          </button>
          {#if inputsOpen}
            <div class="flex flex-col gap-2">
              {#each data.inputs as port (port.id)}
                <div class="rounded-md border border-border-soft bg-bg-well p-2.5">
                  <div class="mb-2 flex items-center justify-between">
                    <span class="font-mono text-xs text-text">{port.id}</span>
                    <span class="font-mono text-[9px] text-text-3">{port.dataType}</span>
                  </div>
                  {@render wiringFields(port, (v) => commitStepPickValue(port.id, v), (v) => commitStepLinkMerge(port.id, v))}
                  <div class="mt-1.5 flex flex-col gap-1">
                    <span class="font-mono text-[9px] tracking-widest text-text-3 uppercase">valueFrom</span>
                    <input
                      class="rounded border border-border-soft bg-bg-surface px-2 py-1 font-mono text-[11px] text-text outline-none focus:border-fairagro-mid-500"
                      value={port.valueFrom ?? ""}
                      placeholder="$(self)"
                      spellcheck="false"
                      onblur={(e) => commitValueFrom(port.id, port.valueFrom ?? "", e.currentTarget.value)}
                    />
                  </div>
                </div>
              {/each}
            </div>
          {/if}
        </div>
      {/if}

      {#if data.outputs.length > 0}
        <div class="border-b border-border-soft px-3.5 py-3.5">
          <button
            type="button"
            class="mb-2 flex w-full items-center justify-between font-mono text-[10px] tracking-widest text-text-3 uppercase"
            onclick={() => (outputsOpen = !outputsOpen)}
          >
            <span>Outputs</span>
            <ChevronDown size={12} strokeWidth={2} class="transition-transform {outputsOpen ? '' : '-rotate-90'}" />
          </button>
          {#if outputsOpen}
            <div class="flex flex-col gap-1.5">
              {#each data.outputs as port (port.id)}
                <div class="flex items-center justify-between rounded-md border border-border-soft bg-bg-well px-2.5 py-2">
                  <span class="font-mono text-xs text-text">{port.id}</span>
                  <span class="font-mono text-[9px] text-text-3">{port.dataType}</span>
                </div>
              {/each}
            </div>
          {/if}
        </div>
      {/if}
    {/if}

    {#if data.diagnostics.length > 0}
      <div class="px-3.5 py-3.5">
        <div class="mb-2 font-mono text-[10px] tracking-widest text-fairagro-red-light uppercase">Diagnostics</div>
        <div class="flex flex-col gap-1.5">
          {#each data.diagnostics as d, i (i)}
            <div class="rounded-md border border-fairagro-red/40 bg-fairagro-red/10 px-2.5 py-2 text-[11.5px] text-fairagro-red-light">{d.message}</div>
          {/each}
        </div>
      </div>
    {/if}
  {/if}

  {#if mutationError}
    <div class="sticky bottom-0 border-t border-fairagro-red/40 bg-fairagro-red/10 px-3.5 py-2.5 font-mono text-[11px] text-fairagro-red-light">
      {mutationErrorMessage(mutationError)}
    </div>
  {/if}
</aside>
