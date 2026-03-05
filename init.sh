#!/usr/bin/env bash
set -euo pipefail

echo "[loopforge] building..."
cargo build

echo "[loopforge] tests..."
cargo test

echo "[loopforge] smoke: CLI help"
cargo build -p rexos-cli
./target/debug/loopforge --help >/dev/null

echo "[loopforge] done"
