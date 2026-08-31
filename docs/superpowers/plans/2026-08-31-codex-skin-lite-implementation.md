# CodexSkinLite Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build an Apple Silicon macOS status-item utility that launches Codex with loopback CDP access, imports current DreamSkin ZIP themes, applies a low-overhead Skin API runtime, and optionally centers the conversation and composer at a configurable width.

**Architecture:** A single Rust binary owns native AppKit UI, settings, theme storage, Codex launch, and one CDP WebSocket session. A separately tested JavaScript asset is injected once into Codex; it exposes stable Skin API parts and shares one mutation/resize lifecycle between theming and centered width.

**Tech Stack:** Rust 2024, Tokio, reqwest, tokio-tungstenite, serde, zip, sha2, objc2/AppKit, dispatch2, tracing; JavaScript ES2022, Vitest, happy-dom; GitHub Actions on macOS ARM64-compatible runners.

**Spec:** `docs/superpowers/specs/2026-08-31-codex-skin-lite-design.md`

## Global Constraints

- Target only `aarch64-apple-darwin`; reject unsupported platforms at compile time.
- Keep a single Rust process with native AppKit UI; do not add Tauri, React, Vite application UI, or an embedded WebView.
- Preserve the current DreamSkin ZIP package format and stable `data-ds-part` API.
- Bind CDP only to `127.0.0.1` and validate the selected Codex target.
- Inject less than 100 KB of uncompressed JavaScript.
- Use exactly one coalesced renderer MutationObserver, at most one on-demand ResizeObserver, and no renderer `setInterval`.
- Keep theme and width cleanup reversible; restore only state owned by CodexSkinLite.
- Do not modify or re-sign the official `Codex.app` bundle.
- Store user data below `~/Library/Application Support/CodexSkinLite` using atomic writes.
- Use `AGPL-3.0-only` and retain Codex++ attribution.
- Create the GitHub repository as private during implementation; public release requires separate user authorization.

## Planned File Structure

```text
Cargo.toml
Cargo.lock
LICENSE
NOTICE
README.md
.gitignore
.github/workflows/ci.yml
assets/renderer/runtime.js
renderer/package.json
renderer/package-lock.json
renderer/vitest.config.mjs
renderer/tests/dom-fixtures.js
renderer/tests/runtime.test.js
resources/Info.plist
scripts/package-app.sh
scripts/measure-renderer.mjs
src/main.rs
src/lib.rs
src/model.rs
src/paths.rs
src/settings.rs
src/theme/mod.rs
src/theme/package.rs
src/theme/safe_css.rs
src/theme/store.rs
src/cdp/mod.rs
src/cdp/discovery.rs
src/cdp/protocol.rs
src/cdp/session.rs
src/launcher.rs
src/renderer.rs
src/controller.rs
src/diagnostics.rs
src/macos/mod.rs
src/macos/app_delegate.rs
src/macos/menu.rs
src/macos/settings_window.rs
tests/fixtures.rs
tests/theme_package.rs
tests/theme_store.rs
tests/cdp_discovery.rs
tests/cdp_session.rs
tests/launcher.rs
tests/controller.rs
docs/acceptance/latest-codex.md
```

---

### Task 1: Repository, Rust Scaffold, and Minimal Domain Model

**Files:**
- Create: `Cargo.toml`
- Create: `.gitignore`
- Create: `LICENSE`
- Create: `NOTICE`
- Create: `README.md`
- Create: `src/lib.rs`
- Create: `src/main.rs`
- Create: `src/model.rs`
- Test: `tests/settings_defaults.rs`

**Interfaces:**
- Consumes: approved design spec.
- Produces: `AppSettings`, `ConnectionState`, `AppSnapshot`, and a compiling `codex-skin-lite` binary used by every later task.

- [ ] **Step 1: Write the failing defaults test**

```rust
use codex_skin_lite::model::AppSettings;

#[test]
fn defaults_match_product_contract() {
    let value = AppSettings::default();
    assert_eq!(value.debug_port, 9222);
    assert!(!value.theme_enabled);
    assert_eq!(value.active_theme_id, None);
    assert!(!value.conversation_centered);
    assert_eq!(value.conversation_max_width, 900);
}
```

- [ ] **Step 2: Run the test and verify the crate is not yet defined**

Run: `cargo test --test settings_defaults`

Expected: FAIL because `Cargo.toml` and `codex_skin_lite::model` do not exist.

- [ ] **Step 3: Create the crate and exact public model**

Use Rust edition 2024 and `rust-version = "1.97"`. Add these dependency families and commit the generated `Cargo.lock`: `anyhow`, `base64`, `directories`, `dispatch2`, `futures-util`, `reqwest 0.13` without TLS defaults, `semver`, `serde`, `serde_json`, `sha2`, `thiserror`, `tokio`, `tokio-tungstenite 0.30` without TLS defaults, `tracing`, `tracing-subscriber`, `zip 8` with only deflate support, `objc2 0.6`, `objc2-app-kit 0.3`, and `objc2-foundation 0.3`. Add `tempfile` as a dev dependency.

```rust
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppSettings {
    pub codex_app_path: std::path::PathBuf,
    pub debug_port: u16,
    pub theme_enabled: bool,
    pub active_theme_id: Option<String>,
    pub conversation_centered: bool,
    pub conversation_max_width: u16,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConnectionState {
    Disconnected,
    Connecting,
    Connected,
    RestartRequired,
    CompatibilityWarning(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppSnapshot {
    pub settings: AppSettings,
    pub connection: ConnectionState,
    pub active_theme_name: Option<String>,
}
```

Default `codex_app_path` to `/Applications/Codex.app`. Add a compile-time error for non-macOS targets and a runtime architecture check before the AppKit loop.

- [ ] **Step 4: Add license and provenance**

Copy the canonical AGPL-3.0-only license text into `LICENSE`. In `NOTICE`, state that CodexSkinLite contains adapted compatibility concepts and code from Codex++ at `https://github.com/BigPizzaV3/CodexPlusPlus`, also AGPL-3.0-only. Keep README limited to project scope, build prerequisites, and the unsigned-app warning.

- [ ] **Step 5: Verify and commit**

Run: `cargo fmt --check && cargo test --test settings_defaults && cargo clippy --all-targets -- -D warnings`

Expected: all commands PASS.

```bash
git add Cargo.toml Cargo.lock .gitignore LICENSE NOTICE README.md src tests/settings_defaults.rs
git commit -m "chore: scaffold native CodexSkinLite app"
gh repo create CodexSkinLite --private --source=. --remote=origin --push
```

