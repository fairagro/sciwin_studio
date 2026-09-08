<script lang="ts">
  import { Dialog, Select } from "bits-ui";
  import {
    Settings as SettingsIcon,
    X,
    Plus,
    Pencil,
    Trash2,
    Server,
    Database,
    Container,
    CircleCheck,
    CircleAlert,
    ChevronDown,
    Check,
    Save,
  } from "@lucide/svelte";
  import { settingsStore, type ContainerEngineChoice, type RemoteBackend, type RemoteBackendKind } from "$lib/state/settings.svelte";
  import { execution, backendKey, type BackendId } from "$lib/state/execution.svelte";

  let { open = $bindable(false) }: { open: boolean } = $props();

  $effect(() => {
    if (open) {
      settingsStore.load();
      execution.refreshBackends();
    }
  });

  const ENGINE_OPTIONS: { value: ContainerEngineChoice | "auto"; label: string }[] = [
    { value: "auto", label: "Auto-detect" },
    { value: "docker", label: "Docker" },
    { value: "podman", label: "Podman" },
    { value: "singularity", label: "Singularity" },
    { value: "apptainer", label: "Apptainer" },
  ];

  const selectedEngine = $derived(settingsStore.settings?.localContainerEngine ?? "auto");
  const selectedEngineLabel = $derived(ENGINE_OPTIONS.find((o) => o.value === selectedEngine)?.label ?? "Auto-detect");

  async function handleEngineChange(value: string) {
    await settingsStore.setLocalContainerEngine(value === "auto" ? null : (value as ContainerEngineChoice));
  }

  function backendStatus(id: BackendId) {
    return execution.backends.find((b) => backendKey(b.id) === backendKey(id));
  }

  const localStatus = $derived(backendStatus({ kind: "local" }));
  const dockerStatus = $derived(backendStatus({ kind: "docker" }));

  let s3Enabled = $state(false);
  let s3EndpointUrl = $state("");
  let s3AccessKeyId = $state("");
  let s3SecretAccessKey = $state("");
  let s3Region = $state("");
  let s3Saving = $state(false);
  let s3Error = $state<string | null>(null);

  $effect(() => {
    const s3 = settingsStore.settings?.s3;
    s3Enabled = s3 != null;
    s3EndpointUrl = s3?.endpointUrl ?? "";
    s3AccessKeyId = s3?.accessKeyId ?? "";
    s3SecretAccessKey = s3?.secretAccessKey ?? "";
    s3Region = s3?.region ?? "";
  });

  async function saveS3() {
    s3Saving = true;
    s3Error = null;
    try {
      await settingsStore.setS3Settings(
        s3Enabled
          ? {
              endpointUrl: s3EndpointUrl,
              accessKeyId: s3AccessKeyId,
              secretAccessKey: s3SecretAccessKey,
              region: s3Region || null,
            }
          : null,
      );
    } catch (error) {
      s3Error = String(error);
    } finally {
      s3Saving = false;
    }
  }

  interface FormState {
    id: string | null;
    name: string;
    kind: "tes" | "reana";
    url: string;
    bucket: string;
    token: string;
  }
  let form = $state<FormState | null>(null);
  let formError = $state<string | null>(null);
  let formBusy = $state(false);

  function startAdd() {
    form = { id: null, name: "", kind: "tes", url: "", bucket: "", token: "" };
    formError = null;
  }

  function startEdit(backend: RemoteBackend) {
    form = {
      id: backend.id,
      name: backend.name,
      kind: backend.kind,
      url: backend.url,
      bucket: backend.bucket ?? "",
      token: backend.token ?? "",
    };
    formError = null;
  }

  function cancelForm() {
    form = null;
    formError = null;
  }

  async function submitForm() {
    if (!form) return;
    formBusy = true;
    formError = null;
    try {
      const kind: RemoteBackendKind =
        form.kind === "tes"
          ? { kind: "tes", url: form.url, bucket: form.bucket, token: form.token || null }
          : { kind: "reana", url: form.url, token: form.token };
      if (form.id) {
        await settingsStore.updateRemoteBackend(form.id, form.name, kind);
      } else {
        await settingsStore.addRemoteBackend(form.name, kind);
      }
      form = null;
    } catch (error) {
      formError = String(error);
    } finally {
      formBusy = false;
    }
  }

  async function removeBackend(id: string) {
    try {
      await settingsStore.removeRemoteBackend(id);
    } catch (error) {
      console.error("Failed to remove backend:", error);
    }
  }
</script>

