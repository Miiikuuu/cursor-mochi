# Current desktop cursor test — 2026-09-16

## Scope and evidence boundaries

The main menu's **Test current cursor** opens one reusable, nonmodal, resizable window. It tests the running system's named cursors, while the existing selected-theme trial continues to use decoded texture cursors. The current test uses native controls independently of the selected-theme playground. The current test never assigns a cursor to the GtkWindow or its GDK surface, leaving real borders/corners under native handling. Input widgets retain their own native cursor behavior; no preview cursor notification guard is installed.

Thirty-four GDK role names have separate hover targets and manual observations. Busy and Working remain separate. Eight additional checkboxes refer specifically to real window edges/corners; hovering a directional tile does not complete those checks. There is no automatic visual PASS. Closing, changing theme/size/window scale, catalog refresh or explicit recheck clears observations; native pointer focus and grabs are left to GTK and the window system on close.

Exact-name file diagnostics reuse `ThemeRepository`, its decoder/cache and the existing bounded worker framework. An audit holds metadata only, probes one file at a time, cooperatively cancels and drops stale request generations. It neither groups eight resize names into generic axes nor substitutes selected-theme textures for the system cursor. File evidence uses GTK's reported theme in normal mode. Fixture GUI checks explicitly use fixture files while their named cursors still belong to the ambient desktop.

The window has no desktop settings port, controller, installer or settings setter. Its only GTK Settings operations read the theme name and size. Demo links and save buttons change local labels/content only. There are no new dependencies or changes to the four-layer architecture.

System lookup, fallback and cursor-shape handling are backend-dependent. The app does not claim its file resolver can identify the compositor's currently loaded image. References: [GDK named cursors](https://docs.gtk.org/gdk4/ctor.Cursor.new_from_name.html), [GTK 4.22 Wayland cursor handling](https://github.com/GNOME/gtk/blob/gtk-4-22/gdk/wayland/gdkcursor-wayland.c). No third-party implementation was copied.

The earlier card-test results below are historical. The native-path revision at the end supersedes the simulated-card behavior.

## Automated validation

Command: `./scripts/check.sh --gui` (local log: `target/qa/current-cursor-check.log`).

- Formatting, core/app tests, workspace clippy with warnings denied, workspace tests and release build: PASS, 73 tests total.
- Regression: a valid generic horizontal resize file cannot make any missing native edge/corner pass its exact-name check. Busy fallback and Working inheritance retain their distinct provenance. Cancellation stops file probing.
- Existing fixture settings writer rejects every write; GUI checks assert unchanged snapshots and absent Undo records.

## Isolated GUI validation

Local GTK 4.22.4, Wayland; private D-Bus session, memory settings and fixture repository. The final diagnostic run (`WAYLAND_DEBUG=client ./scripts/check-gui.sh`) passed Adwaita, Adwaita dark and HighContrast; log: `target/qa/current-cursor-gui-diagnostic.log`. The final main-window capture reported scale 2 for Adwaita and scale 1 for the other two styles; these observations do not certify mixed-DPI pointer behavior.

Two ordinary runs stopped in the existing import smoke sequence, before opening the current cursor test, with Wayland Error 71. The diagnostic rerun did not reproduce the error, so its cause remains unresolved; this is not an unconditional stability claim. Failure logs are retained as `target/qa/current-cursor-wayland-failure.log` and `target/qa/current-cursor-wayland-second-failure.log`.

Checks exercise the menu entry, repeated-open singleton, 34 named requests, eight independent unchecked native controls, input child cursor override, local link, theme/request invalidation, drag cleanup, close/reopen and no auto-accepted observations. Captures inspect layout only; they do not capture or certify the moving system pointer. The private portal may log shutdown/service-availability warnings; these are separate from application GTK/GDK failures.

Layout captures: [Playground](screenshots/current-cursor-playground.png), [All roles](screenshots/current-cursor-roles.png).

## Real GNOME acceptance — pending

No new real-pointer acceptance is recorded by automated tests. Earlier selected-theme trial acceptance does not complete this checklist.

