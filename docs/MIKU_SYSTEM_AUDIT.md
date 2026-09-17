# Miku system cursor audit — 2026-09-17

After explicit user approval, only the two missing pane-divider files were added to the installed Miku theme. All 56 pre-existing files retain their SHA-256 hashes. The desktop remains on Miku at size 32; no settings write, theme switch, Undo record, push or release occurred.

| Added name | Existing source | Independent reader result |
| --- | --- | --- |
| `col-resize` | `ew-resize` | Identical pixels, sizes, hotspots, four-frame order and 150 ms delays |
| `row-resize` | `ns-resize` | Identical pixels, sizes, hotspots, four-frame order and 150 ms delays |

Both names were also read from the native GtkPaned handle properties in the current GTK runtime. The test window continues to use GTK native behavior; no replacement cursor was injected into it.

## Complete check of the 34 test names

Fresh-process libXcursor theme/name lookup at requested size 32 was compared with direct-file loading. All 23 present exact-name files matched in every frame. For the remaining 11, the loader returned an image despite the missing Miku file; its fallback source was not identified. These results do not establish which images the running GNOME compositor has cached or is displaying.

| Use | Requested name | Result |
| --- | --- | --- |
| Normal | `default` | Miku file verified |
| Link | `pointer` | Miku file verified |
| Text | `text` | Miku file verified |
| Busy | `wait` | Miku file verified |
| Working in background | `progress` | Miku file verified |
| Horizontal resize | `ew-resize` | Miku file verified |
| Vertical resize | `ns-resize` | Miku file verified |
| Move | `move` | Miku file verified |
| Unavailable | `not-allowed` | Miku file verified |
| Diagonal ↖ ↘ | `nwse-resize` | Miku file verified |
| Crosshair | `crosshair` | Miku file verified |
| Diagonal ↗ ↙ | `nesw-resize` | Miku file verified |
| Left edge | `w-resize` | Miku file verified |
| Right edge | `e-resize` | Miku file verified |
| Top edge | `n-resize` | Miku file verified |
| Bottom edge | `s-resize` | Miku file verified |
| Top left corner | `nw-resize` | Miku file verified |
| Top right corner | `ne-resize` | Miku file verified |
| Bottom left corner | `sw-resize` | Miku file verified |
| Bottom right corner | `se-resize` | Miku file verified |
| Help | `help` | Miku file verified |
| Grab | `grab` | No Miku file; loader returned fallback image |
| Grabbing | `grabbing` | No Miku file; loader returned fallback image |
| Copy | `copy` | No Miku file; loader returned fallback image |
| Alias | `alias` | No Miku file; loader returned fallback image |
| No drop | `no-drop` | No Miku file; loader returned fallback image |
| Context menu | `context-menu` | No Miku file; loader returned fallback image |
| Cell | `cell` | No Miku file; loader returned fallback image |
| Vertical text | `vertical-text` | No Miku file; loader returned fallback image |
| Column resize | `col-resize` | Miku file verified |
| Row resize | `row-resize` | Miku file verified |
| All scroll | `all-scroll` | No Miku file; loader returned fallback image |
| Zoom in | `zoom-in` | No Miku file; loader returned fallback image |
| Zoom out | `zoom-out` | No Miku file; loader returned fallback image |

## Original package check

The original Desktop ZIP was re-read with the existing bounded importer: 22 CUR/ANI files decoded, 0 failures, 187 frames. The INF was inspected only as text; no directives were executed. Its role declarations and the file names contain no explicit mappings for the 11 missing Linux roles. Location Select, Person Select and five optional replacement assets are outside the current standard import mappings. They were not silently assigned to Grab, Copy, Zoom or other unrelated roles. Filename/declaration evidence alone cannot establish that optional artwork is suitable for those roles.

## Evidence and limits

- Repair receipt and source/destination hashes: `target/qa/miku-pane-repair-receipt.json`.
- Independent per-frame audit: `target/qa/miku-system-audit.json` and `.log`.
- Source decode report: `target/qa/miku-source-audit.log`.
- This turn changed user theme files only as approved, plus this audit documentation. No product code changed; the previous 75-test and three-style isolated GUI results were not rerun or claimed as a new run.
- Actual GNOME moving-pointer shape/animation, cached fallback refresh, scaling and recovery remain manual acceptance. Recheck files refreshes file evidence, not compositor caches.
