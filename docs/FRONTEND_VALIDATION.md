# Frontend reference integration — 2026-09-15

The supplied `cursor-mochi-design-package.zip` was inspected and its self-contained `index.html` rendered locally in headless Chrome. Its editable `src/playground.html` supplied the layout and interaction reference. The original archive is preserved.

The design is implemented as native GTK widgets and CSS in the existing Rust application. There is no WebView, JavaScript runtime, new dependency, decoder replacement, or change to the four crate boundaries. Illustrative SVG cursors and simulated settings actions from the prototype are replaced by the existing real cursor data and settings controller. GTK symbolic icons and a small Cairo drawing provide the UI illustrations; bundled Lucide assets are not copied into the application.

## What changed

| Reference element | Native implementation |
|---|---|
| Theme sidebar and current setting | Existing verified catalog, search, real thumbnails, selection, and independent In use state; current setting moved to sidebar footer |
| Theme heading and small cursor | Actual active role/frame texture, with role label; unavailable roles remain explicit |
| Size and background controls | Trial nominal size and light/dark/checkerboard swatches; independent from inspection zoom, card scale, and GNOME size |
| Notes | Editable title and text, whole-note bold/italic toggles, local Save/Open link feedback |
| Canvas | Cover and Idea cards; move, horizontal/vertical/diagonal resize, keyboard arrows, 12-pixel snapping, 75–125% card scale, click marker |
| Run, Working, Unavailable | Local demo state with separate Busy/Working cursor roles; Reset invalidates the pending Run completion |
| Reset | Restores notes, formatting, cards, snap/scale, demo labels, and click marker; cancels an active drag |
| Apply/Undo | Existing GNOME transaction/no-op/conflict/Undo behavior; size opt-in retained; no simulated success |

Playground is now the default page. Inspect retains the previous role/frame inspector. Open trial window reparents the same playground into the existing nonmodal window; closing restores it to the main page. No second trial engine or worker is created. Cursor loading, shared cache, generation checks, frame timing, child cursor protection, and settings isolation are reused.

Widget construction is in `crates/cursormochi-gtk/src/trial/view.rs`; cursor lifecycle/loading remains in `trial.rs`. Fixed positioning is confined to the draggable demo cards. The surrounding application uses native layout containers. The authoritative card transform is used for drag/keyboard coordinates, avoiding stale allocated positions; see GTK's [Fixed transform API](https://docs.gtk.org/gtk4/method.Fixed.get_child_transform.html).

The shared friendly-role table now also exposes diagonal resize and crosshair aliases without hiding other verified files. Missing roles do not receive illustrative prototype cursors.

## Automated checks

`./scripts/check.sh --gui` exited **0** after the final changes. Local log: `target/qa/frontend-check.log`.

- Formatting, core/app tests, workspace Clippy with `-D warnings`: PASS.
- Locked workspace tests: **46 passed** — core 10, app 17, platform 17, GTK worker 2.
- Locked release build: PASS.
- `python3 scripts/reference-xcursor.py`: PASS for all three reference fixtures; log `target/qa/frontend-reference.log`.
- Existing nonmodal trial smoke: PASS, including static-before-animation object checks, child cursor override protection, stale selection handling, size, close/reopen, and cleanup.
- Frontend smoke: PASS for shared embedded/detached content, Notes reset, card scale, snapped coordinates, keyboard movement, background selection, active-role thumbnail, responsive layout, and access back to Inspect.
- The GUI settings port still fails on any attempted settings write. No Undo record or settings snapshot change occurred.

During development, an assertion exposed a stale card-position read during snapping; the handler now reads the configured transform. An early screenshot ran before the stack finished allocation; it now waits for the new page to settle. Visual inspection also exposed unreadable text when the desktop theme was dark but the playground palette was light. Explicit foreground rules for each playground palette fixed that combination. Full checks were rerun afterward.

## Isolated GUI and visual layout review

All three styles passed using the existing Wayland display, private D-Bus sessions, read-only fixtures, and an explicitly injected memory settings backend. Environment: Ubuntu 26.04.1 LTS x86_64, Rust 1.98.0, GTK 4.22.4, GIO 2.88.0. Backend: `GdkWaylandDisplay`; observed scale factor: **1**.

| Desktop style | 1120×820 window, light playground | 820×680 window, dark playground |
|---|---|---|
| Adwaita | [Large](screenshots/frontend-light.png) | [Compact](screenshots/frontend-light-compact.png) |
| Adwaita:dark | [Large](screenshots/frontend-dark.png) | [Compact](screenshots/frontend-dark-compact.png) |
| HighContrast | [Large](screenshots/frontend-highcontrast.png) | [Compact](screenshots/frontend-highcontrast-compact.png) |

