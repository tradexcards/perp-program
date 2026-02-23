# Deployment Guide

This guide covers deploying the TradeX Solana program to devnet or mainnet.

## Prerequisites

1. **Solana CLI** installed and configured
2. **Rust + cargo-build-sbf** for building Solana programs
3. **Bun** for running TypeScript scripts
4. **SOL** for deployment (~3 SOL for mainnet, ~2 SOL for devnet)

## Quick Start (Devnet)

```bash
# 1. Install dependencies
bun install

# 2. Create a keypair for admin/authority
solana-keygen new -o keypair.json

# 3. Fund with devnet SOL
solana airdrop 2 $(solana address -k keypair.json) --url devnet

# 4. Configure environment
cp .env.example .env
# Edit .env with devnet values

# 5. Deploy (builds, tests, deploys, initializes)
bun run deploy:fresh
```

## Manual Deployment Steps

### Step 1: Build the Program

```bash
# Build and run tests
bun run build

# Or build without tests
bun run build:no-test
```

The compiled program will be at `target/deploy/tradex_perp_program.so`.

### Step 2: Deploy to Network

**For a fresh deployment (new program ID):**

```bash
# Devnet
bun run deploy:fresh

# Mainnet (requires confirmation)
bun run deploy:fresh:mainnet
```

This script will:
1. Generate a new program keypair
2. Build the program
3. Run tests
4. Deploy to the network
5. Update .env with the new program ID
6. Initialize the exchange state

**For upgrading an existing deployment:**

```bash
# Devnet
bun run deploy

# Mainnet
bun run deploy:mainnet
```

### Step 3: Initialize Exchange (if not done by deploy-fresh)

```bash
bun run init-exchange
```

### Step 4: Verify Deployment

```bash
bun run status
```

## Mainnet Deployment

### Security Checklist

- [ ] Generated a new mainnet keypair (don't reuse devnet)
- [ ] Keypair backed up securely (hardware wallet, secure storage)
- [ ] Using a private RPC endpoint (Helius, QuickNode, etc.)
- [ ] Have sufficient SOL for deployment (~3 SOL)

### Configuration

Update `.env` for mainnet:

```bash
PROGRAM_ID=<your-program-id>
SOLANA_RPC_URL=https://mainnet.helius-rpc.com/?api-key=YOUR_KEY
USDC_MINT=EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v
KEYPAIR_PATH=./keypair.json
```

### Deploy

```bash
bun run deploy:fresh:mainnet
```

## Program Upgrade

To upgrade an existing deployment without changing the program ID:

```bash
# 1. Build new version
bun run build

# 2. Deploy upgrade
bun run deploy        # devnet
bun run deploy:mainnet  # mainnet
```

## Admin Operations

### Check Status

```bash
bun run status
```

### Transfer Admin

Transfer admin authority to a new wallet (two-step process):

```bash
# Step 1: Initiate transfer (current admin)
bun run transfer-admin -- --initiate <NEW_ADMIN_PUBKEY>

# Step 2: Accept transfer (new admin)
bun run transfer-admin -- --accept --keypair <NEW_KEYPAIR_PATH>
```

## Network Addresses

| Network | USDC Mint |
|---------|-----------|
| Devnet  | `4zMMC9srt5Ri5X14GAgXhaHii3GnPAEERYPJgZJDncDU` |
| Mainnet | `EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v` |

## Troubleshooting

### Build Fails

```bash
# Make sure you have the Solana BPF tools installed
solana-install update

# Clear build cache
cargo clean
```

### Deployment Fails

- Check you have enough SOL (`solana balance -k keypair.json --url <network>`)
- Verify RPC URL is correct
- For mainnet, ensure you're using a reliable RPC provider

### Transaction Simulation Failed

- Check the program ID in .env matches the deployed program
- Verify USDC mint address is correct for the network
- Ensure admin keypair has SOL for fees

## Files

| File | Description |
|------|-------------|
| `keypair.json` | Admin/authority keypair (git-ignored) |
| `target/deploy/tradex_perp_program.so` | Compiled program binary |
| `target/deploy/tradex_perp_program-keypair.json` | Program keypair (determines program ID) |
| `.env` | Environment configuration (git-ignored) |
