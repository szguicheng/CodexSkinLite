# CodexSkinLite

CodexSkinLite is a lightweight, Apple Silicon-only macOS utility for two
Codex desktop customizations:

- importing and applying DreamSkin-compatible themes;
- constraining the conversation and composer to a centered maximum width.

The project uses a native Rust/AppKit status item and a small CDP-injected
renderer. It does not modify the official `Codex.app` bundle.

## Development status

Implementation is in progress. The approved design and implementation plan
are under `docs/superpowers/`.

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
