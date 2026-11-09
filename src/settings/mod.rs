//! User-editable settings persisted to `~/.config/dicto/settings.toml`.
//!
//! Tracks the list of MDX dictionaries (path + enabled flag) in the order
//! the user wants them queried, including files selected outside the managed
//! directory. Newly discovered managed files are appended; missing files
//! are dropped when settings are reconciled.

use std::fs;
use std::path::PathBuf;
use std::sync::LazyLock;
use std::sync::RwLock;

use serde::{Deserialize, Serialize};
use tracing::{info, warn};

use crate::config::{APP_NAME, discover_mdx_files};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Settings {
    #[serde(default)]
    pub dictionaries: Vec<DictEntry>,
    /// User-selected folders, scanned recursively on launch and settings updates.
    #[serde(default)]
    pub dictionary_directories: Vec<String>,
    /// Telemetry opt-in state. Stored here so it persists in settings.toml
    /// without coupling this crate to any analytics dependency.
    #[serde(default)]
    pub telemetry_consent: TelemetryConsent,
    /// Anonymous installation id (UUID v4), generated once on first launch.
    /// Local-only; identifies an installation, never a person.
    #[serde(default)]
    pub installation_id: Option<String>,
}

/// Three-state telemetry consent, persisted in settings.toml.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TelemetryConsent {
    /// Never shown the consent prompt yet → app shows it on launch.
    #[default]
    Undecided,
    /// User opted in → anonymous usage events are sent.
    OptedIn,
    /// User declined → nothing is sent, ever, until they re-enable.
    OptedOut,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DictEntry {
    pub path: String,
    #[serde(default = "default_enabled")]
    pub enabled: bool,
    /// User-chosen short name for tab titles. Empty string means "auto-generate".
    #[serde(default)]
    pub short_name: String,
}

fn default_enabled() -> bool {
    true
}

pub fn settings_path() -> PathBuf {
    let base = dirs::config_dir().unwrap_or_else(|| PathBuf::from(".config"));
    base.join(APP_NAME).join("settings.toml")
}

fn load_from_disk() -> Settings {
    let path = settings_path();
    if !path.exists() {
        return Settings::default();
    }
    match fs::read_to_string(&path) {
        Ok(s) => match toml::from_str(&s) {
            Ok(parsed) => parsed,
            Err(e) => {
                warn!("settings: parse failed ({e}); using defaults");
                Settings::default()
            }
        },
        Err(e) => {
            warn!("settings: read failed ({e}); using defaults");
            Settings::default()
        }
    }
}

/// Reconcile a stored settings list with what's currently on disk:
/// new files get appended (enabled by default), missing files dropped.
fn merge_with_disk(s: Settings) -> Settings {
    let mut on_disk = discover_mdx_files();
    for root in &s.dictionary_directories {
        match crate::config::mdx_files_in_directory(std::path::Path::new(root)) {
            Ok(paths) => {
                on_disk.extend(paths.into_iter().map(|p| p.to_string_lossy().into_owned()))
            }
            Err(e) => warn!("cannot scan dictionary folder {root}: {e}"),
        }
    }
    merge_discovered(s, on_disk)
}

fn merge_discovered(mut s: Settings, on_disk: Vec<String>) -> Settings {
    // Explicitly selected dictionaries may live outside the managed directory.
    s.dictionaries.retain(|d| PathBuf::from(&d.path).is_file());

    let mut existing: std::collections::HashSet<String> =
        s.dictionaries.iter().map(|d| d.path.clone()).collect();
    for path in on_disk {
        if !existing.contains(&path) {
            s.dictionaries.push(DictEntry {
                path: path.clone(),
                enabled: true,
                short_name: String::new(),
            });
            existing.insert(path);
        }
    }
    s
}

static SETTINGS: LazyLock<RwLock<Settings>> = LazyLock::new(|| {
    let merged = merge_with_disk(load_from_disk());
    if let Err(e) = save(&merged) {
        warn!("settings: initial save failed: {e}");
    }
    RwLock::new(merged)
});

pub fn current() -> Settings {
    SETTINGS.read().unwrap().clone()
}

/// Replace the settings on disk and in memory. Caller is responsible for
/// telling the rest of the system to react (re-index, rebuild pools).
pub fn update(new: Settings) -> anyhow::Result<()> {
    let cleaned = merge_with_disk(new);
    save(&cleaned)?;
    *SETTINGS.write().unwrap() = cleaned;
    Ok(())
}

/// Persist only the telemetry consent, preserving every other field.
/// Returns the newly saved settings so the caller can re-init telemetry
/// without a separate read.
pub fn update_consent(consent: TelemetryConsent) -> anyhow::Result<Settings> {
    let mut current = current();
    current.telemetry_consent = consent;
    // Bypass merge_with_disk — consent has nothing to do with the dict list,
    // and merging could reorder/drop entries based on disk state.
    save(&current)?;
    *SETTINGS.write().unwrap() = current.clone();
    Ok(current)
}

