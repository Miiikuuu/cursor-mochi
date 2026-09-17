# v0.1 closeout validation — 2026-09-15

This is a local closeout of the existing Rust + GTK4/GIO implementation. The four crate boundaries remain unchanged. No store, CUR/ANI import, installer, dependencies, or new architecture were added. This is still a candidate build pending real desktop acceptance.

## Role selection regression

Two tests were added to `cursormochi-app/src/browser.rs` before changing `role_choices`. Both failed against the previous implementation:

- `busy_and_background_work_have_distinct_choices`: `progress` was missing when `watch` was present.
- `every_verified_file_remains_selectable_with_unambiguous_labels`: the verified `default` file had no entry when `left_ptr` was present.

The local failing-run log is `target/qa/closeout-before.log`. After the fix, all five browser tests passed.

Busy now covers `watch` and `wait`; Working in background covers `progress` and `left_ptr_watch`. Each group's first available file retains the friendly label, and additional verified files receive explicit entries such as `Normal · default` or `Working in background · left_ptr_watch`. Unknown roles retain `Other · filename`. Selection IDs remain the original filenames. The existing GTK dropdown consumes every returned entry and retains the selected file in its tooltip/details.

The regression checks all currently mapped filenames together with an unknown filename, requiring exactly one entry per input file and unique labels, plus empty input. This prevents friendly-name grouping from silently removing verified files.

## Minimal GitHub Actions CI

[CI workflow](../.github/workflows/ci.yml) targets **ubuntu-24.04** with **Rust 1.98.0**, rustfmt, and Clippy. It runs on push, pull request, or manual dispatch, with read-only repository permissions and no publication step.

It checks formatting and locked core/app tests before installing native packages. The workspace steps install `build-essential`, `pkg-config`, `libgtk-4-dev`, and `libglib2.0-bin` on the disposable runner, compile private test schemas, run Clippy with warnings denied, run locked workspace tests, and build the release binary. Toolchain and native library versions are printed in the job log. Tests retain Fake/memory settings isolation. Native dependency installation failure fails the job rather than silently skipping checks.

The runner label is explicit instead of tracking `ubuntu-latest`; see GitHub's [runner reference](https://docs.github.com/en/actions/reference/runners/github-hosted-runners). Workflow triggers and permissions follow the official [workflow syntax](https://docs.github.com/en/actions/reference/workflows-and-actions/workflow-syntax).

**Hosted CI: NOT RUN.** The workflow YAML was parsed locally and its declared runner/toolchain checked; this is not a GitHub Actions execution or an Ubuntu 24.04 compatibility result. No workflow was dispatched and no changes were pushed during this task. No GUI, real GNOME session, other OS, or other native runtime is claimed as covered by CI.

## Local automated checks

Actual local environment: Ubuntu 26.04.1 LTS x86_64, Rust 1.98.0, GTK 4.22.4, GIO 2.88.0.

`./scripts/check.sh --gui` exited **0**. Local log: `target/qa/closeout-check.log`.

| Automated check | Result |
|---|---|
| Formatting | PASS |
| Locked core/app tests | PASS |
| Workspace Clippy, all targets, `-D warnings` | PASS |
| Locked workspace tests | PASS: 42 tests — core 10, app 14, platform 17, GTK worker 1 |
| Locked release build | PASS |
| `python3 scripts/reference-xcursor.py` | PASS: all three original fixtures; log `target/qa/closeout-reference.log` |

These results belong to the local environment above, not the proposed hosted runner. Settings tests use Fake or explicitly injected memory backends; no real desktop settings were modified.

## Isolated GUI checks

The existing script completed successfully for Adwaita, Adwaita:dark, and HighContrast. Each run used a private D-Bus session, an available Wayland display, read-only fixtures, and an injected memory settings backend. Each reported `GdkWaylandDisplay`, scale factor **1**, initial window 980×760, compact window 820×680, and expanded window 1100×840.

The existing assertions cover classification, selection independent of current settings, search/refresh, pointer targeting, thumbnails, static/animated previews, pause/hide, details/diagnostics, and unchanged read-only settings. Current logs and captures are in `target/qa/polish-gui-*.log` and `target/qa/polish-*.png`; the combined closeout log records all three successful runs. These fixture runs do not individually exercise all real theme role aliases; their preservation is covered by the new app regression tests and the unchanged GTK dropdown integration.

## Real GNOME acceptance checklist — NOT RUN

These steps remain user-initiated. No checkbox below was completed automatically:

- [ ] Record the original effective theme/size and whether each key has a user override.
- [ ] Apply theme B with size opt-in off; verify settings readback and unchanged size separately from the visible pointer.
- [ ] Apply theme C, then Undo to B; verify session-only history and reset semantics when the original value had no user override.
- [ ] Test explicit size opt-in and restoration of only the keys owned by the operation.
- [ ] Change settings externally after an application change; verify Undo refuses to overwrite conflicting changes. Automated conflict tests are not real desktop acceptance.
- [ ] Inspect actual cursor behavior in GTK, Qt, and browser windows.
- [ ] Check 100%, 200%, fractional scaling, and mixed-DPI monitors in the real session. The isolated fixture's scale factor of 1 is not acceptance of this matrix.

The remaining limitations in [the polish report](POLISH_VALIDATION.md) and [historical validation](VALIDATION.md) remain applicable. No real settings writes, push, or release occurred during this closeout.
