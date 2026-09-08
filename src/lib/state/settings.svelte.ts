import { invoke } from "@tauri-apps/api/core";
import { execution } from "$lib/state/execution.svelte";

// Matches src-tauri/src/settings.rs.
export type ContainerEngineChoice = "docker" | "podman" | "singularity" | "apptainer";

export interface S3Settings {
  endpointUrl: string;
  accessKeyId: string;
  secretAccessKey: string;
  region: string | null;
}

export type RemoteBackendKind =
  | { kind: "tes"; url: string; bucket: string; token: string | null }
  | { kind: "reana"; url: string; token: string };

export interface RemoteBackend {
  id: string;
  name: string;
  kind: RemoteBackendKind["kind"];
  url: string;
  bucket?: string;
  token?: string | null;
}

export interface Settings {
  localContainerEngine: ContainerEngineChoice | null;
  s3: S3Settings | null;
  remoteBackends: RemoteBackend[];
}

// Singleton, same pattern as `workspace`/`execution`. Settings rarely change and every
// consumer wants the same current value, so there's no reason for each component to fetch its
// own copy.
class SettingsStore {
  settings = $state<Settings | null>(null);
  loading = $state(false);
  error = $state<string | null>(null);

  async load() {
    this.loading = true;
    this.error = null;
    try {
      this.settings = await invoke<Settings>("get_settings");
    } catch (error) {
      this.error = String(error);
    } finally {
      this.loading = false;
    }
  }

  // Every mutation refreshes execution's backend list too -- Settings is the only thing that
  // changes what list_backends reports.
  async setLocalContainerEngine(engine: ContainerEngineChoice | null) {
    this.settings = await invoke<Settings>("set_local_container_engine", { engine });
    await execution.refreshBackends();
  }

  async setS3Settings(s3: S3Settings | null) {
    this.settings = await invoke<Settings>("set_s3_settings", { s3 });
    await execution.refreshBackends();
  }

  async addRemoteBackend(name: string, kind: RemoteBackendKind) {
    this.settings = await invoke<Settings>("add_remote_backend", { name, kind });
    await execution.refreshBackends();
  }

  async updateRemoteBackend(id: string, name: string, kind: RemoteBackendKind) {
    this.settings = await invoke<Settings>("update_remote_backend", { id, name, kind });
    await execution.refreshBackends();
  }

  async removeRemoteBackend(id: string) {
    this.settings = await invoke<Settings>("remove_remote_backend", { id });
    await execution.refreshBackends();
  }
}

export const settingsStore = new SettingsStore();
