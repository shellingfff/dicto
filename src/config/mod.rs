use std::path::PathBuf;

use tracing::{info, warn};

pub const APP_NAME: &str = "dicto";

// ── directory helpers ─────────────────────────────────────────────────────────

pub fn discover_mdx_files() -> Vec<String> {
    let config_dir = dirs_config_path();
    let mut files = Vec::new();

    if config_dir.exists() {
        info!("scanning for MDX files in {}", config_dir.display());
        collect_files_with_ext(&config_dir, "mdx", &mut files);
    }

    let fallback = PathBuf::from("./resources/mdx");
    if fallback.exists() && !config_dir.exists() {
        info!("scanning for MDX files in {}", fallback.display());
        collect_files_with_ext(&fallback, "mdx", &mut files);
    }

    if files.is_empty() {
        warn!("no MDX files found in {}", config_dir.display());
    } else {
        info!("found {} MDX file(s)", files.len());
        for f in &files {
            info!("  - {f}");
        }
    }
    files
}

pub fn discover_mdd_files() -> Vec<String> {
    let config_dir = dirs_config_path();
    let mut files = Vec::new();
    if config_dir.exists() {
        collect_files_with_ext(&config_dir, "mdd", &mut files);
    }
    let fallback = PathBuf::from("./resources/mdx");
    if fallback.exists() && !config_dir.exists() {
        collect_files_with_ext(&fallback, "mdd", &mut files);
    }
    if !files.is_empty() {
        info!("found {} MDD file(s)", files.len());
    }
    files
}

fn collect_files_with_ext(dir: &PathBuf, ext: &str, out: &mut Vec<String>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_files_with_ext(&path, ext, out);
        } else if path
            .extension()
            .map(|e| e.eq_ignore_ascii_case(ext))
            .unwrap_or(false)
            && let Some(s) = path.to_str()
        {
            out.push(s.to_string());
        }
    }
}

/// Recursively find MDX files, following directory links without visiting cycles.
/// Canonical paths deduplicate overlapping roots and symbolic links.
pub fn mdx_files_in_directory(root: &std::path::Path) -> std::io::Result<Vec<PathBuf>> {
    let mut pending = vec![root.canonicalize()?];
    let mut visited = std::collections::HashSet::new();
    let mut files = std::collections::BTreeSet::new();
    while let Some(dir) = pending.pop() {
        if !visited.insert(dir.clone()) {
            continue;
        }
        for entry in std::fs::read_dir(&dir)? {
            let path = entry?.path();
            let path = match path.canonicalize() {
                Ok(path) => path,
                // Ignore broken links.
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => continue,
                Err(e) => return Err(e),
            };
            if path.is_dir() {
                pending.push(path);
            } else if path.is_file()
                && path
                    .extension()
                    .is_some_and(|e| e.eq_ignore_ascii_case("mdx"))
            {
                files.insert(path);
            }
        }
    }
    Ok(files.into_iter().collect())
}

pub fn dirs_config_path() -> PathBuf {
    let base = dirs::config_dir().unwrap_or_else(|| PathBuf::from(".config"));
    base.join(APP_NAME).join("dicts")
}

pub fn static_path() -> anyhow::Result<PathBuf> {
    Ok(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("resources/static"))
}

/// No-op after FST migration; cache invalidation happens via registry::reload().
pub fn reset_pools() {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recursive_scan_finds_only_dictionaries_and_avoids_cycles() {
        let root = std::env::temp_dir().join(format!(
            "dicto-scan-{}-{}",
            std::process::id(),
            rand::random::<u64>()
        ));
        let nested = root.join("nested/deeper");
        std::fs::create_dir_all(&nested).unwrap();
        std::fs::write(root.join("first.mdx"), b"first").unwrap();
        std::fs::write(nested.join("second.MDX"), b"second").unwrap();
        std::fs::write(nested.join("second.mdd"), b"resource").unwrap();
        std::fs::write(nested.join("second.mdx.fst"), b"index").unwrap();
        #[cfg(unix)]
        {
            std::os::unix::fs::symlink(&root, nested.join("loop")).unwrap();
            std::os::unix::fs::symlink(root.join("first.mdx"), root.join("alias.mdx")).unwrap();
        }
        let found = mdx_files_in_directory(&root).unwrap();
        assert_eq!(found.len(), 2);
        assert!(found.contains(&root.join("first.mdx").canonicalize().unwrap()));
        assert!(found.contains(&nested.join("second.MDX").canonicalize().unwrap()));
        assert_eq!(std::fs::read(root.join("first.mdx")).unwrap(), b"first");
        std::fs::remove_dir_all(root).unwrap();
    }
}
