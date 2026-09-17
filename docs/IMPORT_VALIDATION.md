# CUR/ANI import implementation and validation

Date: 2026-09-15. Status: local candidate; real GNOME import/apply acceptance is pending.

The current user request explicitly extends the earlier v0.1 “no imports/install” scope. The existing four crates, Xcursor decoder, repository cache, inspector, GDK trial engine and Apply/Undo controller remain in place. No store, downloader, INF executor, root installer or alternative architecture was added. Prior uncommitted work was preserved. Nothing was pushed or published.

## Implemented workflow

**Import…** opens one reusable nonmodal window. A local CUR/ANI file can be decoded and inspected first; folders and ordinary ZIPs use the same asset model. Candidate files, failures and explicit skips remain visible. Multiple source folders require selecting a root; ancestor folders and the entire selected package are available. Names only suggest mappings; the confirmation checkbox is mandatory and is invalidated by edits. Busy and Working in background are separate, and the mapping covers fifteen common Windows roles, including Help, handwriting, both diagonals and alternate selection. Duplicate assignments block installation.

The inspector receives the same `Preview`/`Variant`/`Frame` as installed themes. **Try before installing** feeds those decoded assets into the existing bounded trial preparation and GDK texture cursor engine. Its eleven existing interaction regions are retained; additional roles can still be inspected. No system named cursor is passed off as an imported cursor. Missing trial roles are explicitly unavailable, with ambient pointers identified as such. Role changes cancel older trial requests. Decoding does not occur in pointer callbacks; the decoded package is retained and reused until replaced or closed.

No `Inherits` entry is generated in this first import version. The confirmation summary explicitly lists missing roles and states that desktop loaders may use their own default fallback. The existing post-install inspector/trial preserves its actual source path, chain and Direct/Inherited/Fallback classification. No arbitrary parent is silently chosen or claimed as imported artwork.

Only user theme roots also present in the existing active lookup paths are offered. The default libXcursor policy normally offers `~/.icons`; `$XDG_DATA_HOME/icons` is offered when explicitly included in the active cursor paths. No root is created during preview. A confirmed install creates missing destination components without following symlinks, generates a private staging directory inside that root, writes standard cursor files and alias copies, and validates every staged cursor by reading it back and comparing all pixels, dimensions, hotspots, frame order and delays. The generated theme is nested inside the private staging wrapper, so concurrent refresh cannot expose it as an applicable theme. It then commits with Linux `renameat2(RENAME_NOREPLACE)` through the existing safe rustix dependency. Existing user names and discovered theme/candidate names are rejected; a concurrent same-name winner cannot be overwritten.

The final directory contains `index.theme`, `cursors/*` and `conversion-report.txt`. No input INF, script, executable or arbitrary package file is copied into it. Failure/cancellation before commit explicitly removes staging and reports cleanup failure if one occurs. Cancellation arriving after commit is reported as an installed theme, not as an undone installation; its outcome remains visible. Installation refreshes and selects the theme, without applying it or creating Undo history. Closing the main window during an import operation requests cancellation and waits for its result; another close can finish exiting. Crash/power-loss recovery is not implemented: abrupt process termination may leave an inert `.cursormochi-stage-*` directory. Existing themes are never removed for cleanup.

## Decoder reuse and dependency review

Reviewed upstream metadata and the downloaded source on the date above. Dependency versions/checksums are in `Cargo.lock`; metadata was recorded in `target/qa/import-dependencies.json`. No third-party source was copied into first-party modules.

