#!/usr/bin/env bash
# ══════════════════════════════════════════════════════════════════════════════
# build.sh — Build Solana program and run all tests
# ══════════════════════════════════════════════════════════════════════════════
#
# Usage:
#   ./scripts/build.sh           # build + test
#   ./scripts/build.sh --no-test # build only
#
set -euo pipefail
cd "$(dirname "$0")/.."

ROOT="$(pwd)"
SO_PATH="$ROOT/target/deploy/tradex_perp_program.so"

echo "══════════════════════════════════════════════"
echo "  Building Solana Program"
echo "══════════════════════════════════════════════"

# Build
echo ""
echo "→ cargo build-sbf"
cargo build-sbf 2>&1 | grep -v "^warning:" || true

if [ ! -f "$SO_PATH" ]; then
    echo "ERROR: Build failed — $SO_PATH not found"
    exit 1
fi

SIZE=$(stat -f%z "$SO_PATH" 2>/dev/null || stat -c%s "$SO_PATH" 2>/dev/null)
echo "✓ Build complete: $(basename "$SO_PATH") ($SIZE bytes)"

# Test (unless --no-test)
if [[ "${1:-}" != "--no-test" ]]; then
    echo ""
    echo "══════════════════════════════════════════════"
    echo "  Running Tests"
    echo "══════════════════════════════════════════════"
    echo ""

    SBF_OUT_DIR="$ROOT/target/deploy" cargo test 2>&1 \
        | grep -E "^(running|test |test result)" || true

    echo ""
    echo "✓ Tests complete"
fi

echo ""
echo "══════════════════════════════════════════════"
echo "  Build complete"
echo "══════════════════════════════════════════════"
