<p align="center"><img src="assets/icon.svg" width="128" alt="Dicto"></p>

<h1 align="center">Dicto</h1>

<p align="center">A fast, offline desktop dictionary for Windows and Linux built in Rust. Reads MDX/MDD dictionary files — the same format used by GoldenDict, MDict, and most popular dictionary packs.</p>

![screenshot](screenshot.png)

## Features

- **Tauri frontend** — system web rendering with a Rust backend; no Node runtime or frontend build tool required
- **Instant search** — definitions update as you type using case-insensitive exact matches; missing words show “No definition found.”
- **Faithful rendering** — HTML, CSS, and JavaScript rendered in isolated dictionary frames
- **Multi-dictionary** — load as many MDX files as you like, enable/disable/reorder them without restarting

## Installation

### Arch Linux

Download `dicto-*-x86_64.pkg.tar.zst` from the
[releases page](https://github.com/shellingfff/dicto/releases/latest), then install it:

```bash
sudo pacman -U ./dicto-*-x86_64.pkg.tar.zst
```

Pacman installs the required WebKitGTK and audio dependencies. The package includes
the application, launcher, icons, and license under `/usr`. Releases are built on
Arch Linux; this fork does not publish to the AUR.

If you previously used `sudo make install`, remove the old `/usr/local/bin/dicto`
and its launcher/icons under `/usr/local/share` so they do not override the package.

### Windows

Download from the [releases page](https://github.com/shellingfff/dicto/releases/latest):

| File | Description |
| ---- | ----------- |
| `dicto-*-windows-x86_64-installer.exe` | Recommended NSIS installer |
| `dicto-*-windows-x86_64-portable.zip` | Portable executable |

The installer checks for Microsoft Edge WebView2 and runs its embedded
bootstrapper when the runtime is missing. Installing the missing runtime
requires internet access. The portable ZIP does not install WebView2; install
that runtime separately before running it. The
[Visual C++ Redistributable](https://aka.ms/vs/17/release/vc_redist.x64.exe)
may also be needed on a clean Windows installation.

If upgrading from the old Inno Setup installer, uninstall that version first.
The new installer uses a different installer registration.

### From source

```bash
make linux
# binary at target/release/dicto
sudo make install
```

This installs the binary, desktop launcher, icon, and license under `/usr/local`.
To install for your user without sudo, run `make install PREFIX="$HOME/.local"`
after building, and ensure `~/.local/bin` is in your `PATH`.
Repeat the build and install commands to update. Linux installs use the application
ID `io.github.shellingfff.dicto` for the launcher and icons, matching the GTK
window ID. The GTK program name also matches this ID so Plasma can associate
Wayland windows with the launcher. Installation refreshes available desktop/icon caches and replaces the
old `dicto.desktop` launcher. After upgrading, quit and reopen Dicto; if it was
pinned in GNOME, remove the old favorite and pin the new launcher. Packaging can use
`make install DESTDIR=/path/to/staging PREFIX=/usr`.

**Linux dependencies:** WebKitGTK 4.1, GTK 3, and libappindicator. On Arch:

```bash
sudo pacman -S --needed base-devel rust webkit2gtk-4.1 libayatana-appindicator gst-plugins-good
```

GStreamer (`gst-plugins-good`) supplies the audio sink and common pronunciation
codecs; `gst-plugins-bad` adds other formats. Windows requires the WebView2 runtime.
Use `cargo run -p dicto` for development; the static frontend is embedded into
the binary by Tauri. `make check` runs backend tests and JavaScript syntax checks.

## Keyboard shortcuts

- **Tab / Shift+Tab** — next / previous dictionary result, wrapping around.
- **Ctrl+- / Ctrl+=** — zoom dictionary results out / in (50–200%); Ctrl+0 resets. Zoom is remembered.
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

## Release builds

GitHub Actions builds an Arch Linux package and a Windows NSIS installer.
A `v<version>` tag must match both `desktop/Cargo.toml` and
`desktop/tauri.conf.json`; both platform builds must succeed before publication.
Manual workflow runs create downloadable artifacts without publishing a release.
Artifacts use the application version, not the branch name.

To build the Arch package locally, install the Linux build dependencies listed
above and run as your regular user from the repository root:

```bash
make arch-package
sudo pacman -U target/arch-package/dicto-*-x86_64.pkg.tar.zst
```

This packages the current source tree using `packaging/arch/PKGBUILD.in`, with a
SHA-256 checked source archive. The output is in `target/arch-package/`.

To build the Windows installer, run from `desktop/` with the platform dependencies:

```bash
npx --yes @tauri-apps/cli@2.12.6 build --ci --bundles nsis -- --locked
```

Windows bundles appear under `target/release/bundle/` in the workspace root.
