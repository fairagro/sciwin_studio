import { invoke } from "@tauri-apps/api/core";
import type { ConnectionEndpoint, NodeRef } from "./types";

export interface MutationArgs {
  path: string;
  revision: string;
  dirty: boolean;
  from: ConnectionEndpoint;
  to: ConnectionEndpoint;
}

export interface DeleteNodeArgs {
  path: string;
  revision: string;
  dirty: boolean;
  node: NodeRef;
}

export interface AddStepNodeArgs {
  path: string;
  revision: string;
  dirty: boolean;
  toolPath: string;
  name: string;
}


export function connectWorkflowNodes(args: MutationArgs): Promise<void> {
  return invoke("connect_workflow_nodes", { ...args });
}

export function disconnectWorkflowNodes(args: MutationArgs): Promise<void> {
  return invoke("disconnect_workflow_nodes", { ...args });
}

export function deleteWorkflowNode(args: DeleteNodeArgs): Promise<void> {
  return invoke("delete_workflow_node", { ...args });
}

export function addWorkflowStepNode(args: AddStepNodeArgs): Promise<void> {
  return invoke("add_workflow_step_node", { ...args });
}
