# CodexSkinLite

CodexSkinLite is a lightweight, Apple Silicon-only macOS utility for three
Codex desktop customizations:

- importing and applying DreamSkin-compatible themes;
- constraining the conversation and composer to a centered maximum width.
- previewing and saving bounded visual and composer-position overrides per theme.

The project uses a native Rust/AppKit status item and a small CDP-injected
renderer. It does not modify the official `Codex.app` bundle.

## Usage

1. Start CodexSkinLite. Settings opens automatically once on every launch.
2. Import a DreamSkin ZIP package, or open the remote theme gallery from the
   Appearance section: https://dreamskin.cc/gallery.
3. Choose “无” to use no theme, or choose an imported theme ID and enable it.
4. Open “自定义主题…” to adjust colors, surface appearance, and the composer
   inset. The background section also supports choosing a replacement image,
   pixel X/Y offset, cover/contain/stretch fill, and image opacity. A selected
   theme starts with its package values; “无” opens an empty draft. Click
   “预览” to apply the draft temporarily, then click “保存” to write it to
   the theme.
5. Enable centered conversation width and enter a width from 320 to 4000 px.
6. Use “Open Codex” from CodexSkinLite so Codex starts with a loopback CDP port.

Per-theme overrides are stored in
`~/Library/Application Support/CodexSkinLite/themes/<theme-id>/customization.json`.
The original ZIP and its `theme.json`/`theme.css` files are not modified.

Codex already running without CDP is never terminated automatically. Open
Settings after the restart-required state appears and choose “确认重启”.

## Development

The approved design and implementation plan are under `docs/superpowers/`.

```bash
cargo test --all-targets
npm ci --prefix renderer
npm test --prefix renderer
bash scripts/package-app.sh
```

## Requirements

- Apple Silicon Mac
- Rust 1.97 or newer
- Node.js for renderer tests only

## License and provenance

CodexSkinLite is licensed under `AGPL-3.0-only`. See `NOTICE` for Codex++
attribution.

## Application signing

Release ZIPs are signed with a Developer ID Application certificate but are
not notarized yet. On another Mac, use Finder's Open action once if Gatekeeper
shows an unidentified-developer warning. Direct opening without that warning
requires a separately notarized release.
