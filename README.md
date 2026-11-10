<p align="center"><img src="assets/icon.svg" width="128" alt="Dicto"></p>

<h1 align="center">Dicto</h1>

<p align="center">A fast, offline desktop dictionary for Windows and Linux built in Rust. Reads MDX/MDD dictionary files — the same format used by GoldenDict, MDict, and most popular dictionary packs.</p>

![screenshot](screenshot.png)

## Features

- **Tauri frontend** — system web rendering with a Rust backend; no Node runtime or frontend build tool required
- **Instant search** — definitions update as you type; navigate matching words with Ctrl+N/P
- **Faithful rendering** — HTML, CSS, and JavaScript rendered in isolated dictionary frames
- **Multi-dictionary** — load as many MDX files as you like, enable/disable/reorder them without restarting

## Installation

### Linux

[Download](https://github.com/logi-camp/dicto/releases/latest) — look for:

| File | Description |
| ---- | ----------- |
| `dicto-*-x86_64-unknown-linux-gnu.tar.gz` | Generic Linux binary |

Extract the archive and run the binary.

### Arch Linux (AUR)

Install from AUR using your preferred helper (e.g. `yay`, `paru`, `pikaur`).
Both packages are updated automatically by CI on every tagged release:

```bash
# Build from source
yay -S dicto

# Or install pre-built binary
yay -S dicto-bin
```

Or manually:

```bash
git clone https://aur.archlinux.org/dicto.git
cd dicto
makepkg -si
```

### Windows

[Download](https://github.com/logi-camp/dicto/releases/latest) — look for:

| File | Description |
| ---- | ----------- |
| `dicto-*-windows-x86_64-installer.exe` | Installer |
| `dicto-*-windows-x86_64-portable.zip` | Portable (no install needed) |

Requires Microsoft Edge WebView2 Runtime and the [Visual C++ Redistributable (VC_redist.x64.exe)](https://aka.ms/vs/17/release/vc_redist.x64.exe). The portable executable does not bundle WebView2.

### From source

```bash
make linux
# binary at target/release/dicto
sudo make install
```

This installs the binary, desktop launcher, icon, and license under `/usr/local`.
To install for your user without sudo, run `make install PREFIX="$HOME/.local"`
after building, and ensure `~/.local/bin` is in your `PATH`.
Repeat the build and install commands to update. Packaging can use
`make install DESTDIR=/path/to/staging PREFIX=/usr`.

**Linux dependencies:** WebKitGTK 4.1, GTK 3, and libappindicator. On Arch:

```bash
sudo pacman -S --needed base-devel rust webkit2gtk-4.1 libappindicator-gtk3 gst-plugins-good
```

GStreamer (`gst-plugins-good`) supplies the audio sink and common pronunciation
codecs; `gst-plugins-bad` adds other formats. Windows requires the WebView2 runtime.
Use `cargo run -p dicto` for development; the static frontend is embedded into
the binary by Tauri. `make check` runs backend tests and JavaScript syntax checks.

## Keyboard shortcuts

- **Tab / Shift+Tab** — next / previous dictionary result, wrapping around.
- **Ctrl+N / Ctrl+P** — next / previous word in the suggestion list.
- **Ctrl+L / Ctrl+F** — focus the search field.

## Dictionary setup


**Option 1 — Use existing files:** Open Settings and choose a folder. Dicto recursively finds all `.mdx` dictionaries, registers their original paths without copying them, and remembers the directory to discover new dictionaries on subsequent launches. Keep companion `.mdd` and CSS files alongside the MDX. Search indexes (`.fst` and `.offsets`) are written beside the source files, so the directory must be writable. Keep the files in place after adding them.

**Option 2 — Drop files manually:** Place `.mdx` (and optional `.mdd`) files into:

```text
~/.config/dicto/dicts/
```

Dicto auto-discovers them on next launch. No config editing needed.  
Enable, disable, and reorder dictionaries from the settings dialog (⚙ button).

## Supported formats

| Format | Status |
| ------ | ------ |
| MDX v2 (UTF-8, UTF-16) | ✓ Full support |
| MDX v1 | ✓ Full support |
| MDD resource containers | ✓ Images, audio, CSS |
| Encryption level 0, 2 | ✓ Supported |

## Architecture

The active workspace has two crates:

- **`mdict-rs`** (library) — MDX/MDD parser, FST/offset indexes, query pipeline, settings
- **`dicto`** (`desktop/`) — Tauri desktop shell and commands

`frontend/` contains the interface and dictionary frame bridge. Definition HTML
is served through a dictionary-specific protocol with local/MDD CSS, JavaScript,
images, audio, and fonts. Source dictionaries stay in place. Dictionary frames
are sandboxed and do not receive native application permissions.

The former frontend remains in `gpui/` as reference during migration, excluded
from the active workspace and normal builds.


## References

- [mdict-analysis](https://bitbucket.org/xwang/mdict-analysis) — MDX/MDD format specification
