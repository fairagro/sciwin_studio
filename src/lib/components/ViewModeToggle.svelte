<script lang="ts">
  import { Workflow, FileCode, ChevronDown, Check } from "@lucide/svelte";
  import { Select } from "bits-ui";
  import {
    loadDocType,
    workspace,
    type CWLDocType,
    type Tab,
    type TabViewMode,
  } from "$lib/state/workspace.svelte";

  let { tab }: { tab: Tab } = $props();

  type ViewOption = {
    value: TabViewMode;
    label: string;
    icon: typeof Workflow;
  };

  let docType = $state<CWLDocType | null>(null);

  const viewOptions = $derived.by((): ViewOption[] => {
    if (docType === "Workflow") {
      return [
        { value: "graph", label: "Graph", icon: Workflow },
        { value: "code", label: "Code", icon: FileCode },
      ];
    }

    return [
      { value: "code", label: "Code", icon: FileCode },
    ];
  });

  let selected = $derived(
    viewOptions.find((option) => option.value === tab.viewMode) ??
      viewOptions[0]
  );

  $effect(() => {
    loadDocType(tab.path).then((type) => {
      docType = type;
    });
  });

  function setView(value: string) {
    const option = viewOptions.find((option) => option.value === value);

    if (option) {
      workspace.setViewMode(tab.path, option.value);
    }
  }
</script>

<Select.Root
  type="single"
  value={tab.viewMode}
  onValueChange={setView}
>
  <Select.Trigger
    class="flex items-center gap-1.5 rounded-md border border-border bg-bg-surface px-2.5 py-1 font-mono text-xs text-text-2 hover:bg-border-soft hover:text-text"
    aria-label="Select view"
  >
    {#if selected}
      {@const Icon = selected.icon}
      <Icon size={13} strokeWidth={1.8} />
      <span>{selected.label}</span>
    {/if}

    <ChevronDown
      size={12}
      strokeWidth={2}
      class="text-text-3"
    />
  </Select.Trigger>

  <Select.Portal>
    <Select.Content
      side="bottom"
      sideOffset={4}
      class="z-50 min-w-35 rounded-md border border-border bg-bg-surface p-1 shadow-lg"
    >
      <Select.Viewport>
        {#each viewOptions as option (option.value)}
          {@const Icon = option.icon}

          <Select.Item
            value={option.value}
            label={option.label}
            class="flex cursor-pointer items-center gap-2 rounded px-2 py-1.5 font-mono text-xs text-text-2 outline-none hover:bg-border-soft data-highlighted:bg-border-soft"
          >
            <Icon size={13} strokeWidth={1.8} />

            <span class="flex-1">{option.label}</span>

            {#if tab.viewMode === option.value}
              <Check size={12} strokeWidth={2} />
            {/if}
          </Select.Item>
        {/each}
      </Select.Viewport>
    </Select.Content>
  </Select.Portal>
</Select.Root>