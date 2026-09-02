import { invoke } from "@tauri-apps/api/core";
import type { LayoutPosition } from "./types";

// Absent file (never dragged, or a CLI-made workflow) comes back as {}, not
// an error -- callers fall back to dagre for any node missing an entry.
export function getNodeLayout(projectRoot: string, path: string): Promise<Record<string, LayoutPosition>> {
  return invoke("get_node_layout", { projectRoot, path });
}

export function saveNodeLayout(projectRoot: string, path: string, positions: Record<string, LayoutPosition>): Promise<void> {
  return invoke("save_node_layout", { projectRoot, path, positions });
}

export function resetNodeLayout(projectRoot: string, path: string): Promise<void> {
  return invoke("reset_node_layout", { projectRoot, path });
}
