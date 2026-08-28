use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct LayoutPosition {
    pub x: f32,
    pub y: f32,
}

/// `.sciwin/layout/<relative-cwl-path>.json`, mirroring the CWL file's own
/// location under the project root
fn layout_file_path(project_root: &Path, workflow_path: &Path) -> Result<PathBuf, String> {
    let relative = workflow_path.strip_prefix(project_root).map_err(|_| {
        format!(
            "{} is not inside project root {}",
            workflow_path.display(),
            project_root.display()
        )
    })?;
    let mut layout_path = project_root
        .join(".sciwin")
        .join("layout")
        .join(relative)
        .into_os_string();
    layout_path.push(".json");
    Ok(PathBuf::from(layout_path))
}

/// Absent file means no saved layout yet -- any CLI-made workflow, or one
/// never dragged in the GUI -- so the frontend falls back to dagre for those
/// nodes rather than treating it as an error.
#[tauri::command]
pub fn get_node_layout(
    project_root: String,
    path: String,
) -> Result<HashMap<String, LayoutPosition>, String> {
    let layout_path = layout_file_path(Path::new(&project_root), Path::new(&path))?;
    match std::fs::read(&layout_path) {
        Ok(bytes) => serde_json::from_slice(&bytes).map_err(|e| e.to_string()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(HashMap::new()),
        Err(e) => Err(e.to_string()),
    }
}

#[tauri::command]
pub fn save_node_layout(
    project_root: String,
    path: String,
    positions: HashMap<String, LayoutPosition>,
) -> Result<(), String> {
    let layout_path = layout_file_path(Path::new(&project_root), Path::new(&path))?;
    if let Some(parent) = layout_path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let json = serde_json::to_string_pretty(&positions).map_err(|e| e.to_string())?;
    std::fs::write(&layout_path, json).map_err(|e| e.to_string())
}

/// Deletes the sidecar outright rather than writing `{}`
#[tauri::command]
pub fn reset_node_layout(project_root: String, path: String) -> Result<(), String> {
    let layout_path = layout_file_path(Path::new(&project_root), Path::new(&path))?;
    match std::fs::remove_file(&layout_path) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(e.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn layout_path_mirrors_relative_workflow_location() {
        let dir = tempdir().unwrap();
        let root = dir.path();
        let workflow = root
            .join("workflows")
            .join("scan-image")
            .join("workflow.cwl");

        let layout = layout_file_path(root, &workflow).unwrap();

        assert_eq!(
            layout,
            root.join(".sciwin")
                .join("layout")
                .join("workflows")
                .join("scan-image")
                .join("workflow.cwl.json")
        );
    }

    #[test]
    fn layout_path_refuses_a_workflow_outside_the_project_root() {
        let dir = tempdir().unwrap();
        let other = tempdir().unwrap();
        let workflow = other.path().join("workflow.cwl");

        assert!(layout_file_path(dir.path(), &workflow).is_err());
    }

    #[test]
    fn round_trips_saved_positions() {
        let dir = tempdir().unwrap();
        let project_root = dir.path().to_string_lossy().into_owned();
        let workflow_path = dir.path().join("workflows").join("main.cwl");
        std::fs::create_dir_all(workflow_path.parent().unwrap()).unwrap();
        let workflow_path = workflow_path.to_string_lossy().into_owned();

        let mut positions = HashMap::new();
        positions.insert("step/plot".to_string(), LayoutPosition { x: 12.5, y: -4.0 });

        save_node_layout(project_root.clone(), workflow_path.clone(), positions).unwrap();
        let loaded = get_node_layout(project_root, workflow_path).unwrap();

        let plot = loaded
            .get("step/plot")
            .expect("saved position must round-trip");
        assert_eq!(plot.x, 12.5);
        assert_eq!(plot.y, -4.0);
    }

    #[test]
    fn missing_layout_file_returns_an_empty_map_not_an_error() {
        let dir = tempdir().unwrap();
        let project_root = dir.path().to_string_lossy().into_owned();
        let workflow_path = dir
            .path()
            .join("workflows")
            .join("main.cwl")
            .to_string_lossy()
            .into_owned();

        let loaded = get_node_layout(project_root, workflow_path).unwrap();

        assert!(loaded.is_empty());
    }

    #[test]
    fn save_creates_missing_parent_directories() {
        let dir = tempdir().unwrap();
        let project_root = dir.path().to_string_lossy().into_owned();
        let workflow_path = dir
            .path()
            .join("workflows")
            .join("nested")
            .join("workflow.cwl")
            .to_string_lossy()
            .into_owned();

        let mut positions = HashMap::new();
        positions.insert("input/x".to_string(), LayoutPosition { x: 0.0, y: 0.0 });

        save_node_layout(project_root.clone(), workflow_path.clone(), positions).unwrap();
        let loaded = get_node_layout(project_root, workflow_path).unwrap();

        assert!(loaded.contains_key("input/x"));
    }

    #[test]
    fn reset_deletes_the_sidecar_file() {
        let dir = tempdir().unwrap();
        let project_root = dir.path().to_string_lossy().into_owned();
        let workflow_path = dir.path().join("workflows").join("main.cwl");
        std::fs::create_dir_all(workflow_path.parent().unwrap()).unwrap();
        let workflow_path = workflow_path.to_string_lossy().into_owned();

        let mut positions = HashMap::new();
        positions.insert("step/plot".to_string(), LayoutPosition { x: 1.0, y: 2.0 });
        save_node_layout(project_root.clone(), workflow_path.clone(), positions).unwrap();

        let layout_file = layout_file_path(
            Path::new(&project_root),
            &dir.path().join("workflows").join("main.cwl"),
        )
        .unwrap();
        assert!(
            layout_file.exists(),
            "precondition: sidecar must exist before reset"
        );

        reset_node_layout(project_root.clone(), workflow_path.clone()).unwrap();

        assert!(!layout_file.exists());
        assert!(
            get_node_layout(project_root, workflow_path)
                .unwrap()
                .is_empty()
        );
    }

    #[test]
    fn reset_on_a_never_saved_workflow_is_not_an_error() {
        let dir = tempdir().unwrap();
        let project_root = dir.path().to_string_lossy().into_owned();
        let workflow_path = dir
            .path()
            .join("workflows")
            .join("main.cwl")
            .to_string_lossy()
            .into_owned();

        assert!(reset_node_layout(project_root, workflow_path).is_ok());
    }
}
