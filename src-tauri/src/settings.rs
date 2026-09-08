//! App-wide execution settings (which container engine `Local` uses, the shared S3 storage
//! config, and the list of remote TES/REANA backend instances) -- persisted encrypted-at-rest
//! via `securestore`, in keyfile mode: a randomly generated key sits next to the vault in the
//! app's data dir, so opening `settings.vault` directly (a text editor, an accidental
//! screen-share, a stray backup) doesn't reveal a REANA token or S3 secret key in plain text.
//! It's not protection against someone with access to that whole directory -- there's no
//! master password to unlock, by design, so the app never has to ask for one at launch.
//!
//! Unlike `session.rs` (an opaque blob the frontend owns the shape of), this struct is owned
//! here: `execution.rs`'s backend registry needs typed access to build a runner from it.

use securestore::{KeySource, SecretsManager};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use tauri::{AppHandle, Manager, State};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ContainerEngineChoice {
    Docker,
    Podman,
    Singularity,
    Apptainer,
}

/// One process-wide S3 config, shared by every backend that ends up touching an `s3://` URL --
/// TES requires this be set (it has no local storage option), Local/Docker only need it if a
/// workflow happens to reference an `s3://` input/output directly.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct S3Settings {
    pub endpoint_url: String,
    pub access_key_id: String,
    pub secret_access_key: String,
    #[serde(default)]
    pub region: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum RemoteBackendKind {
    Tes {
        url: String,
        /// Just the bucket (and optional key prefix), e.g. `"my-bucket"` or
        /// `"my-bucket/prefix"` -- turned into an `s3://...` storage URL when building the
        /// backend. The actual S3 credentials for it come from `Settings::s3`, not here.
        bucket: String,
        token: Option<String>,
    },
    Reana {
        url: String,
        token: String,
    },
}

impl RemoteBackendKind {
    /// A REANA server's API lives under `/api` (e.g. `https://reana.example.com/api`) --
    /// `reana::api::client::ReanaClient` sends requests straight to whatever URL it's given
    /// (only normalizing a trailing slash), so a URL entered without that suffix would 404
    /// with no clearer signal than "REANA failed to start". Append it here if it's missing,
    /// once, at the point the URL is saved -- so what a caller reads back out is always
    /// already correct, and editing the entry later shows the URL that's actually in use.
    fn normalize(&mut self) {
        if let Self::Reana { url, .. } = self {
            let trimmed = url.trim().trim_end_matches('/');
            if !trimmed.ends_with("/api") {
                *url = format!("{trimmed}/api");
            } else {
                *url = trimmed.to_string();
            }
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemoteBackend {
    pub id: String,
    pub name: String,
    #[serde(flatten)]
    pub kind: RemoteBackendKind,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Settings {
    /// `None` = auto-detect (first of docker/podman/apptainer/singularity found on `PATH`).
    #[serde(default)]
    pub local_container_engine: Option<ContainerEngineChoice>,
    #[serde(default)]
    pub s3: Option<S3Settings>,
    #[serde(default)]
    pub remote_backends: Vec<RemoteBackend>,
}

const SECRET_KEY: &str = "settings";

pub struct SettingsState {
    manager: Mutex<SecretsManager>,
    vault_path: PathBuf,
}

impl SettingsState {
    /// Loads the existing vault, or creates a fresh one if this is the first run -- or if the
    /// existing one turns out to be unreadable (corrupted, or its keyfile went missing), in
    /// which case whatever was in it is lost rather than blocking the app from starting.
    pub fn init(app: &AppHandle) -> Self {
        let dir = app
            .path()
            .app_data_dir()
            .unwrap_or_else(|_| PathBuf::from("."));
        let _ = fs::create_dir_all(&dir);
        let vault_path = dir.join("settings.vault");
        let keyfile_path = dir.join("settings.key");

        let manager = Self::open(&vault_path, &keyfile_path).unwrap_or_else(|e| {
            eprintln!(
                "settings vault unreadable ({e}), starting a fresh one -- previous settings, if any, are lost"
            );
            Self::create_fresh(&vault_path, &keyfile_path)
                .expect("could not create a settings vault in the app data directory")
        });

        Self {
            manager: Mutex::new(manager),
            vault_path,
        }
    }

    fn open(vault_path: &Path, keyfile_path: &Path) -> Result<SecretsManager, String> {
        if vault_path.exists() && keyfile_path.exists() {
            SecretsManager::load(vault_path, KeySource::Path(keyfile_path)).map_err(|e| e.to_string())
        } else {
            Self::create_fresh(vault_path, keyfile_path)
        }
    }

    fn create_fresh(vault_path: &Path, keyfile_path: &Path) -> Result<SecretsManager, String> {
        let manager = SecretsManager::new(KeySource::Csprng).map_err(|e| e.to_string())?;
        manager
            .export_key(keyfile_path)
            .map_err(|e| e.to_string())?;
        manager.save_as(vault_path).map_err(|e| e.to_string())?;
        Ok(manager)
    }

    pub fn get(&self) -> Settings {
        let manager = self.manager.lock().unwrap();
        manager
            .get(SECRET_KEY)
            .ok()
            .and_then(|json| serde_json::from_str(&json).ok())
            .unwrap_or_default()
    }

    pub fn set(&self, settings: &Settings) -> Result<(), String> {
        let json = serde_json::to_string(settings).map_err(|e| e.to_string())?;
        let mut manager = self.manager.lock().unwrap();
        manager.set(SECRET_KEY, json);
        manager.save_as(&self.vault_path).map_err(|e| e.to_string())
    }
}

#[tauri::command]
pub fn get_settings(state: State<'_, SettingsState>) -> Settings {
    state.get()
}

#[tauri::command]
pub fn set_local_container_engine(
    state: State<'_, SettingsState>,
    engine: Option<ContainerEngineChoice>,
) -> Result<Settings, String> {
    let mut settings = state.get();
    settings.local_container_engine = engine;
    state.set(&settings)?;
    Ok(settings)
}

#[tauri::command]
pub fn set_s3_settings(state: State<'_, SettingsState>, s3: Option<S3Settings>) -> Result<Settings, String> {
    let mut settings = state.get();
    settings.s3 = s3;
    state.set(&settings)?;
    Ok(settings)
}

#[tauri::command]
pub fn add_remote_backend(
    state: State<'_, SettingsState>,
    name: String,
    mut kind: RemoteBackendKind,
) -> Result<Settings, String> {
    kind.normalize();
    let mut settings = state.get();
    settings.remote_backends.push(RemoteBackend {
        id: uuid::Uuid::new_v4().to_string(),
        name,
        kind,
    });
    state.set(&settings)?;
    Ok(settings)
}

#[tauri::command]
pub fn update_remote_backend(
    state: State<'_, SettingsState>,
    id: String,
    name: String,
    mut kind: RemoteBackendKind,
) -> Result<Settings, String> {
    kind.normalize();
    let mut settings = state.get();
    let Some(backend) = settings.remote_backends.iter_mut().find(|b| b.id == id) else {
        return Err("no backend with that id".to_string());
    };
    backend.name = name;
    backend.kind = kind;
    state.set(&settings)?;
    Ok(settings)
}

#[tauri::command]
pub fn remove_remote_backend(state: State<'_, SettingsState>, id: String) -> Result<Settings, String> {
    let mut settings = state.get();
    settings.remote_backends.retain(|b| b.id != id);
    state.set(&settings)?;
    Ok(settings)
}