Expected: a private GitHub repository exists and `main` tracks `origin/main`.

---

### Task 2: Application Paths and Atomic Settings Store

**Files:**
- Create: `src/paths.rs`
- Create: `src/settings.rs`
- Modify: `src/lib.rs`
- Test: `tests/settings_store.rs`

**Interfaces:**
- Consumes: `AppSettings` from Task 1.
- Produces: `AppPaths::discover() -> Result<AppPaths>`, `SettingsStore::load() -> Result<AppSettings>`, and `SettingsStore::save(&AppSettings) -> Result<()>`.

- [ ] **Step 1: Write failing normalization and atomic-write tests**

```rust
#[test]
fn invalid_width_and_port_are_normalized_without_losing_other_fields() {
    let dir = tempfile::tempdir().unwrap();
    let store = SettingsStore::new(AppPaths::for_test(dir.path()));
    std::fs::write(store.paths().settings_file(), r#"{
      "debugPort":0,"themeEnabled":true,"activeThemeId":"eva",
      "conversationCentered":true,"conversationMaxWidth":9000,
      "codexAppPath":"/Applications/Codex.app"
    }"#).unwrap();
    let value = store.load().unwrap();
    assert_eq!(value.debug_port, 9222);
    assert_eq!(value.conversation_max_width, 4000);
    assert!(value.theme_enabled);
}

#[test]
fn save_replaces_settings_atomically() {
    let dir = tempfile::tempdir().unwrap();
    let store = SettingsStore::new(AppPaths::for_test(dir.path()));
    store.save(&AppSettings::default()).unwrap();
    assert!(!store.paths().settings_temp_file().exists());
    assert!(store.paths().settings_file().exists());
}
```

- [ ] **Step 2: Verify failure**

Run: `cargo test --test settings_store`

Expected: FAIL because `AppPaths` and `SettingsStore` are undefined.

- [ ] **Step 3: Implement paths, normalization, and atomic replacement**

```rust
pub struct AppPaths {
    root: PathBuf,
    settings_file: PathBuf,
    themes_dir: PathBuf,
    logs_dir: PathBuf,
}

pub struct SettingsStore {
    paths: AppPaths,
}

impl SettingsStore {
    pub fn new(paths: AppPaths) -> Self;
    pub fn paths(&self) -> &AppPaths;
    pub fn load(&self) -> anyhow::Result<AppSettings>;
    pub fn save(&self, settings: &AppSettings) -> anyhow::Result<()>;
}
```

Derive `Clone` for `AppPaths`. Implement `AppPaths::discover() -> anyhow::Result<Self>` with `directories::ProjectDirs` and application name `CodexSkinLite`, plus `AppPaths::for_test(root: &Path) -> Self` for integration-test isolation. Expose read-only `root()`, `settings_file()`, `settings_temp_file()`, `themes_dir()`, and `logs_dir()` path accessors. Clamp width to 320-4000, replace port zero with 9222, trim an empty active theme ID to `None`, create directories on demand, `sync_all` the temporary file, then rename it over the destination.

- [ ] **Step 4: Run focused and full tests**

Run: `cargo test --test settings_store && cargo test`

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add src/paths.rs src/settings.rs src/lib.rs tests/settings_store.rs
git commit -m "feat: add atomic settings storage"
```

---

### Task 3: DreamSkin ZIP Model and Archive Validation

**Files:**
- Create: `src/theme/mod.rs`
- Create: `src/theme/package.rs`
- Create: `tests/fixtures.rs`
- Create: `tests/theme_package.rs`
- Modify: `src/lib.rs`

**Interfaces:**
- Consumes: raw ZIP bytes and the existing DreamSkin platform value `macos`; Apple Silicon is enforced by the application build target rather than by inventing a new package platform value.
- Produces: `validate_package(bytes: &[u8]) -> Result<ValidatedThemePackage, ThemeError>`, package structs with camelCase serde fields, and fixture helpers `theme_zip(options)` / `valid_theme_zip()`.

- [ ] **Step 1: Write failing valid-package and traversal tests**

```rust
#[test]
fn accepts_minimal_macos_package() {
    let bytes = fixtures::theme_zip(fixtures::ThemeZipOptions::valid());
    let package = validate_package(&bytes).unwrap();
    assert_eq!(package.manifest.theme_id, "eva-warm-cream");
    assert_eq!(package.image_name, "background.webp");
}

#[test]
fn rejects_parent_traversal_before_extracting() {
    let bytes = fixtures::theme_zip(fixtures::ThemeZipOptions {
        extra_entry: Some(("../escape.txt".into(), b"bad".to_vec())),
        ..fixtures::ThemeZipOptions::valid()
    });
    assert!(matches!(validate_package(&bytes), Err(ThemeError::UnsafePath(_))));
}
```

The fixture builder creates manifest, theme, CSS, image, and SHA-256 file declarations in memory so tests do not rely on opaque binary fixtures.

- [ ] **Step 2: Verify failure**

Run: `cargo test --test theme_package`

Expected: FAIL because theme package types are undefined.

- [ ] **Step 3: Implement bounded archive reading**

```rust
pub struct ValidatedThemePackage {
    pub manifest: DreamSkinPackageManifest,
    pub theme: serde_json::Value,
    pub css: String,
    pub image_name: String,
    pub image_bytes: Vec<u8>,
    pub license_text: Option<String>,
}

