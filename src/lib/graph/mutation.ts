import { invoke } from "@tauri-apps/api/core";
import type { ConnectionEndpoint } from "./types";

export interface MutationArgs {
  path: string;
  revision: string;
  dirty: boolean;
  from: ConnectionEndpoint;
  to: ConnectionEndpoint;
}

// Both reject with a MutationError (see types.ts), not a string -- the
// backend's #[serde(tag = "kind")] error enum, not invoke()'s usual
// stringly-typed rejection.
export function connectWorkflowNodes(args: MutationArgs): Promise<void> {
  return invoke("connect_workflow_nodes", { ...args });
}

export function disconnectWorkflowNodes(args: MutationArgs): Promise<void> {
  return invoke("disconnect_workflow_nodes", { ...args });
}
