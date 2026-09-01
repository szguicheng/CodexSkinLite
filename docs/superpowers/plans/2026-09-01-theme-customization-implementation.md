# CodexSkinLite Theme Customization Implementation Plan

> For agentic workers: REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (- [ ]) syntax for tracking.

Goal: Add automatic Settings startup, a DreamSkin gallery link, and a bounded per-theme customization editor with preview, persistence, and stable composer positioning.

Architecture: Keep the existing Rust/AppKit controller and CDP-injected renderer. Store an optional normalized customization.json beside each imported theme, generate only allowlisted CSS from typed values, and pass the customization through the existing versioned theme payload. The editor keeps a native draft, Preview applies an in-memory candidate, and Save applies/atomically persists the candidate without moving the React-owned composer.

Tech Stack: Rust 2024, Tokio, serde/serde_json, objc2/AppKit, existing DreamSkin ZIP and Safe CSS modules, JavaScript ES2022, Vitest, happy-dom, Apple Silicon release packaging.

Spec: docs/superpowers/specs/2026-09-01-theme-customization-design.md and the base architecture in docs/superpowers/specs/2026-08-31-codex-skin-lite-design.md.

## Global Constraints

- Target only aarch64-apple-darwin; reject unsupported platforms at compile time.
- Keep the customization editor native AppKit; do not add Tauri, React, Vite, an embedded WebView, or an arbitrary CSS editor.
- Preserve the current DreamSkin ZIP package format and stable data-ds-part API.
- Bind CDP only to 127.0.0.1 and validate the selected Codex target.
- Keep the renderer below 100 KB uncompressed, with one coalesced MutationObserver, at most one ResizeObserver, and no renderer setInterval.
- Keep theme, customization, centered-width, and composer cleanup reversible; restore only state owned by CodexSkinLite.
- Store user data below ~/Library/Application Support/CodexSkinLite using atomic writes.
- A customization file is optional and is not part of the ZIP manifest.
- Customization values are limited to the approved ranges: colors #RGB/#RGBA/#RRGGBB/#RRGGBBAA, opacity 65..=100, blur 0..=30 px, radius 0..=28 px, background position 0..=100, composer bottom inset 0..=80 px, and horizontal inset 0..=120 px.
- The composer footer remains inside the original React thread scroll tree; no reparenting, cloning, or global window-coordinate offsets are allowed.
- Keep the test set focused on persistence, payload generation, preview/save semantics, cleanup, and one real Codex acceptance pass.

---

## File Map Before Implementation

The implementation follows the repository's existing boundaries:

- Create src/theme/customization.rs for typed customization values, normalization, shadow presets, and generated Safe CSS.
- Modify src/theme/mod.rs to expose customization types and the compiler.
- Modify src/theme/package.rs to report invalid customization data distinctly.
- Modify src/theme/store.rs to load, atomically save, reset, and embed per-theme customization in ThemePayload.
- Modify src/model.rs to expose the active normalized customization in AppSnapshot.
- Modify src/controller.rs to add Preview/Save commands, transient preview state, and verified renderer application.
- Modify src/macos/mod.rs to carry the customization draft and selected component between native controls.
- Modify src/macos/app_delegate.rs to open Settings on every launch, open the gallery, and route editor actions.
- Modify src/macos/settings_window.rs to add the gallery and customization buttons.
- Create src/macos/customization_window.rs for the native scrollable editor and draft controls.
- Modify assets/renderer/runtime.js to apply custom colors, background focus, surface rules, and local composer insets.
- Modify renderer/tests/dom-fixtures.js and renderer/tests/runtime.test.js for the small renderer regression set.
- Create tests/theme_customization.rs and extend tests/theme_store.rs and tests/controller.rs for Rust persistence and controller semantics.
- Modify README.md, docs/acceptance/latest-codex.md, Cargo.toml, and resources/Info.plist for user-facing behavior and the new release version.

---

### Task 1: Typed Customization Model and Per-Theme Persistence

Files:
- Create: src/theme/customization.rs
- Modify: src/theme/mod.rs
- Modify: src/theme/package.rs
- Modify: src/theme/store.rs
- Test: tests/theme_customization.rs
- Test: tests/theme_store.rs

