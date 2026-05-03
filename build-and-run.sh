#!/usr/bin/env bash
set -euo pipefail

# Check prerequisites
if ! command -v solang &>/dev/null; then
  echo "ERROR: 'solang' binary not found. See docs/05-recreate-from-scratch.md Phase 0."
  exit 1
fi
echo "[ok] solang $(solang --version)"

# Install WASM target if needed
if ! rustup target list --installed | grep -q wasm32-unknown-unknown; then
  echo "[setup] Installing wasm32-unknown-unknown..."
  rustup target add wasm32-unknown-unknown
fi

# Build the Rust reference contract WASM
RUST_WASM_PATH="${1:-target/wasm32-unknown-unknown/release/counter_rs.wasm}"

if [ -n "${1:-}" ]; then
  echo "[skip] Using provided WASM: $RUST_WASM_PATH"
else
  echo ""
  echo "══════════════════════════════════════════════════════════════"
  echo "  [1/2] Building Rust Soroban SDK counter WASM..."
  echo "══════════════════════════════════════════════════════════════"
  cargo build -p counter_rs --target wasm32-unknown-unknown --release
  echo "[ok] WASM at: $RUST_WASM_PATH"
fi

echo ""
echo "══════════════════════════════════════════════════════════════"
echo "  [2/2] Running solang-diff..."
echo "══════════════════════════════════════════════════════════════"
echo ""
cargo run --bin solang-diff --features soroban -- "$RUST_WASM_PATH"
