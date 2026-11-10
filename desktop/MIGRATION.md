# Tauri frontend

The default `dicto` package is `desktop/`. The Rust library in `src/` retains
its parsing, indexing, lookup, and settings implementation. `gpui/` remains
available as legacy source, excluded from the workspace and release binary.

The static frontend is embedded by Tauri. Linux uses WebKitGTK 4.1; Windows
uses WebView2. No JavaScript package manager or Node runtime is needed to run
Dicto. Node is used only for optional syntax checks.

## Verification

```sh
cargo test --locked --workspace
cargo clippy --locked --workspace --all-targets -- -D warnings
cargo fmt --all --check
node --check frontend/app.js
node --check frontend/definition.js
make linux
python3 tests/webview_smoke.py --binary target/release/dicto --first-run
python3 tests/webview_smoke.py --binary target/release/dicto --dictionary-root ~/Dictionaries
make install DESTDIR=/tmp/dicto-stage PREFIX=/usr
```

The native smoke test requires WebKitWebDriver, an active Linux desktop, and
GStreamer audio plugins. It creates temporary settings and generated MDX/MDD
fixtures, then checks recursive setup, CSS/JavaScript, numbered senses, inline
labels, relative/root-relative images, audio, entry links, keyboard navigation,
settings persistence, and dictionary-frame isolation. On Hyprland it also
checks desktop identity, close-to-tray, singleton reopening, and focus selection.
`--dictionary-root` checks the installed OALECD hello entry. `--screenshots`
optionally captures images; WebKit screenshot requests can time out if the
window is not being composited.

Dictionary frames use separate path-derived origins. Original local resources
are read before matching MDD resources. Resource containment tests reject
parent traversal, absolute paths, and symlink escapes. Runtime checks verify
that dictionary scripts cannot read the app DOM or access Tauri globals.

OALECD's XHTML tags retain author CSS so generated sense numbers are not
duplicated. Plain XHTML lists without CSS-generated labels receive native list
semantics. Pronunciation links fetch audio into a Blob because WebKit's media
pipeline cannot directly stream this custom protocol.

Windows packaging targets WebView2 but has not been executed on this Linux
machine. Installation is staged for verification; system installation and git
commits are separate from this migration.

Verified on Linux: release build, all 12 Rust tests, Clippy with warnings denied,
formatting and JavaScript syntax checks, and staged installation. The complete
native fixture test passed with GStreamer audio, real keyboard actions,
Hyprland app ID `dicto`, close-to-tray, singleton reopening, and native focus
selection. Tab handling accepts the physical key code because WebKitGTK can
report Shift+Tab's logical key as `Unidentified`.
The real OALECD hello entry also passed with its original stylesheet,
JavaScript, single CSS-generated sense numbers, and inline GREETING label.
