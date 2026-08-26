export interface Tab {
  path: string;
  name: string;
  dirty: boolean;
}

export type SidebarView = "workflows" | "filesystem" | "sourcecontrol";

class WorkspaceState {
  projectPath = $state<string | null>(null);
  projectName = $state<string | null>(null);
  tabs = $state<Tab[]>([]);
  activePath = $state<string | null>(null);

  sidebarView = $state<SidebarView>("workflows");
  sidebarCollapsed = $state(false);
  sidebarWidth = $state(248);
  terminalOpen = $state(false);
  terminalHeight = $state(176);

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

  openProject(path: string) {
    this.projectPath = path;
    this.projectName = path.split(/[\\/]/).pop() ?? path;
    this.tabs = [];
    this.activePath = null;
  }

  closeProject() {
    this.projectPath = null;
    this.projectName = null;
    this.tabs = [];
    this.activePath = null;
  }

  openTab(path: string, name: string) {
    if (!this.tabs.some((t) => t.path === path)) {
      this.tabs.push({ path, name, dirty: false });
    }
    this.activePath = path;
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
