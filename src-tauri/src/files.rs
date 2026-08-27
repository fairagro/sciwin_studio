use ignore::WalkBuilder;
use sciwin::cwl::{documents::CWLDocument, load_cwl_file};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use tauri::{AppHandle, Emitter};

use crate::graph::compute_revision;

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct WorkflowChanged {
    pub path: String,
    pub revision: String,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct FsEntry {
    name: String,
    path: String,
    is_dir: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    children: Option<Vec<FsEntry>>,
}

/// One directory level, honoring .gitignore / .git/info/exclude / the global
/// gitignore and skipping hidden entries, same as `git status` would.
fn read_sorted_dir(dir: &Path) -> Vec<(String, PathBuf, bool)> {
    let mut entries: Vec<(String, PathBuf, bool)> = WalkBuilder::new(dir)
        .max_depth(Some(1))
        .build()
        .filter_map(|e| e.ok())
        .filter(|e| e.depth() > 0)
        .filter_map(|e| {
            let is_dir = e.file_type()?.is_dir();
            let name = e.file_name().to_string_lossy().into_owned();
            Some((name, e.into_path(), is_dir))
        })
        .collect();

    entries.sort_by(|a, b| match (a.2, b.2) {
        (true, false) => std::cmp::Ordering::Less,
        (false, true) => std::cmp::Ordering::Greater,
        _ => a.0.to_lowercase().cmp(&b.0.to_lowercase()),
    });

    entries
}

/// Lists one directory level. Used by the Filesystem view, which expands lazily.
#[tauri::command]
pub fn list_dir(path: String) -> Vec<FsEntry> {
    read_sorted_dir(Path::new(&path))
        .into_iter()
        .map(|(name, path, is_dir)| FsEntry {
            name,
            path: path.to_string_lossy().into_owned(),
            is_dir,
            children: None,
        })
        .collect()
}

fn walk_cwl(dir: &Path) -> Vec<FsEntry> {
    let mut out = Vec::new();

    for (name, path, is_dir) in read_sorted_dir(dir) {
        if is_dir {
            let children = walk_cwl(&path);
            if !children.is_empty() {
                out.push(FsEntry {
                    name,
                    path: path.to_string_lossy().into_owned(),
                    is_dir: true,
                    children: Some(children),
                });
            }
        } else if name.to_lowercase().ends_with(".cwl") {
            out.push(FsEntry {
                name,
                path: path.to_string_lossy().into_owned(),
                is_dir: false,
                children: None,
            });
        }
    }

    out
}

/// Recursively finds every .cwl file, pruning directories that don't contain one.
/// Used by the Workflows view.
#[tauri::command]
pub fn get_cwl_files(root: String) -> Vec<FsEntry> {
    walk_cwl(Path::new(&root))
}

#[tauri::command]
pub fn read_file(path: String) -> Result<String, String> {
    let bytes = std::fs::read(path).map_err(|e| e.to_string())?;
    // NUL byte is the standard binary-content heuristic (same one git uses);
    // catches binary files an editor should refuse rather than silently
    // read as empty/garbled text and risk clobbering on save.
    if bytes.contains(&0) {
        return Err("binary".to_string());
    }
    String::from_utf8(bytes).map_err(|_| "binary".to_string())
}

/// Saving a `.cwl` file from Monaco changes what the graph view should show
/// for that path; `workflow-changed` tells any open graph view to re-fetch
/// instead of going stale until the tab is switched away and back.
#[tauri::command]
pub fn write_file(app: AppHandle, path: String, contents: String) -> Result<(), String> {
    std::fs::write(&path, contents.as_bytes()).map_err(|e| e.to_string())?;
    if path.to_lowercase().ends_with(".cwl") {
        let revision = compute_revision(contents.as_bytes());
        let _ = app.emit("workflow-changed", WorkflowChanged { path, revision });
    }
    Ok(())
}

#[tauri::command]
pub fn path_exists(path: String) -> bool {
    Path::new(&path).exists()
}

#[derive(Serialize, Deserialize)]
pub enum CWLDocType {
    Workflow,
    CommandLineTool,
    ExpressionTool,
    Operation,
}

impl From<CWLDocument> for CWLDocType {
    fn from(value: CWLDocument) -> Self {
        match value {
            CWLDocument::CommandLineTool(_) => Self::CommandLineTool,
            CWLDocument::ExpressionTool(_) => Self::ExpressionTool,
            CWLDocument::Operation(_) => Self::Operation,
            CWLDocument::Workflow(_) => Self::Workflow,
        }
    }
}

#[tauri::command]
pub fn cwl_doc_type(path: String) -> Result<CWLDocType, String> {
    Ok(load_cwl_file(path, false)
        .map_err(|e| e.to_string())?
        .into())
}