#[derive(Debug, thiserror::Error)]
pub enum ThemeError {
    #[error("unsafe ZIP path: {0}")] UnsafePath(String),
    #[error("unsupported package file: {0}")] UnsupportedFile(String),
    #[error("invalid manifest: {0}")] InvalidManifest(String),
    #[error("invalid image: {0}")] InvalidImage(String),
    #[error("invalid CSS: {0}")] InvalidCss(String),
    #[error("archive limit exceeded: {0}")] Limit(String),
    #[error("active theme cannot be deleted")] ActiveTheme,
    #[error("stored theme is incomplete: {0}")] InvalidStoredTheme(String),
    #[error(transparent)] Other(#[from] anyhow::Error),
}
```

Port the established limits: 32 MiB compressed package, 32 entries, 64 MiB total uncompressed, 256 KiB CSS, and exactly one supported background image. Reject links, encryption, absolute paths, parent traversal, unknown files, duplicate normalized names, and multiple background images before publishing any file.

In `tests/fixtures.rs`, define `ThemeZipOptions::valid()`, `theme_zip(options: ThemeZipOptions) -> Vec<u8>`, and `valid_theme_zip() -> Vec<u8>`; calculate manifest file hashes after constructing every non-manifest entry.

- [ ] **Step 4: Add schema, platform, image-magic, and hash cases**

Add tests for wrong platform, duplicate theme ID fields, malformed semantic version, incorrect SHA-256, PNG/JPEG/WebP magic mismatch, unsupported file, too many entries, and oversized content. Run: `cargo test --test theme_package`.

Expected: all package cases PASS.

- [ ] **Step 5: Commit**

```bash
git add src/theme src/lib.rs tests/fixtures.rs tests/theme_package.rs
git commit -m "feat: validate DreamSkin theme packages"
```

---

### Task 4: Safe CSS Parser and Compiler

**Files:**
- Create: `src/theme/safe_css.rs`
- Modify: `src/theme/mod.rs`
- Modify: `src/theme/package.rs`
- Test: `tests/safe_css.rs`

**Interfaces:**
- Consumes: UTF-8 `theme.css` from Task 3.
- Produces: `compile_safe_css(css: &str) -> Result<String, ThemeError>`.

- [ ] **Step 1: Write failing allowlist tests**

```rust
#[test]
fn compiles_registered_part_and_adds_trusted_priority() {
    let result = compile_safe_css(
        r#"[data-ds-part="main"] { background-color: rgba(255,250,240,0.65); backdrop-filter: blur(8px); }"#
    ).unwrap();
    assert!(result.contains("@layer dreamskin-community"));
    assert!(result.contains("backdrop-filter: blur(8px) !important"));
}

