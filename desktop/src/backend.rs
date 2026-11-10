use mdict_rs::{dictionary::Dictionary, settings::Settings};
use serde::Serialize;
use std::{
    collections::{HashMap, HashSet},
    sync::{Arc, Mutex, RwLock},
};
use tauri::{Manager, State};
use tauri_plugin_dialog::DialogExt;

pub struct LoadedDictionary {
    pub id: String,
    pub path: String,
    pub stylesheets: Vec<String>,
    pub dictionary: Arc<dyn Dictionary>,
}

#[derive(Default)]
pub struct Backend {
    pub dictionaries: RwLock<Vec<LoadedDictionary>>,
    pub progress: Mutex<Progress>,
    pub mutation: Mutex<()>,
}

#[derive(Default, Clone, Serialize)]
pub struct Progress {
    pub busy: bool,
    pub done: usize,
    pub total: usize,
    pub current: String,
    pub errors: Vec<String>,
    pub generation: u64,
}

#[derive(Serialize)]
pub struct Hit {
    id: String,
    name: String,
    short_name: String,
}

pub fn refresh(app: tauri::AppHandle) {
    let state = app.state::<Backend>();
    let mut progress = state.progress.lock().unwrap();
    if progress.busy {
        return;
    }
    progress.busy = true;
    progress.errors.clear();
    drop(progress);
    tauri::async_runtime::spawn_blocking(move || {
        let state = app.state::<Backend>();
        let _guard = state.mutation.lock().unwrap();
        let paths = mdict_rs::settings::enabled_mdx();
        {
            let mut p = state.progress.lock().unwrap();
            p.total = paths.len();
            p.done = 0;
        }
        let mut loaded = Vec::new();
        for path in paths {
            state.progress.lock().unwrap().current = path.clone();
            if let Some(dictionary) = mdict_rs::formats::detect(&path) {
                match dictionary.build_index(false) {
                    Ok(()) => loaded.push(LoadedDictionary {
                        id: dictionary_id(&path),
                        stylesheets: stylesheet_names(&path, dictionary.as_ref()),
                        path,
                        dictionary: Arc::from(dictionary),
                    }),
                    Err(e) => state
                        .progress
                        .lock()
                        .unwrap()
                        .errors
                        .push(format!("{path}: {e}")),
                }
            }
            state.progress.lock().unwrap().done += 1;
        }
        *state.dictionaries.write().unwrap() = loaded;
        let mut p = state.progress.lock().unwrap();
        p.busy = false;
        p.current.clear();
        p.generation += 1;
    });
}

#[tauri::command]
pub fn status(state: State<Backend>) -> Progress {
    state.progress.lock().unwrap().clone()
}

#[tauri::command]
pub async fn search(app: tauri::AppHandle, query: String) -> Result<Vec<String>, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let state = app.state::<Backend>();
        let dictionaries = state.dictionaries.read().unwrap();
        let mut seen = HashSet::new();
        let mut words = Vec::new();
        for loaded in dictionaries.iter() {
            for word in loaded.dictionary.suggestions(&query, 50) {
                if seen.insert(word.clone()) {
                    words.push(word);
                }
            }
        }
        let exact = query.to_lowercase();
        words.sort_by_key(|word| word != &exact);
        words.truncate(50);
        words
    })
    .await
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn lookup(app: tauri::AppHandle, word: String) -> Result<Vec<Hit>, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let state = app.state::<Backend>();
        state
            .dictionaries
            .read()
            .unwrap()
            .iter()
            .filter_map(|loaded| {
                loaded.dictionary.lookup(&word)?;
                Some(Hit {
                    id: loaded.id.clone(),
                    name: loaded.dictionary.name().into(),
                    short_name: loaded.dictionary.short_name().into(),
                })
            })
            .collect()
    })
    .await
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_settings() -> Result<SettingsView, String> {
    tauri::async_runtime::spawn_blocking(|| {
        let settings = mdict_rs::settings::current();
        let titles = settings
            .dictionaries
            .iter()
            .filter_map(|entry| {
                let dictionary = mdict_rs::formats::detect(&entry.path)?;
                Some((entry.path.clone(), dictionary.name().to_string()))
            })
            .collect();
        SettingsView { settings, titles }
    })
    .await
    .map_err(|e| e.to_string())
}

#[derive(Serialize)]
pub struct SettingsView {
    #[serde(flatten)]
    settings: Settings,
    titles: HashMap<String, String>,
}

#[tauri::command]
pub async fn save_settings(app: tauri::AppHandle, settings: Settings) -> Result<(), String> {
    let handle = app.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let state = handle.state::<Backend>();
        let _guard = state.mutation.lock().unwrap();
        mdict_rs::settings::update(settings).map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())??;
    refresh(app);
    Ok(())
}

#[tauri::command]
pub async fn choose_folder(app: tauri::AppHandle) -> Result<Option<String>, String> {
    tauri::async_runtime::spawn_blocking(move || {
        app.dialog()
            .file()
            .set_title("Choose dictionary folder")
            .blocking_pick_folder()
            .map(|path| {
                path.into_path()
                    .map(|p| p.to_string_lossy().into_owned())
                    .map_err(|e| e.to_string())
            })
            .transpose()
    })
    .await
    .map_err(|e| e.to_string())?
}

// Stable, path-derived identity: reordering dictionaries cannot redirect an
// already displayed frame's resource requests to another dictionary.
fn dictionary_id(path: &str) -> String {
    use sha2::{Digest, Sha256};
    let hash = format!("{:x}", Sha256::digest(path.as_bytes()));
    // DNS-sized labels also work with WebView2's HTTP protocol mapping.
    format!("{}.{}", &hash[..32], &hash[32..])
}

fn stylesheet_names(path: &str, dictionary: &dyn Dictionary) -> Vec<String> {
    let mut names: std::collections::BTreeSet<String> = dictionary
        .css_resources()
        .into_iter()
        .map(|(name, _)| name)
        .collect();
    let path = std::path::Path::new(path);
    if let (Some(parent), Some(stem)) = (path.parent(), path.file_stem().and_then(|s| s.to_str()))
        && let Ok(entries) = std::fs::read_dir(parent)
    {
        for entry in entries.flatten() {
            let p = entry.path();
            if p.extension()
                .is_some_and(|ext| ext.eq_ignore_ascii_case("css"))
                && p.file_stem()
                    .and_then(|s| s.to_str())
                    .is_some_and(|s| s == stem || s.starts_with(&format!("{stem}_")))
            {
                names.insert(entry.file_name().to_string_lossy().into_owned());
            }
        }
    }
    names.into_iter().collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dictionary_identity_depends_on_full_path() {
        assert_ne!(
            dictionary_id("/one/Example.mdx"),
            dictionary_id("/two/Example.mdx")
        );
        assert!(
            dictionary_id("/词典/a.mdx")
                .bytes()
                .all(|b| b.is_ascii_hexdigit() || b == b'.')
        );
    }
}
