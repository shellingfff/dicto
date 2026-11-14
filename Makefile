# Linux build/install and native Windows builds.
# DESTDIR stages installation for packaging; PREFIX is the final install location.
PREFIX ?= /usr/local
DESTDIR ?=
CARGO ?= cargo
LINUX_TARGET_DIR ?= target

.DEFAULT_GOAL := linux

.PHONY: linux install windows build check help

# Build as your regular user before running a system-wide install with sudo.
linux:
	$(CARGO) build --locked --release --package dicto --target-dir "$(LINUX_TARGET_DIR)"

# Install an already-built binary, without running Cargo as root.
install:
	@test -x "$(LINUX_TARGET_DIR)/release/dicto" || { echo "Run 'make linux' before installing." >&2; exit 1; }
	install -Dm755 "$(LINUX_TARGET_DIR)/release/dicto" "$(DESTDIR)$(PREFIX)/bin/dicto"
	install -Dm644 packaging/arch/dicto.desktop "$(DESTDIR)$(PREFIX)/share/applications/io.github.shellingfff.dicto.desktop"
	install -Dm644 assets/icon.svg "$(DESTDIR)$(PREFIX)/share/icons/hicolor/scalable/apps/io.github.shellingfff.dicto.svg"
	install -Dm644 desktop/icons/icon.png "$(DESTDIR)$(PREFIX)/share/icons/hicolor/128x128/apps/io.github.shellingfff.dicto.png"
	# Retire the previous launcher to avoid duplicate entries after an upgrade.
	rm -f "$(DESTDIR)$(PREFIX)/share/applications/dicto.desktop"
	install -Dm644 LICENSE "$(DESTDIR)$(PREFIX)/share/licenses/dicto/LICENSE"

	@if [ -z "$(DESTDIR)" ]; then \
		if command -v update-desktop-database >/dev/null 2>&1; then update-desktop-database "$(PREFIX)/share/applications"; fi; \
		if command -v gtk-update-icon-cache >/dev/null 2>&1; then gtk-update-icon-cache -q -t -f "$(PREFIX)/share/icons/hicolor"; fi; \
	fi

# Native Windows builds use WebView2; install the MSVC Rust toolchain first.
windows:
	$(CARGO) build --locked --release --package dicto

build: linux

check:
	$(CARGO) test --locked --workspace
	node --check frontend/app.js
	node --check frontend/definition.js

help:
	@echo "make linux        Build the Tauri app (default)"
	@echo "make install      Install binary, launcher, icon and license"
	@echo "                  Supports PREFIX and DESTDIR"
	@echo "make check        Run Rust tests and JavaScript syntax checks"
	@echo "make windows      Build natively on Windows with MSVC/WebView2"
