# CodexSkinLite Design Specification

Date: 2026-08-31

Status: Approved in chat; awaiting written-spec review

## 1. Summary

CodexSkinLite is a macOS Apple Silicon utility that provides exactly two Codex desktop customizations:

1. DreamSkin-compatible theme import, selection, activation, and removal.
2. A centered conversation layout with a configurable maximum width shared by the message column and composer.

The application is a single Rust process with a native AppKit status item and settings window. It launches the official Codex application with a loopback Chrome DevTools Protocol (CDP) endpoint and injects a purpose-built renderer runtime. It does not use Tauri, React, Vite, an embedded WebView, a model proxy, or the full Codex++ renderer.

CodexSkinLite is derived in part from Codex++ behavior and compatibility work. The repository and distributed source will use `AGPL-3.0-only` and retain appropriate Codex++ attribution.

## 2. Goals

- Preserve compatibility with the current DreamSkin ZIP package format.
- Preserve the latest verified Codex UI mappings for main content, headers, threads, composers, and side panels.
- Keep renderer work dormant when the relevant layout has not changed.
- Keep theme CSS independent of volatile Codex private class names by exposing stable Skin API parts.
- Apply and remove themes without reloading Codex.
- Apply, change, and remove centered-width layout without reloading Codex.
- Provide a native, minimal macOS control surface.
- Produce an Apple Silicon `.app.zip` release with a SHA-256 checksum.

## 3. Non-goals

- Windows, Linux, or Intel Mac support.
- Theme marketplace, community browsing, publishing, or remote theme download.
- A theme editor or arbitrary CSS editor.
- Codex session management, model switching, provider proxies, remote control, pets, menu enhancements, localization, or plugin management.
- Modifying, patching, re-signing, or replacing the official `Codex.app` bundle.
- Automatic termination of a normally launched Codex process.
- Telemetry or cloud synchronization.
- DMG packaging or mandatory Developer ID signing in the first release.

## 4. Product Decisions

- Platform: macOS Apple Silicon only (`aarch64-apple-darwin`).
- Implementation: Rust with native AppKit bindings; no Tauri or embedded browser UI.
- Launch model: CodexSkinLite is the explicit Codex launcher.
- Theme compatibility: current DreamSkin ZIP packages remain importable without conversion.
- Runtime model: one CDP session and one small renderer runtime shared by both features.
- Licensing: `AGPL-3.0-only`, with Codex++ source attribution.

## 5. High-level Architecture

```text
Native AppKit status item and settings window
                    |
                    v
             Application controller
          /          |           \
         v           v            v
   Codex launcher  Theme store  Settings store
          \          |           /
                    v
                CDP session
                    |
                    v
          Minimal renderer runtime
          - Skin API registry
          - Theme presentation
          - Centered-width layout
```

The workspace is divided into focused modules:

- `macos_app`: AppKit lifecycle, status item, menus, settings window, dialogs, and user actions.
- `launcher`: Codex discovery, process-state checks, explicit debug launch, and restart confirmation flow.
- `cdp`: endpoint validation, target selection, WebSocket commands, new-document registration, injection, health, and reconnect.
- `theme_package`: ZIP validation, manifest validation, checksum validation, Safe CSS parsing, and CSS compilation.
- `theme_store`: atomic import, listing, activation preparation, deletion, and active-theme lookup.
- `settings`: minimal persisted configuration and atomic updates.
- `renderer`: injected JavaScript runtime and a small host-side payload builder.

Each module exposes a narrow interface and does not depend on the AppKit UI implementation unless it belongs to `macos_app`.

## 6. Native Application and Launch Lifecycle

The application runs as a status-item utility. It does not display a Dock icon during normal use.

The menu exposes:

- Current state: disconnected, connected, or action required.
- Open Codex.
- Reconnect.
- Current theme summary.
- Centered-width summary.
- Settings.
- Quit.

The settings window contains three sections:

### Appearance

- Theme enabled switch.
- Current imported theme selector.
- Import DreamSkin ZIP.
- Delete inactive theme.
- Open local theme directory.

### Layout