/// Generate + persist the installation id if none exists yet. Returns the id
/// (newly generated or existing). Cheap to call every launch — it's a no-op
/// once an id is present.
pub fn ensure_installation_id() -> anyhow::Result<String> {
    let mut current = current();
    if let Some(id) = current.installation_id.clone() {
        return Ok(id);
    }
    let id = format_uuid_v4();
    current.installation_id = Some(id.clone());
    save(&current)?;
    *SETTINGS.write().unwrap() = current;
    Ok(id)
}

/// Minimal RFC 4122 v4 UUID generator without an extra dependency: 16 random
/// bytes with the v4 and variant bits set, formatted as 8-4-4-4-12.
fn format_uuid_v4() -> String {
    use rand::Rng;
    let mut rng = rand::rng();
    let mut bytes = [0u8; 16];
    rng.fill(&mut bytes);
    // Version 4
    bytes[6] = (bytes[6] & 0x0F) | 0x40;
    // Variant (RFC 4122)
    bytes[8] = (bytes[8] & 0x3F) | 0x80;
    let b = bytes;
    format!(
        "{:02x}{:02x}{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}",
        b[0],
        b[1],
        b[2],
        b[3],
        b[4],
        b[5],
        b[6],
        b[7],
        b[8],
        b[9],
        b[10],
        b[11],
        b[12],
        b[13],
        b[14],
        b[15]
    )
}

fn save(s: &Settings) -> anyhow::Result<()> {
    let path = settings_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let body = toml::to_string_pretty(s)?;
    fs::write(&path, body)?;
    info!(
        "settings: saved {} entries to {}",
        s.dictionaries.len(),
        path.display()
    );
    Ok(())
}

/// MDX paths the user wants queried, in display order.
pub fn enabled_mdx() -> Vec<String> {
    current()
        .dictionaries
        .into_iter()
        .filter(|d| d.enabled)
        .map(|d| d.path)
        .collect()
}

/// Look up a user-specified short_name override for the given MDX path.
/// Returns `None` if no override is set (meaning auto-generate).
pub fn short_name_override(mdx_path: &str) -> Option<String> {
    let s = current();
    s.dictionaries
        .iter()
        .find(|d| d.path == mdx_path)
        .and_then(|d| {
            if d.short_name.is_empty() {
                None
            } else {
                Some(d.short_name.clone())
            }
        })
}

/// MDD paths corresponding to enabled MDX entries (matched by stem).
pub fn enabled_mdd() -> Vec<String> {
    enabled_mdx()
        .into_iter()
        .map(|mdx| {
            let p = PathBuf::from(&mdx);
            p.with_extension("mdd").to_string_lossy().to_string()
        })
        .filter(|p| PathBuf::from(p).exists())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn external_paths_survive_settings_roundtrip_and_discovery() {
        let root = std::env::temp_dir().join(format!("dicto-settings-{}", format_uuid_v4()));
        let external = root.join("external");
        let managed = root.join("managed");
        fs::create_dir_all(&external).unwrap();
        fs::create_dir_all(&managed).unwrap();
        let external_path = external.join("dictionary.mdx");
        let managed_path = managed.join("dictionary.mdx");
        fs::write(&external_path, b"source dictionary").unwrap();
        fs::write(&managed_path, b"managed dictionary").unwrap();
        let external_path = external_path.to_string_lossy().into_owned();
        let managed_path = managed_path.to_string_lossy().into_owned();
        let mut settings = Settings::default();
        settings.dictionaries.push(DictEntry {
            path: external_path.clone(),
            enabled: false,
            short_name: "Custom".into(),
        });
        // Simulate loading saved external paths on the next launch.
        let loaded = toml::from_str(&toml::to_string(&settings).unwrap()).unwrap();
        let merged = merge_discovered(loaded, vec![managed_path.clone()]);
        assert_eq!(merged.dictionaries.len(), 2);
        assert_eq!(merged.dictionaries[0].path, external_path);
        assert!(!merged.dictionaries[0].enabled);
        assert_eq!(merged.dictionaries[0].short_name, "Custom");
        assert_eq!(merged.dictionaries[1].path, managed_path);
        let merged = merge_discovered(merged, vec![managed_path]);
        assert_eq!(merged.dictionaries.len(), 2);
        fs::remove_file(&external_path).unwrap();
        let merged = merge_discovered(merged, vec![]);
        assert_eq!(merged.dictionaries.len(), 1);
        fs::remove_dir_all(root).unwrap();
    }
}