| Dependency/candidate | Review and decision |
|---|---|
| [`ico` 0.5.0](https://github.com/mdsteele/rust-ico) | Selected pixel decoder. MIT, pure Rust; release 2025-11-28, small established repository with tests/CI and low release frequency. No declared MSRV; this project verifies it on Rust 1.98 only. Supports CUR with BMP/PNG through its public API. Source review found unbounded entry allocations, BMP compression fields that are read but ignored, and legacy 32-bit masks not handled as required here. A strict preflight limits allocation and rejects those unsupported forms before calling it. |
| [`oxideav-ico` 0.0.7](https://github.com/OxideAV/oxideav-ico) | MIT, declared Rust 1.80, release 2026-06-16. Broader ICO/CUR/ANI alternative, but a young 0.0.x implementation/framework. Evaluated as a candidate, not incorporated or claimed audited. The narrower established pixel decoder and explicit supported container subset were chosen. |
| [`ani2xcur`](https://github.com/nicdgonzalez/ani2xcur) | GPL-3.0 CLI, rewritten in March 2026; includes INF-driven build/install behavior and an xcursorgen dependency. Not adopted as an in-process MIT library or invoked on user packages. |
| [`zip` 8.6.0](https://github.com/zip-rs/zip2) | Selected. MIT, Rust 1.88 minimum, release 2026-04-25; active upstream, newer 9.0 prereleases not selected. A regression exposed upstream duplicate-name coalescing; the wrapper now rejects index counts that differ from the EOCD before traversal. Default features disabled; only `deflate-flate2-zlib-rs` enabled, plus built-in Stored. No C compression dependency added. |
| [`tempfile` 3.27.0](https://github.com/Stebalien/tempfile) | Selected for unique private staging and cleanup. MIT OR Apache-2.0, declared Rust 1.63, release 2026-03-11; maintained established library. Uses OS randomness through its locked dependencies. |

The small first-party ANI adapter validates RIFF chunks, builds the playback sequence and translates jiffies; it delegates every pixel decode to `ico`. It is not a second BMP/PNG decoder. ANI layout was checked against the [ANI editor's format description](https://www.gdgsoft.com/anituner/help/aniformat.htm). The existing core Xcursor reader is unchanged; a small standard writer was added and checked with the independent system libXcursor loader.

Relevant locked transitive dependencies include `png` 0.17.16 (MIT/Apache-2.0), `flate2` 1.1.10 (MIT/Apache-2.0), `zlib-rs` 0.6.7 (Zlib), `miniz_oxide` 0.8.9/0.9.1 (MIT/Zlib/Apache-2.0), and `getrandom` 0.4.3 (MIT/Apache-2.0). This is a source/metadata and license review, not a comprehensive security audit. First-party code remains `forbid(unsafe_code)`; dependencies/native libraries may contain unsafe code.

## Supported data and conversion differences

| Format | Supported subset / explicit rejection |
|---|---|
| CUR directory | Type 2, 1–32 variants, width/height 1–256, in-bounds hotspot, matching directory and payload dimensions. Distinct `max(width,height)` per variant. Ambiguous same-nominal variants are rejected rather than hidden. |
| BMP inside CUR | 40-byte BITMAPINFOHEADER, positive bottom-up dimensions, one plane, uncompressed 24-bit BGR or 32-bit BGRA without palette, full padded pixels and AND mask. Non-black masked 24-bit XOR pixels and contradictory 32-bit alpha/mask pixels are rejected. Legacy all-zero-alpha 32-bit images are explicitly unsupported. |
| PNG inside CUR | RGBA8, noninterlaced. IHDR/IDAT/IEND plus sRGB/gAMA/cHRM/pHYs metadata. Other chunks, APNG, palette/grayscale/16-bit images, compressed metadata and ICC profiles are rejected. PNG gamma metadata is not color-managed. |
| ANI | RIFF/ACON envelope with exact nested chunk spans; 36-byte anih with AF_ICON or AF_ICON+AF_SEQUENCE. An outer length that includes its own 8-byte header is accepted with a visible compatibility note; other mismatches fail. Dimensions/depth/plane hints may be zero or must agree with every embedded CUR image (planes 0/1, supported depth 24/32); CUR images in LIST/fram; optional LIST/INFO and JUNK. Sequence indexes, repeated frames and rates are honored. Rates/default delay must be 1–600 jiffies. Missing/duplicate/unknown chunks, ICO-without-hotspot frames, raw bitmap ANI, inconsistent size variants and malformed sequence/rate tables fail explicitly. |
| ZIP | Ordinary single-disk ZIP with Stored or Deflate entries and a standard bounded central directory. ZIP64, encryption, split archives, alternative compression, links, duplicate paths and special entries are rejected. Nothing is extracted to a filesystem staging area during preview; only decoded cursor data survives loading. |

Image geometry and hotspots are preserved without resizing at export. Nominal Xcursor size is explicitly inferred as `max(width,height)` because CUR has no separate nominal size. Source RGBA is converted to the existing **premultiplied** RGBA model using `(channel * alpha + 127) / 255`; hidden RGB is discarded and partial-alpha channels may round. ANI jiffies are rounded to the nearest integer millisecond; every affected step is listed in the confirmation summary and saved report. For example, 1 jiffy becomes 17 ms. Millisecond rounding may accumulate over repeated animation cycles. Supported ANI delays already fall within the inspector's existing 16–10000 ms playback guard. Animation is never silently reduced to one frame, including when trial texture limits are exceeded: that trial role instead reports a limit error, and its full animation remains available to inspection/export if within those limits.

## Resource and path limits

| Resource | Limit |
|---|---:|
| CUR/ANI input | 16 MiB per file |
| ZIP input | 32 MiB |
| All package entries, including directories/non-cursor files | 1024 |
| Cursor candidate files | 128 |
| Relative path | 1024 UTF-8 bytes, 16 components |
| Expanded package / retained decoded pixels | 64 MiB each |
| CUR variants / image side | 32 / 256 pixels |
| ANI physical frames / sequence steps | 256 / 256 |
| Single source decoded and expanded-sequence pixel budgets | 32 MiB each |
| ZIP central directory | 2 MiB; EOCD counts checked before index allocation |
| Xcursor output frames | 2048 |
| Xcursor output file / generated theme | 16 MiB / 128 MiB |
| Trial | Existing 16 MiB, 256 frames per role, 256×256 frame, nominal size 16–64 |

Encoded spans, PNG headers/chunks, BMP layout, image sizes and aggregate allocations are checked before invoking the decoder. Directory traversal opens one component at a time using NOFOLLOW; regular file handles are opened nonblocking and read with caps. Symlinks, hardlinks and special files are refused for import (installed-theme browsing retains its different existing link policy). ZIP paths reject absolute paths, `..`, drive syntax, backslashes, control characters and excessive depth. Ignored non-cursor files still count against package limits; ZIP data is CRC checked by the library. No INF/script command is parsed or executed.

Budgets are for explicit input/output buffers, not a claim of a fixed process RSS cap. There can be one retained package, one bounded decoder working set, inspector textures and bounded trial textures. Worker cancellation is cooperative between bounded operations, not hard process isolation. First-party checks do not claim protection against every same-UID filesystem race or decoder-library defect.

## Automated checks

Local environment: Linux x86_64, Rust 1.98.0, GTK 4.22.4, GLib/GIO 2.88.0. Older GTK, other Rust versions and other OS environments were not tested.

- `./scripts/check.sh --gui`: PASS. Formatting, core/app tests, workspace Clippy (`-D warnings`), workspace tests and locked release build. 62 workspace tests (16 core, 20 app, 24 platform, 2 GTK worker tests). Log: `target/qa/import-check.log`.
- `python3 scripts/reference-import.py`: PASS. Original BMP CUR, PNG CUR and sequenced ANI are converted by Rust into a temporary theme; system `libXcursor.so.1` independently checks every pixel, dimensions, hotspot, frame count/order and delay at both nominal sizes. Source expectations are hand-defined; fixture generation does not use the production decoder/writer. Log: `target/qa/import-reference.log`.
- New regression coverage includes malformed/truncated inputs, indexed BMP rejection, 24-bit mask semantics, out-of-range delays, allocation limits, explicit confirmation, distinct Busy/background roles, missing roles, preinstallation animation, strict links/paths/ZIP limits and duplicate ZIP names, cancellation, staging cleanup, corrupt export failure, concurrent same-name install and refresh during staging (a regression observed failing before the staging layout fix).
- Import modules have no settings object or Undo controller. An architecture tripwire rejects settings-write dependencies/calls in the app/platform/GTK import path. GUI smoke retains the existing settings port wrapper that fails any write call and verifies unchanged snapshots/no Undo record.

Original fixtures and their generator (`scripts/generate-import-fixtures.py`) are MIT licensed. Only QA/temp directories were written during tests. No real user theme was installed, no desktop setting was changed, and no automatic push/release occurred. The updated GitHub Actions independent-reader step is **NOT RUN remotely**; local success does not certify the Ubuntu 24.04 runner.

## Isolated GUI

PASS: private D-Bus session, injected read-only memory settings, existing Wayland display, Adwaita / Adwaita dark / HighContrast. The runs reported GDK scales 1 and 2 at different times; the final three-style run reported scale 1. These are programmatic fixture checks, not human verification of scaled hardware-pointer output.

The GUI smoke opens single BMP CUR, animated ANI, unsupported input, folder, PNG CUR and ZIP; requires explicit selection for a ZIP with two theme roots, edits a role and checks confirmation reset; opens uninstalled animated GDK texture cursors; installs only into a unique QA root; verifies main-list refresh/selection, same-name refusal and read cancellation/close cleanup. The existing inspector/playground/Apply/Undo regression checks still run. Initial capture after closing import needed a layout-settling interval; this was fixed. The compact import inspector also has a geometry assertion ensuring its cursor image remains inside the default viewport. The complete checks were rerun.

Screenshots: [light](screenshots/import-light.png), [dark](screenshots/import-dark.png), [high contrast](screenshots/import-highcontrast.png). Logs: `target/qa/polish-gui-{light,dark,highcontrast}.log`. Screenshots and successful cursor setters do **not** prove the appearance of the moving hardware cursor.

## Real GNOME acceptance — NOT RUN for imported themes

The user's earlier acceptance of the original Xcursor trial is retained in `TRIAL_VALIDATION.md`. It does not automatically accept this new decoder/import/install path.

- [ ] With a representative supported CUR, compare transparent/partial-alpha edges, actual moving-pointer geometry and click-hotspot alignment before installation.
- [ ] With a representative supported ANI, visually compare sequence, repeated frames and dwell times; review the reported jiffy rounding.
- [ ] Move between ordinary background, text selection, GtkEntry's internal text, link, drag/resize, busy/background and unavailable regions. Verify leaving a region, drag cancellation, theme/source change and closing restores normal behavior.
- [ ] Repeat actual moving-pointer checks on real Wayland at 100%, 200%, fractional scaling and mixed-monitor transitions. Programmatic scale-1/2 rendering above is not this acceptance.
- [ ] Confirm a real user destination, install a new name, inspect actual missing-role/fallback sources, then explicitly Apply. Observe pointer behavior in GTK, Qt and a browser separately.
- [ ] Explicitly Undo; verify previous desktop setting/override restoration while the newly installed files remain installed.
- [ ] Change cursor settings externally before Undo and verify conflict refusal. Do not automatically overwrite the external change.
- [ ] Confirm cancellation before commit removes staging; if cancellation loses to commit, verify the successful installation is clearly reported.

Real native file-picker interaction and a broad third-party Windows cursor corpus have not been manually validated. This version claims only the supported subset and test environments above.

## ANI compatibility follow-up: local Miku package

The user reported that the desktop's `Miku 完整版.zip` could not be imported. Read-only diagnosis reproduced 22/22 failures with `Invalid("ANI RIFF/ACON length")`. Each ANI stores total file length in the RIFF size field instead of excluding its 8-byte header, and supplies 160×160, 32-bit, one-plane hints which the initial implementation rejected. The package fits the existing resource limits; they were not raised.

Compatibility is now narrowly defined: accept only the exact header-inclusive length discrepancy, still require complete nested chunks, CUR directory/payload spans and full frames, and report the discrepancy in conversion notes. Nonzero format hints must match embedded images; conflicting hints and actual truncation remain errors. The older [AniTuner format note](https://www.gdgsoft.com/anituner/help/aniformat.htm) describes those fields as reserved/zero; accepting verified nonzero values is an explicit compatibility extension for observed writer output, not a claim that the source has a canonical header. Neither source bytes nor artwork are changed by diagnosis.

Added original synthetic 160×160 fixtures reproducing these header conventions, with two independent colored-pixel frames and a repeated/reordered three-step animation. No Miku artwork, archive, screenshots or INF instructions were incorporated into the repository. The initial synthetic compatibility test was observed failing before the fix. Regression checks preserve pixels/hotspots/sequence/delays and reject conflicting geometry/depth/planes or truncated payloads. Common Windows labels such as Normal Select, Text Select, Help Select and Work now produce editable suggestions. Numbered diagonal directions and personalized alternatives are not guessed.

Current local checks: `./scripts/check.sh --gui` — PASS, 65 workspace tests (18 core, 21 app, 24 platform, 2 GTK worker tests), locked release build and three isolated GUI styles. The GUI exercises synthetic noncanonical-header ANI inspection, its visible warning, and animated texture-cursor trial. Log: `target/qa/ani-compat-check.log`. Independent synthetic libXcursor comparison: `target/qa/ani-compat-reference.log`, PASS.

`cargo run --locked -q -p cursormochi-platform --example diagnose-import -- '/absolute/path/to/package.zip'` is a new read-only diagnostic entry. On the user's local package, all 22 ANI files / 187 frames decoded and exported/round-tripped in memory successfully; four non-cursor files were ignored. Evidence: `target/qa/miku-import-diagnosis.log`. The actual Miku package has not been installed, applied or visually accepted on a moving hardware pointer. Its original archive remains untouched. Restart the rebuilt application, open the ZIP, choose the Miku root, review mappings/explicit skips, and enter a new theme ID such as `Miku` before confirming installation.


## Multi-folder import appeared empty — 2026-09-15

The running executable matched the current release build. Read-only diagnostics of the desktop Miku ZIP again decoded all 22 ANI files (187 frames, zero failures). The package includes a base folder and a customization subfolder. The importer was waiting for explicit root selection and filtered out every row while that selection was absent; a blank dropdown and a bottom-only hint made successful reading look like no response.

The importer now immediately shows all decoded/failed file rows with individual-file inspection. A prominent source status reports the count and required next step, and the dropdown displays “Choose a theme folder…”. Selecting a folder narrows the list. Mapping edits, theme trial, confirmation and installation remain disabled until a root is selected. The install handler separately rejects a missing root even if triggered directly. File-read errors also appear above the list.

Regression evidence: `target/qa/import-root-regression-before.log` failed on the old implementation (zero visible rows instead of two for `roots.zip`). After the fix, `./scripts/check.sh --gui` exited 0; log `target/qa/import-root-check.log`. Formatting, core/app tests, workspace Clippy, all 65 workspace tests and release build passed. Adwaita, dark and HighContrast isolated Wayland GUI runs passed, including visible rows/preview before root selection, explicit folder filtering, confirmation reset, and rejection of direct installation before root selection. No GTK warnings or criticals occurred in those final runs.

The original desktop ZIP was only read; it was not installed or copied into the repository. Isolated installation tests wrote only disposable QA fixtures under `target/qa`. Real GNOME settings, Apply/Undo and moving-pointer acceptance were not exercised. No push or publication occurred.


## Guided import workflow — 2026-09-15

This revision replaces the previous single-page form and blank/placeholder root dropdown with three steps in the existing GTK importer. The decoder, import plan, worker, staging/commit code, preview and trial engines are unchanged.

1. **Source:** open files or a folder. Ambiguous roots appear as folder buttons with counts; clicking one explicitly selects it and proceeds to Review. A redundant entire-package choice is omitted when a listed ancestor already contains every cursor. Users can inspect all files before selecting a root, then use the active **Choose folder** action to resolve that requirement.
2. **Review & try:** filenames, parent folders and editable roles sit beside the frame inspector. Duplicate assignments are identified at the affected rows; skipped files explicitly explain how to include them. Invalid mapping disables continuation with a visible reason. Busy and Working remain separate. Long conversion reports are expandable. Trial remains available before installation.
3. **Install:** an editable portable name suggestion, mapped/skipped counts, missing-role names, destination, full conversion details and confirmation are grouped together. Invalid name, missing target and unchecked confirmation each receive a visible next-step message. Success shows Done and keeps Apply separate in the main window.

Checks: `./scripts/check.sh --gui` exited **0**, recorded in `target/qa/import-flow-check.log`. Formatting, core/app tests, workspace Clippy with denied warnings, **65 workspace tests**, and the release build passed. The existing architecture tests still reject settings capabilities in the import path.

Isolated Wayland GUI runs passed for Adwaita, dark and HighContrast. Added checks exercise automatic Review for a single source, folder-card selection for ambiguous packages, inspect-before-selection and return to Source, duplicate-role row hints, invalid-name explanations, explicit installation confirmation, Back navigation, success/Done, conflicts, read cancellation and close cleanup. Final logs contain no GTK warning/critical messages. Existing animation, imported trial, and QA-only installation tests also passed. Scale factors 1 and 2 were observed in local runs; this does not replace controlled scaling acceptance.

Screenshots: [source selection](screenshots/import-source.png), [review](screenshots/import-light.png), [successful isolated installation](screenshots/import-installed.png). Source/success captures use generated test fixtures, not the user's Miku artwork. The installation shown is in a disposable QA directory.

Real moving-pointer and GNOME Apply/Undo/external-conflict acceptance remain pending. No user theme was installed, no real desktop setting was changed, and nothing was pushed or published.


## Inline package display — 2026-09-15 (current)

The user's subsequent request supersedes the separate Source and Review steps above. Files now open directly into a single import view containing every package file, real 48-pixel first-frame thumbnails, frame counts, role menus and the existing selected-file inspector. There is no folder-choice gate or separate import-preview screen. The whole package is the explicit default scope; an optional folder filter can narrow it. Final installation confirmation covers the included files and mappings.

Manual role assignments, including explicit skips, survive filtering away from a folder and back. Duplicate assignments still block trial/continuation, with row-level explanations. Missing roles and conversion differences remain available before final confirmation. Installation is still a distinct confirmation step and never automatically applies the theme.

Thumbnail textures reuse already decoded data: at most 16 viewport-adjacent images are retained and at most two textures are created per tick. Hidden/out-of-view thumbnails release their textures. No extra decoder or mouse-move I/O was added. Thumbnails show the first frame only; original animation remains available in the inspector and real GDK cursor trial.

Validation command: `./scripts/check.sh --gui`, log `target/qa/import-inline-check.log`. Formatting, core/app tests, workspace Clippy, all 65 workspace tests and release build passed. Three isolated Wayland styles passed, with assertions covering whole-package display, real thumbnails, release when hidden, duplicate-role blocking, installation confirmation, optional filtering and preservation of manual mappings. Existing animation, installation conflict and cancellation tests passed. No GTK warning/critical messages were present in the final logs.

Current captures: [all package files](screenshots/import-all-files.png) and [import interface](screenshots/import-light.png). Earlier source-card screenshots are historical. Real moving-pointer, GNOME Apply/Undo, external-conflict and controlled scaling acceptance remain separate and pending. No real desktop settings, user theme installation, push or publication occurred.


## More space for imported cursors — 2026-09-15 (current)

Default import window size is now 1280×900 logical pixels, subject to the compositor's available workspace. The toolbar contains source selection and a Details menu; scope/filtering and conversion reports live in that menu. The redundant page heading, repeated path labels and routine ready-state guidance no longer occupy the browsing area. File thumbnails (40 pixels), names/frame counts and role selectors share a compact row. Duplicate-role/decode errors remain visible.

The importer configures the existing PreviewPane with a flexible canvas and a 1× default. This fixes the former mismatch between 3× display and a fixed 180-pixel viewport, which cropped 160-pixel ANI images. Pause remains beside the filename; zoom, nominal size, background, hotspot and source controls are folded into Preview settings & details. The main window's Inspect mode retains its original layout and zoom.

`./scripts/check.sh --gui` exited 0; log `target/qa/import-space-check.log`. Formatting, core/app checks, workspace Clippy, all 65 workspace tests and release build passed. Adwaita, dark and HighContrast isolated Wayland GUI runs passed with no GTK warnings/criticals. Added geometry assertions require ordinary rows to stay at most 80 pixels high, the default import canvas viewport to occupy at least half the window height, and the complete 160-pixel ANI image to fit inside its viewport at the default zoom. Existing animation and no-settings-write checks passed.

Current [light](screenshots/import-light.png), [dark](screenshots/import-dark.png) and [high-contrast](screenshots/import-highcontrast.png) captures were refreshed. These are generated-fixture layout checks, not real moving-pointer or GNOME Apply/Undo acceptance; those remain pending. No user theme installation, desktop setting change, push or publication occurred.


### Transparent import preview background

The import canvas now defaults to **No background**, exposing the surrounding window through transparent cursor pixels. Light, dark and checkerboard remain optional under Preview settings & details; the main inspector default is unchanged. `./scripts/check.sh --gui` passed (65 tests, formatting, Clippy, release build and three isolated GUI styles); log: `target/qa/import-background-check.log`. Refreshed captures show the background removal. Real GNOME acceptance remains separate; no desktop settings or user theme directories were modified.


## Real window resize names — 2026-09-15

The user reported fallback cursors while resizing real windows. Read-only inspection confirmed that GNOME's selected theme was `miku`. Its installed files had generic horizontal/vertical resize names but no directional edge names, and its conversion report listed both diagonal roles as missing. At that revision, the source ZIP’s Diagonal Resize 1/2 filenames were still left unassigned. The numbered-diagonal follow-up below corrects this overly conservative treatment of standard Windows names.

[GTK 4.22 window resize handling](https://github.com/GNOME/gtk/blob/gtk-4-22/gtk/gtkwindow.c) requests eight directional names for window edges/corners. [Mutter's name and legacy-name mapping](https://github.com/GNOME/mutter/blob/gnome-49/src/backends/meta-cursor-sprite-xcursor.c) distinguishes these from the two-ended axis names. This explains why a generic axis preview could work while a real window edge fell back. These sources were consulted for name compatibility; no third-party implementation was copied.

Confirmed horizontal, vertical and both diagonal import roles now generate the directional names and their legacy counterparts on the same axis. Original images, hotspots and animation are retained without rotation/mirroring; the conversion report describes this reuse. Missing diagonal mappings still produce a missing-role report and no fabricated corner images. Role selection retains explicit access to every verified filename.

Regression: the new staged-install test failed before the fix with `window resize cursor e-resize missing` (`target/qa/window-resize-before.log`). After the fix it verifies all eight directional and eight legacy names, axis-specific source pixels, animation equivalence and the absence of corner files when diagonals were not assigned.

Automated checks: final `./scripts/check.sh --gui` exited 0 (`target/qa/window-resize-final-check.log`). Formatting, core/app tests, workspace Clippy with warnings denied, all **70 workspace tests**, and release build passed. `python3 scripts/reference-import.py` also exited 0 (`target/qa/window-resize-reference.log`): independent system libXcursor reads both by filename and by theme/name with an isolated process-local XCURSOR_PATH, validating original fixtures' pixels, sizes, hotspots, frame order and delays for all 16 new names plus the four existing reference roles. Output is confined to temporary directories.

Isolated GUI: all three styles (Adwaita, Adwaita:dark, HighContrast) passed on the available Wayland display at scale 1 with the no-write settings backend. An initial run exposed a smoke screenshot race after import completion; the test now waits for frame-clock progress after worker completion before capturing the newly laid-out page. The final full run passed.

Real GNOME window resizing with the repaired theme is **NOT RUN**. Acceptance still needs all four edges and four corners, hover and active dragging, release/cancellation, and the scaling checklist. The installed user theme and real settings were not changed. Existing imports require a new-name reimport and explicit Apply; both diagonal files need confirmed mappings for all corners. No push or publication occurred.


## Numbered diagonal mapping — 2026-09-15

After the user confirmed horizontal window resizing worked, read-only inspection found that the newly installed `Miku` still listed both diagonal roles as missing, with none of the four corner files present. The earlier alias fix was correct for assigned roles, but the importer still defaulted the two numbered diagonal files to Skip.

[Microsoft's cursor definitions](https://learn.microsoft.com/en-us/windows/win32/menurc/about-cursors) explicitly associate Diagonal resize 1 with SIZENWSE and Diagonal resize 2 with SIZENESW. These standard filename labels now suggest roles 9 and 13. The importer shows their directions as **Resize diagonal ↖ ↘** and **Resize diagonal ↗ ↙**. Suggestions remain editable and installation still requires confirmation. Explicitly skipping a recognized diagonal displays a warning that its corners will use desktop fallback. Unknown diagonal numbers remain unassigned. No INF instructions are executed.

Regression evidence: the standard-label test first failed with `None` versus `Some(9)` (`target/qa/diagonal-mapping-before.log`). The corrected test covers both numbered labels and rejects unrecognized numbers. A new directory/ZIP-to-suggestion-to-install regression verifies that both recognized files survive default mapping, both missing-role entries disappear, confirmation is still required, and all four corner files retain source frames and metadata. Existing tests continue to check axis-specific aliases and missing-role behavior.

Automated checks: `./scripts/check.sh --gui` exited 0 (`target/qa/diagonal-mapping-check.log`): formatting, core/app tests, workspace Clippy, **71 workspace tests**, and release build passed. Formatting and all-target Clippy were rerun after extending the read-only diagnostic example (`target/qa/diagonal-final-clippy.log`). Independent libXcursor file and theme/name checks passed (`target/qa/diagonal-reference.log`). Read-only diagnostics on the actual Desktop Miku ZIP reported **22 decoded files, zero failures, 187 frames**, including both 160-pixel, four-frame diagonal animations with `nwse-resize` and `nesw-resize` suggestions (`target/qa/diagonal-miku-readonly.log`). No user artwork was copied into fixtures.

Isolated GUI: Adwaita, Adwaita:dark and HighContrast passed with the no-write settings port on Wayland at scale 1. New smoke coverage verifies initial numbered-diagonal mappings, direction labels, visible skip warnings, manual remapping, duplicate-role blocking and confirmation invalidation. [Original synthetic fixture capture](screenshots/import-diagonals.png).

Real desktop: the user's horizontal-edge acceptance applies to the prior edge-name fix. Repaired corners, actual moving-pointer animation/hotspots, release behavior and scaling remain **NOT RUN**. Current `Miku` was inspected but not modified; it must be reimported under a new name and explicitly applied to receive the newly recognized corner files. Nothing was pushed or published.

## Closed role menus must not consume file-list scrolling — 2026-09-16

Read-only accessibility inspection of the running import window found multiple role conflicts, including Busy, Move and both diagonals. The user confirmed they had only scrolled the list. A default mapping diagnostic of their source ZIP found unique suggestions for all 15 supported roles; deleting the old installed theme was unrelated to this gate.

Reproduction: GTK's built-in ComboBox scroll controller changes the active item even when its popup is closed. The new GUI regression failed before the fix with active index 5 instead of 4 after one scroll signal (`target/qa/import-wheel-before.log`). Closed per-file role menus now disable only that controller's propagation, allowing events to reach the file scroller. Keyboard/click selection and popup behavior retain their normal controllers.

**Reset roles** restores filename suggestions across all folders, clears cached manual mappings, rebuilds rows and invalidates installation confirmation. It changes only in-memory import choices. It neither installs nor resolves genuinely ambiguous duplicate suggestions automatically. The disabled-button explanation now points directly to this recovery action instead of exposing only a Rust error.

Tests cover scroll stability, explicit conflicting selection, reset recovery, hidden-folder reset, retained genuine-duplicate blocking, both numbered diagonal mappings and unchanged settings/Undo. The architecture tripwire now matches complete type identifiers: `app::Controller` remains forbidden in the import path while the unrelated `gtk::EventControllerScroll` is permitted. The rejecting settings backend continues to verify zero settings-write calls.

Validation: PASS — formatting, core/app tests, workspace clippy, all 73 workspace tests, release build and all three isolated GUI styles (Adwaita, dark, HighContrast) on Wayland, scale 1 (`target/qa/import-wheel-fix.log`). No application GTK/GDK warnings or protocol errors were reported in this final run. No host settings change, installation into a user theme directory, push or publication was performed. Real-user wheel/popup/keyboard interaction remains a separate acceptance check.

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

## Multi-size export and Miku-Desktop — 2026-09-17

The installer now prepares actual variants at nominal sizes 16, 24, 32, 48, 64, 96 and 128, retaining every original size unchanged. The bounded `core::resize` implementation is shared with the existing texture trial; it replaces the trial's local nearest-neighbor loop with integer area sampling of premultiplied RGBA. Existing native variants win over resampling. Non-square aspect ratios, hotspot scaling/rounding/clamping, all frames and their delays are preserved within the documented rounding limits. Resampled pixels are explicitly disclosed in the import review and conversion report. Unsupported dimensions, per-variant frame counts, aggregate frames and byte budgets fail explicitly; cancellation is checked during validation and row processing. No new dependencies, decoder or architectural layer was introduced.

The serializer remains lossless with respect to its prepared input. Installation verifies each staged alias against the prepared variants; tests independently assert that original-size variants are still byte/pixel equivalent to the decoded source. Requests outside the provided sizes may select a nearest-size variant and are not covered by an unconditional cross-toolkit visual-size guarantee.

The large-ANI regression failed before the change (`target/qa/desktop-sizes-before.log`: real desktop-size variant missing). It now requires actual 32×32 and 64×64 output, adjusted hotspots and complete animations. Additional tests cover alpha-aware area reduction, integer enlargement, native pixel reuse, non-square dimensions, malformed inputs, memory/frame limits, duplicate sizes and cancellation.

### Automated and independent checks

- `./scripts/check.sh --gui`: PASS, final log `target/qa/desktop-sizes-final-check.log`: formatting, core/app tests, workspace clippy, 79 workspace tests and release build.
- `python3 scripts/reference-import.py`: PASS, `target/qa/desktop-sizes-reference.log`. System libXcursor directly reads files and resolves theme/name requests. The 160 px fixture now checks all seven added sizes, independently calculated premultiplied pixel values, hotspots, frame order and millisecond delays; original sizes remain covered.
- The user's Miku was normalized from its existing confirmed import mappings, without guessing original ZIP names, into the new `~/.icons/Miku-Desktop` theme. The maintenance example `normalize-imported-theme` reuses the bounded repository and create-only installer; it rejects unknown roles, differing alias artwork, inheritance and conflicts. It does not Apply.
- All 56 installed cursor files match the independently validated staged output. A fresh libXcursor process verified 448 name/size combinations in both staging and the installed theme: exact dimensions, scaled hotspots, complete frames, unchanged delays and original-size image hashes (`target/qa/miku-desktop-independent.json`, `target/qa/miku-desktop-installed-independent.json`). Existing Miku files and desktop settings stayed unchanged (`target/qa/miku-before-desktop-install.json`). The new theme still provides 23 of the 34 tested system names; missing roles were not silently filled.

### Isolated GUI and real-session limits

- Wayland: PASS in Adwaita, Adwaita dark and HighContrast, reported integer GTK scale 2.
- X11 through the host XWayland: PASS with private D-Bus, the rejecting settings backend and QA-only fixture installation (`target/qa/desktop-sizes-xwayland-final2.log`, GdkX11Display scale 2). This exercises GUI behavior and texture trials; named fixture cursors still use the ambient desktop.
- Earlier XWayland attempts failed at screenshot allocation and animation timing. The import smoke test now checks preview animation before opening the covering trial, closes that trial after its check, presents the import window, and waits for frame-clock progress before capturing the review. Failure logs remain `desktop-sizes-xwayland.log` and `desktop-sizes-xwayland-final.log`.
- Final GUI runs contained no application GTK/GDK warning or protocol error. Private portal/service teardown messages are not real-pointer evidence.
- Real QQ/Clash pointer size, animation/hotspots, fractional/mixed-monitor scaling and post-close recovery remain manual acceptance. The user must explicitly Apply Miku-Desktop (current target size 32); no real desktop setting was written, no running application was restarted, and nothing was pushed or published.