Interfaces:
- Consumes: existing ThemeError, ThemeStore, AppPaths, and imported theme directories.
- Produces: ThemeCustomization, BackgroundCustomization, PaletteCustomization, SurfacePart, SurfaceCustomization, ComposerCustomization, ShadowPreset, compile_customization_css, ThemeStore::load_customization, and ThemeStore::save_customization.

- [ ] Step 1: Add focused failing tests for normalization and round-trip storage

Create tests/theme_customization.rs with tests that assert the public value contract before implementation:

~~~rust
use std::collections::BTreeMap;

use codex_skin_lite::theme::{
    ComposerCustomization, PaletteCustomization, ShadowPreset, SurfaceCustomization,
    SurfacePart, ThemeCustomization,
};

#[test]
fn default_customization_has_safe_baseline_values() {
    let value = ThemeCustomization::default();

    assert_eq!(value.background.position_x, 50);
    assert_eq!(value.background.position_y, 50);
    assert_eq!(value.composer, ComposerCustomization::default());
    assert_eq!(value.colors, PaletteCustomization::default());
    assert!(value.surfaces.is_empty());
}

#[test]
fn surface_parts_are_stable_and_serializable() {
    let mut surfaces = BTreeMap::new();
    surfaces.insert(
        SurfacePart::Composer,
        SurfaceCustomization {
            opacity: Some(88),
            blur_px: Some(12),
            radius_px: Some(20),
            shadow: Some(ShadowPreset::Soft),
        },
    );
    let value = ThemeCustomization {
        surfaces,
        ..ThemeCustomization::default()
    };

    let json = serde_json::to_value(&value).unwrap();
    assert_eq!(json["surfaces"]["composer"]["opacity"], 88);
    assert_eq!(json["surfaces"]["composer"]["shadow"], "soft");
}
~~~

Extend tests/theme_store.rs with persistence and corrupt-file fallback:

~~~rust
#[test]
fn customization_round_trips_without_changing_the_theme_package() {
    let env = fixtures::theme_environment();
    env.store
        .import_zip_bytes(&fixtures::valid_theme_zip())
        .unwrap();
    let customization = codex_skin_lite::theme::ThemeCustomization {
        background: codex_skin_lite::theme::BackgroundCustomization {
            position_x: 18,
            position_y: 72,
        },
        ..Default::default()
    };

    env.store
        .save_customization("eva-warm-cream", &customization)
        .unwrap();
    let loaded = env.store.load_customization("eva-warm-cream").unwrap();

    assert_eq!(loaded, customization);
    assert!(env
        .paths
        .themes_dir()
        .join("eva-warm-cream/theme.css")
        .is_file());
}

#[test]
fn corrupt_customization_falls_back_to_defaults() {
    let env = fixtures::theme_environment();
    env.store
        .import_zip_bytes(&fixtures::valid_theme_zip())
        .unwrap();
    std::fs::write(
        env.paths
            .themes_dir()
            .join("eva-warm-cream/customization.json"),
        b"{not-json}",
    )
    .unwrap();

    assert_eq!(
        env.store.load_customization("eva-warm-cream").unwrap(),
        codex_skin_lite::theme::ThemeCustomization::default()
    );
}
~~~

- [ ] Step 2: Run the focused tests and verify the new interfaces are absent

Run:

~~~bash
cargo test --test theme_customization --test theme_store
~~~

Expected: compilation fails because the customization types and store methods do not exist yet.

- [ ] Step 3: Implement the typed model and normalization

In src/theme/customization.rs, define the six stable surface parts and the exact typed model:

~~~rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Ord, PartialOrd, Hash, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum SurfacePart { Main, Sidebar, Thread, Message, Composer, Header }

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ShadowPreset { None, Soft, Strong }

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields, default)]
pub struct ThemeCustomization {
    pub schema_version: u8,
    pub background: BackgroundCustomization,
    pub colors: PaletteCustomization,
    pub surfaces: std::collections::BTreeMap<SurfacePart, SurfaceCustomization>,
    pub composer: ComposerCustomization,
}
~~~

