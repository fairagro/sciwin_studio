import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

export type RunStatus = "created" | "queued" | "running" | "finished" | "failed" | "cancelled" | "stopped";
export type StepStatus = "started" | "finished";

type StepEventPayload =
  | { kind: "started"; runId: string; stepId: string; at: string }
  | { kind: "finished"; runId: string; stepId: string; at: string }
  | { kind: "output"; runId: string; stepId: string; stdout: string; stderr: string };

interface RunStatusPayload {
  runId: string;
  status: RunStatus;
}

// Bare CWL step id (e.g. "plot", not the graph's "step/plot" node id) -> whether it has
// started, or already finished. Matches commonwl::engine::StepEvent one-to-one.
export type StepStatusMap = Record<string, StepStatus>;

export interface StepOutput {
  stepId: string;
  stdout: string;
  stderr: string;
}

const TERMINAL: ReadonlySet<RunStatus> = new Set(["finished", "failed", "cancelled", "stopped"]);

// Singleton, same pattern as `workspace`: execution state (and the events feeding it) outlive
// any one component, so it's tracked independently of whichever view happens to be mounted.
class ExecutionState {
  runId = $state<string | null>(null);
  cwlfile = $state<string | null>(null);
  status = $state<RunStatus | null>(null);
  stepStatus = $state<StepStatusMap>({});
  // In step-finish order, not scheduling order -- a step's output only exists once it's done.
  // Not tailable mid-command: crankshaft only hands back a step's stdout/stderr as a file
  // copied over after its process exits, so this is "as steps complete", not a live tail.
  output = $state<StepOutput[]>([]);
  error = $state<string | null>(null);
  // cwlfile path -> job file path, so each open tab keeps its own choice. In-memory only for
  // now -- not persisted across app restarts (unlike, say, node layout).
  jobFileByCwlFile = $state<Record<string, string>>({});

  isRunning = $derived(this.status !== null && !TERMINAL.has(this.status));

  constructor() {
    listen<StepEventPayload>("execution://step-event", (event) => {
      const payload = event.payload;
      if (payload.runId !== this.runId) return; // a run this session is no longer tracking

      if (payload.kind === "output") {
        this.output = [...this.output, { stepId: payload.stepId, stdout: payload.stdout, stderr: payload.stderr }];
        return;
      }
      this.stepStatus = { ...this.stepStatus, [payload.stepId]: payload.kind };
    });

    listen<RunStatusPayload>("execution://status", (event) => {
      const payload = event.payload;
      if (payload.runId !== this.runId) return;
      this.status = payload.status;
    });
  }

  jobFileFor(cwlfile: string): string | null {
    return this.jobFileByCwlFile[cwlfile] ?? null;
  }

  setJobFile(cwlfile: string, jobFile: string | null) {
    if (jobFile === null) {
      const { [cwlfile]: _removed, ...rest } = this.jobFileByCwlFile;
      this.jobFileByCwlFile = rest;
    } else {
      this.jobFileByCwlFile = { ...this.jobFileByCwlFile, [cwlfile]: jobFile };
    }
  }

  async run(cwlfile: string) {
    if (this.isRunning) return;
    this.error = null;
    this.cwlfile = cwlfile;
    this.stepStatus = {};
    this.output = [];
    this.status = "queued";
    try {
      this.runId = await invoke<string>("execute_workflow", {
        cwlfile,
        inputFile: this.jobFileFor(cwlfile),
        outDir: null,
      });
      // The backend already emits this itself, but it does so before this `invoke` call
      // resolves -- by the time that event reaches the listener below, `this.runId` isn't
      // set yet, so its `payload.runId !== this.runId` guard drops it. Set it here instead,
      // now that submit succeeding is itself proof the run is under way.
      this.status = "running";
    } catch (error) {
      this.status = null;
      this.error = String(error);
    }
  }

  async cancel() {
    if (!this.runId) return;
    try {
      await invoke("cancel_workflow", { runId: this.runId });
    } catch (error) {
      console.error("Failed to cancel run:", error);
    }
  }
}

export const execution = new ExecutionState();
