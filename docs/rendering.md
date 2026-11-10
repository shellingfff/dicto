# Dictionary rendering

Tauri renders the interface with the platform webview. Definitions use the
browser's HTML/CSS/JavaScript engine rather than the former GPUI subset parser.

`desktop/src/document.rs` serves a dictionary document and its resources through
the `dict` protocol. On Linux the URL is `dict://<id>.localhost/...`; Tauri maps
custom schemes to `http://dict.<id>.localhost/...` on Windows.

Each dictionary has a stable origin derived from its full source path, so
root-relative font/image URLs stay within that dictionary. Each result gets its
own sandboxed iframe. CSS cannot affect the app or another
dictionary. Relative CSS, JavaScript, image, font, and audio URLs resolve within
that dictionary. Resources are read from the original dictionary directory,
then from the matching MDD. Neither the MDX nor MDD source is copied.

The document includes header-derived CSS and a small frame bridge in
`frontend/definition.js`. The bridge handles `entry://` and `d:` lookups,
`sound://` audio, and dictionary/word keyboard shortcuts. Dictionary-specific XHTML tags retain their original CSS behavior, including
OALECD sense labels generated with `::before`. XHTML lists without generated
markers receive native HTML list semantics; XHTML breaks receive line-break
semantics. This avoids duplicating authored numbers or changing ordinary text
into browser-styled hyperlinks.

Dictionary JavaScript runs in the frame. Its sandbox, content policy, and Tauri
capabilities isolate it from native commands and arbitrary network access.
External website links are not opened by the definition bridge. Audio format
support depends on the system webview and installed media codecs.

The previous parser/renderer is documented in [rendering-gpui.md](rendering-gpui.md)
for reference only; it is not part of the Tauri binary.