Use integer opacity percentages so AppSnapshot remains Eq. Put serde(default, deny_unknown_fields) on every nested value struct so omitted optional overrides load correctly. Define defaults of schema version 1, background 50/50, empty palette overrides, empty surface overrides, and zero composer insets. Make PaletteCustomization fields optional strings for background, panel, accent, text, and line. Make SurfaceCustomization fields optional for opacity, blur_px, radius_px, and shadow. Make ComposerCustomization use bottom_inset_px: u16 and horizontal_inset_px: u16.

Implement ThemeCustomization::normalized(self) -> Result<Self, ThemeError> to trim optional color fields, accept only #RGB, #RGBA, #RRGGBB, and #RRGGBBAA, clamp numeric values to the approved ranges, and reject schema_version != 1. Empty color strings become None. Add is_default() for reset handling and SurfacePart::ALL/css_name() for deterministic UI and CSS generation.

Add ThemeError::InvalidCustomization(String) in src/theme/package.rs and re-export all public customization types from src/theme/mod.rs.

- [ ] Step 4: Add Safe CSS generation without accepting user CSS

Implement compile_customization_css(&ThemeCustomization) -> Result<String, ThemeError> in src/theme/customization.rs. For each nonempty surface override, generate a single registered selector such as:

~~~css
[data-ds-part="composer"] {
  opacity: 0.88;
  backdrop-filter: blur(12px);
  border-radius: 20px;
  box-shadow: 0 8px 24px rgba(0,0,0,0.16);
}
~~~

Use the existing compile_safe_css for the final validation and priority wrapper. Map ShadowPreset::None to box-shadow: none, Soft to a fixed bounded shadow, and Strong to a fixed larger but still bounded shadow. Do not interpolate a user-provided selector, property, URL, or raw CSS value.

- [ ] Step 5: Implement atomic customization store operations

In ThemeStore, add:

~~~rust
pub fn load_customization(&self, id: &str) -> Result<ThemeCustomization, ThemeError>;
pub fn save_customization(
    &self,
    id: &str,
    customization: &ThemeCustomization,
) -> Result<(), ThemeError>;
~~~

Validate the stored theme ID and real theme directory before reading or writing. A missing file returns ThemeCustomization::default(). A malformed or out-of-version file logs a local warning with tracing::warn! and returns the default without rejecting the base theme. Save normalizes first, writes customization.json.tmp, calls sync_all, and renames it over customization.json. Saving ThemeCustomization::default() removes the optional customization file after validating the directory.

Keep the original manifest.json, theme.json, theme.css, compiled.css, and background image untouched.

- [ ] Step 6: Run persistence tests and commit the storage layer

Run:

~~~bash
cargo fmt --check
cargo test --test theme_customization --test theme_store
~~~

Expected: all focused tests pass, including atomic file replacement and corrupt-file fallback.

Commit:

~~~bash
git add src/theme/customization.rs src/theme/mod.rs src/theme/package.rs src/theme/store.rs tests/theme_customization.rs tests/theme_store.rs
git commit -m "feat: persist bounded theme customization"
~~~

---

### Task 2: Theme Payload and Renderer Customization

Files:
- Modify: src/theme/store.rs
- Modify: src/theme/mod.rs
- Modify: assets/renderer/runtime.js
- Modify: renderer/tests/dom-fixtures.js
- Test: renderer/tests/runtime.test.js

Interfaces:
- Consumes: ThemeCustomization, compile_customization_css, and the existing ThemePayload/runtime API.
- Produces: ThemePayload.customization, ThemeStore::load_payload_with_customization, and renderer API version 7 behavior for custom colors, background focus, surface rules, and composer insets.

- [ ] Step 1: Add a failing renderer fixture and assertions

Add a helper payload in renderer/tests/dom-fixtures.js that extends evaPayload with:

~~~js
theme.customization = {
  schemaVersion: 1,
  background: { positionX: 18, positionY: 72 },
  colors: {
    background: null,
    panel: null,
    accent: "#00aacc",
    text: null,
    line: null,
  },
  surfaces: {
    composer: { opacity: 88, blurPx: 12, radiusPx: 20, shadow: "soft" },
  },
  composer: { bottomInsetPx: 14, horizontalInsetPx: 22 },
};
~~~

Add one focused test in renderer/tests/runtime.test.js that currently fails because the runtime ignores the object:

~~~js
it("applies and restores bounded customization values", async () => {
  const window = installRuntime({ fixture: "modernScrollingComposerWithRightPanel" });
  const footer = window.document.querySelector("[data-thread-scroll-footer]");

  await window.__CODEX_SKIN_LITE__.apply(customizedEvaPayload(1));

  expect(window.document.documentElement.style.getPropertyValue("--ds-theme-color-accent")).toBe("#00aacc");
  expect(window.document.querySelector("#codex-skin-lite-theme").textContent).toContain("background-position: 18% 72%");
  expect(footer.style.bottom).toBe("14px");
  expect(footer.style.left).toBe("22px");
  expect(footer.style.right).toBe("22px");

  await window.__CODEX_SKIN_LITE__.apply(evaPayload(2));
  expect(footer.style.bottom).toBe("0px");
  expect(footer.style.left).toBe("0px");
  expect(footer.style.right).toBe("0px");
});
~~~

- [ ] Step 2: Run the renderer test to confirm the failure

Run:

~~~bash
npm test --prefix renderer -- --run renderer/tests/runtime.test.js
~~~

Expected: the new customization test fails while the existing composer and route-replacement tests continue to pass.

- [ ] Step 3: Extend ThemePayload and stored-payload construction

Add pub customization: ThemeCustomization to ThemePayload. Refactor payload loading into a shared private loader that accepts an optional candidate customization:

~~~rust
pub fn load_payload(&self, id: &str) -> Result<ThemePayload, ThemeError>;
pub fn load_payload_with_customization(
    &self,
    id: &str,
    customization: &ThemeCustomization,
) -> Result<ThemePayload, ThemeError>;
~~~

The normal method loads the optional stored customization; the candidate method normalizes the provided value. Append compile_customization_css output to the package's compiled CSS while retaining the original image signature calculation so editing a layout value does not recreate the background Blob unnecessarily. Serialize the normalized customization alongside the existing theme payload.

- [ ] Step 4: Update the renderer without touching the composer DOM tree

Change API_VERSION from 6 to 7 so a running API 6 instance is cleaned up and replaced by the new runtime. In applyTheme:

- apply only the five optional custom color variables through the existing owned-variable map;
- clamp background position to 0..=100 and write it into the managed document background rule;
- keep base theme colors and custom colors separate so removing an override restores the package color.

Change syncComposerPosition to accept theme.customization.composer and set only the existing fixed footer's local bottom, left, and right values. Use zero insets when only centered width is enabled. Continue setting top: auto, width: auto, and z-index: 10; keep the footer in its current parent and retain the existing style snapshot/restore logic.

Do not add scroll listeners, polling, DOM cloning, or reparenting. Keep the thread marker on the stable viewport parent and preserve the current ambiguous-route fail-closed behavior.

- [ ] Step 5: Run focused renderer checks and commit

Run:

~~~bash
node --check assets/renderer/runtime.js
npm test --prefix renderer
node scripts/measure-renderer.mjs
~~~

Expected: all renderer tests pass, the runtime remains below the existing size gate, and cleanup restores the original composer inline values.

Commit:

~~~bash
git add src/theme/store.rs src/theme/mod.rs assets/renderer/runtime.js renderer/tests/dom-fixtures.js renderer/tests/runtime.test.js
git commit -m "feat: preview theme customization in renderer"
~~~

---

### Task 3: Controller Preview, Save, and Reconnect Semantics

Files:
- Modify: src/model.rs
- Modify: src/controller.rs
- Test: tests/controller.rs
- Test: tests/macos_menu.rs

Interfaces:
- Consumes: ThemeStore::load_payload_with_customization, ThemeStore::save_customization, and the existing verified revision protocol.
- Produces: AppCommand::PreviewThemeCustomization, AppCommand::SaveThemeCustomization, AppSnapshot.active_theme_customization, and transient preview behavior that never silently persists.

- [ ] Step 1: Add failing controller tests

Extend tests/controller.rs with these focused cases:

~~~rust
#[tokio::test]
async fn preview_customization_applies_without_writing_the_file() {
    let mut env = controller_environment(false);
    let candidate = codex_skin_lite::theme::ThemeCustomization {
        background: codex_skin_lite::theme::BackgroundCustomization {
            position_x: 12,
            position_y: 84,
        },
        ..Default::default()
    };

    env.controller
        .handle(AppCommand::PreviewThemeCustomization(candidate.clone()))
        .await
        .unwrap();

    assert_eq!(env.themes.load_customization("eva-warm-cream").unwrap(), Default::default());
    assert_eq!(
        env.applied_payloads.lock().unwrap().last().unwrap()
            .theme.as_ref().unwrap().customization,
        candidate
    );
}