- Centered conversation switch.
- Maximum width field, clamped to 320-4000 px.
- Default maximum width: 900 px.

### Codex

- Codex application path.
- Connection status and debug port.
- Open, restart with confirmation, and reconnect actions.
- Open diagnostics log.

When the user chooses Open Codex:

1. If Codex is not running, launch the configured official application with a loopback remote-debugging port.
2. If a compatible Codex CDP endpoint already exists, attach without restarting.
3. If Codex is running without the expected endpoint, show a restart-required dialog.
4. Only an explicit confirmation may terminate and relaunch Codex.
5. Cancellation leaves the existing Codex process untouched.

The launcher uses the proven macOS `open -W -a <Codex.app> --args ...` pattern, adds a loopback debug address, and passes no unrelated Codex++ arguments.

## 7. CDP Session

The CDP module:

- Binds only to `127.0.0.1`.
- Validates that the endpoint returns a Codex target list.
- Selects the primary injectable Codex page and rejects ordinary browser pages, quick-chat-only targets, and unrelated auxiliary targets.
- Registers the renderer bootstrap for new documents.
- Evaluates the bootstrap immediately in the current renderer.
- Sends small settings and theme update payloads after bootstrap.
- Maintains one WebSocket session while Codex is running.

There is no fixed one-second polling loop. A disconnected session uses bounded exponential backoff only while Codex still appears to be running. Successful connection resets the backoff. User-triggered reconnect bypasses the current delay.

## 8. Settings and Theme Storage

Data is stored below:

```text
~/Library/Application Support/CodexSkinLite/
├── settings.json
├── logs/
│   └── codex-skin-lite.log
└── themes/
    └── <theme-id>/
        ├── manifest.json
        ├── theme.json
        ├── theme.css
        ├── compiled.css
        └── background.<webp|jpg|png>
```

`settings.json` contains only:

- `codexAppPath`
- `debugPort`
- `themeEnabled`
- `activeThemeId`
- `conversationCentered`
- `conversationMaxWidth`

All settings and theme activation writes use a temporary sibling followed by atomic replacement. Unknown or invalid settings values are normalized without deleting valid themes.

## 9. DreamSkin Package Compatibility

A compatible package contains:

- `manifest.json`
- `theme.json`
- `theme.css`
- exactly one of `background.webp`, `background.jpg`, or `background.png`
- optional `LICENSE.txt`
- optional supported signature metadata

Import runs in a temporary directory and validates before publishing the theme directory. Validation includes:

- Compressed and uncompressed size limits.
- File-count and per-file size limits.
- Rejection of absolute paths, parent traversal, symbolic links, encrypted entries, and unsupported files.
- Manifest schema, identifiers, semantic versions, supported platform, capabilities, provenance, and file list.
- SHA-256 hashes for declared files.
- Image type and content.
- UTF-8 JSON and CSS.
- Safe CSS selectors and property values.

Failed import removes the temporary directory and leaves the theme library unchanged.

Safe CSS accepts only registered Skin API selectors in this form:

```css
[data-ds-part="main"] { ... }
[data-ds-part="composer"]:focus-visible { ... }
```

It rejects arbitrary selectors, remote URLs, scriptable content, `@import`, CSS escapes, nested rules, comments, and unsupported properties. Valid CSS is compiled once during import into the trusted cascade. Package parsing and compilation do not run during normal scrolling or typing.

## 10. Skin API Semantics

CodexSkinLite exposes these stable parts:

- `root`: document-wide base presentation.
- `sidebar`: left project navigation and right contextual side panels.
- `main`: the complete main content viewport below the top-level window chrome, including layout space around the scroll container.
- `header`: the Codex title/header bar, independently themeable.
- `home`: new-conversation home surface.
- `home-hero`: home hero content.
- `project-list`: project list surface.
- `thread`: existing-conversation scroll surface.
- `message`: individual message surfaces.
- `composer`: complete input surface.
- `composer-toolbar`: composer action/footer region.
- `composer-toolbar-empty`: toolbar state when no attachments are present.
- `dialog`: dialogs and floating panels.

