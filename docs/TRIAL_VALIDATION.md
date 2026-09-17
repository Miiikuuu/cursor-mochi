# Interactive cursor trial — implementation and validation

Date: 2026-09-15. This record describes the original trial-window layout; the subsequent [frontend integration](FRONTEND_VALIDATION.md) makes the shared Playground the default page and retains a detachable window. This adds a read-only companion to inspection. It does **not** implement or replace CUR/ANI parsing, conversion, import, or installation. Rust + GTK4/GIO and the four existing crate boundaries remain unchanged.

## Using the trial window

Select a theme and click **Try this cursor theme** in its details. The window is nonmodal; clicking again presents the same window. Main-window selection and refresh update its theme. Changing the trial's nominal size (16–64) reloads its cursors independently of inspection zoom or GNOME cursor size.

Move over the ordinary background, select text, type in the entry, click the demo link, and drag the card's header or resize edges. Background-work, busy, and unavailable regions exercise their respective roles when available. Actions affect demo content only. The link is a normal button with an internal callback, not a URI launcher. The cross records actual click coordinates for manual hotspot checks; it is not a pointer-following image.

Source details list each role's file, Direct/Inherited/Fallback resolution, chain, actual dimensions, hotspot, and frame count. Missing or rejected roles are explicitly unavailable and use the ambient widget pointer, which is not evidence of the selected theme. Role regions also expose their source details as tooltips. Busy and Working in background remain separate.

## Implementation

- `app::trial` accepts the existing decoded `Preview` model. It chooses the nearest nominal variant, preserves frame order and delays, scales pixels and hotspots together, and enforces budgets. Its public `prepare` function can process a future importer's decoded data before installation; it has no settings capability or installed-theme precondition. The GTK renderer consumes the same prepared previews in `TrialSet`.
- Role aliases reuse `browser::ROLE_GROUPS`. Bounded probes prefer direct or explicit inherited sources over automatic default fallback, which remains marked as fallback. No new decoder was added.
- A dedicated instance of the existing bounded worker handles trial loading, sharing the repository's 128 MiB cache with inspection and thumbnails. There are now three worker threads in total. The trial queue and output channel each hold one result/request. Generation IDs, selected theme, and catalog epoch prevent stale replacement; close and theme/size changes cancel pending work and clear old cursors.
- Preparation runs off the GTK thread. At most 256 frames per role, 256×256 pixels per frame, and 16 MiB of prepared pixel data across the trial set are retained. Budget failures are visible errors, not silent first-frame truncation. These limits do not describe total process RSS or driver memory.
- GTK creates actual `GdkCursor` objects using each prepared texture and hotspot, with no named fallback cursor supplied. A 16 ms UI timer selects frames using the existing delay policy (16–10000 ms), replacing widget cursor objects when frames change. Static roles retain their cursor. No file decoding occurs in motion or drag callbacks. Hidden/minimized windows pause the trial clock.
- Cursor properties are installed on each region and its children, including the entry's internal `GtkText`; property notifications restore the trial cursor if a child replaces it. Dragging temporarily retains the drag role across regions, then restores per-region cursors. Close, theme changes, gesture cancellation, and drag end clear the drag state. GTK handles ordinary region leave; no global desktop/device cursor is changed.
- The trial has no `DesktopSettingsPort` or `Controller`. It writes neither theme directories nor desktop settings and cannot create an Undo record. The original role/frame inspector is retained.