#[tokio::test]
async fn save_customization_persists_and_is_sent_on_the_next_connection() {
    let mut env = controller_environment(false);
    let candidate = codex_skin_lite::theme::ThemeCustomization {
        composer: codex_skin_lite::theme::ComposerCustomization {
            bottom_inset_px: 16,
            horizontal_inset_px: 24,
        },
        ..Default::default()
    };

    env.controller
        .handle(AppCommand::SaveThemeCustomization(candidate.clone()))
        .await
        .unwrap();
    assert_eq!(env.themes.load_customization("eva-warm-cream").unwrap(), candidate);

    env.controller.handle(AppCommand::Reconnect).await.unwrap();
    assert_eq!(
        env.applied_payloads.lock().unwrap().last().unwrap()
            .theme.as_ref().unwrap().customization,
        candidate
    );
}
~~~

Expose ThemeStore as themes in the test environment so the assertions read the per-theme file through the store contract.

- [ ] Step 2: Run the controller tests and verify the commands are missing

Run:

~~~bash
cargo test --test controller
~~~

Expected: compilation fails on the new commands, snapshot field, and test environment accessor.

- [ ] Step 3: Add the model and command interfaces

In src/model.rs, add active_theme_customization: ThemeCustomization to AppSnapshot and initialize it to ThemeCustomization::default() wherever snapshots are constructed, including the AppSnapshot literal in tests/macos_menu.rs. In src/controller.rs, add:

~~~rust
AppCommand::PreviewThemeCustomization(ThemeCustomization),
AppCommand::SaveThemeCustomization(ThemeCustomization),
~~~

Add preview_customization: Option<ThemeCustomization> to Controller. Keep it separate from AppSettings; it is memory-only and must be cleared by theme switching, successful Save, reconnect, or any failed rollback.

- [ ] Step 4: Implement verified Preview

Add a preview_theme_customization helper that:

1. requires an active theme ID;
2. normalizes the candidate;
3. clones settings with theme_enabled: true for a transient preview when the checkbox is currently off;
4. builds a candidate payload through load_payload_with_customization;
5. increments the renderer revision and requires the exact acknowledgment;
6. on success stores only preview_customization and publishes a snapshot;
7. on failure reapplies the previous effective payload with a newer revision and leaves disk unchanged.

If Codex is not connected, return a compatibility warning and do not change the renderer or file.

- [ ] Step 5: Implement verified Save and offline persistence

Add a save_theme_customization helper. Require an active theme ID and normalize the candidate. If the theme is enabled and the connection is Connected, apply the candidate through the same exact-revision helper before writing. If that apply fails, leave the old file and renderer state untouched. After a successful apply, atomically call ThemeStore::save_customization; if the file write fails, reapply the previous saved payload and report the write error. If Codex is offline or the theme is disabled, write the validated customization without calling the renderer; the next successful Open/Reconnect/ConfirmRestart loads it.

Update payload() so every normal connection path loads the saved customization. Clear transient preview state before Open, Reconnect, ConfirmRestart, theme activation, and any saved layout/theme setting change. Extend publish() to load the active customization for AppSnapshot; corrupt files resolve to the default via the store.

- [ ] Step 6: Run focused controller and full Rust checks, then commit

Run:

~~~bash
cargo fmt --check
cargo test --test controller --test theme_customization --test theme_store
cargo clippy --all-targets -- -D warnings
~~~

Expected: preview never writes a file, Save round-trips, reconnect sends saved values, and existing theme/width rollback tests remain green.

Commit:

~~~bash
git add src/model.rs src/controller.rs tests/controller.rs
git commit -m "feat: add theme customization preview and save"
~~~

---

### Task 4: Native Settings Startup, Gallery Link, and Customization Editor

Files:
- Create: src/macos/customization_window.rs
- Modify: src/macos/mod.rs
- Modify: src/macos/app_delegate.rs
- Modify: src/macos/settings_window.rs

