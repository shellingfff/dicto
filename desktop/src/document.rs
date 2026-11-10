use crate::backend::Backend;
use std::path::{Component, Path};
use tauri::http::{Response, Uri};

pub fn respond(state: &Backend, uri: &Uri) -> Response<Vec<u8>> {
    match content(state, uri) {
        Some((body, mime)) => Response::builder().header("Content-Type", mime)
            .header("Access-Control-Allow-Origin", "*")
            .header("Cache-Control", "no-store")
            .header("Referrer-Policy", "same-origin")
            .header("Content-Security-Policy", "default-src 'none'; script-src 'self' 'unsafe-inline'; style-src 'self' 'unsafe-inline'; img-src 'self' data:; font-src 'self' data:; media-src 'self' blob:; connect-src 'self'")
            .body(body).unwrap(),
        None => Response::builder().status(404).body(b"Dictionary resource not found".to_vec()).unwrap(),
    }
}

fn content(state: &Backend, uri: &Uri) -> Option<(Vec<u8>, String)> {
    let host = uri.host()?.strip_suffix(".localhost")?;
    let id = host.strip_prefix("dict.").unwrap_or(host);
    let dictionaries = state.dictionaries.read().ok()?;
    let loaded = dictionaries.iter().find(|dictionary| dictionary.id == id)?;
    let resource = uri.path().trim_start_matches('/');
    let resource = percent_encoding::percent_decode_str(resource)
        .decode_utf8()
        .ok()?
        .replace('\\', "/");
    if resource == "__entry" {
        let word = uri.query()?.strip_prefix("word=")?;
        let word = percent_encoding::percent_decode_str(word)
            .decode_utf8()
            .ok()?;
        let html = loaded.dictionary.lookup(&word)?;
        let header_css = mdict_rs::formats::mdict::mdx_header_stylesheet(&loaded.path);
        let stylesheet_links: String = loaded
            .stylesheets
            .iter()
            .map(|path| {
                let encoded = path
                    .replace('\\', "/")
                    .trim_start_matches('/')
                    .split('/')
                    .map(|part| {
                        percent_encoding::utf8_percent_encode(
                            part,
                            percent_encoding::NON_ALPHANUMERIC,
                        )
                        .to_string()
                    })
                    .collect::<Vec<_>>()
                    .join("/");
                format!("<link data-dicto-fallback rel=\"stylesheet\" href=\"{encoded}\">")
            })
            .collect();
        let html = format!(
            "<!doctype html><html><head><meta charset=\"utf-8\"><style>body{{margin:16px;font:16px/1.5 sans-serif;color:#222;background:white}}img{{max-width:100%}}{header_css}</style>{stylesheet_links}<script>{}</script></head><body>{html}</body></html>",
            include_str!("../../frontend/definition.js")
        );
        return Some((html.into_bytes(), "text/html; charset=utf-8".into()));
    }
    let relative = Path::new(resource.trim_start_matches('/'));
    if relative
        .components()
        .any(|c| !matches!(c, Component::Normal(_) | Component::CurDir))
    {
        return None;
    }
    let root = Path::new(&loaded.path).parent()?;
    let bytes =
        read_local_resource(root, relative).or_else(|| loaded.dictionary.resource(&resource))?;
    Some((
        bytes,
        mime_guess::from_path(relative)
            .first_or_octet_stream()
            .to_string(),
    ))
}

fn read_local_resource(root: &Path, relative: &Path) -> Option<Vec<u8>> {
    if relative
        .components()
        .any(|c| !matches!(c, Component::Normal(_) | Component::CurDir))
    {
        return None;
    }
    let root = root.canonicalize().ok()?;
    let path = root.join(relative).canonicalize().ok()?;
    if !path.starts_with(&root) {
        return None;
    }
    std::fs::read(path).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn local_resources_cannot_escape_dictionary_directory() {
        let base = std::env::temp_dir().join(format!("dicto-resource-test-{}", std::process::id()));
        let root = base.join("dictionary");
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(root.join("style.css"), b"body{}").unwrap();
        std::fs::write(base.join("outside"), b"private").unwrap();
        assert_eq!(
            read_local_resource(&root, Path::new("style.css")),
            Some(b"body{}".to_vec())
        );
        assert!(read_local_resource(&root, Path::new("../outside")).is_none());
        assert!(read_local_resource(&root, &base.join("outside")).is_none());
        #[cfg(unix)]
        {
            std::os::unix::fs::symlink(base.join("outside"), root.join("escape")).unwrap();
            assert!(read_local_resource(&root, Path::new("escape")).is_none());
        }
        std::fs::remove_dir_all(base).unwrap();
    }
}
