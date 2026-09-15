#!/usr/bin/env bash
# Opt-in, read-only fixture verification; requires an existing Wayland display.
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
    rg 'GUI_SMOKE PASS|GUI_CAPTURE' "$log"
    cp target/qa/polish-smoke.png "target/qa/polish-$variant.png"
    cp target/qa/polish-smoke-large.png "target/qa/polish-$variant-details.png"
done