Interfaces:
- Consumes: AppSnapshot.active_theme_customization, AppCommand preview/save commands, and the existing AppKit status-item target.
- Produces: automatic one-shot Settings display on every process launch, a native gallery action, and a native editor with draft/Preview/Save/Reset/Close behavior.

- [ ] Step 1: Add the native action routes before adding controls

In ActionTargetIvars, add:

~~~rust
customization_window: RefCell<Option<Retained<objc2_app_kit::NSWindow>>>,
~~~

Initialize it in ActionTarget::new. Add target actions:

~~~rust
#[unsafe(method(openThemeGallery:))]
fn open_theme_gallery(&self, _sender: Option<&AnyObject>);

#[unsafe(method(customizeTheme:))]
fn customize_theme(&self, _sender: Option<&AnyObject>);

#[unsafe(method(selectCustomizationComponent:))]
fn select_customization_component(&self, sender: &NSPopUpButton);

#[unsafe(method(previewCustomization:))]
fn preview_customization(&self, _sender: Option<&AnyObject>);

#[unsafe(method(saveCustomization:))]
fn save_customization(&self, _sender: Option<&AnyObject>);

#[unsafe(method(resetCustomization:))]
fn reset_customization(&self, _sender: Option<&AnyObject>);
~~~

open_theme_gallery creates the fixed HTTPS URL and calls NSWorkspace to open it in the default browser. It does not use reqwest and does not import anything automatically. The editor actions collect or reset a plain ThemeCustomization draft from customization_window and send the corresponding controller command.

- [ ] Step 2: Make Settings open once on every launch

Remove the CODEX_SKIN_LITE_OPEN_SETTINGS branch from app_delegate::run. Keep the status-item activation policy as an accessory utility, create the target/status item/menu, call app.finishLaunching(), activate the application, and then call settings_window::show(...) exactly once before entering app.run().

The existing openSettings: action continues to call the same window factory. Do not call the factory from a timer, observer, or menu callback during startup; this prevents a second Settings window.

- [ ] Step 3: Add the gallery and customization controls to the Appearance section

Update settings_window.rs so the Appearance row contains 导入 ZIP… followed by 远程主题画廊. Add 自定义主题… near the active theme popup and disable it when active_theme_id is None. Keep the existing theme checkbox, popup, delete behavior, width controls, connection actions, and status refresher.

Use meaningful native labels and button titles. The gallery button remains an ordinary native button with a visible hover/click state; the URL is fixed in the action implementation rather than accepted from a text field.

- [ ] Step 4: Build the editor window and draft bridge

Create src/macos/customization_window.rs with a native resizable window and a scrollable form. Use the existing AppKit helper style and assign stable tags to controls so ActionTarget can collect a complete draft:

~~~rust
const TAG_BACKGROUND_X: isize = 1001;
const TAG_BACKGROUND_Y: isize = 1002;
const TAG_COLOR_BACKGROUND: isize = 1010;
const TAG_COLOR_PANEL: isize = 1011;
const TAG_COLOR_ACCENT: isize = 1012;
const TAG_COLOR_TEXT: isize = 1013;
const TAG_COLOR_LINE: isize = 1014;
const TAG_SURFACE_OPACITY: isize = 1020;
const TAG_SURFACE_BLUR: isize = 1021;
const TAG_SURFACE_RADIUS: isize = 1022;
const TAG_SURFACE_SHADOW: isize = 1023;
const TAG_COMPOSER_BOTTOM: isize = 1030;
const TAG_COMPOSER_HORIZONTAL: isize = 1031;
const TAG_STATUS: isize = 1040;
~~~

The form contains background position fields, five optional hex color fields, the six-part component popup, opacity/blur/radius/shadow controls for the selected component, composer bottom/horizontal fields, and Preview/Save/Reset/Close buttons. Empty color and surface fields mean follow the package theme.

Add plain AppKitState methods for the editor draft and selected SurfacePart, initialized from the active snapshot when the window opens. On component selection, save the visible surface controls into the draft map, update the selected part, and populate the controls from that part. Preview and Save first collect the visible values, normalize them in the Rust theme model, and then send AppCommand::PreviewThemeCustomization or AppCommand::SaveThemeCustomization.

