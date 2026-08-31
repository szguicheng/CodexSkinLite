# CodexSkinLite

CodexSkinLite is a lightweight, Apple Silicon-only macOS utility for two
Codex desktop customizations:

- importing and applying DreamSkin-compatible themes;
- constraining the conversation and composer to a centered maximum width.

The project uses a native Rust/AppKit status item and a small CDP-injected
renderer. It does not modify the official `Codex.app` bundle.

## Usage

1. Start CodexSkinLite from the menu bar application.
2. Open Settings and import a DreamSkin ZIP package.
3. Reopen Settings, choose the imported theme ID, and enable it.
4. Enable centered conversation width and enter a width from 320 to 4000 px.
5. Use “Open Codex” from CodexSkinLite so Codex starts with a loopback CDP port.

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

## Unsigned application notice

Early `.app.zip` builds are not Developer ID signed or notarized. macOS may
quarantine downloaded builds; inspect the source and build locally if that
is not acceptable.
