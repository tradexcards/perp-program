#!/usr/bin/env bash
# ══════════════════════════════════════════════════════════════════════════════
# deploy.sh — Upgrade existing Solana program deployment
# ══════════════════════════════════════════════════════════════════════════════
#
# Deploys the built .so binary to the existing program address.
# The program keypair must already exist at the default path.
#
# Usage:
#   ./scripts/deploy.sh              # deploy to devnet (default)
#   ./scripts/deploy.sh mainnet      # deploy to mainnet-beta
#   ./scripts/deploy.sh devnet       # deploy to devnet (explicit)
#
# Prerequisites:
#   - Run ./scripts/build.sh first
#   - keypair.json at project root (authority/payer)
#   - target/deploy/tradex_perp_program-keypair.json (program keypair)
#
set -euo pipefail
cd "$(dirname "$0")/.."

ROOT="$(pwd)"
NETWORK="${1:-devnet}"
SO_PATH="$ROOT/target/deploy/tradex_perp_program.so"
PROGRAM_KEYPAIR="$ROOT/target/deploy/tradex_perp_program-keypair.json"
PAYER_KEYPAIR="$ROOT/keypair.json"

# Validate network
case "$NETWORK" in
    devnet)   RPC_URL="https://api.devnet.solana.com" ;;
    mainnet)  RPC_URL="https://api.mainnet-beta.solana.com" ;;
    *)        echo "ERROR: Unknown network '$NETWORK'. Use 'devnet' or 'mainnet'."; exit 1 ;;
esac

# Validate files exist
for f in "$SO_PATH" "$PROGRAM_KEYPAIR" "$PAYER_KEYPAIR"; do
    if [ ! -f "$f" ]; then
        echo "ERROR: Required file not found: $f"
        [ "$f" = "$SO_PATH" ] && echo "  Run ./scripts/build.sh first"
        exit 1
    fi
done

PROGRAM_ID=$(solana address -k "$PROGRAM_KEYPAIR")
PAYER_ADDR=$(solana address -k "$PAYER_KEYPAIR")
SO_SIZE=$(stat -f%z "$SO_PATH" 2>/dev/null || stat -c%s "$SO_PATH" 2>/dev/null)

echo "══════════════════════════════════════════════"
echo "  Deploying Program (Upgrade)"
echo "══════════════════════════════════════════════"
echo ""
echo "  Network:    $NETWORK"
echo "  RPC:        $RPC_URL"
echo "  Program ID: $PROGRAM_ID"
echo "  Payer:      $PAYER_ADDR"
echo "  Binary:     $SO_SIZE bytes"
echo ""

# Safety check for mainnet
if [ "$NETWORK" = "mainnet" ]; then
    echo "WARNING: You are about to deploy to MAINNET."
    read -p "Type 'yes' to continue: " confirm
    if [ "$confirm" != "yes" ]; then
        echo "Aborted."
        exit 0
    fi
fi

# Check balance
BALANCE=$(solana balance "$PAYER_KEYPAIR" --url "$RPC_URL" 2>/dev/null | awk '{print $1}')
echo "→ Payer balance: $BALANCE SOL"
echo ""

# Deploy
echo "→ Deploying..."
solana program deploy "$SO_PATH" \
    --program-id "$PROGRAM_KEYPAIR" \
    --url "$RPC_URL" \
    -k "$PAYER_KEYPAIR"

echo ""

# Verify
echo "→ Verifying deployment..."
solana program show "$PROGRAM_ID" --url "$RPC_URL"

BALANCE_AFTER=$(solana balance "$PAYER_KEYPAIR" --url "$RPC_URL" 2>/dev/null | awk '{print $1}')
echo ""
echo "✓ Deployment complete"
echo "  Balance: $BALANCE SOL → $BALANCE_AFTER SOL"