- [ ] Confirm desktop and GTK theme/size, and compare the visible cursor with the applied theme.
- [ ] Normal pointer; selection and typing in both native input widgets; native pane divider; remaining named roles on All roles.
- [ ] Busy and Working animation, including direction/order/timing where animated assets exist.
- [ ] Both diagonal tiles and each remaining role in All roles; record deliberate fallback separately from theme-provided imagery.
- [ ] Real left/right/top/bottom border resize, observing cursor and successful resize.
- [ ] Real top-left/top-right/bottom-left/bottom-right corner resize, observing cursor and successful resize.
- [ ] Drag the real title bar and native divider; release, leave the window, close and reopen; no stuck cursor.
- [ ] Change applied theme while the test is open; old requests/observations do not survive.
- [ ] Wayland fractional/integer and mixed-monitor scaling, hotspots, input widgets and post-close cursor recovery.
- [ ] If files were replaced in place, verify what the compositor actually loaded; Recheck files is not a system cursor cache reload.

No host Apply/Undo, external-conflict, theme-directory write, push or release was performed for this change. Existing real-GNOME Apply/Undo and external-conflict acceptance gates remain pending in their respective validation documents.

## Card hover and drag correction — 2026-09-16

User feedback: real window resizing was correct, but card move/resize areas in the test window appeared wrong already on hover. The center-of-handle hit test passed; it did not establish correctness over the whole visible card. A new regression then reproduced a concrete defect at the card border/body: the nearest effective cursor was `crosshair` inherited from the canvas, not `move` (`target/qa/card-surface-regression.log`, failed before the fix).

Both card containers now own their move cursor and drag gesture, covering body and margins. Container assignment does not recurse over independently configured resize children. Resize gestures claim their sequence to prevent an ancestor card from moving at the same time. Right/bottom handles are wider and visibly marked; their actual hit targets and cursor properties are checked. In the current-system window, right, bottom and bottom-right handles request `e-resize`, `s-resize` and `se-resize`, matching those real border directions. The All roles page retains independent generic-axis tests. Selected-theme trials retain their existing decoded texture roles.

Missing or inherited/fallback evidence is also available beside the active Playground role, so users need not leave the interaction to discover a missing source. The installed Miku files were read only: move and both straight resize axes exist; both diagonals are absent in this installation. Its conversion report explicitly records those missing roles. An independent libXcursor lookup found the themed straight resize assets but a fallback diagonal. The Move asset itself is a cyan four-way arrow, not a character illustration. No user theme file was rewritten, and no desktop setting was toggled to clear caches.

Final checks: PASS — format, core/app tests, workspace clippy, all 73 workspace tests, release build, and all three isolated GUI styles on Wayland at scale 1 (`target/qa/card-cursor-fix-final.log`). No application GTK/GDK warnings or Wayland protocol error occurred in this final run. The new checks cover allocated card-border hit testing and inherited cursor lookup, handle targets, directional names, whole-card drag/end, missing diagonal indication, and existing close/theme-change/no-write behavior. They still do not certify the user's moving pointer; that real-session retest remains pending.

## Native path revision — 2026-09-16

The latest installed Miku was checked read-only and includes both diagonal assets and directional aliases. Its conversion report no longer lists missing roles; the older installation findings above do not describe the current installation.

The former current-system playground reused the texture trial's child `cursor` notification guard. A regression first demonstrated that a child changing its native cursor to `default` was forcibly changed back to `text` (`target/qa/system-native-before.log`). This proves unwanted interference with native behavior; it does not establish the cause of every previously reported pointer mismatch.

**Real window** now tests actual title-bar movement and outer-edge/corner resizing. Its GtkEntry, GtkTextView and GtkPaned use their native behavior. The current test no longer builds the simulated-card view, binds child cursor overrides, applies cursor changes on each frame, or controls any drag gesture. All roles retains explicit named requests on ordinary buttons without recursive cursor forcing. The selected-theme texture trial remains unchanged. No desktop settings, theme files or system cursor cache are modified.

Validation for this revision is recorded separately below. Real pointer shapes, animations, border operations, native input/divider behavior, scaling and post-close recovery remain pending user acceptance; a successful cursor property test or screenshot is not visual acceptance.

Final validation: `./scripts/check.sh --gui` PASS (`target/qa/system-native-final2.log`). Formatting, core/app tests, workspace clippy with warnings denied, all 73 workspace tests and release build passed. Isolated Wayland GUI checks passed in Adwaita, Adwaita dark and HighContrast at scale 1, using the existing rejecting settings writer and fixture installation paths. No application GTK/GDK warning or Wayland protocol error was found in these final runs. The captures below show the native page and role page, not the rendered pointer.

