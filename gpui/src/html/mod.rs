//! Lightweight HTML rendering for MDX definition content.
//!
//! MDX definitions arrive as small fragments of HTML. We tokenize them
//! into a flat stream of structural events, fold those into a list of
//! block + inline nodes, then turn the nodes into a GPUI element tree.
//! Inline images and sounds are pulled out of the companion MDD file.

pub mod css;
pub mod parser;
pub mod render;

use std::collections::HashMap;
use std::sync::{LazyLock, RwLock};

pub use css::Stylesheet;
pub use parser::{Block, parse_with_styles};
pub use render::render_blocks;

/// Per-dictionary stylesheets, keyed by dictionary name (file stem).
/// Loaded at startup; each entry holds the merged CSS for the
/// directory containing that dictionary's `.mdx` file. This keeps
/// rules from different dictionaries from cross-contaminating each
/// other — e.g. an LSC4 rule like `.hw { display: none }` would
/// otherwise wipe out the Cambridge headword class with the same
/// name but different semantics.
pub static STYLESHEETS: LazyLock<RwLock<HashMap<String, Stylesheet>>> =
    LazyLock::new(|| RwLock::new(HashMap::new()));

/// Parse HTML against the stylesheet registered for `dict_name`.
/// Falls back to an empty stylesheet if none is registered.
pub fn parse_styled(html: &str, dict_name: &str) -> Vec<Block> {
    let mut sheet = STYLESHEETS
        .read()
        .unwrap()
        .get(dict_name)
        .cloned()
        .unwrap_or_default();
    for href in parser::linked_stylesheets(html) {
        if let Some(linked) = load_linked_stylesheet(dict_name, &href) {
            sheet.extend(linked);
        }
    }
    parse_with_styles(html, &sheet)
}

static LINKED_STYLES: LazyLock<RwLock<HashMap<(String, String), Stylesheet>>> =
    LazyLock::new(|| RwLock::new(HashMap::new()));

fn load_linked_stylesheet(dict_name: &str, href: &str) -> Option<Stylesheet> {
    let path = href.trim().split(['?', '#']).next()?.replace('\\', "/");
    if path.is_empty() || path.contains(':') || path.starts_with("//") {
        return None;
    }
    let key = (dict_name.to_string(), path.clone());
    if let Some(sheet) = LINKED_STYLES.read().unwrap().get(&key) {
        return Some(sheet.clone());
    }
    let settings = mdict_rs::settings::current();
    let dictionary = settings.dictionaries.iter().find(|d| {
        std::path::Path::new(&d.path)
            .file_stem()
            .and_then(|s| s.to_str())
            == Some(dict_name)
    })?;
    let directory = std::path::Path::new(&dictionary.path).parent()?;
    let local_path = directory.join(path.trim_start_matches('/'));
    let body = std::fs::read_to_string(local_path).ok().or_else(|| {
        let bytes = mdict_rs::registry::resource_for_dict(dict_name, &path)?;
        String::from_utf8(bytes).ok()
    })?;
    let sheet = Stylesheet::parse(&body);
    LINKED_STYLES.write().unwrap().insert(key, sheet.clone());
    Some(sheet)
}

pub fn set_dict_styles(map: HashMap<String, Stylesheet>) {
    *STYLESHEETS.write().unwrap() = map;
    LINKED_STYLES.write().unwrap().clear();
}