The APIs are documented by GTK: [texture cursors](https://docs.gtk.org/gdk4/ctor.Cursor.new_from_texture.html) and [widget-local cursor assignment/inheritance](https://docs.gtk.org/gtk4/method.Widget.set_cursor.html). Successful API calls alone are not visual acceptance.

## Automated checks

`./scripts/check.sh --gui` exited **0** after the final implementation. Local log: `target/qa/trial-check.log`.

| Check | Result |
|---|---|
| Formatting, core/app tests, workspace Clippy with `-D warnings` | PASS |
| Locked workspace tests | PASS: 46 tests — core 10, app 17, platform 17, GTK worker 2 |
| Locked release build | PASS |
| `python3 scripts/reference-xcursor.py` | PASS, all three original reference fixtures |
| Uninstalled decoded preview preparation | PASS: dimensions/hotspot scaling, delays, provenance, and frames retained |
| Trial limits and cancellation | PASS: explicit errors |
| Alias lookup | PASS: prefer direct over default fallback; distinguish fallback and missing |
| Worker replacement | PASS: latest trial theme and size retained |

The first static/animation GUI run passed before expanded child-widget checks were added. A later layout-capture attempt failed because it ran before GTK allocation; the capture was moved to the settled stage and the complete checks rerun. No moving-pointer visual result was inferred from either run.

## Isolated GUI checks

Adwaita, Adwaita:dark, and HighContrast all passed on the existing Wayland display, using private D-Bus sessions, original fixtures, and an explicitly injected memory backend. Actual backend: `GdkWaylandDisplay`; observed scale factor: **1**. Local native environment: Ubuntu 26.04.1 LTS x86_64, Rust 1.98.0, GTK 4.22.4, GIO 2.88.0.

The trial smoke sequence checks static cursor textures and hotspots first, then animation frame transitions. It exercises the real details button, repeated presentation, latest-theme replacement, independent trial size, local link/text actions, programmatic move/resize gestures, closing during replacement, cursor release, and reopening. It checks the internal entry child and simulates that child resetting its cursor property. The complete GUI flow wraps the memory settings port with `NoWrites`, which fails on **any** write call, including a rejected/no-op/compensated write; it also asserts no Undo record and an unchanged settings snapshot.

These are programmatic checks. Gesture signals and cursor properties do not prove the shape of a physically moving pointer. The [light](screenshots/trial-light.png), [dark](screenshots/trial-dark.png), and [high-contrast](screenshots/trial-highcontrast.png) layout captures were opened and inspected. They are not cursor screenshots.

## Real pointer and desktop acceptance

**Basic moving-pointer visual acceptance: PASS — user reported, 2026-09-15.** After inspecting the opened read-only fixture session, the user confirmed: “已检查，上述行为正常” (“Checked; the described behavior is normal”). This is human confirmation of the requested checks below, separate from the automated cursor-property and layout checks. It is not an instrumented frame-timing measurement or certification of other environments.

- [x] Actual cursor shapes and transitions in Mochi-Light's ordinary area, text/input field, demo link, and move/resize edges.
- [x] Visible pointer hotspot when clicking the cross in the tested configuration.
- [x] Visible animation after switching to Mochi-Motion.
- [x] Pointer recovery after closing the trial window.

The fixture session was launched with the Wayland backend. Its automated runs observed scale factor 1; the user did not separately report a monitor scale or a multi-scale test. No usable desktop pointer-control tool was available to the agent; this visual result comes from the user's inspection, not agent-controlled pointer movement.

Remaining targeted checks, **NOT RUN / not individually confirmed**:

- [ ] Working, busy, and unavailable regions individually with themes supplying those roles.
- [ ] Hotspot accuracy at multiple trial sizes and measured animation timing/frame order/transparency.
- [ ] Release outside a drag handle, leave/re-enter during a drag, change themes during a drag, and close while loading.
- [ ] Extended focused editing, text-selection and context-menu behavior in native input children.
- [ ] 100%, 200%, fractional scaling, and mixed-DPI monitors. The confirmed fixture behavior does not certify this scaling matrix.

Real GNOME Apply/Undo, external settings conflicts, and cross-application installed-theme behavior remain **NOT RUN**; the [existing acceptance checklist](CLOSEOUT_VALIDATION.md#real-gnome-acceptance-checklist--not-run) is unchanged. This task did not push or publish, modify real settings, or install themes. The user-confirmed fixture animation does not imply full desktop acceptance or validation on other compositors.
