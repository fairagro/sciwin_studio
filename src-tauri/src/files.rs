use serde::Serialize;
use std::fs;
use std::path::{Path, PathBuf};

const SKIP_DIRS: &[&str] = &[".git", "node_modules", "target", "__pycache__", ".sciwin"];

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct FsEntry {
    name: String,
    path: String,
    is_dir: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    children: Option<Vec<FsEntry>>,
}

fn read_sorted_dir(dir: &Path) -> std::io::Result<Vec<(String, PathBuf, bool)>> {
    let mut entries: Vec<(String, PathBuf, bool)> = fs::read_dir(dir)?
        .filter_map(|e| e.ok())
        .filter_map(|e| {
            let path = e.path();
            let name = e.file_name().to_string_lossy().into_owned();
            let is_dir = path.is_dir();
            if is_dir && SKIP_DIRS.contains(&name.as_str()) {
                return None;
            }
            Some((name, path, is_dir))
        })
        .collect();

    entries.sort_by(|a, b| match (a.2, b.2) {
        (true, false) => std::cmp::Ordering::Less,
        (false, true) => std::cmp::Ordering::Greater,
        _ => a.0.to_lowercase().cmp(&b.0.to_lowercase()),
    });

    Ok(entries)
}

/// Lists one directory level. Used by the Filesystem view, which expands lazily.
#[tauri::command]
pub fn list_dir(path: String) -> Result<Vec<FsEntry>, String> {
    let entries = read_sorted_dir(&PathBuf::from(path)).map_err(|e| e.to_string())?;
    Ok(entries
        .into_iter()
        .map(|(name, path, is_dir)| FsEntry {
            name,
            path: path.to_string_lossy().into_owned(),
            is_dir,
            children: None,
        })
        .collect())
}

fn walk_cwl(dir: &Path) -> std::io::Result<Vec<FsEntry>> {
    let entries = read_sorted_dir(dir)?;
    let mut out = Vec::new();

    for (name, path, is_dir) in entries {
        if is_dir {
            let children = walk_cwl(&path)?;
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

    Ok(out)
}

/// Recursively finds every .cwl file, pruning directories that don't contain one.
/// Used by the Workflows view.
#[tauri::command]
pub fn get_cwl_files(root: String) -> Result<Vec<FsEntry>, String> {
    walk_cwl(&PathBuf::from(root)).map_err(|e| e.to_string())
}