The semantic boundary prevents the previously observed three-layer mismatch:

- `main` owns the full main viewport background.
- `thread` adds conversation-scroll-specific styling but does not create an unnamed gap layer.
- `header` explicitly controls title-bar presentation.
- Both persistent side-panel directions use `sidebar`.
- Centered width changes content geometry, not background-surface geometry.

## 11. Renderer Runtime

The renderer runtime has one central state object containing:

- Current settings and active theme payload signature.
- Cached Skin API part nodes.
- Cached conversation-content and composer nodes.
- Owned inline style snapshots for reversible changes.
- One pending animation-frame identifier.
- One MutationObserver.
- One on-demand ResizeObserver.

### Update algorithm

1. Perform one initial discovery after bootstrap.
2. Mark recognized nodes with `data-ds-part` and thread-surface attributes.
3. Observe the smallest stable Codex application-shell root available.
4. Filter mutation records for layout-relevant additions, removals, and selected attributes.
5. Coalesce all relevant records into one `requestAnimationFrame` update.
6. Keep connected cached nodes and rediscover only missing or affected parts.
7. Write attributes or inline styles only when the desired value differs.
8. Observe size only for the main viewport, conversation content, composer, and active side-panel layout.

Ordinary scroll events, caret blinking, selection changes, and message text updates must not trigger a full Skin API scan.

Theme switching replaces one managed style element in place. The background image is transferred once, converted to a renderer Blob URL, and the previous Blob URL is revoked. The transfer payload is discarded after Blob creation so image bytes are not retained in duplicate application state.

Theme cleanup removes managed style nodes, managed attributes, Blob URLs, and only those inline values owned by CodexSkinLite.

## 12. Centered Conversation Layout

When enabled, the layout runtime resolves the message-column container and composer container, then applies:

- `box-sizing: border-box`
- `width: 100%`
- configured `max-width`
- automatic horizontal margins

If side panels make the native content region asymmetric, the runtime aligns both elements to the visual center of the available main viewport without overwriting unrelated transforms. It stores and restores only offsets it owns.

The centered layout:

- Shares discovery, mutation batching, and resize observation with the Skin API.
- Reapplies only after target replacement or meaningful viewport geometry changes.
- Does not respond to ordinary thread scrolling.
- Restores original inline values when disabled.

## 13. Composer Compatibility

CodexSkinLite always keeps the React-managed Codex composer footer in the active
thread scroll container. It does not reparent the footer or impose a second
fixed-position copy, because changing the parent of React-owned DOM can leave a
stale composer behind during route transitions.

At bootstrap and on relevant layout mutations, the runtime resolves the active
thread scroll surface using the stable Codex anchor and visibility state. If
that surface contains a footer, it retains the first active footer and removes
only duplicate footer nodes within the same main surface. If the active route
cannot be resolved unambiguously, the runtime fails closed and leaves the DOM
untouched until the next layout update.

When replacing a runtime from an older version, the new bootstrap first runs
the previous cleanup, then removes an older marked footer only when a different
active native footer is present. This migration is limited to the current main
surface and does not alter unrelated composer or route nodes.

## 14. Live Update Data Flows

### Theme activation

1. Load and revalidate the stored theme metadata and compiled assets.
2. Build a versioned theme payload.
3. Send it over the established CDP session.
4. Update the managed style and image Blob.
5. Run one mapping/layout pass.
6. Verify the renderer payload signature.
7. Commit the active theme ID only after verification.

Failure preserves the previous active theme and settings.

### Width update

1. Normalize the requested width.
2. Send a small settings payload.
3. Apply one scheduled layout pass.
4. Verify the renderer setting value.
5. Persist settings after verification.

### Disable and cleanup

Disabling a feature removes only that feature's owned state. Disabling themes does not disable centered width, and disabling centered width does not remove Skin API markers needed by an active theme.

## 15. Error Handling and Diagnostics

