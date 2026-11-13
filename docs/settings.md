# Settings and dictionaries

The existing Rust settings module persists `~/.config/dicto/settings.toml`
(or the platform configuration directory). Tauri uses this same format, so
existing selections migrate without an export/import step.

```toml
dictionary_directories = ["/home/you/Dictionaries"]

[[dictionaries]]
path = "/home/you/Dictionaries/OALECD/OALECD.mdx"
enabled = true
```

On initial load and settings updates the core recursively scans configured
folders, appends new MDX files, and drops missing dictionary files. It preserves
existing paths, enabled flags, and order. The legacy
`~/.config/dicto/dicts/` directory is also discovered for compatibility.

Settings edits stay in the frontend draft until Save. Cancel discards them.
Choose folder adds a recursive scan root; Rescan saves the draft and rescans.
Removing a scan root stops future discovery there but does not delete sources
or unregister dictionaries already in the list. Disable a dictionary to exclude
it from lookup. Move arrows control tab/query order. Tab names are generated automatically
from dictionary metadata, including full Unicode names. Legacy custom tab-name
values in the settings file are ignored by the desktop frontend.

Source MDX/MDD and companion CSS stay where they are. The core writes `.fst`
and `.offsets` indexes beside them, so indexing requires a writable directory.