The editor status label says 未预览, 预览已发送（未保存）, or 已保存. Invalid hex values and invalid numeric input are displayed next to the relevant field before a command is sent. Reset restores an in-memory default draft and does not remove the file until Save is pressed.

- [ ] Step 5: Add minimal native compile checks and commit

Run:

~~~bash
cargo fmt --check
cargo check --target aarch64-apple-darwin
cargo test --test macos_menu --test controller
~~~

Expected: AppKit symbols, target selectors, window ownership, and the existing menu tests compile without changing renderer behavior. On an Apple Silicon macOS host, launch the binary once and verify Settings appears exactly once, the gallery opens in the default browser, and the editor opens only for a selected theme.

Commit:

~~~bash
git add src/macos/mod.rs src/macos/app_delegate.rs src/macos/settings_window.rs src/macos/customization_window.rs
git commit -m "feat: add startup settings and theme editor UI"
~~~

---

### Task 5: Documentation, Release Build, and Targeted Acceptance

Files:
- Modify: README.md
- Modify: docs/acceptance/latest-codex.md
- Modify: Cargo.toml
- Modify: resources/Info.plist

Interfaces:
- Consumes: all four implementation tasks and the approved customization spec.
- Produces: a documented 0.1.3 Apple Silicon application archive with checksum and a concise acceptance record.

- [ ] Step 1: Update user-facing behavior and version metadata

Update README.md to state that Settings opens once on each launch, the Appearance section links to the DreamSkin gallery, and customization follows Preview → Save. Document the location of customization.json and that the original ZIP remains unchanged.

Bump Cargo.toml from 0.1.2 to 0.1.3, set CFBundleShortVersionString to 0.1.3, and set CFBundleVersion to 4 in resources/Info.plist. Add the new customization acceptance cases to docs/acceptance/latest-codex.md without changing prior verified evidence.

- [ ] Step 2: Run the focused verification set

Run:

~~~bash
cargo fmt --check
cargo test --test theme_customization --test theme_store --test controller --test macos_menu
cargo clippy --all-targets -- -D warnings
node --check assets/renderer/runtime.js
npm test --prefix renderer
node scripts/measure-renderer.mjs
~~~

Expected: all commands pass, the renderer remains under the existing size target, and no tracked files outside the planned list are modified.

- [ ] Step 3: Run one real Codex acceptance pass

With the local Codex CDP endpoint available, run the existing ignored live test for theme application and its width/composer geometry companion. Manually verify only the new paths:

1. Start CodexSkinLite and confirm Settings appears once.
2. Click 远程主题画廊 and confirm the default browser opens the fixed HTTPS URL.
3. Import/select a theme, open customization, change background position and composer insets, click Preview, and check the current Codex page.
4. Click Save, quit/relaunch or reconnect, and confirm the same customization is loaded.
5. Click Reset → Preview → Save and confirm the package defaults return.
6. Switch chats, resize the Codex window, and scroll a long conversation; confirm there is exactly one composer footer fixed at the bottom.

Record the tested ZIP path, runtime API version, package version, and any observed compatibility warning in docs/acceptance/latest-codex.md.

- [ ] Step 4: Build the new app archive and checksum

Run:

~~~bash
bash scripts/package-app.sh
~~~

Verify:

~~~bash
file dist/CodexSkinLite-0.1.3-macos-arm64.zip
shasum -a 256 dist/CodexSkinLite-0.1.3-macos-arm64.zip
unzip -l dist/CodexSkinLite-0.1.3-macos-arm64.zip
~~~

The archive must contain CodexSkinLite.app, the executable must be Mach-O arm64, the bundle version must be 0.1.3/4, and the .sha256 sidecar must match the final ZIP. Do not add dist/, .tmp/, or outputs/ to Git unless they are already tracked by repository policy.

- [ ] Step 5: Review the final diff and commit the release changes

Run:

~~~bash
git diff --check
git status --short --branch
git diff --stat HEAD~1..HEAD
~~~

Confirm that .tmp/ and outputs/ remain untracked and no unrelated files are staged. Commit the documentation, version, and acceptance record:

~~~bash
git add README.md docs/acceptance/latest-codex.md Cargo.toml resources/Info.plist
git commit -m "release: package CodexSkinLite theme customization"
~~~

Report the final archive path, SHA-256, commit hashes, and the exact focused verification commands in the handoff.