The screenshots capture client content and omit the native title bar. They were reviewed against the supplied reference. The narrow layout stacks Notes and Canvas vertically; scroll to reach the lower demo area, while the real settings actions remain outside the scroller. The Inspector was also exercised at 1100×840. Local `polish-gui-*.log` files contain GUI_SMOKE, TRIAL_SMOKE, and FRONTEND_SMOKE results.

## Acceptance boundaries

These new GUI runs verify widget behavior and rendered layouts. They are not recordings of a moving pointer. The user's earlier confirmation of basic trial cursor behavior remains recorded in [TRIAL_VALIDATION](TRIAL_VALIDATION.md); it is not automatically extended to the new embedded layout, added roles, or new input regions.

Real-pointer behavior across new regions and detach/reattach, focused Notes editing, diagonal handles, 200%/fractional/mixed-DPI scaling, and full screen-reader acceptance remain pending. Real GNOME Apply/Undo and external conflict acceptance also remain pending under the [existing checklist](CLOSEOUT_VALIDATION.md#real-gnome-acceptance-checklist--not-run).

The new GitHub workflow has not been pushed or run on a hosted runner. No real desktop settings, theme directories, or Undo history were modified by the demo. No store, importer, installer, push, or release was added or performed.

## Sidebar follow-up

Removed the theme-count line at the user’s request. The sidebar now uses the same theme background as the main area, and cursor thumbnails have transparent backgrounds. Selection and keyboard-focus indicators remain. `./scripts/check.sh --gui` passed again (46 tests, formatting, Clippy, release build, and all three isolated GUI styles); local log: `target/qa/sidebar-colors-check.log`. The frontend captures above have been refreshed.

## Later import extension

The subsequent user-authorized CUR/ANI import workflow and its current acceptance boundaries are documented in [IMPORT_VALIDATION.md](IMPORT_VALIDATION.md). Earlier “no import” statements in this report describe the frontend-only change at that time.


## Layout consolidation — 2026-09-15

The current UI supersedes the earlier sidebar-footer and source-summary layout above.

- The main header retains Playground / Inspect. Refresh is beside Themes; the pop-out icon is at the right of the Playground chrome and opens/focuses the existing nonmodal window.
- The theme heading uses a 44-pixel actual cursor thumbnail with the current role beside the name. Availability appears only for nonzero missing/inherited/fallback counts, with expandable reasons. Counts use the resolution enum, not matching source-path text.
- Notes has a single formatting toolbar and bottom link/save actions. Canvas owns snapping and scale. Wide layouts allocate about 40% to Notes and 60% to Canvas; narrow layouts stack them and scroll.
- Larger cards have clearer drag headers and wider resize targets. Busy, Working, and Unavailable are equal-sized demo controls with local feedback; Reset restores their labels too.
- Body and button text is 14 pixels; secondary labels are 12 pixels. Independent preview palettes cover spin controls, checkboxes and sliders as well as text. The checkerboard remains a transparency inspection aid.
- Source/hotspot details, current desktop settings and capability notes are in Inspect. The bottom action bar contains Apply size, the current state, Undo and Apply theme. Matching settings show In use; operational errors remain visible and may wrap rather than being truncated.
- Hidden trial source details are cleared when selecting a new theme, so Inspect cannot display another theme's stale trial metadata. The existing individual-role inspector still loads its own current selection.

### Verification for this revision

Command: `./scripts/check.sh --gui`; local log: `target/qa/layout-check.log`.

Automated checks: formatting, locked core/app tests, workspace Clippy (`-D warnings`), all **65 workspace tests**, and the locked release build passed.

Isolated GUI: Adwaita, Adwaita:dark and HighContrast passed on the available Wayland display with a private D-Bus session and the injected no-write settings port. Regression checks cover refresh/pop-out parentage, source details outside Playground, conditional availability text, 40/60 allocation, compact layout, the horizontal action bar and visible In use state, as well as the existing cursor, drag, note, import and cancellation scenarios. Import smoke only installs fixtures under unique `target/qa` directories. No desktop settings write or Undo creation occurred.

Screenshots above are refreshed for this revision. Layout inspection covered both wide and narrow windows and opposite desktop/preview palettes. Early checks exposed proportional reallocation drift, repeated invalidation at the vertical divider's minimum, and preview control contrast problems; these were corrected before the final run. Display scale factors of 1 and 2 were observed during local runs; this is not controlled fractional or mixed-monitor pointer acceptance.

Real GNOME acceptance remains **NOT RUN for this revision**: actual moving-pointer shape, hotspot, animation and region transitions, focused text input, detach/close recovery, controlled scaling, Apply/Undo and external settings conflicts. The earlier user acceptance is not automatically carried forward to these UI changes. No user theme directory or real desktop setting was modified, and nothing was pushed or published.


## Brighter header, user themes first, larger thumbnails — 2026-09-15

The main native header now has a white surface with dark text and controls. User-directory themes, including imported themes, precede system themes after every scan. Ordering uses discovered locations rather than session-only import history, retains the existing order within each group, and does not change resolution precedence or merge away locations. Search and the selected theme remain independent of the ordering.

Availability counts now occupy a menu at the right of the theme heading. Nonzero missing, inherited and fallback counts remain explicit; clicking opens their reasons. The current role stays beside the theme name. The actual cursor thumbnail is 64 pixels in the heading and 44 pixels in the sidebar, without an icon background. Trial cursor size is unchanged and remains separate from these thumbnails.

Automated verification: `./scripts/check.sh --gui` exited 0; log: `target/qa/main-header-check.log`. Formatting, locked core/app tests, workspace Clippy with warnings denied, all **66 workspace tests**, and the locked release build passed. The new sorting regression covers mixed user/system locations, stable order, a similarly named non-user directory, retained locations, selection and search. Import GUI checks also verify the installed fixture appears first after refresh.

Isolated GUI: Adwaita, Adwaita:dark and HighContrast passed on the available Wayland display at scale 1, using a private D-Bus session and the no-write settings port. Wide and compact layouts were reviewed. These additional captures include the native header, unlike the content-only captures above: [Light](screenshots/frontend-light-header.png), [Dark](screenshots/frontend-dark-header.png), [High contrast](screenshots/frontend-highcontrast-header.png). No GTK/GDK warning or critical was found in the final logs. The session portal emitted shutdown/service notices; these are not cursor verification results.

Real GNOME moving-pointer, focused input, scaling, Apply/Undo and external conflict acceptance were **not rerun** and remain separate from these automated/layout checks. Fixture installation stayed inside disposable `target/qa` directories. No real desktop settings or user theme directories were modified; nothing was pushed or published.


## Thumbnail clarity — 2026-09-15

The previous size increase exposed two image-quality problems: sidebar variant selection still targeted nominal size 24, and the theme heading enlarged the already resized trial cursor texture. Default texture interpolation also blurred enlarged pixel edges.

Sidebar selection now prefers a source with enough actual pixels for its display size and current integer widget scale. The theme heading independently retains a suitable original variant (target 128 source pixels for its 64-pixel view), preserving that variant's animation order and timing. It never uses the reduced trial-cursor texture as its image source. Both main thumbnail locations use a small GTK paintable with nearest-neighbor enlargement and Cairo Best downsampling. Alpha remains premultiplied; aspect ratio and transparent backgrounds are retained. This uses existing GTK/Cairo APIs without changing dependencies, runtime requirements, decoding or real cursor rendering.

Decoded source buffers are shared. Heading source retention is bounded to 64 MiB across roles and 256 frames per role, with one visible frame surface created on frame changes. Retention errors appear in the thumbnail tooltip instead of substituting a low-resolution cursor. Existing workers, stale-result checks and the 16-nearby-thumbnail limit remain in place. Import trials use the same source-frame entry point.

Automated checks: `./scripts/check.sh --gui` exited 0; local log `target/qa/thumbnail-clarity-check.log`. Formatting, core/app tests, workspace Clippy with warnings denied, all **69 workspace tests**, and release build passed. New regression checks cover sufficient source-size selection, independence from 24-pixel trial preparation, original animation metadata, shared buffers, retention limits, and exact enlarged pixel/alpha values without interpolated edge colors.

Isolated GUI: Adwaita, Adwaita:dark and HighContrast passed on Wayland at scale 1 with private D-Bus and the no-write settings port. The frontend captures were refreshed and inspected. A separate local, read-only comparison reused the existing decoder, old trial preparation and new paintable on the installed Miku normal cursor: its source is 160×160. `target/qa/miku-thumbnail-comparison.png` shows the old and new 64-pixel rendering. That user artwork stays outside repository screenshots. No GTK/GDK warning or critical appeared in the smoke logs; the private session portal still emits service/shutdown notices.

Real GNOME moving-pointer, focused-input, Apply/Undo, external-conflict and controlled fractional/mixed-monitor scaling acceptance were not rerun. No user theme files or desktop settings were written, and nothing was pushed or published.


## Spacious Inspect layout — 2026-09-15

Inspect now shares the importer's spacious preview setup. The image viewport expands into available height instead of staying at 180 pixels, starts at 1×, and has no added background by default. Its compact toolbar keeps the theme name, verified role selector and animated cursor Pause/Play visible. Long role labels ellipsize to keep the toolbar usable in narrower windows; every role remains selectable.

Nominal size, zoom, background, hotspot controls, frame metadata, file/source information, trial source details, theme summary and environment/current-settings information now live under **Preview settings & details**. The main inspector bounds the expanded details scroller to 220 pixels so it cannot consume the entire viewport. Existing source warnings remain above the canvas, and operational errors remain in the action bar. Import preview behavior and actual widget cursors are unchanged. No decoder, worker, cache, installation or settings-writing behavior was changed.

Automated checks: `./scripts/check.sh --gui` exited 0 (`target/qa/inspect-space-check.log`). Formatting, locked core/app tests, workspace Clippy with warnings denied, all **71 workspace tests**, and release build passed. The GUI regression verifies the default 1×/transparent setup, a role selector outside collapsed details, and preview allocation of at least half the initial window height. Existing role, size, animation pause/hide, hotspot, details and import checks also passed.

Isolated GUI: Adwaita, Adwaita:dark and HighContrast passed on the available Wayland display at scale 1 with the no-write settings port. Screenshots were reviewed for default and expanded-details layouts; final logs contain no GTK/GDK warning or critical.

| Style | Default Inspect | Expanded details |
| --- | --- | --- |
| Adwaita | [Preview](screenshots/inspect-light.png) | [Details](screenshots/inspect-light-details.png) |
| Dark | [Preview](screenshots/inspect-dark.png) | [Details](screenshots/inspect-dark-details.png) |
| High contrast | [Preview](screenshots/inspect-highcontrast.png) | [Details](screenshots/inspect-highcontrast-details.png) |

These original fixture captures verify layout, not moving-pointer acceptance. Real GNOME Apply/Undo, corner resizing, focused input and controlled scaling acceptance were not rerun. No real desktop settings or user themes were modified; nothing was pushed or published.


## Persistent Inspect role thumbnails — 2026-09-16

Inspect now has a permanent **Cursors** thumbnail list to the left of its large preview. Clicking a row selects that exact role file; GTK list navigation remains available. Rows show a 44-pixel real source thumbnail, a friendly role name and the filename. All verified files remain accessible, including additional files within friendly groups. The previous role dropdown is hidden; the preview header shows the selected role alongside the theme, with animation Pause/Play retained. The theme browser stays separate. Preview settings/details remain collapsible.

The role list reuses the existing thumbnail job and source-pixel paintable. Theme and role requests alternate through the same bounded worker, with one outstanding thumbnail request; no decoding occurs in mouse/selection callbacks and no new worker is added. Each list retains at most 16 viewport-adjacent thumbnails. Rebuilding roles increments a generation, and both generation and catalog epoch prevent obsolete results from reaching a new theme. Hidden/offscreen thumbnails are released; their entries remain available. Source state or a thumbnail failure is available in each row's tooltip, with complete sources and warnings in the selected preview.

Automated verification: `./scripts/check.sh --gui` exited 0 (`target/qa/inspect-role-list-check.log`): formatting, core/app tests, workspace Clippy with warnings denied, **71 workspace tests**, and release build passed. GUI regressions verify the role list is visible without the old dropdown, row hit testing targets selection rather than text, row selection updates the exact preview role, every verified file has an entry, nearby thumbnails load within the retention bound, and a previous generation cannot modify the new row images. Existing animation, settings no-write, import and cancellation checks also passed.

Isolated GUI: Adwaita, Adwaita:dark and HighContrast passed on the available Wayland display at scale 1. Wide 1120×820 and compact 820×680 layouts were inspected; the role list remains on the left. Updated [light](screenshots/inspect-light.png), [dark](screenshots/inspect-dark.png), [high contrast](screenshots/inspect-highcontrast.png), and [compact](screenshots/inspect-light-compact.png) captures use original fixture artwork. Final logs contain no GTK/GDK warnings or criticals.

Real desktop cursor, Apply/Undo, external-conflict and controlled scaling acceptance were not rerun. This change does not alter real cursor rendering or theme files. No desktop settings writes, push or publication occurred.
