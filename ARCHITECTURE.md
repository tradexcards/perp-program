# TradeX Architecture

This document provides a deep technical overview of the TradeX perpetual DEX architecture.

## Table of Contents

1. [System Overview](#system-overview)
2. [Account Model](#account-model)
3. [Instruction Flow](#instruction-flow)
4. [Oracle System](#oracle-system)
5. [Trading Mechanics](#trading-mechanics)
6. [Risk Management](#risk-management)
7. [Fee Structure](#fee-structure)

---

## System Overview

TradeX is a perpetual futures exchange where users can:

```
User Wallet
    │
    ├── create_user_account ──► UserAccount PDA (tracks balance, stats)
    │
    ├── deposit ──────────────► USDC transferred to Vault
    │
    ├── open_position ────────► Position PDA created
    │                           Market OI updated
    │                           Margin deducted from balance
    │
    ├── close_position ───────► PnL calculated
    │                           Position closed
    │                           Balance credited
    │
    └── withdraw ─────────────► USDC transferred to user
```

### Why Pinocchio?

Pinocchio provides zero-copy account access, meaning:
- No heap allocations during instruction processing
- Direct memory access to account data
- Smaller binary size (optimized for on-chain)
- ~30% lower compute units vs Anchor

### Anchor Compatibility

Despite using Pinocchio, the program maintains Anchor wire compatibility:
- 8-byte discriminators: `SHA256("global:<fn_name>")[0..8]`
- Account discriminators: `SHA256("account:<StructName>")[0..8]`
- Same instruction data layout
- Existing Anchor clients work unchanged

---

## Account Model

### Global State Accounts

```
┌─────────────────────────────────────────────────────┐
│                   OracleState                       │
│  Seeds: ["oracle"]                                  │
├─────────────────────────────────────────────────────┤
│  authority: Pubkey         ← Signs oracle prices    │
│  pending_authority: Pubkey ← For 2-step transfer    │
│  feed_count: u32           ← Unused (legacy)        │
│  last_update_slot: u64     ← Unused (legacy)        │
│  last_update_ts: i64       ← Unused (legacy)        │
└─────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────┐
│                  ExchangeState                      │
│  Seeds: ["exchange"]                                │
├─────────────────────────────────────────────────────┤
│  admin: Pubkey             ← Admin authority        │
│  pending_admin: Pubkey     ← For 2-step transfer    │
│  usdc_mint: Pubkey         ← Collateral token       │
│  vault: Pubkey             ← Main vault address     │
│  insurance_fund: Pubkey    ← Insurance address      │
│  market_count: u32         ← Number of markets      │
│  total_deposits: u64       ← User deposits sum      │
│  total_volume: u128        ← Cumulative volume      │
│  total_fees_collected: u64 ← Protocol fees          │
│  insurance_balance: u64    ← Insurance fund value   │
│  paused: u8                ← Emergency stop         │
└─────────────────────────────────────────────────────┘
```

### Per-Market Accounts

```
┌─────────────────────────────────────────────────────┐
│                     Market                          │
│  Seeds: ["market", product_id.to_le_bytes()]        │
├─────────────────────────────────────────────────────┤
│  product_id: u32           ← Unique market ID       │
│  oracle_price_feed: Pubkey ← Unused (legacy)        │
│  max_leverage: u32         ← Market leverage cap    │
│  max_open_interest: u64    ← OI limit               │
│  long_open_interest: u64   ← Current long OI        │
│  short_open_interest: u64  ← Current short OI       │
│  total_volume: u128        ← Market volume          │
│  last_funding_time: i64    ← Last funding epoch     │
│  cumulative_funding_rate_long: i64  ← For calcs     │
│  cumulative_funding_rate_short: i64 ← For calcs     │
│  active: u8                ← Market enabled         │
└─────────────────────────────────────────────────────┘
```

### Per-User Accounts

```
┌─────────────────────────────────────────────────────┐
│                   UserAccount                       │
│  Seeds: ["user_account", owner]                     │
├─────────────────────────────────────────────────────┤
│  owner: Pubkey             ← Wallet address         │
│  balance: u64              ← Available USDC         │
│  total_deposited: u64      ← Lifetime deposits      │
│  total_withdrawn: u64      ← Lifetime withdrawals   │
│  total_realized_pnl: i64   ← Lifetime PnL           │
│  total_fees_paid: u64      ← Lifetime fees          │
│  open_position_count: u32  ← Active positions       │
└─────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────┐
│                    Position                         │
│  Seeds: ["position", owner, product_id.to_le_bytes()]│
├─────────────────────────────────────────────────────┤
│  owner: Pubkey             ← Position owner         │
│  product_id: u32           ← Market ID              │
│  side: u8                  ← 0=Long, 1=Short        │
│  size: u64                 ← Position size (USDC)   │
│  entry_price: u64          ← Entry price (8 dec)    │
│  leverage: u32             ← Position leverage      │
│  margin: u64               ← Collateral amount      │
│  liquidation_price: u64    ← Auto-liq price         │
│  realized_pnl: i64         ← Realized PnL           │
│  entry_funding_rate: i64   ← For funding calcs      │
│  opened_at: i64            ← Open timestamp         │
│  last_updated: i64         ← Update timestamp       │
└─────────────────────────────────────────────────────┘
```

---

## Instruction Flow

### Opening a Position

```
┌────────────────────────────────────────────────────────────────┐
│                    OPEN POSITION FLOW                          │
└────────────────────────────────────────────────────────────────┘

1. CLIENT PREPARES TRANSACTION
   ┌─────────────────────────────────────────────────────────┐
   │  Instruction 0: Ed25519SigVerify                        │
   │    - Signature over [product_id, price, timestamp]      │
   │    - Signed by oracle authority                         │
   ├─────────────────────────────────────────────────────────┤
   │  Instruction 1: open_position                           │
   │    - product_id, side, size, leverage, price, timestamp │
   └─────────────────────────────────────────────────────────┘

2. PROGRAM VALIDATES
   ┌─────────────────────────────────────────────────────────┐
   │  ✓ Exchange not paused                                  │
   │  ✓ Market exists and is active                          │
   │  ✓ Leverage within market limits                        │
   │  ✓ Size >= $1 and <= $100k                              │
   │  ✓ Ed25519 signature valid (scans instructions sysvar)  │
   │  ✓ Price not stale (< 15 seconds old)                   │
   │  ✓ User has sufficient balance                          │
   └─────────────────────────────────────────────────────────┘

3. PROGRAM COMPUTES
   ┌─────────────────────────────────────────────────────────┐
   │  margin = size / leverage                               │
   │  fee = size * 0.05%                                     │
   │  total_cost = margin + fee                              │
   │  liq_price = f(entry_price, leverage, side)             │
   └─────────────────────────────────────────────────────────┘

4. STATE CHANGES
   ┌─────────────────────────────────────────────────────────┐
   │  UserAccount:                                           │
   │    balance -= total_cost                                │
   │    fees_paid += fee                                     │
   │    open_position_count += 1                             │
   ├─────────────────────────────────────────────────────────┤
   │  Position: (CREATED)                                    │
   │    owner, product_id, side, size, entry_price           │
   │    leverage, margin, liquidation_price                  │
   │    entry_funding_rate = market.cumulative_funding       │
   ├─────────────────────────────────────────────────────────┤
   │  Market:                                                │
   │    long/short_open_interest += size                     │
   │    total_volume += size                                 │
   ├─────────────────────────────────────────────────────────┤
   │  ExchangeState:                                         │
   │    total_fees_collected += fee                          │
   │    total_volume += size                                 │
   └─────────────────────────────────────────────────────────┘
```

### Closing a Position

```
┌────────────────────────────────────────────────────────────────┐
│                   CLOSE POSITION FLOW                          │
└────────────────────────────────────────────────────────────────┘

1. PNL CALCULATION
   ┌─────────────────────────────────────────────────────────┐
   │  For LONG:                                              │
   │    pnl = size * (exit_price - entry_price) / entry      │
   │                                                         │
   │  For SHORT:                                             │
   │    pnl = size * (entry_price - exit_price) / entry      │
   └─────────────────────────────────────────────────────────┘

2. FUNDING CALCULATION
   ┌─────────────────────────────────────────────────────────┐
   │  rate_diff = current_cumulative - entry_cumulative      │
   │  funding = size * rate_diff / 10000                     │
   │                                                         │
   │  (positive = owed, negative = received)                 │
   └─────────────────────────────────────────────────────────┘

3. FINAL SETTLEMENT
   ┌─────────────────────────────────────────────────────────┐
   │  closing_fee = size * 0.05%                             │
   │  net_pnl = pnl - funding - closing_fee                  │
   │  payout = margin + net_pnl                              │
   │                                                         │
   │  if payout < 0:                                         │
   │    payout = 0                                           │
   │    bad_debt = |net_pnl| - margin                        │
   │    → insurance covers bad_debt                          │
   │                                                         │
   │  if payout > vault_balance:                             │
   │    → insurance covers shortfall                         │
   └─────────────────────────────────────────────────────────┘

4. STATE CHANGES
   ┌─────────────────────────────────────────────────────────┐
   │  UserAccount:                                           │
   │    balance += payout                                    │
   │    total_realized_pnl += net_pnl                        │
   │    total_fees_paid += closing_fee                       │
   │    open_position_count -= 1                             │
   ├─────────────────────────────────────────────────────────┤
   │  Position: (CLOSED - rent refunded to owner)            │
   ├─────────────────────────────────────────────────────────┤
   │  Market:                                                │
   │    long/short_open_interest -= size                     │
   │    total_volume += size                                 │
   ├─────────────────────────────────────────────────────────┤
   │  ExchangeState:                                         │
   │    total_fees_collected += closing_fee                  │
   │    insurance_balance -= bad_debt (if any)               │
   └─────────────────────────────────────────────────────────┘
```

---

## Oracle System

### Ed25519 Signature Verification

The oracle uses Ed25519 signatures for price authentication:

```
┌─────────────────────────────────────────────────────────────┐
│                  ORACLE PRICE FLOW                          │
└─────────────────────────────────────────────────────────────┘

1. OFF-CHAIN (Oracle Server)
   ┌─────────────────────────────────────────────────────────┐
   │  message = [                                            │
   │    product_id: u32 (4 bytes),                           │
   │    price: u64 (8 bytes),                                │
   │    timestamp: i64 (8 bytes)                             │
   │  ]  // Total: 20 bytes                                  │
   │                                                         │
   │  signature = Ed25519.sign(oracle_private_key, message)  │
   └─────────────────────────────────────────────────────────┘

2. CLIENT TRANSACTION
   ┌─────────────────────────────────────────────────────────┐
   │  Instruction 0: Ed25519SigVerify (precompile)           │
   │    - num_signatures: 1                                  │
   │    - signature: 64 bytes                                │
   │    - public_key: 32 bytes (oracle authority)            │
   │    - message: 20 bytes                                  │
   ├─────────────────────────────────────────────────────────┤
   │  Instruction 1: open_position/close_position/liquidate  │
   │    - price, timestamp passed in instruction data        │
   └─────────────────────────────────────────────────────────┘

3. ON-CHAIN VERIFICATION
   ┌─────────────────────────────────────────────────────────┐
   │  verify_signed_price():                                 │
   │    1. Load expected pubkey from OracleState             │
   │    2. Reconstruct expected message from ix data         │
   │    3. Scan instructions sysvar for Ed25519SigVerify     │
   │    4. Verify pubkey matches oracle authority            │
   │    5. Verify message matches expected                   │
   │    6. Check timestamp within staleness window           │
   └─────────────────────────────────────────────────────────┘
```

### Why Ed25519 Instead of On-Chain Oracle?

1. **Low Latency**: Prices signed off-chain, verified on-chain
2. **No Oracle Account Reads**: Reduces compute units
3. **Flexible**: Works with any price source
4. **Secure**: Cryptographically verifiable

---

## Trading Mechanics

### Leverage Calculation

```
Position opens with 10x leverage on $1000 size:

margin = size / leverage = $1000 / 10 = $100
fee = size * 0.05% = $1000 * 0.0005 = $0.50
total_cost = $100 + $0.50 = $100.50

The user controls $1000 notional with $100 collateral.
```

### Liquidation Price Calculation

```
For LONG position:
  liq_price = entry_price * (L - 1 + MM * L) / L
  
  Where:
    L = leverage
    MM = maintenance_margin (1% = 0.01)
    
  Example: entry = $100, leverage = 10x
    liq_price = 100 * (10 - 1 + 0.01 * 10) / 10
              = 100 * (9.1) / 10
              = $91
    
For SHORT position:
  liq_price = entry_price * (L + 1 - MM * L) / L
  
  Example: entry = $100, leverage = 10x
    liq_price = 100 * (10 + 1 - 0.01 * 10) / 10
              = 100 * (10.9) / 10
              = $109
```

### PnL Calculation (Precision)

```rust
// Uses i128 intermediate to prevent overflow and maintain precision
let pnl = (size as i128)
    .checked_mul(price_diff as i128)?
    .checked_mul(PRICE_SCALE as i128)?  // Scale up first
    .checked_div(entry_price as i128)?
    .checked_div(PRICE_SCALE as i128)?; // Scale down after

// This prevents truncation for:
// - Small positions on expensive assets
// - Large positions with small price changes
```

### Funding Rate Mechanism

```
Funding paid every 8 hours based on OI imbalance:

funding_rate = (long_OI - short_OI) / total_OI * 0.05%
              (capped at +/- 0.05%)

Example:
  long_OI = $1,000,000
  short_OI = $600,000
  total_OI = $1,600,000
  
  imbalance = ($1M - $600k) / $1.6M = 0.25 = 25%
  funding_rate = 25% * 0.05% = 0.0125%
  
  Longs PAY 0.0125% of position size
  Shorts RECEIVE 0.0125% of position size

Cumulative rates track total funding since market creation:
  - Position entry records current cumulative rate
  - On close: funding = size * (current_rate - entry_rate)
```

---

## Risk Management

### Insurance Fund

```
┌─────────────────────────────────────────────────────────────┐
│                  INSURANCE FUND FLOW                        │
└─────────────────────────────────────────────────────────────┘

INFLOWS:
  ├── Admin deposits (fund_insurance instruction)
  ├── Liquidation insurance fee (0.5% of remaining margin)
  └── Keeper liquidation fees (100% of remaining margin)

OUTFLOWS:
  ├── Bad debt coverage (loss > margin on close)
  └── Vault shortfall coverage (vault < payout)

PROTECTION:
  The insurance fund ensures:
  1. Winning traders always get paid
  2. System remains solvent during extreme moves
  3. No socialized losses to other traders
```

### Position Limits

```
MIN_POSITION_SIZE: $1
  - Prevents dust positions
  - Ensures meaningful trades

MAX_POSITION_SIZE_PER_USER: $100,000
  - Limits whale concentration risk
  - Prevents market manipulation
  - Reduces oracle exploitation vectors

MAX_LEVERAGE: 50x
  - Per-market configurable (default 50x)
  - Higher leverage = faster liquidation
  - Admin can reduce per-market
```

### Emergency Controls

```
Pause System:
  set_paused(true)
  - Blocks: deposits, withdrawals, open_position, liquidate
  - Allows: close_position (users must exit)
  
Market Deactivation:
  set_market_active(false)
  - Blocks: open_position
  - Allows: close_position (users must exit)
```

---

## Fee Structure

### Trading Fees

```
Opening Fee: 0.05% of position size
Closing Fee: 0.05% of position size
Total Round-Trip: 0.10%

Example: $10,000 position
  Open fee: $10,000 * 0.0005 = $5
  Close fee: $10,000 * 0.0005 = $5
  Total fees: $10
```

### Liquidation Fees

```
Regular Liquidation (process_liquidate):
  Liquidator Reward: 0.5% of remaining margin
  Insurance Fee: 0.5% of remaining margin
  Total: 1% of remaining margin

Keeper Liquidation (process_liquidate_keeper):
  Insurance Fee: 100% of remaining margin
  (No liquidator reward - keeper is trusted)
```

### Fee Withdrawal

```
Fees accumulate in exchange_state.total_fees_collected
Admin can withdraw to TREASURY_WALLET only:

withdraw_fees(amount):
  1. Verify admin signature
  2. Verify treasury_token_account.owner == TREASURY_WALLET
  3. Verify vault has enough (vault - amount >= total_deposits)
  4. Transfer vault -> treasury
  5. Decrement total_fees_collected

Security: Treasury address is hardcoded - admin cannot redirect fees
```

---

## Account Validation System

The `instruction_accounts!` macro provides compile-time account validation:

```rust
instruction_accounts! {
    pub struct OpenPositionAccounts<'a> {
        pub user_account:         account_mut,        // Line 1
        pub position:             init_pda,           // Line 2
        pub market:               account_mut,        // Line 3
        pub exchange_state:       pda_mut(EXCHANGE_SEED),  // Line 4
        pub oracle_state:         pda(ORACLE_SEED),   // Line 5
        pub owner:                signer_mut,         // Line 6
        pub system_program:       program,            // Line 7
        pub instructions_sysvar:  instructions_sysvar,// Line 8
    }
}
```

Generated validation:

```rust
// account_mut: owned by program
if !user_account.owned_by(program_id) {
    return Err(IllegalOwner);
}

// init_pda: no check (will be created)
// (position skipped)

// account_mut: owned by program
if !market.owned_by(program_id) {
    return Err(IllegalOwner);
}

// pda_mut: owned by program + PDA verified
if !exchange_state.owned_by(program_id) {
    return Err(IllegalOwner);
}
verify_pda(program_id, exchange_state, &[EXCHANGE_SEED])?;

// pda: owned by program + PDA verified
if !oracle_state.owned_by(program_id) {
    return Err(IllegalOwner);
}
verify_pda(program_id, oracle_state, &[ORACLE_SEED])?;

// signer_mut: must sign
if !owner.is_signer() {
    return Err(MissingRequiredSignature);
}

// program: must be known program
if !is_known_program(system_program.address()) {
    return Err(IncorrectProgramId);
}

// instructions_sysvar: must be instructions address
if instructions_sysvar.address() != INSTRUCTIONS_ID {
    return Err(UnsupportedSysvar);
}
```

This ensures all account checks happen before any state modification, preventing partial state updates on validation failures.
