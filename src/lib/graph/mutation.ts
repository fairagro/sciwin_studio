import { invoke } from "@tauri-apps/api/core";
import type { ConnectionEndpoint, LinkMerge, NodeRef, PickValue, ScatterMethod } from "./types";

export interface MutationArgs {
  path: string;
  revision: string;
  dirty: boolean;
  from: ConnectionEndpoint;
  to: ConnectionEndpoint;
}

export interface ConnectArgs extends MutationArgs {
  // Set once the user has answered the corresponding dialog in GraphView --
  // omitted (false/null) on the first attempt, which is what lets the
  // backend refuse with needsScatterConfirmation/needsPickValue instead of
  // silently guessing.
  scatterConfirmed?: boolean;
  pickValue?: PickValue | null;
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

// Shared by every step-scoped Inspector edit below.
export interface StepArgs {
  path: string;
  revision: string;
  dirty: boolean;
  stepId: string;
}

export function renameWorkflowStep(args: StepArgs & { newId: string }): Promise<void> {
  return invoke("rename_workflow_step", { ...args });
}

export function setStepWhen(args: StepArgs & { expression: string | null }): Promise<void> {
  return invoke("set_step_when", { ...args });
}

export function setStepScatterMethod(args: StepArgs & { method: ScatterMethod | null }): Promise<void> {
  return invoke("set_step_scatter_method", { ...args });
}

export function setStepScattered(args: StepArgs & { port: string; scattered: boolean }): Promise<void> {
  return invoke("set_step_scattered", { ...args });
}

export function setStepPickValue(args: StepArgs & { port: string; method: PickValue | null }): Promise<void> {
  return invoke("set_step_pick_value", { ...args });
}

export function setStepInputValueFrom(args: StepArgs & { port: string; valueFrom: string | null }): Promise<void> {
  return invoke("set_step_input_value_from", { ...args });
}

export function setStepInputLinkMerge(args: StepArgs & { port: string; method: LinkMerge | null }): Promise<void> {
  return invoke("set_step_input_link_merge", { ...args });
}

// WorkflowStepInput has no type field of its own -- CWL infers it from
// whatever gets wired in later -- so a slot is just a name.
export function addStepInputSlot(args: StepArgs & { port: string }): Promise<void> {
  return invoke("add_step_input_slot", { ...args });
}

// Shared by the two workflow-output-scoped edits below.
export interface OutputArgs {
  path: string;
  revision: string;
  dirty: boolean;
  outputId: string;
}

export function setOutputPickValue(args: OutputArgs & { method: PickValue | null }): Promise<void> {
  return invoke("set_output_pick_value", { ...args });
}

export function setOutputLinkMerge(args: OutputArgs & { method: LinkMerge | null }): Promise<void> {
  return invoke("set_output_link_merge", { ...args });
}

export function connectWorkflowNodes(args: ConnectArgs): Promise<void> {
  return invoke("connect_workflow_nodes", {
    ...args,
    scatterConfirmed: args.scatterConfirmed ?? false,
    pickValue: args.pickValue ?? null,
  });
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