- Invalid import: report the precise invalid file, selector, property, or manifest field; publish nothing.
- Activation failure: retain the previous theme and active ID.
- Image failure: do not activate a partial theme.
- Port conflict: report the port and allow selection of another loopback port.
- Running Codex without CDP: request explicit restart confirmation.
- CDP disconnect: preserve settings and reconnect with bounded backoff.
- Missing critical UI parts: stop geometry mutations, apply only confirmed-safe presentation, and expose a compatibility warning.
- Composer adapter failure: restore original DOM and inline values immediately.
- Corrupt settings: load normalized defaults while preserving the theme directory.

Logs are local, size-rotated, and contain no conversation text, prompt contents, credentials, or theme image bytes.

## 16. Security

- CDP listens on loopback only and selected targets are validated.
- No theme JavaScript is supported.
- No theme network access is supported.
- No package executable is run.
- ZIP extraction is path-safe and bounded.
- Theme CSS is parsed against an allowlist and compiled before injection.
- File operations remain below the application support directory after canonical-path checks.
- The tool does not modify the official Codex bundle.
- The application performs no telemetry or background network request.

## 17. Performance Requirements

- Uncompressed injected JavaScript target: less than 100 KB.
- Exactly one coalesced MutationObserver for both features.
- At most one ResizeObserver, attached only while relevant elements exist.
- No renderer `setInterval`.
- No layout refresh in an unchanged idle state.
- No full Skin API scan caused by ordinary scrolling or caret blinking.
- Layout refresh mean below 2 ms and P95 below 8 ms in the reference real-Codex scenario.
- Connected status-item process average idle CPU below 0.5% over 60 seconds.
- Status-item process idle resident memory target below 50 MB.
- Theme switch completes without Codex reload and targets completion within 300 ms for ordinary package sizes.
- No visible background clear-frame during focused-composer scrolling or streaming output.
- Opening the right context panel must not cover the composer and must inherit `sidebar` presentation.

Performance measurements are release gates, not best-effort observations. A missed target must be documented with measurements before release.

## 18. Testing Strategy

### Rust unit tests

- ZIP traversal, absolute paths, links, encryption, size limits, and unsupported files.
- Manifest schema, platform, version, hash, and image validation.
- Safe CSS accepted and rejected selectors/properties.
- Atomic theme import, replacement, activation, deletion, and rollback.
- Settings normalization and atomic persistence.
- CDP URL validation, target classification, command/result parsing, and reconnect state.
- macOS launch command construction and running-without-CDP decision logic.

### Renderer tests

- Skin API mapping for new-conversation and existing-thread fixtures.
- Correct `main`, `header`, `thread`, `composer`, and both side-panel semantics.
- Attachment-empty toolbar transition without an extra divider.
- Cached-node replacement and cleanup.
- Theme update and Blob revocation.
- Centered-width enable, update, side-panel alignment, and disable restoration.
- Mutation coalescing.
- Scroll, caret, and message-text mutations do not schedule full scans.
- Native footer containment, route replacement, duplicate-footer cleanup, and
  older-runtime migration.

### Real Codex acceptance matrix

- New conversation.
- Existing conversation.
- Focused and unfocused composer scrolling.
- Streaming output.
- Add and remove image/file attachments.
- Open and close right context panel.
- Resize and full-screen transitions.
- Theme hot switch and disable.
- Centered-width hot update and disable.
- Codex quit, relaunch, and reconnect.
- CodexSkinLite quit and later reconnect.

The real-Codex pass includes visual checks for blur reversal, clear-frame flicker, title/main/thread seams, composer scrolling, side-panel coverage, and input-toolbar dividers.

## 19. Release

The initial release produces:

- `CodexSkinLite.app.zip` for Apple Silicon.
- SHA-256 checksum.
- Source archive generated by GitHub.
- Local verification record for Rust tests, renderer tests, release build, and real-Codex acceptance.

The first release does not require a DMG or Developer ID signature. Documentation explains the macOS quarantine implications of an unsigned application.

## 20. Repository Scope Guard

Feature additions are accepted only when they directly support theme import/application, centered conversation width, Codex launch/connection, compatibility diagnostics, or safety/performance of those flows. Provider management, session tools, model controls, plugin markets, remote control, and unrelated Codex enhancements are explicitly out of scope.
