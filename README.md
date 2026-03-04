# TradeX Solana Program

A high-performance Solana on-chain perpetual futures exchange for Pokemon card prices. Built with [Pinocchio](https://github.com/anza-xyz/pinocchio) for zero-copy, no-allocation performance.

## Program Address

| Network     | Program ID                                     |
| ----------- | ---------------------------------------------- |
| **Mainnet** | `TRDXCHHWWZdQXxkGFJxek1KU2hn19687xztNBy99Ca1` |

## Overview

TradeX allows traders to go long or short on Pokemon card prices with up to 20x leverage. The program handles:

- **Oracle price management** with Ed25519 signature verification
- **Position lifecycle** (open, close, add margin, liquidate)
- **Funding rate calculations** to balance long/short positions
- **Insurance fund** for bad debt socialization
- **Admin controls** for pausing, market management, and fee collection

## Architecture

```
src/
  lib.rs          Entrypoint + instruction dispatch table
  oracle.rs       Oracle authority management
  perpdex.rs      Trading instruction handlers
  state.rs        Zero-copy account state definitions
  accounts.rs     Declarative account validation macro
  helpers.rs      PDA derivation, math, signature verification
  token_cpi.rs    SPL Token CPI helpers
  constants.rs    Program constants and seeds
  error.rs        Custom error codes
```

For detailed architecture with flow diagrams, see [ARCHITECTURE.md](./ARCHITECTURE.md).

## Source Files

### `lib.rs` - Entrypoint & Dispatch

The main entrypoint that:
1. Validates instruction data has at least 8 bytes (discriminator)
2. Extracts the 8-byte Anchor-compatible discriminator
3. Routes to the appropriate instruction handler

Uses SHA-256 hashed discriminators (`sha256("global:fn_name")[0..8]`) for Anchor IDL compatibility.

### `state.rs` - Account State Definitions

Zero-copy packed structs representing on-chain account data:

| Account         | Size (bytes) | Description                       |
| --------------- | ------------ | --------------------------------- |
| `OracleState`   | 92           | Oracle authority + pending auth   |
| `ExchangeState` | 213          | Exchange settings, vault, stats   |
| `Market`        | 113          | Per-product market state + OI     |
| `UserAccount`   | 84           | User balance + trading stats      |
| `Position`      | 113          | Individual position details       |

### `oracle.rs` - Oracle Management

| Instruction           | Description                          |
| --------------------- | ------------------------------------ |
| `initialize_oracle`   | Creates oracle state PDA             |
| `transfer_authority`  | Initiates 2-step authority transfer  |
| `accept_authority`    | Completes authority transfer         |

### `perpdex.rs` - Trading Instructions

**Exchange Setup**: `initialize_exchange`, `create_market`, `create_market_permissionless`

**User Management**: `create_user_account`, `close_user_account`, `deposit`, `withdraw`

**Position Management**: `open_position`, `close_position`, `add_margin`, `liquidate`

**Market Operations**: `update_funding_rate`

**Admin**: `set_paused`, `set_market_active`, `update_market_params`, `fund_insurance`, `withdraw_fees`

### `accounts.rs` - Account Validation Macro

Declarative macro for type-safe account validation:

```rust
instruction_accounts! {
    pub struct OpenPositionAccounts<'a> {
        pub user_account:   account_mut,
        pub position:       init_pda,
        pub market:         account_mut,
        pub exchange_state: pda_mut(EXCHANGE_SEED),
        pub owner:          signer_mut,
        pub system_program: program,
    }
}
```

### `helpers.rs` - Utility Functions

- **PDA Functions**: `verify_pda()`, `find_pda()`
- **Oracle Verification**: `verify_signed_price()` - validates Ed25519 signatures
- **Math**: `calculate_pnl()`, `calculate_liquidation_price()`, `calculate_funding_payment()`
- **Binary Readers**: `read_u32()`, `read_u64()`, `read_i64()`, `read_pubkey()`

### `token_cpi.rs` - SPL Token CPI

Helpers for SPL Token operations: `token_transfer_user_signed()`, `token_transfer_pda_signed()`, `read_token_amount()`, etc.

### `constants.rs` - Program Constants

| Constant                    | Value         | Description                    |
| --------------------------- | ------------- | ------------------------------ |
| `PRICE_SCALE`               | 10^8          | 8 decimal price precision      |
| `MAX_LEVERAGE`              | 20            | Maximum leverage allowed       |
| `MIN_POSITION_SIZE`         | 1_000_000     | $1 minimum position            |
| `MAX_POSITION_SIZE_PER_USER`| 100_000_000_000 | $100k max per user           |
| `MAINTENANCE_MARGIN_BPS`    | 100           | 1% maintenance margin          |
| `TRADING_FEE_BPS`           | 5             | 0.05% trading fee              |
| `FUNDING_INTERVAL`          | 28_800        | 8 hours                        |
| `MAX_ORACLE_PRICE_AGE`      | 15            | 15 second staleness limit      |

### `error.rs` - Error Codes

Custom error codes (base 6000): `InvalidAmount`, `InsufficientBalance`, `ExchangePaused`, `InvalidLeverage`, `PositionTooSmall`, `MaxOpenInterestExceeded`, `PositionNotLiquidatable`, `OraclePriceStale`, etc.

## Account PDAs

| Account         | Seeds                             |
| --------------- | --------------------------------- |
| `OracleState`   | `["oracle"]`                      |
| `ExchangeState` | `["exchange"]`                    |
| `Vault`         | `["vault"]`                       |
| `InsuranceFund` | `["insurance"]`                   |
| `Market`        | `["market", product_id]`          |
| `UserAccount`   | `["user_account", owner]`         |
| `Position`      | `["position", owner, product_id]` |

## Testing

Tests use [mollusk-svm](https://github.com/anza-xyz/mollusk) for local BPF execution without a validator.

| File            | Tests | Focus                                    |
| --------------- | ----- | ---------------------------------------- |
| `functional.rs` | 25+   | Unit tests per instruction               |
| `e2e.rs`        | 17    | End-to-end trading flows                 |
| `fuzz.rs`       | 15    | Boundary conditions, invalid inputs      |
| `stress.rs`     | 16    | Large values, rapid updates, edge cases  |

### Running Tests

```bash
# Build the program
cargo build-sbf

# Run all tests
SBF_OUT_DIR=target/deploy cargo test

# Run specific test file
SBF_OUT_DIR=target/deploy cargo test --test functional
```

## Security Features

1. **Ed25519 Signature Verification** - Oracle prices are signed off-chain and verified on-chain
2. **Two-Step Authority Transfers** - Admin/oracle authority transfers require acceptance
3. **Pause Enforcement** - All trading operations check exchange pause state
4. **PDA Address Verification** - All PDAs verified against expected derivation
5. **Checked Arithmetic** - All math uses `checked_*` operations
6. **Per-User Position Limits** - $100k max position size per user
7. **Price Staleness Checks** - Oracle prices must be fresh (15 second max age)
8. **Close Position Escapes Pause** - Users can always exit positions even when paused

## Build

```bash
cargo build-sbf
```

Release profile optimized for size:
- `opt-level = "z"` (optimize for size)
- `lto = "fat"` (link-time optimization)
- `panic = "abort"` (no unwinding)

## License

Business Source License 1.1 (BUSL-1.1), non-commercial use only.
Automatically changes to GNU GPLv3 on 2030-02-18.

## Author

Built by [@ankushKun_](https://x.com/ankushKun_)

## Links

- Website: https://tradex.cards
- API Docs: https://tradex.cards/docs