<Dialog.Root bind:open>
  <Dialog.Portal>
    <Dialog.Overlay class="fixed inset-0 z-40 bg-black/60" />
    <Dialog.Content
      class="fixed top-1/2 left-1/2 z-50 max-h-[85vh] w-140 -translate-x-1/2 -translate-y-1/2 overflow-y-auto rounded-xl border border-border bg-bg-surface text-text shadow-2xl"
    >
      <div class="flex items-center gap-2 border-b border-border-soft px-4 py-3.5">
        <SettingsIcon size={16} strokeWidth={1.8} />
        <Dialog.Title class="font-display flex-1 text-[15px] font-semibold">Settings</Dialog.Title>
        <Dialog.Close class="rounded p-1 text-text-3 hover:bg-border-soft hover:text-text">
          <X size={14} strokeWidth={1.8} />
        </Dialog.Close>
      </div>

      <div class="space-y-6 px-4 py-4">
        <section>
          <h3 class="mb-2 flex items-center gap-1.5 font-mono text-[11px] font-semibold text-text-2">
            <Container size={13} strokeWidth={1.8} /> LOCAL
          </h3>
          <Select.Root type="single" value={selectedEngine} onValueChange={handleEngineChange}>
            <Select.Trigger
              class="flex w-full items-center gap-1.5 rounded-md border border-border bg-bg-well px-2.5 py-1.5 text-left font-mono text-xs text-text hover:bg-border-soft"
            >
              <span class="flex-1">{selectedEngineLabel}</span>
              <ChevronDown size={12} strokeWidth={2} class="text-text-3" />
            </Select.Trigger>
            <Select.Portal>
              <Select.Content class="z-50 min-w-40 rounded-md border border-border bg-bg-surface p-1 shadow-lg">
                <Select.Viewport>
                  {#each ENGINE_OPTIONS as option (option.value)}
                    <Select.Item
                      value={option.value}
                      label={option.label}
                      class="flex cursor-pointer items-center gap-2 rounded px-2 py-1.5 font-mono text-xs text-text-2 outline-none hover:bg-border-soft data-highlighted:bg-border-soft"
                    >
                      <span class="flex-1">{option.label}</span>
                      {#if option.value === selectedEngine}<Check size={12} strokeWidth={2} />{/if}
                    </Select.Item>
                  {/each}
                </Select.Viewport>
              </Select.Content>
            </Select.Portal>
          </Select.Root>
          {#if localStatus?.message}
            <p class="mt-2 flex items-start gap-1.5 font-mono text-[11px] text-amber-400">
              <CircleAlert size={13} strokeWidth={1.8} class="mt-0.5 shrink-0" />
              {localStatus.message}
            </p>
          {/if}
        </section>

        <section>
          <h3 class="mb-2 flex items-center gap-1.5 font-mono text-[11px] font-semibold text-text-2">
            <Server size={13} strokeWidth={1.8} /> DOCKER
          </h3>
          <div class="flex items-center gap-1.5 font-mono text-[11px]">
            {#if dockerStatus?.available}
              <CircleCheck size={13} strokeWidth={1.8} class="shrink-0 text-fairagro-mid-400" />
              <span class="text-text-2">Connected</span>
            {:else}
              <CircleAlert size={13} strokeWidth={1.8} class="shrink-0 text-amber-400" />
              <span class="text-text-2">{dockerStatus?.message ?? "Not reachable"}</span>
            {/if}
          </div>
        </section>

        <section>
          <div class="mb-2 flex items-center justify-between">
            <h3 class="flex items-center gap-1.5 font-mono text-[11px] font-semibold text-text-2">
              <Database size={13} strokeWidth={1.8} /> S3 STORAGE
            </h3>
            <label class="flex items-center gap-1.5 font-mono text-[11px] text-text-2">
              <input type="checkbox" bind:checked={s3Enabled} />
              Enabled
            </label>
          </div>
          <p class="mb-2 font-mono text-[10px] text-text-3">
            One process-wide S3 config, shared by every backend that touches an s3:// URL. Required for TES.
          </p>
          {#if s3Enabled}
            <div class="space-y-2">
              <input
                class="w-full rounded-md border border-border bg-bg-well px-2.5 py-1.5 font-mono text-xs text-text"
                placeholder="Endpoint URL (e.g. https://s3.example.com)"
                bind:value={s3EndpointUrl}
              />
              <input
                class="w-full rounded-md border border-border bg-bg-well px-2.5 py-1.5 font-mono text-xs text-text"
                placeholder="Access key ID"
                bind:value={s3AccessKeyId}
              />
              <input
                class="w-full rounded-md border border-border bg-bg-well px-2.5 py-1.5 font-mono text-xs text-text"
                placeholder="Secret access key"
                type="password"
                bind:value={s3SecretAccessKey}
              />
              <input
                class="w-full rounded-md border border-border bg-bg-well px-2.5 py-1.5 font-mono text-xs text-text"
                placeholder="Region (optional, defaults to us-east-1)"
                bind:value={s3Region}
              />
            </div>
          {/if}
          {#if s3Error}
            <p class="mt-2 font-mono text-[11px] text-fairagro-red-light">{s3Error}</p>
          {/if}
          <button
            type="button"
            onclick={saveS3}
            disabled={s3Saving}
            class="mt-2 flex items-center gap-1.5 rounded-md bg-fairagro-mid-500 px-2.5 py-1 font-mono text-[11px] font-semibold text-bg-well hover:bg-fairagro-mid-400 disabled:opacity-60"
          >
            <Save size={11} strokeWidth={2} />
            {s3Saving ? "Saving…" : "Save"}
          </button>
        </section>

        <section>
          <div class="mb-2 flex items-center justify-between">
            <h3 class="flex items-center gap-1.5 font-mono text-[11px] font-semibold text-text-2">
              <Server size={13} strokeWidth={1.8} /> REMOTE BACKENDS
            </h3>
            {#if !form}
              <button
                type="button"
                onclick={startAdd}
                class="flex items-center gap-1 rounded-md border border-border px-2 py-1 font-mono text-[11px] text-text-2 hover:bg-border-soft hover:text-text"
              >
                <Plus size={11} strokeWidth={2} /> Add
              </button>
            {/if}
          </div>

          {#if settingsStore.settings && settingsStore.settings.remoteBackends.length === 0 && !form}
            <p class="font-mono text-[11px] text-text-3">No remote backends configured.</p>
          {/if}

          <ul class="space-y-1.5">
            {#each settingsStore.settings?.remoteBackends ?? [] as backend (backend.id)}
              {@const status = backendStatus({ kind: "remote", id: backend.id })}
              <li class="flex items-center gap-2 rounded-md border border-border bg-bg-well px-2.5 py-1.5">
                {#if status?.available}
                  <CircleCheck size={13} strokeWidth={1.8} class="shrink-0 text-fairagro-mid-400" />
                {:else}
                  <CircleAlert size={13} strokeWidth={1.8} class="shrink-0 text-amber-400" />
                {/if}
                <div class="min-w-0 flex-1">
                  <div class="truncate font-mono text-[11px] text-text">
                    {backend.name} <span class="text-text-3">({backend.kind.toUpperCase()})</span>
                  </div>
                  {#if status?.message}
                    <div class="truncate font-mono text-[10px] text-text-3">{status.message}</div>
                  {/if}
                </div>
                <button type="button" onclick={() => startEdit(backend)} class="rounded p-1 text-text-3 hover:bg-border-soft hover:text-text">
                  <Pencil size={12} strokeWidth={1.8} />
                </button>
                <button
                  type="button"
                  onclick={() => removeBackend(backend.id)}
                  class="rounded p-1 text-text-3 hover:bg-fairagro-red/20 hover:text-fairagro-red-light"
                >
                  <Trash2 size={12} strokeWidth={1.8} />
                </button>
              </li>
            {/each}
          </ul>

          {#if form}
            <div class="mt-3 space-y-2 rounded-md border border-border bg-bg-well p-3">
              <div class="flex gap-2">
                <button
                  type="button"
                  onclick={() => form && (form.kind = "tes")}
                  class="flex-1 rounded-md border px-2 py-1 font-mono text-[11px] {form.kind === 'tes'
                    ? 'border-fairagro-mid-500 text-fairagro-mid-300'
                    : 'border-border text-text-2'}"
                >
                  TES
                </button>
                <button
                  type="button"
                  onclick={() => form && (form.kind = "reana")}
                  class="flex-1 rounded-md border px-2 py-1 font-mono text-[11px] {form.kind === 'reana'
                    ? 'border-fairagro-mid-500 text-fairagro-mid-300'
                    : 'border-border text-text-2'}"
                >
                  REANA
                </button>
              </div>
              <input
                class="w-full rounded-md border border-border bg-bg-surface px-2.5 py-1.5 font-mono text-xs text-text"
                placeholder="Name"
                bind:value={form.name}
              />
              <input
                class="w-full rounded-md border border-border bg-bg-surface px-2.5 py-1.5 font-mono text-xs text-text"
                placeholder={form.kind === "reana" ? "Server URL (/api is added automatically)" : "Server URL"}
                bind:value={form.url}
              />
              {#if form.kind === "tes"}
                <input
                  class="w-full rounded-md border border-border bg-bg-surface px-2.5 py-1.5 font-mono text-xs text-text"
                  placeholder="Bucket (e.g. my-bucket/prefix)"
                  bind:value={form.bucket}
                />
                <input
                  class="w-full rounded-md border border-border bg-bg-surface px-2.5 py-1.5 font-mono text-xs text-text"
                  placeholder="Bearer token (optional)"
                  type="password"
                  bind:value={form.token}
                />
              {:else}
                <input
                  class="w-full rounded-md border border-border bg-bg-surface px-2.5 py-1.5 font-mono text-xs text-text"
                  placeholder="Access token"
                  type="password"
                  bind:value={form.token}
                />
              {/if}
              {#if formError}
                <p class="font-mono text-[11px] text-fairagro-red-light">{formError}</p>
              {/if}
              <div class="flex gap-2">
                <button
                  type="button"
                  onclick={cancelForm}
                  class="flex-1 rounded-md border border-border-soft py-1.5 font-mono text-[11px] text-text-2 hover:bg-border-soft"
                >
                  Cancel
                </button>
                <button
                  type="button"
                  onclick={submitForm}
                  disabled={formBusy}
                  class="flex-1 rounded-md bg-fairagro-mid-500 py-1.5 font-mono text-[11px] font-semibold text-bg-well hover:bg-fairagro-mid-400 disabled:opacity-60"
                >
                  {formBusy ? "Saving…" : form.id ? "Save" : "Add"}
                </button>
              </div>
            </div>
          {/if}
        </section>
      </div>
    </Dialog.Content>
  </Dialog.Portal>
</Dialog.Root>
