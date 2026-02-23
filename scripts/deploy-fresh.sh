#!/usr/bin/env bash
# ══════════════════════════════════════════════════════════════════════════════
# deploy-fresh.sh — Full fresh deployment from scratch
# ══════════════════════════════════════════════════════════════════════════════
#
# Performs a complete fresh deployment:
#   1. Generate new program keypair
#   2. Build program
#   3. Run tests
#   4. Deploy to network
#   5. Update .env with new program ID
#   6. Initialize exchange on-chain
#
# Usage:
#   ./scripts/deploy-fresh.sh              # devnet (default)
#   ./scripts/deploy-fresh.sh mainnet      # mainnet (with confirmation)
#
set -euo pipefail
cd "$(dirname "$0")/.."

ROOT="$(pwd)"
NETWORK="${1:-devnet}"
SO_PATH="$ROOT/target/deploy/tradex_perp_program.so"
PROGRAM_KEYPAIR="$ROOT/target/deploy/tradex_perp_program-keypair.json"
PAYER_KEYPAIR="$ROOT/keypair.json"
ENV_FILE="$ROOT/.env"
ENV_NETWORK_FILE="$ROOT/.env.$NETWORK"

# Validate network
case "$NETWORK" in
    devnet)   RPC_URL="https://api.devnet.solana.com" ;;
    mainnet)  RPC_URL="https://api.mainnet-beta.solana.com" ;;
    *)        echo "ERROR: Unknown network '$NETWORK'. Use 'devnet' or 'mainnet'."; exit 1 ;;
esac

# Validate payer keypair
if [ ! -f "$PAYER_KEYPAIR" ]; then
    echo "ERROR: Payer keypair not found at $PAYER_KEYPAIR"
    echo "  Generate one with: solana-keygen new -o keypair.json"
    exit 1
fi

PAYER_ADDR=$(solana address -k "$PAYER_KEYPAIR")

echo "══════════════════════════════════════════════"
echo "  Fresh Deployment — $NETWORK"
echo "══════════════════════════════════════════════"
echo ""
echo "  Payer: $PAYER_ADDR"
echo "  RPC:   $RPC_URL"
echo ""

# Safety check for mainnet
if [ "$NETWORK" = "mainnet" ]; then
    echo "WARNING: This will deploy a NEW program to MAINNET."
    echo "         The old program will NOT be affected."
    read -p "Type 'yes' to continue: " confirm
    if [ "$confirm" != "yes" ]; then
        echo "Aborted."
        exit 0
    fi
fi

# ── Step 1: Generate new program keypair ─────────────────────────────────────
echo "──────────────────────────────────────────────"
echo "  Step 1/6: Generate program keypair"
echo "──────────────────────────────────────────────"

mkdir -p "$(dirname "$PROGRAM_KEYPAIR")"

# Back up old keypair if it exists
if [ -f "$PROGRAM_KEYPAIR" ]; then
    BACKUP="$ROOT/target/deploy/tradex_perp_program-keypair.$(date +%s).bak.json"
    cp "$PROGRAM_KEYPAIR" "$BACKUP"
    echo "  Backed up old keypair to $(basename "$BACKUP")"
fi

solana-keygen new -o "$PROGRAM_KEYPAIR" --force --no-bip39-passphrase 2>/dev/null
PROGRAM_ID=$(solana address -k "$PROGRAM_KEYPAIR")
echo "  New Program ID: $PROGRAM_ID"
echo ""

# ── Step 2: Build ────────────────────────────────────────────────────────────
echo "──────────────────────────────────────────────"
echo "  Step 2/6: Build program"
echo "──────────────────────────────────────────────"
cargo build-sbf 2>&1 | grep -v "^warning:" || true

if [ ! -f "$SO_PATH" ]; then
    echo "ERROR: Build failed"
    exit 1
fi
SO_SIZE=$(stat -f%z "$SO_PATH" 2>/dev/null || stat -c%s "$SO_PATH" 2>/dev/null)
echo "  Binary: $SO_SIZE bytes"
echo ""

# ── Step 3: Test ─────────────────────────────────────────────────────────────
echo "──────────────────────────────────────────────"
echo "  Step 3/6: Run tests"
echo "──────────────────────────────────────────────"
SBF_OUT_DIR="$ROOT/target/deploy" cargo test 2>&1 \
    | grep -E "^(test result)" || true
echo ""

# ── Step 4: Deploy ───────────────────────────────────────────────────────────
echo "──────────────────────────────────────────────"
echo "  Step 4/6: Deploy to $NETWORK"
echo "──────────────────────────────────────────────"

BALANCE=$(solana balance "$PAYER_KEYPAIR" --url "$RPC_URL" 2>/dev/null | awk '{print $1}')
echo "  Payer balance: $BALANCE SOL"

solana program deploy "$SO_PATH" \
    --program-id "$PROGRAM_KEYPAIR" \
    --url "$RPC_URL" \
    -k "$PAYER_KEYPAIR"

echo ""

# ── Step 5: Update .env files ────────────────────────────────────────────────
echo "──────────────────────────────────────────────"
echo "  Step 5/6: Update config files"
echo "──────────────────────────────────────────────"

update_env_program_id() {
    local file="$1"
    local new_id="$2"
    if [ ! -f "$file" ]; then
        echo "  SKIP: $file (not found)"
        return
    fi
    if grep -q "^PROGRAM_ID=" "$file"; then
        sed -i.bak "s/^PROGRAM_ID=.*/PROGRAM_ID=$new_id/" "$file"
        rm -f "$file.bak"
        echo "  Updated PROGRAM_ID in $(basename "$file")"
    fi
}

update_env_program_id "$ENV_FILE" "$PROGRAM_ID"
update_env_program_id "$ENV_NETWORK_FILE" "$PROGRAM_ID"
echo ""

# ── Step 6: Initialize exchange ──────────────────────────────────────────────
echo "──────────────────────────────────────────────"
echo "  Step 6/6: Initialize exchange on-chain"
echo "──────────────────────────────────────────────"

if [ -f "$ROOT/scripts/init-exchange.ts" ]; then
    bun run "$ROOT/scripts/init-exchange.ts"
else
    echo "  SKIP: init-exchange.ts not found"
    echo "  Run manually: bun run scripts/init-exchange.ts"
fi
echo ""

# ── Summary ──────────────────────────────────────────────────────────────────
BALANCE_AFTER=$(solana balance "$PAYER_KEYPAIR" --url "$RPC_URL" 2>/dev/null | awk '{print $1}')
echo "══════════════════════════════════════════════"
echo "  Fresh Deployment Complete"
echo "══════════════════════════════════════════════"
echo ""
echo "  Network:        $NETWORK"
echo "  Program ID:     $PROGRAM_ID"
echo "  Authority:      $PAYER_ADDR"
echo "  Balance:        $BALANCE_AFTER SOL"
echo ""
