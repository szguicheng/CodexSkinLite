# CodexSkinLite Manual Acceptance — 2026-08-31

Status: awaiting user retest of the 0.1.1 connection fix

## Candidate

- Source commit: `dd3aeaf`
- macOS: 26.5.2 (25F84)
- Official Codex desktop bundle: `/Applications/ChatGPT.app`
- Bundle ID: `com.openai.codex`
- Codex version/build: 26.825.51511 (7377)
- ZIP: `dist/CodexSkinLite-0.1.1-macos-arm64.zip`
- ZIP bytes: 1,428,594
- SHA-256: `9f136ac0670f6ef13feb6c27fc3d78a7635e4ad8d0b9d8bad79163d2061a65e5`
- Renderer bytes: 14,927
- Renderer timers: 0
- Automated Rust tests: 33 passed
- Automated renderer tests: 12 passed, including 20 repeated regression runs during development
- Live current-Codex CDP probes: DOM discovery, 777px width injection, and EVA theme injection all passed with cleanup

## 0.1.1 repaired root causes

- Accept current `title=ChatGPT, url=app://-/index.html` as the main Codex renderer.
- Reject the separate `avatar-overlay` target and prefer the exact main page.
- Make renderer `apply()` return its revision synchronously instead of returning a Promise that CDP decoded as `null`.
- Push controller snapshots into the open settings window so `Connected`, `RestartRequired`, and compatibility errors are visible immediately.
- Convert command failures into visible `CompatibilityWarning(...)` state instead of silently retaining `Disconnected`.

## Test preparation

1. Finish or save current Codex work because one test restarts the official app.
2. Quit Codex++ completely and disable the previous user blur script. The comparison must contain only CodexSkinLite.
3. Extract `CodexSkinLite-0.1.1-macos-arm64.zip` and replace the old `CodexSkinLite.app` before opening it.
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
