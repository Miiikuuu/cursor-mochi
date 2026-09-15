# v0.1 theme classification and UI polish validation

Implementation and validation date: **2026-09-14**. English documentation prepared on **2026-09-15**.

Work proceeded through theme classification, list/preview interaction, and visual consistency with regression checks. This remains a candidate build; real GNOME acceptance has not been performed. The implementation follows AGENTS.md and preserves Rust + GTK4/GIO and the core/app/platform/gtk boundaries. No dependencies were added.

## Baseline and changes

The workspace and existing implementation were reviewed first. The supplied polish brief was at the repository root; it was preserved and copied to `docs/CursorMochi_v0.1_Polish_Brief.md`. Existing English labels, nonselectable list text, actual Xcursor decoding, settings transactions, and conflict protection were retained.

**Phase A — classification.** A new icon-only regression test failed against the previous implementation: an icon directory with `index.theme` was counted as a cursor theme. Classification now requires successful decoding from a direct source or an explicit `Inherits` chain. Automatic default fallback is not evidence. Candidates with no source, damaged or unreadable files, or insufficient verification budgets are excluded from the main list with reasons retained. There is no theme-name blacklist. Partially usable themes remain browsable with a visible warning. Regression coverage includes icon-only directories, empty hicolor, valid inheritance, default fallback, malformed files, broken links, missing parents, inheritance cycles, partially valid roles, budgets, and metadata errors.

**Phase B — interaction.** The app layer maintains independent search and selection state. Startup prefers the current theme; refresh preserves the query and surviving selection, and clears a removed selection with a notice. Rows show real static thumbnails and an independent **In use** badge. Long display names are ellipsized; duplicate display names include theme IDs. Friendly role names map only to verified files. Preview controls provide backgrounds, logical-pixel zoom, and animation playback. Hotspots default to off. Technical details are collapsed, candidate diagnostics are in the menu, and inheritance or important warnings remain visible. Apply names its target, matching settings show **Already in use**, size changes require opt-in, and Undo names the restoration target while retaining existing transaction protection.

**Phase C — presentation.** GTK list, preview, UI assembly, and smoke checks are separate modules with consistent spacing and native theme styling. Preview controls sit above the canvas, the detail pane scrolls, and bottom actions remain visible. The duplicate first-frame card was removed. Static previews do not update textures every tick; pause and hidden windows freeze playback time.

## Automated checks

`./scripts/check.sh --gui` exited **0**. Local log: `target/qa/polish-final-check.log`. Its automated portion produced:

| Check | Result |
|---|---|
| `cargo fmt --all --check` | PASS |
| Locked core/app tests | PASS |
| Workspace Clippy, all targets, `-D warnings` | PASS |
| Locked workspace tests | PASS: 40 tests — core 10, app 12, platform 17, GTK worker 1 |
| Locked release build | PASS: `target/release/cursor-mochi` |
| `python3 scripts/reference-xcursor.py` | PASS: three original fixtures matched libXcursor sizes, frames, pixels, hotspots, delays, and inheritance |
| Release `--diagnose` | PASS: read-only local scan found 28 verified themes, 8 excluded candidates, and 54 diagnostics |

Tests use Fake settings or an explicitly injected GIO memory backend. They do not write real user dconf. Diagnostics preserve candidate and partial-role problems; PASS does not mean every installed asset is valid.

Classification allows at most 64 decode probes per theme and 2048 per scan, with additional role-count, traversal-depth, file, and pixel budgets. Two workers share a 128 MiB decode cache. The list retains up to 16 nearby thumbnails and releases offscreen textures; stale generations cannot overwrite a new selection. Checks observed no extra static-frame rendering over 350 ms and no playback-time growth during 350 ms pauses or hidden-window intervals. No CPU or RSS percentage improvement is claimed. Long-running stress measurements were not performed.

## Isolated GUI checks

`scripts/check-gui.sh` ran three styles using private D-Bus sessions, an existing Wayland display, read-only fixtures, and the injected memory backend. All exited **0**. `GTK_A11Y=none` is limited to these checks; normal startup keeps accessibility enabled.

Assertions cover initial current-theme selection, candidate exclusion, independent **In use** state, row pointer targeting, search focus and empty results, refresh preservation, rapid selection replacement, real thumbnails, static/animated playback, pause/hide behavior, details/menu access, disabled Apply/Undo, and unchanged settings snapshots. App tests cover removed selections and missing current themes.

| Style | 820×680 content capture | 1100×840 expanded details |
|---|---|---|
| Adwaita | [Light](screenshots/polish-light.png) | [Light details](screenshots/polish-light-details.png) |
| Adwaita:dark | [Dark](screenshots/polish-dark.png) | [Dark details](screenshots/polish-dark-details.png) |
| HighContrast | [High contrast](screenshots/polish-highcontrast.png) | [High contrast details](screenshots/polish-highcontrast-details.png) |

All six actual GTK captures were opened and inspected. The initial window was 980×760, the backend was `GdkWaylandDisplay`, and the observed `scale_factor` was **1**. PNGs capture window content, not physical-screen scaling. Local style logs are `target/qa/polish-gui-{light,dark,highcontrast}.log`.

## Real GNOME acceptance and unverified items

Real desktop Apply/Undo, desktop conflicts after external changes, and actual pointers in GTK/Qt/browser windows: **NOT RUN**. No real desktop settings were written.

This build's 200%, fractional, and mixed-DPI scaling checks: **NOT RUN**. The isolated window's observed scale of 1 does not replace user-initiated desktop scaling acceptance; historical scale-2 results are not attributed to this build.

Full human keyboard/screen-reader acceptance, actual minimization, dedicated long/duplicate-name GUI fixtures, animation pause after scrolling the preview out of view, long-running RSS stress, and older native runtimes: **NOT RUN**. Other uncovered cases in the historical validation record remain unverified.

No store, import, or installation functionality was added. No system components were installed or user themes deleted. The implementation and validation run did not push or publish; a subsequent user request explicitly authorized committing and pushing the changes with English documentation. A Git push does not constitute a release or real desktop acceptance.
