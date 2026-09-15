#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
glib-compile-schemas tests/schemas
cargo fmt --all --check
cargo test --locked -p cursormochi-core -p cursormochi-app
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
cargo build --release -p cursormochi-gtk --locked
if [[ "${1:-}" == "--gui" ]]; then
    ./scripts/check-gui.sh
fi
