use ignore::WalkBuilder;
use serde::Serialize;
use std::path::{Path, PathBuf};

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
    std::fs::read_to_string(path).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn write_file(path: String, contents: String) -> Result<(), String> {
    std::fs::write(path, contents).map_err(|e| e.to_string())
}
