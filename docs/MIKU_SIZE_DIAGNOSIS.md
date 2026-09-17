# Miku size diagnosis — 2026-09-17

Read-only checks in the current session found:

- GNOME cursor theme `Miku`, requested size 32.
- Logical monitor scale approximately 1.333333 (133.33%).
- X11 resource values `Xcursor.theme=Miku`, `Xcursor.size=64`, `Xft.dpi=192`. These are separate from the logical desktop size; 64 alone is not evidence of an erroneous user preference.
- Both QQ (`qq` / `QQ`) and Clash Party (`mihomo-party`) have X11 client windows, hence use XWayland in this Wayland session. The main processes have no explicit Ozone or forced device-scale command-line option.
- Installed Miku's normal, text, link and pane-divider cursors contain only nominal-size 160 variants with actual 160×160 frames. There are no smaller variants for these roles.

GNOME's [Mutter implementation](https://github.com/GNOME/mutter/blob/gnome-50/src/backends/meta-cursor-xcursor.c) uses the requested theme size divided by the image's nominal size to set the viewport dimensions for its themed cursor path. In contrast, [Chromium's X11 cursor loader](https://chromium.googlesource.com/chromium/src/+/main/ui/base/x/x11_cursor_loader.cc) selects the nearest available nominal size and creates the X cursor from the selected bitmap dimensions. With only a 160 variant available, merely requesting 32 or 64 cannot select a smaller bitmap. These source paths and local observations explain the reported mismatch; the exact displayed pointer dimensions in each running application have not been captured or visually measured, and the cited Chromium main branch is not a build-identical audit of QQ's bundled version.

The expected target is the configured nominal 32 desktop size after display scaling, not a literal 32 physical-pixel character silhouette. The large pointer in an application should not become the sizing reference. The converter preserved the original artwork but did not generate practical desktop-size variants; previous pixel-preservation and role-coverage tests do not establish cross-application size compatibility.

A durable export fix must generate actual size variants (not just relabel a 160×160 image), scale hotspots consistently, preserve frame order/timing and explicitly report pixel resampling. Keep the original input untouched. Validation needs native GNOME/GTK and XWayland clients at the available scale, plus independent file and loader checks. No theme assets, desktop settings or application launch options were changed during this diagnosis.

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