#[test]
fn rejects_arbitrary_selectors_and_remote_urls() {
    assert!(compile_safe_css("body div { color: red; }").is_err());
    assert!(compile_safe_css(r#"[data-ds-part="main"] { background-image: url(https://example.com/x); }"#).is_err());
}
```

- [ ] **Step 2: Verify failure**

Run: `cargo test --test safe_css`

Expected: FAIL because `compile_safe_css` is undefined.

- [ ] **Step 3: Implement parser and registered-part/property validation**

Accept only `root`, `sidebar`, `main`, `header`, `home`, `home-hero`, `project-list`, `thread`, `message`, `composer`, `composer-toolbar`, `composer-toolbar-empty`, and `dialog`, optionally followed by `:hover` or `:focus-visible`. Reject comments, escapes, control characters, nested braces, commas, at-rules, `!important` input, remote/local URLs, more than 128 rules, more than 512 declarations, and values longer than 512 characters.

```rust
pub fn compile_safe_css(css: &str) -> Result<String, ThemeError> {
    let rules = parse_safe_css(css)?;
    Ok(render_trusted_layer(&rules))
}
```

Implement property validators for colors, opacity, borders, radii, shadows, fonts, spacing, transitions, and `backdrop-filter` with bounded blur/saturation values. Add trusted `!important` only after parsing.

- [ ] **Step 4: Run parser tests and package integration test**

Run: `cargo test --test safe_css && cargo test --test theme_package`

Expected: PASS, including invalid CSS rejection through `validate_package`.

- [ ] **Step 5: Commit**

```bash
git add src/theme tests/safe_css.rs tests/theme_package.rs
git commit -m "feat: compile allowlisted DreamSkin CSS"
```

---

### Task 5: Atomic Theme Library

**Files:**
- Create: `src/theme/store.rs`
- Modify: `src/theme/mod.rs`
- Modify: `tests/fixtures.rs`
- Create: `tests/theme_store.rs`

**Interfaces:**
- Consumes: `AppPaths`, `ValidatedThemePackage`, and `compile_safe_css`.
- Produces: `ThemeStore::{import_zip,list,load_payload,delete}` and `ThemePayload`.

- [ ] **Step 1: Write failing import/switch/delete tests**

```rust
#[test]
fn import_publishes_complete_theme_atomically() {
    let env = fixtures::theme_environment();
    let summary = env.store.import_zip_bytes(&fixtures::valid_theme_zip()).unwrap();
    let dir = env.paths.themes_dir().join(&summary.id);
    assert!(dir.join("manifest.json").is_file());
    assert!(dir.join("compiled.css").is_file());
    assert!(!env.paths.themes_dir().join(".importing").exists());
}

#[test]
fn active_theme_cannot_be_deleted() {
    let env = fixtures::theme_environment_with_active("eva-warm-cream");
    assert!(matches!(env.store.delete("eva-warm-cream", Some("eva-warm-cream")), Err(ThemeError::ActiveTheme)));
}
```

- [ ] **Step 2: Verify failure**

Run: `cargo test --test theme_store`

Expected: FAIL because `ThemeStore` is undefined.

- [ ] **Step 3: Implement immutable theme directories and payload loading**

```rust
#[derive(Debug, Clone, serde::Serialize)]
pub struct ThemeSummary { pub id: String, pub name: String, pub version: String }

#[derive(Debug, Clone, serde::Serialize)]
pub struct ThemePayload {
    pub id: String,
    pub signature: String,
    pub theme: serde_json::Value,
    pub compiled_css: String,
    pub image_mime: String,
    pub image_base64: String,
}

impl ThemeStore {
    pub fn new(paths: AppPaths) -> Self;
    pub fn import_zip_bytes(&self, bytes: &[u8]) -> Result<ThemeSummary, ThemeError>;
    pub fn list(&self) -> Result<Vec<ThemeSummary>, ThemeError>;
    pub fn load_payload(&self, id: &str) -> Result<ThemePayload, ThemeError>;
    pub fn delete(&self, id: &str, active_id: Option<&str>) -> Result<(), ThemeError>;
}
```

Write into a `tempfile::TempDir` under `themes/`, sync files, reject unknown existing destination types, then atomically rename. Compute payload signature from theme JSON, compiled CSS, image digest, and package version. Extend `tests/fixtures.rs` with `theme_environment()` and `theme_environment_with_active(id)` constructors backed by a temporary `AppPaths` root.

- [ ] **Step 4: Test replacement rollback and traversal-resistant IDs**

Add cases for a failed replacement preserving the old theme, invalid theme IDs, missing stored file, symlinked theme directory, deterministic listing, and deterministic payload signature. Run: `cargo test --test theme_store`.

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add src/theme tests/fixtures.rs tests/theme_store.rs
git commit -m "feat: add atomic local theme library"
```

---

### Task 6: CDP Discovery and Target Validation

**Files:**
- Create: `src/cdp/mod.rs`
- Create: `src/cdp/discovery.rs`
- Modify: `src/lib.rs`
- Create: `tests/cdp_discovery.rs`

**Interfaces:**
- Consumes: loopback debug port.
- Produces: `async fn list_targets(port: u16) -> Result<Vec<CdpTarget>>`, `pick_primary_target(&[CdpTarget]) -> Result<CdpTarget>`, and `async fn endpoint_available(port: u16) -> bool`.

- [ ] **Step 1: Write failing target-classification tests**

```rust
#[test]
fn selects_primary_codex_target_and_rejects_quick_chat() {
    let targets = vec![
        target("Quick Chat", "file:///quick-chat.html", "page"),
        target("Codex", "file:///index.html", "page"),
    ];
    assert_eq!(pick_primary_target(&targets).unwrap().title, "Codex");
}

fn target(title: &str, url: &str, kind: &str) -> CdpTarget {
    CdpTarget {
        id: title.to_lowercase().replace(' ', "-"),
        title: title.into(),
        url: url.into(),
        kind: kind.into(),
        web_socket_debugger_url: Some("ws://127.0.0.1:9222/devtools/page/1".into()),
    }
}

#[test]
fn rejects_non_loopback_websocket_url() {
    let value = "ws://192.168.1.10:9222/devtools/page/1";
    assert!(validate_websocket_url(value, 9222).is_err());
}
```

- [ ] **Step 2: Verify failure**

Run: `cargo test --test cdp_discovery`

Expected: FAIL because CDP types are undefined.

- [ ] **Step 3: Implement loopback HTTP discovery and deterministic selection**

```rust
#[derive(Debug, Clone, serde::Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CdpTarget {
    pub id: String,
    pub title: String,
    pub url: String,
    #[serde(rename = "type")]
    pub kind: String,
    pub web_socket_debugger_url: Option<String>,
}
```

Use a reqwest client with proxy disabled, short connect/read timeouts, and URL `http://127.0.0.1:{port}/json/list`. Accept only page targets with loopback WebSocket URLs and known Codex file/app-shell URL/title characteristics. Prefer the exact main target over fallbacks.

- [ ] **Step 4: Run focused tests with a local fake HTTP endpoint**

Add an async test server bound to `127.0.0.1:0` that returns target JSON and assert endpoint availability, timeout behavior, malformed JSON rejection, and no proxy use. Run: `cargo test --test cdp_discovery`.

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add src/cdp src/lib.rs tests/cdp_discovery.rs
git commit -m "feat: discover and validate Codex CDP targets"
```

---

### Task 7: CDP Protocol, Session, and Renderer Injection

**Files:**
- Create: `src/cdp/protocol.rs`
- Create: `src/cdp/session.rs`
- Modify: `src/cdp/mod.rs`
- Modify: `tests/fixtures.rs`
- Create: `tests/cdp_session.rs`

**Interfaces:**
- Consumes: validated target WebSocket URL and renderer bootstrap string.
- Produces: `CdpSession::{connect,install_bootstrap,evaluate,apply_payload,close}` and `ReconnectBackoff`.

- [ ] **Step 1: Write failing command-correlation and backoff tests**

```rust
#[tokio::test]
async fn correlates_out_of_order_cdp_results_by_id() {
    let server = fixtures::fake_cdp_server().await;
    let mut session = CdpSession::connect(server.url()).await.unwrap();
    let (left, right) = tokio::join!(session.evaluate("1+1"), session.evaluate("2+2"));
    assert_eq!(left.unwrap(), serde_json::json!(2));
    assert_eq!(right.unwrap(), serde_json::json!(4));
}

#[test]
fn reconnect_backoff_is_bounded_and_resets() {
    let mut value = ReconnectBackoff::default();
    assert_eq!(value.next_delay(), Duration::from_millis(250));
    for _ in 0..20 { value.next_delay(); }
    assert_eq!(value.current(), Duration::from_secs(30));
    value.reset();
    assert_eq!(value.current(), Duration::from_millis(250));
}
```

- [ ] **Step 2: Verify failure**

Run: `cargo test --test cdp_session`

Expected: FAIL because session types are undefined.

- [ ] **Step 3: Implement the CDP request loop**

```rust
pub struct CdpSession { /* writer, pending map, reader task, next id */ }

impl CdpSession {
    pub async fn connect(url: &str) -> anyhow::Result<Self>;
    pub async fn evaluate(&self, expression: &str) -> anyhow::Result<serde_json::Value>;
    pub async fn install_bootstrap(&self, script: &str) -> anyhow::Result<()>;
    pub async fn apply_payload(&self, payload: &serde_json::Value) -> anyhow::Result<serde_json::Value>;
    pub async fn close(self) -> anyhow::Result<()>;
}
```

`install_bootstrap` sends `Page.addScriptToEvaluateOnNewDocument`, then `Runtime.evaluate` for the current document. `apply_payload` serializes the payload once to JSON, serializes that JSON string again as a JavaScript string literal, and evaluates the fixed expression `window.__CODEX_SKIN_LITE__.apply(JSON.parse(quoted_json))` with promise awaiting disabled. Theme content never becomes executable source.

Extend `tests/fixtures.rs` with `async fn fake_cdp_server() -> FakeCdpServer`; it must bind `127.0.0.1:0`, expose `url()`, record received methods, support scripted out-of-order responses/events, and close on drop.

- [ ] **Step 4: Test disconnect and pending-request failure**

Add fake-server cases for events interleaved with results, malformed messages, socket close failing all pending requests, duplicate close, and bootstrap registration order. Run: `cargo test --test cdp_session`.

Expected: PASS with no hung tests.

- [ ] **Step 5: Commit**

```bash
git add src/cdp tests/fixtures.rs tests/cdp_session.rs
git commit -m "feat: add bounded Codex CDP session"
```

---

### Task 8: macOS Codex Launcher and Explicit Restart Decisions

**Files:**
- Create: `src/launcher.rs`
- Modify: `src/lib.rs`
- Create: `tests/launcher.rs`

**Interfaces:**
- Consumes: `AppSettings`, process state, and CDP endpoint state.
- Produces: `LaunchDecision`, `build_open_command`, `MacCodexLauncher::{inspect,launch,terminate_after_confirmation}`.

- [ ] **Step 1: Write failing decision and command tests**

```rust
#[test]
fn running_without_cdp_requires_confirmation() {
    assert_eq!(decide_launch(true, false), LaunchDecision::RestartConfirmationRequired);
}

#[test]
fn command_binds_debugging_to_loopback() {
    let cmd = build_open_command(Path::new("/Applications/Codex.app"), 9222);
    assert_eq!(&cmd[..5], &[
        "open".to_string(), "-W".to_string(), "-a".to_string(),
        "/Applications/Codex.app".to_string(), "--args".to_string(),
    ]);
    assert!(cmd.contains(&"--remote-debugging-address=127.0.0.1".to_string()));
    assert!(cmd.contains(&"--remote-debugging-port=9222".to_string()));
}
```

- [ ] **Step 2: Verify failure**

Run: `cargo test --test launcher`

Expected: FAIL because launcher APIs are undefined.

- [ ] **Step 3: Implement launcher policy and macOS adapter**

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LaunchDecision { Launch, Attach, RestartConfirmationRequired }

pub fn decide_launch(process_running: bool, cdp_available: bool) -> LaunchDecision;
pub fn build_open_command(app: &Path, port: u16) -> Vec<String>;
```

Use `NSWorkspace`/`NSRunningApplication` for process inspection and explicit termination. Validate that the selected path is a directory ending in `.app` with `Contents/MacOS/Codex`. Do not terminate from `inspect` or `launch`; expose termination only through the confirmed action.

- [ ] **Step 4: Test invalid paths and cancellation safety**

Add tests using a `ProcessInspector`/`CommandRunner` fake to prove that cancellation performs zero terminate/launch calls, invalid bundle path fails before process changes, attach performs no launch, and confirmed restart waits for exit before opening. Run: `cargo test --test launcher`.

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add src/launcher.rs src/lib.rs tests/launcher.rs
git commit -m "feat: add explicit macOS Codex launch flow"
```

---

### Task 9: Renderer Test Harness and Bootstrap Contract

**Files:**
- Create: `renderer/package.json`
- Create: `renderer/vitest.config.mjs`
- Create: `renderer/tests/dom-fixtures.js`
- Create: `renderer/tests/runtime.test.js`
- Create: `assets/renderer/runtime.js`
- Create: `src/renderer.rs`
- Modify: `.gitignore`

**Interfaces:**
- Consumes: `RendererPayload` serialized by Rust.
- Produces: `window.__CODEX_SKIN_LITE__.apply(payload)`, `.status()`, and `.cleanup()` plus `renderer::bootstrap_script()`.

- [ ] **Step 1: Create the failing JavaScript bootstrap test**

```javascript
import { describe, expect, it } from "vitest";
import { installRuntime } from "./dom-fixtures.js";

describe("bootstrap", () => {
  it("installs one idempotent API", () => {
    installRuntime();
    const first = window.__CODEX_SKIN_LITE__;
    installRuntime();
    expect(window.__CODEX_SKIN_LITE__).toBe(first);
    expect(typeof first.apply).toBe("function");
    expect(typeof first.cleanup).toBe("function");
  });
});
```

- [ ] **Step 2: Install test-only JavaScript dependencies and verify failure**

Run inside `renderer/`: `npm install --save-dev vitest happy-dom && npm test`

Expected: FAIL because the runtime and fixture loader are missing. Commit `package-lock.json`; Node dependencies are development-only and never bundled in the app.

- [ ] **Step 3: Implement bootstrap and Rust payload contract**

```rust
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RendererPayload {
    pub revision: u64,
    pub theme_enabled: bool,
    pub theme: Option<ThemePayload>,
    pub conversation_centered: bool,
    pub conversation_max_width: u16,
}

pub fn bootstrap_script() -> &'static str {
    include_str!("../assets/renderer/runtime.js")
}
```

The JavaScript IIFE returns early when the same API version already exists. Store all runtime state under one non-enumerable `window.__CODEX_SKIN_LITE__` object. `apply` ignores payload revisions older than the current revision.

- [ ] **Step 4: Run Rust and JavaScript tests**

Run: `npm --prefix renderer test && cargo test`

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add renderer assets/renderer src/renderer.rs .gitignore Cargo.lock
git commit -m "feat: add idempotent renderer bootstrap"
```

---

### Task 10: Skin API Registry and Theme Presentation

**Files:**
- Modify: `assets/renderer/runtime.js`
- Modify: `renderer/tests/dom-fixtures.js`
- Modify: `renderer/tests/runtime.test.js`

**Interfaces:**
- Consumes: modern/classic Codex DOM fixtures and `ThemePayload`.
- Produces: stable `data-ds-part` markers, one managed style element, one managed image Blob URL, and reversible cleanup.

- [ ] **Step 1: Add failing semantic mapping tests**

```javascript
it("maps the complete main viewport and both side panels", async () => {
  const ui = installRuntime({ fixture: "modernThreadWithRightPanel" });
  await ui.applyTheme(evaPayload());
  expect(ui.main.dataset.dsPart).toBe("main");
  expect(ui.header.dataset.dsPart).toBe("header");
  expect(ui.thread.dataset.dsPart).toBe("thread");
  expect(ui.leftSidebar.dataset.dsPart).toBe("sidebar");
  expect(ui.rightSidebar.dataset.dsPart).toBe("sidebar");
});

it("replaces one managed style and revokes the previous blob", async () => {
  const ui = installRuntime();
  await ui.applyTheme(evaPayload());
  await ui.applyTheme(blueEyesPayload());
  expect(document.querySelectorAll("#codex-skin-lite-theme")).toHaveLength(1);
  expect(ui.revokedBlobUrls).toHaveLength(1);
});
```

- [ ] **Step 2: Verify failure**

Run: `npm --prefix renderer test`

Expected: FAIL because no parts or managed theme style exist.

- [ ] **Step 3: Implement cached part discovery and theme lifecycle**

Implement a `registry` object keyed by the exact part names in the spec. Resolve the smallest stable application shell first, then query descendants using the currently verified selectors for modern and classic Codex. Keep connected cached nodes; remove and rediscover only disconnected or affected entries. Assign the same `sidebar` part to left navigation and right contextual panels.

```javascript
const PARTS = ["root", "sidebar", "main", "header", "home", "home-hero",
  "project-list", "thread", "message", "composer", "composer-toolbar",
  "composer-toolbar-empty", "dialog"];
const registry = new Map(PARTS.map((part) => [part, new Set()]));
function reconcileParts(affectedRoot) { /* query affectedRoot, retain connected cached nodes, diff attributes */ }
```

Theme application must create or update `#codex-skin-lite-theme`, set documented `--ds-theme-*` variables on the root, decode image bytes once, create a Blob URL, and clear the base64 reference. Cleanup removes only known attributes, variables, style nodes, and Blob URLs owned by this runtime.

- [ ] **Step 4: Add home/thread transition and cleanup cases**

Test new-conversation home, existing thread, route transition replacing the main node, header independence, no unnamed gap surface, duplicate apply idempotence, and cleanup preserving unrelated attributes/styles. Run: `npm --prefix renderer test`.

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add assets/renderer/runtime.js renderer/tests
git commit -m "feat: map stable Codex Skin API parts"
```

---

### Task 11: Shared Observer and Centered Conversation Width

**Files:**
- Modify: `assets/renderer/runtime.js`
- Modify: `renderer/tests/dom-fixtures.js`
- Modify: `renderer/tests/runtime.test.js`
- Create: `scripts/measure-renderer.mjs`

**Interfaces:**
- Consumes: cached registry and width settings.
- Produces: one coalesced layout scheduler, one MutationObserver, one on-demand ResizeObserver, and reversible centered width.

- [ ] **Step 1: Add failing observer and width tests**

```javascript
it("shares one observer and does not refresh on scroll or caret activity", async () => {
  const ui = installRuntime({ fixture: "modernThread" });
  await ui.enableCenteredWidth(920);
  const before = ui.status().metrics.layoutPasses;
  ui.thread.dispatchEvent(new Event("scroll"));
  ui.composer.dispatchEvent(new Event("selectionchange"));
  await ui.nextFrame();
  expect(ui.status().metrics.layoutPasses).toBe(before);
  expect(ui.observerCounts()).toEqual({ mutation: 1, resize: 1, intervals: 0 });
});

it("centers content and composer together and restores owned styles", async () => {
  const ui = installRuntime({ fixture: "modernThreadWithRightPanel" });
  await ui.enableCenteredWidth(900);
  expect(ui.content.style.maxWidth).toBe("900px");
  expect(ui.composer.style.maxWidth).toBe("900px");
  await ui.disableCenteredWidth();
  expect(ui.content.getAttribute("style")).toBe(ui.originalContentStyle);
});
```

- [ ] **Step 2: Verify failure**

Run: `npm --prefix renderer test`

Expected: FAIL because scheduler, metrics, and width runtime are absent.

- [ ] **Step 3: Implement filtered mutation batching and width ownership**

Create one `schedule(reason)` function that coalesces relevant records into one `requestAnimationFrame`. Observe the smallest stable app shell for child-list changes and only the documented layout attributes. Attach one ResizeObserver to current main/content/composer/side-panel nodes and update observed targets when cached identities change. Never create an interval.

```javascript
function schedule(reason) {
  state.pendingReasons.add(reason);
  if (state.rafId) return;
  state.rafId = requestAnimationFrame(() => {
    state.rafId = 0;
    reconcileLayout(state.pendingReasons);
    state.pendingReasons.clear();
  });
}
```

Store each changed inline property and its original value in a WeakMap before writing `box-sizing`, `width`, `max-width`, `margin-left`, `margin-right`, and any owned visual-center offset. Compute the available main viewport after active side-panel geometry and apply the same center to content and composer.

- [ ] **Step 4: Add mutation coalescing and node-replacement tests**

Test 100 same-frame mutations causing one pass, message text mutation causing zero full scans, content/composer replacement, resize, side-panel open/close, width clamping, repeated enable, and disable restoration. Add `scripts/measure-renderer.mjs` to print uncompressed byte size and test metrics; fail when runtime exceeds 100,000 bytes or contains `setInterval(`.

Run: `npm --prefix renderer test && node scripts/measure-renderer.mjs`.

Expected: PASS and size below 100,000 bytes.

- [ ] **Step 5: Commit**

```bash
git add assets/renderer/runtime.js renderer/tests scripts/measure-renderer.mjs
git commit -m "perf: share renderer layout lifecycle"
```

---

### Task 12: Composer Compatibility, Attachments, and Flicker Regression

**Files:**
- Modify: `assets/renderer/runtime.js`
- Modify: `renderer/tests/dom-fixtures.js`
- Modify: `renderer/tests/runtime.test.js`

**Interfaces:**
- Consumes: known modern Codex composer structure, side-panel state, and active theme state.
- Produces: empty-toolbar part transitions and optional reversible composer adapter.

- [ ] **Step 1: Add failing regressions from the verified bugs**

```javascript
it("marks empty toolbar without creating an attachment divider", async () => {
  const ui = installRuntime({ fixture: "composerWithoutAttachments" });
  await ui.applyTheme(evaPayload());
  expect(ui.toolbar.dataset.dsPart).toBe("composer-toolbar-empty");
  expect(ui.composer.querySelectorAll("[data-csl-divider]")).toHaveLength(0);
});

it("focused scrolling never clears the managed background", async () => {
  const ui = installRuntime({ fixture: "longFocusedThread" });
  await ui.applyTheme(evaPayload());
  for (let i = 0; i < 100; i += 1) ui.scrollAndStreamOneMutation();
  await ui.nextFrame();
  expect(ui.styleRemovalCount()).toBe(0);
  expect(ui.status().metrics.fullScansDuringScroll).toBe(0);
});

it("restores composer when a known docking attempt fails", async () => {
  const ui = installRuntime({ fixture: "brokenDockTarget" });
  const originalParent = ui.composer.parentElement;
  await ui.applyTheme(evaPayload());
  expect(ui.composer.parentElement).toBe(originalParent);
});
```

- [ ] **Step 2: Verify failure**

Run: `npm --prefix renderer test`

Expected: at least the empty-toolbar and adapter status assertions FAIL.

- [ ] **Step 3: Implement attachment-state mapping and guarded adapter**

Set `composer-toolbar-empty` only when the recognized attachment container exists and has no children. Do not synthesize dividers. Prefer native composer layout. Activate the adapter only for a versioned, known-incompatible signature and only when source, destination, and restoration anchors are connected. Record original parent, next sibling, and owned style values before mutation; catch every failure and synchronously restore.

```javascript
function shouldAdaptComposer(layout) {
  return layout.signature === KNOWN_SCROLLING_COMPOSER_SIGNATURE
    && layout.source?.isConnected
    && layout.destination?.isConnected
    && layout.restoreParent?.isConnected;
}
```

- [ ] **Step 4: Run all renderer regressions repeatedly**

Run: `for i in {1..20}; do npm --prefix renderer test -- --run >/dev/null || exit 1; done && node scripts/measure-renderer.mjs`

Expected: 20 clean passes, no style-clear frame in fixture instrumentation, and runtime below size limit.

- [ ] **Step 5: Commit**

```bash
git add assets/renderer/runtime.js renderer/tests
git commit -m "fix: stabilize composer theme compatibility"
```

---

### Task 13: Application Controller and Transactional Live Updates

**Files:**
- Create: `src/controller.rs`
- Modify: `src/model.rs`
- Modify: `src/lib.rs`
- Modify: `tests/fixtures.rs`
- Create: `tests/controller.rs`

**Interfaces:**
- Consumes: settings/theme stores, launcher, discovery, session, and renderer payload builder.
- Produces: `AppCommand`, `ControllerHandle`, `UiSink`, transactional theme/width actions, and `AppSnapshot` publication.

- [ ] **Step 1: Write failing transactional tests with fakes**

```rust
#[tokio::test]
async fn failed_theme_apply_keeps_previous_active_theme() {
    let env = fixtures::controller_environment().with_active_theme("eva").with_cdp_apply_failure();
    let result = env.controller.handle(AppCommand::ActivateTheme("blue-eyes".into())).await;
    assert!(result.is_err());
    assert_eq!(env.settings.load().unwrap().active_theme_id.as_deref(), Some("eva"));
}

#[tokio::test]
async fn width_persists_only_after_renderer_verification() {
    let env = fixtures::controller_environment().with_renderer_revision_ack(7);
    env.controller.handle(AppCommand::SetConversationWidth(880)).await.unwrap();
    assert_eq!(env.settings.load().unwrap().conversation_max_width, 880);
}
```

- [ ] **Step 2: Verify failure**

Run: `cargo test --test controller`

Expected: FAIL because controller APIs are undefined.

- [ ] **Step 3: Implement command/state interfaces**

```rust
pub enum AppCommand {
    OpenCodex,
    ConfirmRestart,
    Reconnect,
    ImportTheme(PathBuf),
    ActivateTheme(String),
    SetThemeEnabled(bool),
    DeleteTheme(String),
    SetConversationCentered(bool),
    SetConversationWidth(u16),
    SetCodexPath(PathBuf),
    Shutdown,
}

pub trait UiSink: Send + Sync {
    fn publish(&self, snapshot: AppSnapshot);
    fn report_error(&self, title: &str, message: &str);
}

impl ControllerHandle {
    pub fn send(&self, command: AppCommand) -> anyhow::Result<()>;
}
```

The controller owns one monotonically increasing payload revision. `ActivateTheme(id)` selects and enables a validated theme; `SetThemeEnabled(false)` disables presentation while preserving the selected ID, and `SetThemeEnabled(true)` requires an existing selected theme. Build and verify a renderer payload before saving active theme or width settings. On failure, reapply the previous payload when a session remains available, then publish the unchanged snapshot. Use reconnect backoff only after an actual disconnect.

Extend `tests/fixtures.rs` with `controller_environment() -> ControllerTestEnvironment` and fluent methods `with_active_theme`, `with_cdp_apply_failure`, and `with_renderer_revision_ack`; the environment owns fake launcher, CDP, settings, theme store, and a recording `UiSink`.

- [ ] **Step 4: Test launch decisions, reconnect, rollback, and independent cleanup**

Add cases for attach, restart-required snapshot, cancellation, import without activation, theme disable preserving width, width disable preserving theme, stale renderer acknowledgment, disconnect, reconnect reset, and shutdown. Run: `cargo test --test controller`.

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add src/controller.rs src/model.rs src/lib.rs tests/fixtures.rs tests/controller.rs
git commit -m "feat: coordinate transactional live updates"
```

---

### Task 14: Native AppKit Status Item and Settings Window

**Files:**
- Create: `src/macos/mod.rs`
- Create: `src/macos/app_delegate.rs`
- Create: `src/macos/menu.rs`
- Create: `src/macos/settings_window.rs`
- Modify: `src/main.rs`
- Modify: `src/lib.rs`

**Interfaces:**
- Consumes: `ControllerHandle`, `AppCommand`, `AppSnapshot`, `UiSink`.
- Produces: `macos::run(controller) -> !` and a native settings UI with no WebView.

- [ ] **Step 1: Add a compile-time native UI smoke test**

```rust
#[cfg(target_os = "macos")]
#[test]
fn menu_model_exposes_only_approved_actions() {
    assert_eq!(MenuAction::ALL, [
        MenuAction::OpenCodex,
        MenuAction::Reconnect,
        MenuAction::OpenSettings,
        MenuAction::Quit,
    ]);
}
```

Run: `cargo test menu_model_exposes_only_approved_actions`.

Expected: FAIL because the macOS module and menu model are undefined.

- [ ] **Step 2: Implement main-thread AppKit lifecycle**

Use `objc2`, `objc2-foundation`, and `objc2-app-kit` to create `NSApplication`, an `NSStatusItem`, `NSMenu`, and one retained settings `NSWindow`. The AppKit main thread constructs controls; a Tokio runtime runs on a background thread. `dispatch2` schedules `UiSink` snapshot updates back onto the main queue.

```rust
pub fn run(controller: ControllerHandle) -> !;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MenuAction { OpenCodex, Reconnect, OpenSettings, Quit }
```

Do not show a Dock icon. Do not create a WebView. Hold Objective-C delegates and targets for the full application lifetime.

- [ ] **Step 3: Implement the three approved settings sections**

Build native labels, switches, text fields, popup buttons, and buttons for Appearance, Layout, and Codex. Convert control callbacks directly into `AppCommand`. Use `NSOpenPanel` restricted to ZIP files for theme import and to `.app` directories for Codex selection. Use `NSAlert` for restart confirmation and errors.

```rust
fn dispatch_control_action(controller: &ControllerHandle, action: ControlAction) {
    match action {
        ControlAction::ThemeEnabled(value) => controller.send(AppCommand::SetThemeEnabled(value)),
        ControlAction::Centered(value) => controller.send(AppCommand::SetConversationCentered(value)),
        ControlAction::Width(value) => controller.send(AppCommand::SetConversationWidth(value)),
        ControlAction::Reconnect => controller.send(AppCommand::Reconnect),
    }
}
```

- [ ] **Step 4: Build and run manual AppKit smoke checks**

Run: `cargo test && cargo build --target aarch64-apple-darwin && cargo run --target aarch64-apple-darwin`.

Expected: status item appears; Settings opens once and re-focuses; controls mirror the current snapshot; Cancel never sends `ConfirmRestart`; Quit shuts down the controller and exits cleanly. Record the smoke result in the commit message body.

- [ ] **Step 5: Commit**

```bash
git add src/macos src/main.rs src/lib.rs
git commit -m "feat: add native macOS status interface"
```

---

### Task 15: Diagnostics, CI, Packaging, and Real Codex Acceptance

**Files:**
- Create: `src/diagnostics.rs`
- Modify: `src/main.rs`
- Create: `.github/workflows/ci.yml`
- Create: `resources/Info.plist`
- Create: `scripts/package-app.sh`
- Create: `docs/acceptance/latest-codex.md`
- Modify: `README.md`

**Interfaces:**
- Consumes: complete application and renderer metrics.
- Produces: rotated local logs, CI gates, `dist/CodexSkinLite.app.zip`, checksum, and a filled real-Codex acceptance record.

- [ ] **Step 1: Write failing redaction and rotation tests**

```rust
#[test]
fn diagnostics_redact_conversation_and_credentials() {
    let event = DiagnosticEvent::cdp_error(
        "Authorization: Bearer secret prompt=private conversation",
    );
    let rendered = event.safe_message();
    assert!(!rendered.contains("secret"));
    assert!(!rendered.contains("private conversation"));
}
```

Run: `cargo test diagnostics_redact_conversation_and_credentials`.

Expected: FAIL because diagnostics are undefined.

- [ ] **Step 2: Implement local-only diagnostics and performance counters**

Log lifecycle, target identity class, payload revision, compatibility status, duration, and error category. Never log CDP expression contents, conversation text, prompt text, credentials, image bytes, or raw theme ZIP bytes. Rotate at 1 MiB and retain three files. Expose renderer metrics through `.status()` and write a one-shot performance report only when the user selects diagnostics.

```rust
pub fn init_local_logging(paths: &AppPaths) -> anyhow::Result<DiagnosticsGuard>;
pub fn sanitize_diagnostic_message(input: &str) -> String;
pub async fn capture_performance_report(session: &CdpSession) -> anyhow::Result<PerformanceReport>;
```

- [ ] **Step 3: Add CI gates**

Create a GitHub Actions workflow with `runs-on: macos-15`, which is an arm64 M1 standard runner. Begin with `uname -m | grep -qx arm64`, install Rust 1.97.1 through rustup, and then run:

```yaml
- run: cargo fmt --check
- run: cargo clippy --all-targets -- -D warnings
- run: cargo test --all-targets
- run: npm ci --prefix renderer
- run: npm test --prefix renderer
- run: node scripts/measure-renderer.mjs
- run: cargo build --release --target aarch64-apple-darwin
```

Cache Cargo registry/target and npm cache, but do not upload user themes or logs.

- [ ] **Step 4: Implement unsigned `.app.zip` packaging**

`scripts/package-app.sh` must:

1. Require `uname -m` to equal `arm64`.
2. Run all Rust and renderer gates.
3. Build release for `aarch64-apple-darwin`.
4. Create `dist/CodexSkinLite.app/Contents/{MacOS,Resources}`.
5. Copy the binary and static `Info.plist` without modifying Codex.app.
6. Use `ditto -c -k --sequesterRsrc --keepParent` to produce `CodexSkinLite.app.zip`.
7. Write `shasum -a 256` output to `CodexSkinLite.app.zip.sha256`.

```bash
#!/bin/zsh
set -euo pipefail
[[ "$(uname -m)" == "arm64" ]]
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test --all-targets
npm ci --prefix renderer
npm test --prefix renderer
node scripts/measure-renderer.mjs
cargo build --release --target aarch64-apple-darwin
stage_dir="$(mktemp -d)"
trap 'rm -rf "$stage_dir"' EXIT
app_dir="$stage_dir/CodexSkinLite.app"
mkdir -p "$app_dir/Contents/MacOS" "$app_dir/Contents/Resources"
cp target/aarch64-apple-darwin/release/codex-skin-lite "$app_dir/Contents/MacOS/CodexSkinLite"
cp resources/Info.plist "$app_dir/Contents/Info.plist"
mkdir -p dist
ditto -c -k --sequesterRsrc --keepParent "$app_dir" dist/CodexSkinLite.app.zip
shasum -a 256 dist/CodexSkinLite.app.zip > dist/CodexSkinLite.app.zip.sha256
```

Run: `bash scripts/package-app.sh`.

Expected: ZIP and checksum exist; `file` reports an arm64 Mach-O binary; archive contains no `node_modules`, tests, theme fixtures, or signing identity.

- [ ] **Step 5: Execute the real Codex acceptance matrix**

Use the latest installed official Codex and test every row in the design spec: new conversation, existing conversation, focused/unfocused scrolling, streaming output, attachments, right panel, resize/full screen, theme switch/disable, width update/disable, Codex restart/reconnect, and utility restart/reconnect.

Fill `docs/acceptance/latest-codex.md` with:

- Codex version/build and macOS version.
- Git commit under test.
- Theme package IDs used.
- Pass/fail for every matrix row.
- Renderer script bytes.
- Layout refresh mean and P95.
- 60-second idle CPU average and RSS.
- Confirmation that no clear-frame flicker, width reversal, composer scrolling, side-panel overlap, or empty-toolbar divider was observed.

Do not mark a failed row as accepted. Fix failures through a new test-first commit, rerun the full matrix, and replace the acceptance record with the successful run.

- [ ] **Step 6: Final verification and commit**

Run:

```bash
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test --all-targets
npm ci --prefix renderer
npm test --prefix renderer
node scripts/measure-renderer.mjs
bash scripts/package-app.sh
git status --short
```

Expected before commit: every command PASS, the acceptance record has no failed row, release artifacts exist, and `git status --short` lists only the intended tracked source/documentation changes because `dist/` is ignored.

```bash
git add src/diagnostics.rs src/main.rs .github resources scripts docs/acceptance README.md renderer/package-lock.json
git commit -m "release: verify CodexSkinLite on latest Codex"
git push origin main
git status --short
```

Expected after commit: `git status --short` is empty and private GitHub CI is green. Do not change repository visibility or create a public release without separate user authorization.
