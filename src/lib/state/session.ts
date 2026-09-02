import { invoke } from "@tauri-apps/api/core";
import { workspace, type SidebarView } from "./workspace.svelte";

interface PersistedSession {
  projectPath?: string;
  tabs?: string[];
  activePath?: string;
  sidebarView?: SidebarView;
  sidebarCollapsed?: boolean;
  sidebarWidth?: number;
  terminalOpen?: boolean;
  terminalHeight?: number;
}

const SIDEBAR_VIEWS: SidebarView[] = ["workflows", "filesystem", "sourcecontrol"];

function nameFor(path: string): string {
  return path.split(/[\\/]/).pop() ?? path;
}

async function exists(path: string): Promise<boolean> {
  return invoke<boolean>("path_exists", { path }).catch(() => false);
}

/** Restores window layout and, if the project folder and its open files are
 * still there, the last project/tabs too. Silent no-op on a fresh install or
 * anything that no longer resolves - never blocks the app from opening. */
export async function restoreSession(): Promise<void> {
  const session = await invoke<PersistedSession | null>("load_session").catch(() => null);
  if (!session) return;

  if (session.sidebarView && SIDEBAR_VIEWS.includes(session.sidebarView)) workspace.sidebarView = session.sidebarView;
  if (typeof session.sidebarCollapsed === "boolean") workspace.sidebarCollapsed = session.sidebarCollapsed;
  if (typeof session.sidebarWidth === "number") workspace.sidebarWidth = session.sidebarWidth;
  if (typeof session.terminalOpen === "boolean") workspace.terminalOpen = session.terminalOpen;
  if (typeof session.terminalHeight === "number") workspace.terminalHeight = session.terminalHeight;

  if (!session.projectPath || !(await exists(session.projectPath))) return;

  const hasConfig = await invoke<boolean>("has_workflow_config", { path: session.projectPath }).catch(() => false);
  workspace.openProject(session.projectPath, hasConfig);

  for (const path of session.tabs ?? []) {
    if (await exists(path)) workspace.openTab(path, nameFor(path));
  }
  if (session.activePath && workspace.tabs.some((t) => t.path === session.activePath)) {
    workspace.activePath = session.activePath;
  }
}

let saveTimer: ReturnType<typeof setTimeout> | undefined;

/** Call from a `$effect` that reads the persisted fields, so Svelte re-runs
 * this on every change. Reads workspace synchronously (for correct effect
 * dependency tracking) but debounces the actual disk write, since this also
 * fires on every pixel of a sidebar/terminal resize drag. */
export function scheduleSave(): void {
  const session: PersistedSession = {
    projectPath: workspace.projectPath ?? undefined,
    tabs: workspace.tabs.map((t) => t.path),
    activePath: workspace.activePath ?? undefined,
    sidebarView: workspace.sidebarView,
    sidebarCollapsed: workspace.sidebarCollapsed,
    sidebarWidth: workspace.sidebarWidth,
    terminalOpen: workspace.terminalOpen,
    terminalHeight: workspace.terminalHeight,
  };
  clearTimeout(saveTimer);
  saveTimer = setTimeout(() => {
    invoke("save_session", { session }).catch(() => {});
  }, 400);
}
