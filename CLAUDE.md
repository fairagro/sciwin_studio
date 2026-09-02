# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Overview

SciWIn Studio is the graphical desktop companion to SciWIn-Client (`s4n`), the FAIRagro Scientific Workflow Infrastructure. It is a [Tauri v2](https://tauri.app/) app: a Rust backend (`src-tauri/`) plus a SvelteKit/Svelte 5 frontend (`src/`), giving researchers a visual way to build, inspect, and run CWL (Common Workflow Language) workflows.

Every SciWIn Studio project is an `s4n` project on disk (a `workflow.toml` plus a git repository), so projects created or edited in the GUI stay fully compatible with the `s4n` CLI and vice versa.

Depends on the `sciwin` and `commonwl` crates (and `cwl-lsp`, a crate inside `commonwl/crates/lsp`) from the sibling `../sciwin` and `../commonwl` repos.

## Structure

Backend, `src-tauri/src/` (each file's `#[tauri::command]`s are registered in `lib.rs`'s `invoke_handler!`):
- `lib.rs` — Tauri app setup: plugins (`tauri-plugin-opener`, `tauri-plugin-dialog`), managed state (`PtyState`), the single `invoke_handler!` list.
- `files.rs` — filesystem tree reading (honors `.gitignore` via the `ignore` crate), CWL file read/write/delete, CWL document type detection.
- `project.rs` — `workflow.toml` detection, `sciwin::project::initialize_project` wrapper.
- `graph.rs` / `graph_types.rs` — loads a CWL `Workflow` document (via `commonwl`) into a `WorkflowView` (nodes/edges/ports) for the graph canvas; `compute_revision` hashes raw file bytes to detect external changes.
- `mutation.rs` — the workflow graph editing commands (connect/disconnect nodes, add/remove steps, rename, scatter/when/pick-value/link-merge settings), delegating to `sciwin::authoring::workflow::*_mut` functions.
- `layout.rs` — persists node positions per workflow file under `.sciwin/layout/<relative-cwl-path>.json` in the project root.
- `lsp.rs` — starts `cwl-lsp` in-process over an in-memory duplex pipe (not stdio) and forwards Content-Length-framed JSON-RPC messages to the frontend via the `lsp://message` event; `lsp_send` pushes messages the other way.
- `terminal.rs` — spawns a PTY (`portable-pty`) for the in-app terminal, streams output via the `pty-output` event; also `check_s4n` (detects an installed `s4n` binary and gives an install hint).
- `session.rs` — persists/restores frontend UI state (`session.json` in the app data dir) as an opaque JSON blob; the shape is owned by `src/lib/state/workspace.svelte.ts`, not duplicated in Rust.

Frontend, `src/`:
- `routes/` — SvelteKit routes/layout (this is a single-page desktop app, not a multi-route site).
- `lib/components/` — UI components: `Editor.svelte` (Monaco), `GraphView.svelte`/`WorkflowNode.svelte` (`@xyflow/svelte` canvas), `Terminal.svelte` (`@xterm/xterm`), `Sidebar.svelte`/`FileTreeNode.svelte`, `Inspector.svelte`, dialogs, `context-menu/`.
- `lib/graph/` — client-side graph helpers: `layout.ts` (dagre auto-layout), `mutation.ts`, `styling.ts`, `transform.ts`, `types.ts` — the TS-side counterparts to `graph.rs`/`mutation.rs`.
- `lib/lsp/` — `connection.ts` wires the `lsp://message` Tauri event to a JSON-RPC client; `providers.ts` adapts that to Monaco's language-feature providers.
- `lib/state/` — `workspace.svelte.ts` (Svelte 5 runes-based global state: open tabs, active path, sidebar/terminal layout, dirty tracking), `session.ts` (save/restore via the `session.rs` commands).

## Common commands

```bash
npm install                 # install frontend dependencies
npm run dev                 # SvelteKit dev server only (no Tauri window)
npx tauri dev                # full app in dev mode (backend + frontend, hot reload)
npx tauri build               # production bundle
npm run check                # svelte-check (frontend typecheck), matches CI

cargo build                                    # build src-tauri
cargo clippy --workspace                       # matches CI lint
cargo nextest run --workspace --no-fail-fast   # run Rust tests (or `cargo test`)
```

`npx tauri dev`/`build` need the native GTK/WebKit dependencies (see `.github/workflows/ci.yml` for the exact apt package list on Linux) and the Tauri CLI (`@tauri-apps/cli`, already a devDependency, invocable as `npm run tauri -- <args>` too).

### CI

`.github/workflows/ci.yml` runs `cargo clippy --workspace`, `cargo nextest run --workspace`, and `npm run check` (frontend typecheck) on every push/PR to `main`. `.github/workflows/release.yml` handles bundling and publishing tagged releases.

## Architecture notes

- Rust and TypeScript each maintain their own view of a workflow graph (`graph.rs`/`graph_types.rs` vs. `lib/graph/`); the Rust side is the source of truth read from the CWL file on disk, the frontend applies optimistic updates and re-syncs via `compute_revision`.
- All workflow edits happen through `mutation.rs` commands, which delegate to `sciwin::authoring::workflow::*_mut` functions in the `sciwin` crate rather than duplicating CWL-editing logic here — keep new mutation commands as thin wrappers.
- `cwl-lsp` runs in-process inside the Tauri backend (not as a separate subprocess), communicating with Monaco over a Tauri event bridge rather than stdio; see `lsp.rs` for the framing protocol if editor language features misbehave.
- The GUI has no independent CWL parsing or execution logic; it is a thin front end over `sciwin`/`commonwl`, same as the `s4n` CLI in the `sciwin` repo.
