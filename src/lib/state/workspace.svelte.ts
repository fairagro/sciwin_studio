import { invoke } from "@tauri-apps/api/core";
import type { FlowNodeData } from "$lib/graph/types";

export type TabViewMode = "graph" | "code";
export type CWLDocType = "Workflow" | "CommandLineTool" | "ExpressionTool" | "Operation";

export interface Tab {
  path: string;
  name: string;
  dirty: boolean;
  viewMode: TabViewMode;
}

export type SidebarView = "workflows" | "filesystem" | "sourcecontrol";

export async function loadDocType(path: string): Promise<CWLDocType | null> {
  try {
    return await invoke<CWLDocType>("cwl_doc_type", {
      path
    });
  } catch (error) {
    console.error("Failed to determine CWL document type:", error);
    return null
  }
}

class WorkspaceState {
  projectPath = $state<string | null>(null);
  projectName = $state<string | null>(null);
  // null = not yet checked, false = user declined init; other features gate on this
  projectHasConfig = $state<boolean | null>(null);
  tabs = $state<Tab[]>([]);
  activePath = $state<string | null>(null);

  sidebarView = $state<SidebarView>("workflows");
  sidebarCollapsed = $state(false);
  sidebarWidth = $state(248);
  terminalOpen = $state(false);
  terminalHeight = $state(176);

  selectedNodeId = $state<string | null>(null);
  selectedNodeData = $state<FlowNodeData | null>(null);
  inspectorOpen = $state(false);
  inspectorWidth = $state(280);
  // The open graph's revision, for the Inspector's own mutation calls --
  // kept in sync by GraphView, same as selectedNodeData.
  graphRevision = $state<string | null>(null);

  activeTab = $derived(this.tabs.find((t) => t.path === this.activePath) ?? null);

  selectSidebarView(view: SidebarView) {
    if (this.sidebarView === view && !this.sidebarCollapsed) {
      this.sidebarCollapsed = true;
    } else {
      this.sidebarView = view;
      this.sidebarCollapsed = false;
    }
  }

  toggleTerminal() {
    this.terminalOpen = !this.terminalOpen;
  }

  resizeSidebar(deltaPx: number) {
    this.sidebarWidth = Math.min(480, Math.max(180, this.sidebarWidth + deltaPx));
  }

  resizeTerminal(deltaPx: number) {
    this.terminalHeight = Math.min(560, Math.max(120, this.terminalHeight + deltaPx));
  }

  selectNode(id: string, data: FlowNodeData) {
    this.selectedNodeId = id;
    this.selectedNodeData = data;
    this.inspectorOpen = true;
  }

  closeInspector() {
    this.inspectorOpen = false;
    this.selectedNodeId = null;
    this.selectedNodeData = null;
  }

  resizeInspector(deltaPx: number) {
    this.inspectorWidth = Math.min(420, Math.max(220, this.inspectorWidth - deltaPx));
  }

  openProject(path: string, hasConfig: boolean) {
    this.projectPath = path;
    this.projectName = path.split(/[\\/]/).pop() ?? path;
    this.projectHasConfig = hasConfig;
    this.tabs = [];
    this.activePath = null;
  }

  closeProject() {
    this.projectPath = null;
    this.projectName = null;
    this.projectHasConfig = null;
    this.tabs = [];
    this.activePath = null;
  }

  async openTab(path: string, name: string) {
    if (!this.tabs.some((t) => t.path === path)) {
      const is_cwl = name.toLowerCase().endsWith(".cwl");
      const cwltype = await loadDocType(path);
      const viewMode: TabViewMode = is_cwl && cwltype == "Workflow" as CWLDocType ? "graph" : "code";
      this.tabs.push({ path, name, dirty: false, viewMode });
    }
    this.activePath = path;
  }

  setViewMode(path: string, mode: TabViewMode) {
    const tab = this.tabs.find((t) => t.path === path);
    if (tab) tab.viewMode = mode;
  }

  closeTab(path: string) {
    const index = this.tabs.findIndex((t) => t.path === path);
    if (index === -1) return;
    this.tabs.splice(index, 1);
    if (this.activePath === path) {
      this.activePath = (this.tabs[index] ?? this.tabs[index - 1])?.path ?? null;
    }
  }
}

export const workspace = new WorkspaceState();
