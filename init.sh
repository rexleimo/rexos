#!/usr/bin/env bash
set -euo pipefail

echo "[rexos] building..."
cargo build

echo "[rexos] tests..."
cargo test

echo "[rexos] smoke: CLI help"
cargo build -p rexos-cli
./target/debug/rexos --help >/dev/null

echo "[rexos] done"
