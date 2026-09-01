# CodexSkinLite Manual Acceptance — 2026-09-01

Status: image customization build ready; full user retest pending

## Candidate

- Source commits: `e51847bd`, `5a487341`, `75b1a956`, `b34da733`, `fa747f78`, `f95c4956`, `8f4acade`, `33ef9322`, `130c33cc`, `8afb9e6b`
- macOS: 26.5.2 (25F84)
- Official Codex desktop bundle: `/Applications/ChatGPT.app`
- Bundle ID: `com.openai.codex`
- Codex version/build: 26.825.51511 (7377)
- ZIP: `dist/CodexSkinLite-0.1.7-macos-arm64.zip`
- SHA-256: `8de18c603c9fe89669b598c5022dce9bb366ce52f092ea4ede9cadfc9d12d5af`
- Bundle: `0.1.7 (8)`, Mach-O arm64
- Renderer API/bytes: `10` / `23,723`
- Renderer timers: 0
- Automated Rust tests: 50 non-ignored tests passed
- Automated renderer tests: 28 passed
- Live current-Codex CDP probes: theme injection/cleanup and centered-width mutation tests passed against the current 9222 page; after a real 320px thread scroll, footer left/top/bottom deltas and the bottom gap all remained within 1px
- Startup smoke: Quartz reported one `CodexSkinLite 设置` window above ordinary ChatGPT/PaperDiff windows while the process retained accessory activation.

## Follow-up working-tree repaired root causes

- Never reparent the React-managed `[data-thread-scroll-footer]`; keep the native footer and composer in the active thread scroll container.
- Apply the `thread` theme part to the stable viewport parent and keep the real scroll node separately marked, preventing theme filters from turning scroll coordinates into the footer's fixed-position coordinate system.
- Remove duplicate footer nodes during bootstrap and relevant route/layout reconciliation, while failing closed when retained routes are ambiguous.
- Apply centered width to `[data-pip-obstacle="thread-footer"]`, not the inner composer surface, so conversation and input use the same coordinate space.
- Treat the modern `data-app-shell-tabs` and `data-browser-sidebar-webview*` layers as right-panel surfaces for both available-width calculation and theme mapping.
- Make the current title toolbar's direct native white surface transparent only while a theme is active.
- Keep stable `ResizeObserver` subscriptions and ignore ordinary message class churn, preventing the renderer from repeatedly rescanning an unchanged layout.
- Upgrade the injected runtime API to version 10 and migrate stale footer state left by versions 2 through 9.
- Apply the persisted theme and centered-width settings immediately after every successful Open, Reconnect, or confirmed restart, retrying above a renderer revision retained across utility restarts.
- Store per-theme bounded customization in `customization.json`; preview uses an in-memory payload and Save persists it without changing the imported ZIP.
- Keep composer customization limited to local bottom/horizontal insets on the existing fixed footer.
- For new conversations, apply centered width to the outer home composer content wrapper, not the nested ComposerLayoutRoot surface; this avoids double margins and right drift.
- Initialize the customization editor from the selected package's theme.json and safe theme.css values; saved values still override the package values.
- Add an explicit `无` appearance option. It clears the active theme and disables presentation; opening customization with `无` shows an empty draft.
- Treat missing background-position overrides as package artwork focus values, so an empty draft does not replace the theme's original image focus.
- Add a per-theme replacement image with bounded pixel offsets, cover/contain/stretch fill, and image-only opacity; the original ZIP background remains unchanged.
- Keep the theme background visible when Codex loses focus by isolating the managed background layer; scroll the native customization editor to its background-image controls on open.

## Test preparation

1. Finish or save current Codex work because one test restarts the official app.
2. Quit Codex++ completely and disable the previous user blur script. The comparison must contain only CodexSkinLite.
3. Extract `CodexSkinLite-0.1.7-macos-arm64.zip` and replace the old `CodexSkinLite.app` before opening it.
4. Because this build is unsigned, use Finder’s Open action if macOS blocks the first launch.
5. Confirm Settings shows `/Applications/ChatGPT.app` as the Codex path.