Two intermediate checks failed in the test harness: the first incorrectly assumed GtkTextView's native cursor was unset (it owns a native text cursor); the second changed the pane allocation immediately before taking a screenshot. The checks now compare with a fresh native control and exercise divider position after captures. These corrections do not override the controls' cursor behavior.

Real GNOME acceptance remains pending for this revision. The already-running desktop application must be closed and relaunched to use the rebuilt binary. No push or release was performed.

## Native pane divider aliases — 2026-09-17

The installed Miku had `ew-resize` and `ns-resize`, but no `col-resize` or `row-resize`. Read-only inspection of the local GTK 4.22.4 GtkPaned handle confirmed those latter requests for horizontal/vertical panes, matching [GTK's native implementation](https://github.com/GNOME/gtk/blob/gtk-4-22/gtk/gtkpaned.c). Real window-edge names are not sufficient coverage for a pane divider. The earlier native-view revision removed preview interference but did not fix this export omission.

A regression first failed with `window resize cursor col-resize missing` (`target/qa/pane-alias-before.log`). Import now exports the confirmed horizontal artwork under `col-resize` and vertical artwork under `row-resize`, without rotation, mirroring or animation changes. The conversion notes explicitly describe this reuse. Missing-role summaries specify the Windows import set and do not imply that every Linux cursor role is present. Current-system tests still use native GTK behavior; they do not change the separator's requested cursor to disguise missing theme files.

Automated checks: `./scripts/check.sh --gui` PASS (`target/qa/pane-alias-check.log`): formatting, core/app tests, workspace clippy, 73 workspace tests and release build. The installed-output regression checks byte identity and decoded pixels, dimensions, hotspots, frame order and delays for both new names and all existing edge/corner aliases. The current-cursor audit still reports absent divider names independently of generic resize files.

Independent reader: `python3 scripts/reference-import.py` PASS (`target/qa/pane-alias-reference.log`), including direct-file and theme/name libXcursor lookup of both pane names, multiple sizes and animated frames. Isolated Wayland GUI: PASS in Adwaita, Adwaita dark and HighContrast, reported scale 2; the smoke test now compares native GtkPaned handle requests against the exported alias lists. No application GTK/GDK warnings or protocol error were found. This does not certify actual moving pointers or mixed-DPI behavior.

Real GNOME acceptance remains pending. Two byte-identical copies of the user's existing Miku resize files were staged and independently decoded under `target/qa/miku-pane-repair-f7p9em4o/`; each contains four 160×160 frames at 150 ms per frame with preserved hotspots. The manifest lists the exact source/destination paths and SHA-256 hashes. No files were added to the real theme, no settings changed, and nothing was pushed or published. Applying this staged repair to the existing theme requires user confirmation under the earlier create-only installation constraint. A running compositor may retain an already-loaded fallback even after files are added; no automatic settings toggle/cache reset is performed.

## Pre-install system file coverage — 2026-09-17

Import review and installation confirmation now display **System files: covered/34 · missing**, using the same exact-name set as Test current cursor. Details list each missing role's friendly name and requested filename. Windows-role completeness is reported separately. No inherited or generic-axis substitute is counted as an exported exact-name file, and no new cursor images are fabricated. Partial themes remain installable after explicit review; missing names may use desktop fallback. The fixed set does not claim coverage of all possible application cursor names.

The installer compares coverage from successfully written, decoded and verified staged files with the reviewed plan before committing the new theme. It saves the resulting missing-name list in `conversion-report.txt`. This does not test the compositor's cached or rendered cursor. Existing-theme audit remains in **Test current cursor → All roles**.

Regression first failed against the old report (`target/qa/system-coverage-before.log`): a complete 15-role Windows import did not report its remaining system gaps. It now reports 23/34 covered and 11 missing, including Grab and Zoom in, while both pane divider names are covered. Additional tests check exact-name counting, duplicate/unknown names, Busy versus Working, absent divider names and all/empty coverage. GUI checks assert visible review/confirmation counts and expandable missing-name details.

Final command: `./scripts/check.sh --gui` PASS (`target/qa/system-coverage-check.log`). Automatic: format, core/app tests, workspace clippy with warnings denied, all 75 workspace tests and release build. Isolated GUI: Adwaita, Adwaita dark and HighContrast on local Wayland, reported scale 2, with the existing rejecting settings writer and QA-only import destination. No application GTK/GDK warnings or Wayland protocol error were found in the final run. Real GNOME moving-pointer, scaling and post-close acceptance remains pending. No installed Miku file was changed; the previously staged two-file repair is still awaiting confirmation. No settings writes, push or release were performed.

## Approved installed-theme repair — 2026-09-17

The user subsequently authorized the pending two-file Miku repair. Both pane names have now been added without overwriting any file; all 56 existing files and the desktop theme/size were unchanged. A fresh independent libXcursor audit verified 23 of the 34 exact-name Miku files and identified the remaining 11 missing names. See [the complete audit](MIKU_SYSTEM_AUDIT.md) for the per-role results, original ZIP checks and real-pointer acceptance boundary. Earlier statements that this repair is awaiting approval are historical and superseded by this record.


## System-test interface refresh — 2026-09-19

The reusable window is now titled **System cursor test**. Open it from the existing **Test current cursor** menu item. The **Everyday use** page replaces the former Real window layout: a larger native entry/editor, a normal-background area, and clearly visible horizontal and vertical GtkPaned separators. Both separators use GTK's own cursor and drag handling. The eight real-border observations are arranged spatially around a central reminder; they are checkboxes, not simulated resize handles. Actual border tests still require dragging the outer window edges and corners.

**All roles** retains all 34 exact-name targets, including separate Busy and Working in background targets. Four categories make everyday, move/drag, resize/split and precision roles easier to find. Category filtering preserves observations and provides an explicit All categories entry. The active hover label identifies the current named request. Observation menus show Not checked, Looks correct or Problem; scrolling the page cannot change these values. Only explicit observation changes affect card borders. No image preview or selected-theme cursor has been substituted for a system cursor.

The theme/size badge reports GTK's current values. Session metadata and aggregate file evidence are expandable; setting mismatches, audit failures and inherited/fallback/unverified-file notices stay visible. Per-role source paths and chains remain in tooltips. The main surfaces use the native light/dark palette, with larger headings and restrained borders. Both pages scroll at smaller window sizes, and the observation summary remains in the footer. Reset & recheck clears observations and re-reads files without claiming to reload any desktop cache.

### Automated checks

PASS: `./scripts/check.sh`, exit 0; log `target/qa/system-ui-final-check.log`. Rust 1.98.0, GTK 4.22.4. This runs formatting, core/app tests, workspace clippy with warnings denied, all 79 workspace tests, and the release build. The change stays within the GTK layer; no new dependency, settings writer, installation behavior, or architecture was introduced.

### Isolated GUI

PASS: final `./scripts/check-gui.sh`, exit 0; log `target/qa/system-ui-gui.log`. Adwaita, Adwaita dark and HighContrast completed on the local Wayland display, reported scale 1, using private D-Bus sessions, fixture files and the existing rejecting settings writer. Checks cover all role targets attached to groups, four category filters and return to All categories, scroll-safe observation controls, native entry/editor behavior, both native pane orientations, theme/request invalidation, singleton/reopen/close, and the 780 × 640 compact layout. Existing fixture checks continue to assert unchanged settings and no Undo record. Screenshot inspection covered default, filtered, compact, light, dark and high-contrast layouts; it does not establish pointer rendering.

An earlier run passed the light style, then failed in the existing import smoke test at stage 8 while waiting to capture its window, before the current-system test opened in the dark run. That failure is retained in `target/qa/system-ui-gui-first.log`. No import assertion or timeout was weakened; the final complete run passed. The intermittent import capture timeout is not claimed fixed by this UI change. Final application logs contain no GTK/GDK warning or critical; private portal service/shutdown warnings remain separate.

Layout captures: [Everyday use](screenshots/current-cursor-playground.png), [All roles](screenshots/current-cursor-roles.png), [Resize category](screenshots/current-cursor-resize.png), [Dark](screenshots/current-cursor-dark.png), [High contrast](screenshots/current-cursor-highcontrast.png), [Compact](screenshots/current-cursor-compact.png).

### Real GNOME and unverified environments

NOT RUN for this revision: actual moving-pointer shapes/hotspots/animation, real outer-border resizing, native input and both dividers under physical pointer interaction, fractional/mixed-monitor scaling, and recovery after closing. Keep the real GNOME checklist above pending until explicitly observed. X11/XWayland was not rerun for this layout revision. Automatic checks did not Apply/Undo host settings, modify installed themes, or certify QQ/Clash behavior. No push or release was performed.
