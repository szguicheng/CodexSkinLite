# CodexSkinLite Theme Customization Design

Date: 2026-09-01

Status: Approved in chat; awaiting written-spec review

This document extends the base CodexSkinLite specification in
`docs/superpowers/specs/2026-08-31-codex-skin-lite-design.md`. It covers the
three requested changes: opening Settings on every application launch, opening
the remote DreamSkin gallery from Settings, and adding a bounded customization
editor for the active imported theme.

## 1. Goals and non-goals

### Goals

- Show the native Settings window once on every CodexSkinLite process launch.
- Keep the status-item Settings command available after the automatic window is
  closed.
- Add a native action beside ZIP import that opens
  `https://dreamskin.cc/gallery` in the user's default browser.
- Let a user customize the active imported theme, preview the result in the
  current Codex page, and save the result per theme.
- Keep the customization reversible and independent from the original ZIP
  package.
- Preserve the stable thread viewport/composer architecture that keeps the
  React-owned composer in its original DOM tree.

### Non-goals

- Arbitrary CSS input, JavaScript, selectors, remote theme fetching, or package
  execution.
- Dragging arbitrary Codex DOM nodes or changing private Codex layout trees.
- Forcing unsupported sidebar or window dimensions that Codex itself owns.

## 2. Native application behavior

`app_delegate` creates the target, status item, and menu as it does today. After
the application has finished launching, it calls the existing Settings window
factory exactly once. The window is activated and brought to the front. This
behavior is unconditional; it is not controlled by
`CODEX_SKIN_LITE_OPEN_SETTINGS`. Closing the window does not reopen it until the
next process launch. The status-item menu continues to expose Settings.

The Settings Appearance section contains:

- theme enabled checkbox;
- active theme popup;
- `导入 ZIP…` button;
- `远程主题画廊` button, which delegates the fixed HTTPS URL to
  `NSWorkspace`/the default browser;
- `自定义主题…` button, disabled when no theme is selected;
- delete and local-directory actions already supported by the application.

The application does not request or download the gallery page itself. The
browser is the trust boundary for remote browsing, and importing a theme still
requires the existing local ZIP validation flow.

## 3. Customization editor

The editor is a separate native AppKit window so the connection controls and
the compact import controls remain readable. It shows the selected theme name,
an unsaved-preview indicator, and a scrollable form. Values are held in a
local draft until Preview or Save is pressed.

### Background

- Horizontal focus: `0..=100` percent.
- Vertical focus: `0..=100` percent.

The default is 50/50, matching the existing centered background behavior. The
renderer writes these values only to the managed document background style.

### Palette

Each field is an optional override using strict three-, four-, six-, or
eight-digit hexadecimal CSS colors:

- background;
- panel;
- accent;
- text;
- line.

An empty field means “use the package value.” The client validates colors before
preview and before save.

### Stable surface components

The component popup exposes only these registered Skin API parts:

- main content;
- sidebar;
- thread viewport;
- message;
- composer;
- header.

For the selected part, the form exposes optional overrides for:

- opacity: `0.65..=1.0`;
- backdrop blur: `0..=30` px;
- radius: `0..=28` px;
- shadow preset: none, soft, or strong.

The `thread` part always refers to the stable viewport parent, never the
scrolling node. The `composer` part refers to the existing composer surface,
not a new wrapper.

### Composer position

- Bottom inset: `0..=80` px.
- Horizontal inset: `0..=120` px.

These values are local insets on the existing fixed footer. They do not change
the footer's parent, insert a duplicate, or calculate coordinates from the
window's global origin.

### Commands and keyboard behavior

- `预览` sends the current draft but does not write it.
- `保存` applies the current draft through the same verified path when Codex is
  connected, then writes it; if Codex is offline it writes the validated draft
  for the next connection.
- `恢复默认` resets the draft to package defaults. The reset is previewed and
  becomes persistent only after Save.
- `关闭` discards an unsaved draft.
- Command-S invokes Save and Escape closes the editor.

## 4. Persistent data model

Each theme may contain an optional file:

```text
themes/<theme-id>/customization.json
```

The file is not part of the imported ZIP manifest and never changes the ZIP's
original `theme.json` or `theme.css`. Its versioned shape is:

```json
{
  "schemaVersion": 1,
  "background": { "positionX": 50, "positionY": 50 },
  "colors": {
    "background": "#fffaf0",
    "panel": null,
    "accent": null,
    "text": null,
    "line": null
  },
  "surfaces": {
    "main": { "opacity": null, "blurPx": null, "radiusPx": null, "shadow": null },
    "sidebar": { "opacity": null, "blurPx": null, "radiusPx": null, "shadow": null },
    "thread": { "opacity": null, "blurPx": null, "radiusPx": null, "shadow": null },
    "message": { "opacity": null, "blurPx": null, "radiusPx": null, "shadow": null },
    "composer": { "opacity": null, "blurPx": null, "radiusPx": null, "shadow": null },
    "header": { "opacity": null, "blurPx": null, "radiusPx": null, "shadow": null }
  },
  "composer": { "bottomInsetPx": 0, "horizontalInsetPx": 0 }
}
```

The implementation may omit empty optional fields when serializing, but it
must normalize all values to the ranges above. `schemaVersion` values other
than 1 are ignored as corrupt data. A malformed customization file falls back
to the package defaults without preventing the theme from loading.

The store writes through a temporary sibling, flushes it, and atomically
renames it over `customization.json`. Reset removes the optional file only
after the user saves the reset state. A missing file means no customization.

## 5. Controller and renderer data flow

The controller adds preview/save customization commands and keeps an optional
in-memory preview draft separate from `AppSettings`. The active theme snapshot
includes its normalized customization so the native editor can initialize from
the saved value.

Theme payload construction has two paths:

1. normal activation loads the stored customization;
2. preview/save constructs a candidate payload without writing it first.

The candidate is validated, converted to generated Safe CSS plus bounded
renderer layout fields, and sent through the current CDP `apply` call. The
controller treats the renderer's matching revision acknowledgment as success.
When a connected apply fails, the previous effective payload remains active and
the customization file is not changed. When offline, Save writes the validated
candidate and the next successful Open/Reconnect/ConfirmRestart loads it.

The serialized theme payload gains only the data needed by the existing runtime:

- optional custom color variables;
- background position;
- generated component overrides;
- composer bottom and horizontal insets.

Generated component rules use only the existing `data-ds-part` selectors and
the existing Safe CSS compiler. No user string becomes a selector, declaration
name, script, URL, or DOM operation. The renderer keeps ownership snapshots for
custom inline values and restores them on theme disable, reset, or cleanup.

The composer update remains:

```text
stable thread viewport
  └── original React thread scroll node
       └── original React composer footer
```

The runtime never reparents or clones the footer. It applies the custom bottom
and horizontal insets to the same fixed footer whose containing block is the
stable viewport parent. If route discovery is ambiguous, no geometry mutation
is performed.

On every successful connection, saved settings and the saved active-theme
customization are sent together. An unsaved preview is local to the current
editor session and is not silently promoted during reconnect.

## 6. Failure and recovery behavior

- Invalid color or out-of-range values prevent Preview/Save and identify the
  offending field inline.
- Corrupt customization JSON is ignored and reported as a compatibility/status
  message; the original theme remains usable.
- Missing Skin API parts cause presentation-only values to be applied only where
  discovery succeeds. Composer geometry is skipped if the active footer or
  stable viewport cannot be resolved uniquely.
- A failed Preview restores the last effective renderer payload.
- A failed connected Save leaves the old file and old renderer state intact.
- An offline Save never contacts the gallery and is applied on the next
  connection.
- Disabling the theme removes customization-owned styles but leaves centered
  conversation width independent.

## 7. Focused verification

The implementation uses a small targeted verification set rather than a broad
new test matrix:

### Rust

- customization normalization and round-trip persistence;
- atomic save/reset and corrupt-file fallback;
- controller preview/save behavior and offline save;
- generated CSS values stay inside the Safe CSS contract.

### Renderer

- background focus and surface overrides are applied and restored;
- custom composer insets keep the footer in the original scroll tree;
- preview/reset does not create duplicate footers or respond to ordinary scroll.

### Manual release check

- launch CodexSkinLite and confirm Settings opens once;
- open the gallery button;
- preview, save, restart/reconnect, and reset one imported theme;
- switch chats, resize the Codex window, scroll a long thread, and confirm one
  input footer remains fixed at the bottom.

The existing compile, focused unit tests, renderer tests, package build, and one
real-Codex acceptance pass remain the release gates. No unrelated feature or
test suite is added.
