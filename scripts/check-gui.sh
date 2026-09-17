#!/usr/bin/env bash
# Opt-in, fixture verification (imports install only into unique target/qa directories); requires an existing Wayland display.
set -euo pipefail
cd "$(dirname "$0")/.."
mkdir -p target/qa
glib-compile-schemas tests/schemas
for style in Adwaita Adwaita:dark HighContrast; do
    case "$style" in
        Adwaita) variant=light ;;
        Adwaita:dark) variant=dark ;;
        HighContrast) variant=highcontrast ;;
    esac
    log="target/qa/polish-gui-$variant.log"
    if ! timeout 45s dbus-run-session -- env GTK_A11Y=none GDK_BACKEND=wayland \
        GTK_THEME="$style" target/release/cursor-mochi --smoke-test > "$log" 2>&1; then
        cat "$log"
        exit 1
    fi
    rg 'GUI_SMOKE PASS|TRIAL_SMOKE PASS|FRONTEND_SMOKE PASS|IMPORT_SMOKE PASS|CURRENT_CURSOR_SMOKE PASS|GUI_CAPTURE' "$log"
    cp target/qa/frontend-playground.png "target/qa/frontend-$variant.png"
    cp target/qa/inspect-default.png "target/qa/inspect-$variant.png"
    cp target/qa/frontend-header.png "target/qa/frontend-$variant-header.png"
    cp target/qa/frontend-compact.png "target/qa/frontend-$variant-compact.png"
    cp target/qa/import-window.png "target/qa/import-$variant.png"
    cp target/qa/current-cursor-playground.png "target/qa/current-cursor-$variant.png"
    cp target/qa/current-cursor-roles.png "target/qa/current-cursor-$variant-roles.png"
    cp target/qa/trial-window.png "target/qa/trial-$variant.png"
    cp target/qa/polish-smoke.png "target/qa/polish-$variant.png"
    cp target/qa/polish-smoke-large.png "target/qa/polish-$variant-details.png"
done
