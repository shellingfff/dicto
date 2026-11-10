# Architecture

The active Cargo workspace contains the existing `mdict-rs` library and the
`dicto` desktop binary in `desktop/`. The dictionary parser, FST/offset indexes,
redirect handling, resource decompression, and settings format live in the
library. Tauri supplies the window, system tray, native folder picker, and IPC.

`frontend/` is plain HTML/CSS/JavaScript embedded by Tauri at build time. It needs
no Node server or npm install. Linux uses WebKitGTK 4.1; Windows uses WebView2.
The old `gpui/` implementation is retained as reference, outside the workspace.

## Lookup

1. The search field debounces input for 150 ms and calls the `search` command.
2. The Rust backend queries enabled dictionaries through the existing
   `Dictionary::suggestions` API, deduplicates matches, and puts exact matches
   first. Ctrl+N/P selects adjacent matches without showing a word sidebar.
3. `lookup` uses `Dictionary::lookup` and returns dictionary IDs and tab titles.
4. The selected tab opens a sandboxed frame at the dictionary protocol's
   `/__entry?word=...` URL on its own dictionary origin. Its HTML and resources are loaded by
   `desktop/src/document.rs`. See [rendering](rendering.md).

A frontend revision counter ignores superseded lookup responses. Dictionary
refresh increments a generation counter; the UI polls progress and reruns the
current query after a refresh.

## Background work

Indexing, lookup, settings I/O, and protocol responses run in Tauri blocking
workers, not on the GTK event loop. A mutation mutex serializes settings changes
with indexing. The completed dictionary catalog replaces the old catalog under
a read/write lock. The UI displays index progress and errors.

## Native boundaries

Only the main application origin receives the named IPC command permissions.
Dictionary frames retain their own `dict` origin, distinct from the main
`tauri` origin, while the sandbox blocks top navigation, forms, and popups.
Tauri injects native IPC only into the main frame; the dictionary frame cannot
read the main DOM or native IPC globals. Their bridge accepts lookup and
keyboard events; it exposes no filesystem or settings commands. Local resource
paths are canonicalized and confined to the dictionary directory, with MDD
lookup as fallback. No network service, downloader, or telemetry is started by
the desktop app.

The historical `mdict-rs` HTTP server remains a separate binary; the desktop app
does not launch it.