## Manual matrix

Mark each row Pass or Fail and attach a screenshot/video for failures.

| ID | Test | Expected result | Result |
|---|---|---|---|
| A1 | Import the current EVA DreamSkin ZIP | Import succeeds; the theme ID appears and is selected automatically, but is not enabled before Codex connects | Pending |
| A2 | Import Blue Eyes ZIP | Both theme IDs remain available; no duplicate or partial theme directory appears | Pending |
| A3 | Open official Codex through CodexSkinLite | If Codex was closed, it opens and status becomes Connected | Pending |
| A4 | Codex already open without CDP | Status becomes RestartRequired; Codex is not closed until “确认重启” is pressed | Pending |
| B1 | Enable EVA on a new conversation | Theme background and header appear; new-conversation presentation is not reversed | Pending |
| B2 | Open an existing conversation | Main viewport and thread use the package CSS; no unnamed high-blur strip appears below the title bar | Pending |
| B3 | Focus the composer and scroll for 30 seconds | No clear/unblurred frame flashes; composer stays fixed | Pending |
| B4 | Scroll while a response streams | No flicker and no visible UI slowdown caused by rescanning | Pending |
| B5 | Composer with no attachment | No extra horizontal divider appears | Pending |
| B6 | Add then remove an image/file attachment | Attachment layout and empty-toolbar state both remain correct | Pending |
| B7 | Open and close the right context panel | Composer is not covered; content recenters inside the remaining viewport; panel inherits sidebar theme | Pending |
| B8 | Switch EVA → Blue Eyes → EVA without restarting | One theme style remains active; no background clear frame during switching | Pending |
| C1 | Enable centered width at 900px | Conversation and composer have the same centered maximum width | Pending |
| C2 | Change width to 700px then 1100px | Both regions update together without reload | Pending |
| C3 | Disable centered width | Native width returns; active theme remains | Pending |
| C4 | Disable theme while centered width stays enabled | Theme is removed; width remains enabled | Pending |
| D1 | Resize and enter/leave full screen | Theme coverage and centering remain correct | Pending |
| D2 | Quit and reopen CodexSkinLite, then Reconnect | Imported themes/settings persist and renderer reconnects | Pending |
| D3 | Compare with Codex++ fully quit | Scrolling, typing, opening panels, and general UI should feel like native Codex rather than the previous slowed state | Pending |
| E1 | Launch CodexSkinLite from the new 0.1.7 bundle | Settings opens once automatically and is in front; closing it does not create another window | Pass in prior startup smoke; user retest pending |
| E2 | Click `远程主题画廊` beside ZIP import | The default browser opens `https://dreamskin.cc/gallery`; the app does not download the page itself | Pending |
| E3 | Open `自定义主题…` for the active imported theme | Editor is prefilled from the selected theme's existing values; opening it with `无` shows a blank draft | Pending |
| E7 | Choose a replacement image and adjust image options | The image controls are visible on open; Preview changes only the background image; Save persists the selected image, pixel offset, fill mode, and opacity without changing the original theme image | Pending |
| E4 | Change values and click `预览` | Current Codex updates; no customization file is written and the composer stays in its original DOM tree | Pending |
| E5 | Click `保存`, reconnect, then reopen the editor | Values survive reconnect/relaunch and the original ZIP files are unchanged | Pending |
| E6 | Click `恢复默认` → `预览` → `保存` | Package theme values return and the optional customization file is removed | Pending |

## Result report format

Reply with IDs, for example:

```text
A1-A4 Pass
B1 Pass
B2 Fail: title bar and thread have a seam (screenshot attached)
B3 Fail: one flash at 00:12 (video attached)
B4-B8 Pass
C1-C4 Pass
D1-D3 Pass
```

Do not mark the candidate accepted until every failed row has a test-first fix and the affected rows are rerun.
