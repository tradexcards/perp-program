use pinocchio::{
    account::AccountView,
    cpi::Seed,
    cpi::Signer,
    error::ProgramError,
    sysvars::{clock::Clock, Sysvar},
    Address, ProgramResult,
};
// M-03: PDA creation uses create_pda_account from helpers (resilient to prefunding DoS)

use crate::constants::*;
use crate::error::PerpDexError;
use crate::helpers::*;
use crate::oracle::rent_exempt_minimum;
use crate::state::*;
use crate::token_cpi::*;

// ══════════════════════════════════════════════════════════════════════════════
// Account context structs (matching Anchor field order exactly)
// ══════════════════════════════════════════════════════════════════════════════

// InitializeExchange: exchange_state, usdc_mint, vault, insurance_fund, admin, system_program, token_program, rent
instruction_accounts! {
    pub struct InitializeExchangeAccounts<'a> {
        pub exchange_state:  init_pda,
        pub usdc_mint:       unchecked,
        pub vault:           init_pda,
        pub insurance_fund:  init_pda,
        pub admin:           signer_mut,
        pub system_program:  program,
        pub token_program:   program,
        pub rent:            sysvar,
    }
}

// CreateMarket: market, exchange_state, admin, system_program
instruction_accounts! {
    pub struct CreateMarketAccounts<'a> {
        pub market:           init_pda,
        pub exchange_state:   pda_mut(EXCHANGE_SEED),
        pub admin:            signer_mut,
        pub system_program:   program,
    }
}

// CreateMarketPermissionless: market, exchange_state, payer, system_program
instruction_accounts! {
    pub struct CreateMarketPermissionlessAccounts<'a> {
        pub market:           init_pda,
        pub exchange_state:   pda_mut(EXCHANGE_SEED),
        pub payer:            signer_mut,
        pub system_program:   program,
    }
}

// CreateUserAccount: user_account, owner, system_program
instruction_accounts! {
    pub struct CreateUserAccountAccounts<'a> {
        pub user_account:    init_pda,
        pub owner:           signer_mut,
        pub system_program:  program,
    }
}

// CloseUserAccount: user_account, owner
instruction_accounts! {
    pub struct CloseUserAccountAccounts<'a> {
        pub user_account:    account_mut,
        pub owner:           signer_mut,
    }
}

// Deposit: user_account, exchange_state, vault, user_token_account, owner, token_program
instruction_accounts! {
    pub struct DepositAccounts<'a> {
        pub user_account:       account_mut,
        pub exchange_state:     pda_mut(EXCHANGE_SEED),
        pub vault:              token_mut,
        pub user_token_account: token_mut,
        pub owner:              signer_mut,
        pub token_program:      program,
    }
}

// Withdraw: user_account, exchange_state, vault, user_token_account, owner, token_program
instruction_accounts! {
    pub struct WithdrawAccounts<'a> {
        pub user_account:       account_mut,
        pub exchange_state:     pda_mut(EXCHANGE_SEED),
        pub vault:              token_mut,
        pub user_token_account: token_mut,
        pub owner:              signer_mut,
        pub token_program:      program,
    }
}

// OpenPosition: user_account, position, market, exchange_state, oracle_state, owner, system_program, instructions_sysvar
instruction_accounts! {
    pub struct OpenPositionAccounts<'a> {
        pub user_account:         account_mut,
        pub position:             init_pda,
        pub market:               account_mut,
        pub exchange_state:       pda_mut(EXCHANGE_SEED),
        pub oracle_state:         pda(ORACLE_SEED),
        pub owner:                signer_mut,
        pub system_program:       program,
        pub instructions_sysvar:  instructions_sysvar,
    }
}

// ClosePosition: user_account, position, market, exchange_state, vault, insurance_fund, oracle_state, owner, token_program, instructions_sysvar
instruction_accounts! {
    pub struct ClosePositionAccounts<'a> {
        pub user_account:         account_mut,
        pub position:             account_mut,
        pub market:               account_mut,
        pub exchange_state:       pda_mut(EXCHANGE_SEED),
        pub vault:                token_mut,
        pub insurance_fund:       token_mut,
        pub oracle_state:         pda(ORACLE_SEED),
        pub owner:                signer_mut,
        pub token_program:        program,
        pub instructions_sysvar:  instructions_sysvar,
    }
}

// AddMargin: user_account, position, owner
instruction_accounts! {
    pub struct AddMarginAccounts<'a> {
        pub user_account:    account_mut,
        pub position:        account_mut,
        pub owner:           signer_mut,
    }
}

// Liquidate: position_owner_account, position, liquidator_account, market, exchange_state,
//            vault, insurance_fund, oracle_state, position_owner, liquidator, token_program, instructions_sysvar
instruction_accounts! {
    pub struct LiquidateAccounts<'a> {
        pub position_owner_account: account_mut,
        pub position:               account_mut,
        pub liquidator_account:     account_mut,
        pub market:                 account_mut,
        pub exchange_state:         pda_mut(EXCHANGE_SEED),
        pub vault:                  token_mut,
        pub insurance_fund:         token_mut,
        pub oracle_state:           pda(ORACLE_SEED),
        pub position_owner:         unchecked_mut,
        pub liquidator:             signer_mut,
        pub token_program:          program,
        pub instructions_sysvar:    instructions_sysvar,
    }
}

// LiquidateKeeper: keeper-only liquidation (no liquidator user account needed)
// position_owner_account, position, market, exchange_state, vault, insurance_fund,
// oracle_state, position_owner, keeper, token_program, instructions_sysvar
instruction_accounts! {
    pub struct LiquidateKeeperAccounts<'a> {
        pub position_owner_account: account_mut,
        pub position:               account_mut,
        pub market:                 account_mut,
        pub exchange_state:         pda_mut(EXCHANGE_SEED),
        pub vault:                  token_mut,
        pub insurance_fund:         token_mut,
        pub oracle_state:           pda(ORACLE_SEED),
        pub position_owner:         unchecked_mut,
        pub keeper:                 signer_mut,
        pub token_program:          program,
        pub instructions_sysvar:    instructions_sysvar,
    }
}

// UpdateFundingRate: market, exchange_state, cranker
instruction_accounts! {
    pub struct UpdateFundingRateAccounts<'a> {
        pub market:          account_mut,
        pub exchange_state:  pda(EXCHANGE_SEED),
        pub cranker:         signer,
    }
}

// AdminOnly (set_paused, transfer_admin): exchange_state, admin
instruction_accounts! {
    pub struct AdminOnlyAccounts<'a> {
        pub exchange_state:  pda_mut(EXCHANGE_SEED),
        pub admin:           signer,
    }
}

// AcceptAdmin: exchange_state, new_admin
instruction_accounts! {
    pub struct AcceptAdminAccounts<'a> {
        pub exchange_state:  pda_mut(EXCHANGE_SEED),
        pub new_admin:       signer,
    }
}

// AdminMarket (set_market_active, update_market_params): market, exchange_state, admin
instruction_accounts! {
    pub struct AdminMarketAccounts<'a> {
        pub market:          account_mut,
        pub exchange_state:  pda(EXCHANGE_SEED),
        pub admin:           signer,
    }
}

// FundInsurance: exchange_state, insurance_fund, admin_token_account, admin, token_program
instruction_accounts! {
    pub struct FundInsuranceAccounts<'a> {
        pub exchange_state:      pda_mut(EXCHANGE_SEED),
        pub insurance_fund:      token_mut,
        pub admin_token_account: token_mut,
        pub admin:               signer_mut,
        pub token_program:       program,
    }
}

// WithdrawFees: exchange_state, vault, treasury_token_account, admin, token_program
instruction_accounts! {
    pub struct WithdrawFeesAccounts<'a> {
        pub exchange_state:         pda_mut(EXCHANGE_SEED),
        pub vault:                  token_mut,
        pub treasury_token_account: token_mut,
        pub admin:                  signer,
        pub token_program:          program,
    }
}

// ══════════════════════════════════════════════════════════════════════════════
// Instruction handlers
// ══════════════════════════════════════════════════════════════════════════════

// ── 1. Initialize Exchange ──────────────────────────────────────────────────

#[inline(never)]
pub fn process_initialize_exchange(
    program_id: &Address,
    accounts: &[AccountView],
) -> ProgramResult {
    let ctx = InitializeExchangeAccounts::parse(program_id, accounts)?;

    // M-04: Only the bootstrap authority can initialize the exchange
    if *ctx.admin.address().as_array() != BOOTSTRAP_AUTHORITY {
        return Err(PerpDexError::Unauthorized.into());
    }

    // ── Create exchange_state PDA ────────────────────────────────────────
    let (exchange_pda, exchange_bump) = find_pda(program_id, &[EXCHANGE_SEED])?;
    if exchange_pda != *ctx.exchange_state.address() {
        return Err(ProgramError::InvalidArgument);
    }

    let exchange_bump_seed = [exchange_bump];
    let exchange_seeds: [Seed; 2] = [Seed::from(EXCHANGE_SEED), Seed::from(&exchange_bump_seed)];
    let exchange_signer = Signer::from(&exchange_seeds);

    create_pda_account(
        ctx.admin,
        ctx.exchange_state,
        rent_exempt_minimum(ExchangeState::ACCOUNT_SIZE),
        ExchangeState::ACCOUNT_SIZE as u64,
        program_id,
        &[exchange_signer],
    )?;

    // ── Create vault token account PDA ──────────────────────────────────
    let (vault_pda, vault_bump) = find_pda(program_id, &[VAULT_SEED])?;
    if vault_pda != *ctx.vault.address() {
        return Err(ProgramError::InvalidArgument);
    }

    let vault_bump_seed = [vault_bump];
    let vault_seeds: [Seed; 2] = [Seed::from(VAULT_SEED), Seed::from(&vault_bump_seed)];
    let vault_signer = Signer::from(&vault_seeds);

    create_pda_account(
        ctx.admin,
        ctx.vault,
        rent_exempt_minimum(165),
        165,
        &Address::from(TOKEN_PROGRAM_ID),
        &[vault_signer],
    )?;

    // Initialize the vault as a token account
    initialize_token_account(
        ctx.token_program,
        ctx.vault,
        ctx.usdc_mint,
        ctx.exchange_state,
        ctx.rent,
    )?;

    // ── Create insurance fund token account PDA ─────────────────────────
    let (insurance_pda, insurance_bump) = find_pda(program_id, &[INSURANCE_SEED])?;
    if insurance_pda != *ctx.insurance_fund.address() {
        return Err(ProgramError::InvalidArgument);
    }

    let insurance_bump_seed = [insurance_bump];
    let insurance_seeds: [Seed; 2] = [Seed::from(INSURANCE_SEED), Seed::from(&insurance_bump_seed)];
    let insurance_signer = Signer::from(&insurance_seeds);

    create_pda_account(
        ctx.admin,
        ctx.insurance_fund,
        rent_exempt_minimum(165),
        165,
        &Address::from(TOKEN_PROGRAM_ID),
        &[insurance_signer],
    )?;

    initialize_token_account(
        ctx.token_program,
        ctx.insurance_fund,
        ctx.usdc_mint,
        ctx.exchange_state,
        ctx.rent,
    )?;

    // ── Init exchange_state data ─────────────────────────────────────────
    {
        let mut data = ctx.exchange_state.try_borrow_mut()?;
        let exchange = ExchangeState::init(&mut data)?;
        exchange.admin = *ctx.admin.address().as_array();
        exchange.pending_admin = [0u8; 32];
        exchange.usdc_mint = *ctx.usdc_mint.address().as_array();
        exchange.vault = *ctx.vault.address().as_array();
        exchange.insurance_fund = *ctx.insurance_fund.address().as_array();
        exchange.market_count = 0;
        exchange.total_deposits = 0;
        exchange.total_volume = 0;
        exchange.total_fees_collected = 0;
        exchange.insurance_balance = 0;
        exchange.paused = 0;
    }

    Ok(())
}

// ── 2. Create Market ────────────────────────────────────────────────────────

#[inline(never)]
pub fn process_create_market(
    program_id: &Address,
    accounts: &[AccountView],
    product_id: u32,
    max_leverage: u32,
    max_open_interest: u64,
) -> ProgramResult {
    let ctx = CreateMarketAccounts::parse(program_id, accounts)?;

    // Verify has_one = admin
    {
        let data = ctx.exchange_state.try_borrow()?;
        let exchange = ExchangeState::load(&data)?;
        if exchange.admin != *ctx.admin.address().as_array() {
            return Err(PerpDexError::Unauthorized.into());
        }
        if exchange.is_paused() {
            return Err(PerpDexError::ExchangePaused.into());
        }
    }

    if max_leverage == 0 || max_leverage > MAX_LEVERAGE {
        return Err(PerpDexError::InvalidLeverage.into());
    }
    if max_open_interest == 0 {
        return Err(PerpDexError::InvalidAmount.into());
    }

    // Check market count
    {
        let data = ctx.exchange_state.try_borrow()?;
        let exchange = ExchangeState::load(&data)?;
        if exchange.market_count >= MAX_MARKETS {
            return Err(PerpDexError::TooManyMarkets.into());
        }
    }

    // ── Create market PDA ────────────────────────────────────────────────
    let product_id_bytes = product_id.to_le_bytes();
    let (market_pda, market_bump) = find_pda(program_id, &[MARKET_SEED, &product_id_bytes])?;
    if market_pda != *ctx.market.address() {
        return Err(ProgramError::InvalidArgument);
    }

    let market_bump_seed = [market_bump];
    let market_seeds: [Seed; 3] = [
        Seed::from(MARKET_SEED),
        Seed::from(product_id_bytes.as_ref()),
        Seed::from(&market_bump_seed),
    ];
    let market_signer = Signer::from(&market_seeds);

    create_pda_account(
        ctx.admin,
        ctx.market,
        rent_exempt_minimum(Market::ACCOUNT_SIZE),
        Market::ACCOUNT_SIZE as u64,
        program_id,
        &[market_signer],
    )?;

    // ── Init market data ─────────────────────────────────────────────────
    {
        let mut data = ctx.market.try_borrow_mut()?;
        let market = Market::init(&mut data)?;
        market.product_id = product_id;
        market.oracle_price_feed = [0u8; 32]; // dead field — kept for layout compat
        market.max_leverage = max_leverage;
        market.max_open_interest = max_open_interest;
        market.long_open_interest = 0;
        market.short_open_interest = 0;
        market.total_volume = 0;
        market.last_funding_time = Clock::get()?.unix_timestamp;
        market.cumulative_funding_rate_long = 0;
        market.cumulative_funding_rate_short = 0;
        market.active = 1;
    }

    // Increment market count
    {
        let mut data = ctx.exchange_state.try_borrow_mut()?;
        let exchange = ExchangeState::load_mut(&mut data)?;
        exchange.market_count = exchange
            .market_count
            .checked_add(1)
            .ok_or(PerpDexError::MathOverflow)?;
    }

    Ok(())
}

// ── 3. Create Market Permissionless ─────────────────────────────────────────

#[inline(never)]
pub fn process_create_market_permissionless(
    program_id: &Address,
    accounts: &[AccountView],
    product_id: u32,
) -> ProgramResult {
    let ctx = CreateMarketPermissionlessAccounts::parse(program_id, accounts)?;

    // Verify not paused
    {
        let data = ctx.exchange_state.try_borrow()?;
        let exchange = ExchangeState::load(&data)?;
        if exchange.is_paused() {
            return Err(PerpDexError::ExchangePaused.into());
        }
        if exchange.market_count >= MAX_MARKETS {
            return Err(PerpDexError::TooManyMarkets.into());
        }
    }

    // ── Create market PDA ────────────────────────────────────────────────
    let product_id_bytes = product_id.to_le_bytes();
    let (market_pda, market_bump) = find_pda(program_id, &[MARKET_SEED, &product_id_bytes])?;
    if market_pda != *ctx.market.address() {
        return Err(ProgramError::InvalidArgument);
    }

    let market_bump_seed = [market_bump];
    let market_seeds: [Seed; 3] = [
        Seed::from(MARKET_SEED),
        Seed::from(product_id_bytes.as_ref()),
        Seed::from(&market_bump_seed),
    ];
    let market_signer = Signer::from(&market_seeds);

    create_pda_account(
        ctx.payer,
        ctx.market,
        rent_exempt_minimum(Market::ACCOUNT_SIZE),
        Market::ACCOUNT_SIZE as u64,
        program_id,
        &[market_signer],
    )?;

    // ── Init market data ─────────────────────────────────────────────────
    {
        let mut data = ctx.market.try_borrow_mut()?;
        let market = Market::init(&mut data)?;
        market.product_id = product_id;
        market.oracle_price_feed = [0u8; 32]; // dead field — kept for layout compat
        market.max_leverage = MAX_LEVERAGE;
        market.max_open_interest = 1_000_000_000_000;
        market.long_open_interest = 0;
        market.short_open_interest = 0;
        market.total_volume = 0;
        market.last_funding_time = Clock::get()?.unix_timestamp;
        market.cumulative_funding_rate_long = 0;
        market.cumulative_funding_rate_short = 0;
        market.active = 1;
    }

    // Increment market count
    {
        let mut data = ctx.exchange_state.try_borrow_mut()?;
        let exchange = ExchangeState::load_mut(&mut data)?;
        exchange.market_count = exchange
            .market_count
            .checked_add(1)
            .ok_or(PerpDexError::MathOverflow)?;
    }

    Ok(())
}

// ── 4. Create User Account ──────────────────────────────────────────────────

#[inline(never)]
pub fn process_create_user_account(
    program_id: &Address,
    accounts: &[AccountView],
) -> ProgramResult {
    let ctx = CreateUserAccountAccounts::parse(program_id, accounts)?;

    // ── Create user_account PDA ──────────────────────────────────────────
    let owner_key = ctx.owner.address().as_ref();
    let (ua_pda, ua_bump) = find_pda(program_id, &[USER_ACCOUNT_SEED, owner_key])?;
    if ua_pda != *ctx.user_account.address() {
        return Err(ProgramError::InvalidArgument);
    }

    let ua_bump_seed = [ua_bump];
    let ua_seeds: [Seed; 3] = [
        Seed::from(USER_ACCOUNT_SEED),
        Seed::from(owner_key),
        Seed::from(&ua_bump_seed),
    ];
    let ua_signer = Signer::from(&ua_seeds);

    create_pda_account(
        ctx.owner,
        ctx.user_account,
        rent_exempt_minimum(UserAccount::ACCOUNT_SIZE),
        UserAccount::ACCOUNT_SIZE as u64,
        program_id,
        &[ua_signer],
    )?;

    // ── Init user_account data ───────────────────────────────────────────
    {
        let mut data = ctx.user_account.try_borrow_mut()?;
        let ua = UserAccount::init(&mut data)?;
        ua.owner = *ctx.owner.address().as_array();
        ua.balance = 0;
        ua.total_deposited = 0;
        ua.total_withdrawn = 0;
        ua.total_realized_pnl = 0;
        ua.total_fees_paid = 0;
        ua.open_position_count = 0;
    }

    Ok(())
}

// ── 5. Close User Account ───────────────────────────────────────────────────

#[inline(never)]
pub fn process_close_user_account(
    _program_id: &Address,
    accounts: &[AccountView],
) -> ProgramResult {
    let ctx = CloseUserAccountAccounts::parse(_program_id, accounts)?;

    // Load and verify user_account
    {
        let data = ctx.user_account.try_borrow()?;
        let ua = UserAccount::load(&data)?;
        if ua.owner != *ctx.owner.address().as_array() {
            return Err(PerpDexError::Unauthorized.into());
        }
        if ua.balance != 0 {
            return Err(PerpDexError::HasBalance.into());
        }
        if ua.open_position_count != 0 {
            return Err(PerpDexError::HasOpenPositions.into());
        }
    }

    // Close account: transfer lamports to owner and zero data
    close_account(ctx.user_account, ctx.owner)?;

    Ok(())
}

// ── 6. Deposit ──────────────────────────────────────────────────────────────

#[inline(never)]
pub fn process_deposit(
    _program_id: &Address,
    accounts: &[AccountView],
    amount: u64,
) -> ProgramResult {
    let ctx = DepositAccounts::parse(_program_id, accounts)?;

    if amount == 0 {
        return Err(PerpDexError::InvalidAmount.into());
    }

    // Verify has_one = owner on user_account
    {
        let data = ctx.user_account.try_borrow()?;
        let ua = UserAccount::load(&data)?;
        if ua.owner != *ctx.owner.address().as_array() {
            return Err(PerpDexError::Unauthorized.into());
        }
    }

    // Verify exchange not paused (C-3/H-6)
    {
        let exchange_data = ctx.exchange_state.try_borrow()?;
        let exchange = ExchangeState::load(&exchange_data)?;
        if exchange.is_paused() {
            return Err(PerpDexError::ExchangePaused.into());
        }
    }

    // Verify vault address matches exchange_state.vault (H-4)
    {
        let exchange_data = ctx.exchange_state.try_borrow()?;
        let exchange = ExchangeState::load(&exchange_data)?;
        if *ctx.vault.address().as_array() != exchange.vault {
            return Err(ProgramError::InvalidArgument);
        }
    }

    // Verify user_token_account owner and mint
    {
        let token_owner = read_token_owner(ctx.user_token_account)?;
        if token_owner != *ctx.owner.address().as_array() {
            return Err(PerpDexError::Unauthorized.into());
        }
        let token_mint = read_token_mint(ctx.user_token_account)?;
        let exchange_data = ctx.exchange_state.try_borrow()?;
        let exchange = ExchangeState::load(&exchange_data)?;
        if token_mint != exchange.usdc_mint {
            return Err(PerpDexError::InvalidAmount.into());
        }
    }

    // Transfer tokens: user -> vault
    token_transfer_user_signed(
        ctx.token_program,
        ctx.user_token_account,
        ctx.vault,
        ctx.owner,
        amount,
    )?;

    // Update user_account
    {
        let mut data = ctx.user_account.try_borrow_mut()?;
        let ua = UserAccount::load_mut(&mut data)?;
        ua.balance = ua
            .balance
            .checked_add(amount)
            .ok_or(PerpDexError::MathOverflow)?;
        ua.total_deposited = ua
            .total_deposited
            .checked_add(amount)
            .ok_or(PerpDexError::MathOverflow)?;
    }

    // Update exchange total_deposits
    {
        let mut data = ctx.exchange_state.try_borrow_mut()?;
        let exchange = ExchangeState::load_mut(&mut data)?;
        exchange.total_deposits = exchange
            .total_deposits
            .checked_add(amount)
            .ok_or(PerpDexError::MathOverflow)?;
    }

    Ok(())
}

// ── 7. Withdraw ─────────────────────────────────────────────────────────────

#[inline(never)]
pub fn process_withdraw(
    program_id: &Address,
    accounts: &[AccountView],
    amount: u64,
) -> ProgramResult {
    let ctx = WithdrawAccounts::parse(program_id, accounts)?;

    if amount == 0 {
        return Err(PerpDexError::InvalidAmount.into());
    }

    // Verify exchange not paused (M-3: withdrawals should also respect pause)
    {
        let exchange_data = ctx.exchange_state.try_borrow()?;
        let exchange = ExchangeState::load(&exchange_data)?;
        if exchange.is_paused() {
            return Err(PerpDexError::ExchangePaused.into());
        }
    }

    // Verify vault address matches exchange_state.vault (H-4)
    {
        let exchange_data = ctx.exchange_state.try_borrow()?;
        let exchange = ExchangeState::load(&exchange_data)?;
        if *ctx.vault.address().as_array() != exchange.vault {
            return Err(ProgramError::InvalidArgument);
        }
    }

    // SECURITY: Verify user_token_account is owned by the withdrawing user
    // This prevents accidentally sending funds to the wrong address
    {
        let token_owner = read_token_owner(ctx.user_token_account)?;
        if token_owner != *ctx.owner.address().as_array() {
            return Err(PerpDexError::InvalidTokenAccountOwner.into());
        }
    }

    // Verify and update user_account
    {
        let mut data = ctx.user_account.try_borrow_mut()?;
        let ua = UserAccount::load_mut(&mut data)?;
        if ua.owner != *ctx.owner.address().as_array() {
            return Err(PerpDexError::Unauthorized.into());
        }
        if ua.balance < amount {
            return Err(PerpDexError::InsufficientBalance.into());
        }
        ua.balance = ua
            .balance
            .checked_sub(amount)
            .ok_or(PerpDexError::MathOverflow)?;
        ua.total_withdrawn = ua
            .total_withdrawn
            .checked_add(amount)
            .ok_or(PerpDexError::MathOverflow)?;
    }

    // Update exchange total_deposits
    {
        let mut data = ctx.exchange_state.try_borrow_mut()?;
        let exchange = ExchangeState::load_mut(&mut data)?;
        exchange.total_deposits = exchange
            .total_deposits
            .checked_sub(amount)
            .ok_or(PerpDexError::MathOverflow)?;
    }

    // Transfer tokens: vault -> user (PDA-signed by exchange_state)
    let (_, exchange_bump) = find_pda(program_id, &[EXCHANGE_SEED])?;
    let exchange_bump_seed = [exchange_bump];
    let signer_seeds: [Seed; 2] = [Seed::from(EXCHANGE_SEED), Seed::from(&exchange_bump_seed)];

    token_transfer_pda_signed(
        ctx.token_program,
        ctx.vault,
        ctx.user_token_account,
        ctx.exchange_state,
        amount,
        &signer_seeds,
    )?;

    Ok(())
}

// ── 8. Open Position ────────────────────────────────────────────────────────

#[inline(never)]
pub fn process_open_position(
    program_id: &Address,
    accounts: &[AccountView],
    product_id: u32,
    side: Side,
    size: u64,
    leverage: u32,
    price: u64,
    timestamp: i64,
) -> ProgramResult {
    let ctx = OpenPositionAccounts::parse(program_id, accounts)?;

    // SECURITY: Verify market PDA address matches expected derivation
    let pid_bytes = product_id.to_le_bytes();
    verify_pda(program_id, ctx.market, &[MARKET_SEED, &pid_bytes])?;

    // Verify exchange not paused
    {
        let data = ctx.exchange_state.try_borrow()?;
        let exchange = ExchangeState::load(&data)?;
        if exchange.is_paused() {
            return Err(PerpDexError::ExchangePaused.into());
        }
    }

    // Read market data
    let market_max_leverage: u32;
    let cum_funding_long: i64;
    let cum_funding_short: i64;
    {
        let data = ctx.market.try_borrow()?;
        let market = Market::load(&data)?;
        if market.active == 0 {
            return Err(PerpDexError::MarketInactive.into());
        }
        if market.product_id != product_id {
            return Err(PerpDexError::ProductMismatch.into());
        }
        market_max_leverage = market.max_leverage;
        cum_funding_long = market.cumulative_funding_rate_long;
        cum_funding_short = market.cumulative_funding_rate_short;
    }

    if leverage == 0 || leverage > market_max_leverage {
        return Err(PerpDexError::InvalidLeverage.into());
    }
    if size < MIN_POSITION_SIZE {
        return Err(PerpDexError::PositionTooSmall.into());
    }
    // SECURITY: Per-user position size limit to prevent whale concentration risk
    if size > MAX_POSITION_SIZE_PER_USER {
        return Err(PerpDexError::PositionTooLarge.into());
    }

    // Verify Ed25519-signed oracle price
    verify_signed_price(
        ctx.instructions_sysvar,
        ctx.oracle_state,
        product_id,
        price,
        timestamp,
        0, // Ed25519 instruction at index 0
    )?;
    let oracle_price = price;

    // Calculate costs
    let margin = size
        .checked_div(leverage as u64)
        .ok_or(PerpDexError::MathOverflow)?;
    let fee = size
        .checked_mul(TRADING_FEE_BPS)
        .ok_or(PerpDexError::MathOverflow)?
        .checked_div(BPS_DENOMINATOR)
        .ok_or(PerpDexError::MathOverflow)?;
    let total_cost = margin.checked_add(fee).ok_or(PerpDexError::MathOverflow)?;

    // Verify and update user_account
    {
        let mut data = ctx.user_account.try_borrow_mut()?;
        let ua = UserAccount::load_mut(&mut data)?;
        if ua.owner != *ctx.owner.address().as_array() {
            return Err(PerpDexError::Unauthorized.into());
        }
        if ua.balance < total_cost {
            return Err(PerpDexError::InsufficientBalance.into());
        }
        ua.balance = ua
            .balance
            .checked_sub(total_cost)
            .ok_or(PerpDexError::MathOverflow)?;
        ua.total_fees_paid = ua
            .total_fees_paid
            .checked_add(fee)
            .ok_or(PerpDexError::MathOverflow)?;
        ua.open_position_count = ua
            .open_position_count
            .checked_add(1)
            .ok_or(PerpDexError::MathOverflow)?;
    }

    // ── Create position PDA ──────────────────────────────────────────────
    let owner_key = ctx.owner.address().as_ref();
    let product_id_bytes = product_id.to_le_bytes();
    let (position_pda, position_bump) =
        find_pda(program_id, &[POSITION_SEED, owner_key, &product_id_bytes])?;
    if position_pda != *ctx.position.address() {
        return Err(ProgramError::InvalidArgument);
    }

    let position_bump_seed = [position_bump];
    let position_seeds: [Seed; 4] = [
        Seed::from(POSITION_SEED),
        Seed::from(owner_key),
        Seed::from(product_id_bytes.as_ref()),
        Seed::from(&position_bump_seed),
    ];
    let position_signer = Signer::from(&position_seeds);

    create_pda_account(
        ctx.owner,
        ctx.position,
        rent_exempt_minimum(Position::ACCOUNT_SIZE),
        Position::ACCOUNT_SIZE as u64,
        program_id,
        &[position_signer],
    )?;

    // ── Init position data ───────────────────────────────────────────────
    let liq_price = calculate_liquidation_price(oracle_price, leverage, side)?;
    let clock = Clock::get()?;
    let now = clock.unix_timestamp;
    {
        let mut data = ctx.position.try_borrow_mut()?;
        let position = Position::init(&mut data)?;
        position.owner = *ctx.owner.address().as_array();
        position.product_id = product_id;
        position.side = side as u8;
        position.size = size;
        position.entry_price = oracle_price;
        position.leverage = leverage;
        position.margin = margin;
        position.liquidation_price = liq_price;
        position.realized_pnl = 0;
        position.entry_funding_rate = match side {
            Side::Long => cum_funding_long,
            Side::Short => cum_funding_short,
        };
        position.opened_at = now;
        position.last_updated = now;
    }

    // ── Update market OI ─────────────────────────────────────────────────
    {
        let mut data = ctx.market.try_borrow_mut()?;
        let market = Market::load_mut(&mut data)?;
        match side {
            Side::Long => {
                market.long_open_interest = market
                    .long_open_interest
                    .checked_add(size)
                    .ok_or(PerpDexError::MathOverflow)?;
            }
            Side::Short => {
                market.short_open_interest = market
                    .short_open_interest
                    .checked_add(size)
                    .ok_or(PerpDexError::MathOverflow)?;
            }
        }
        let total_oi = market
            .long_open_interest
            .checked_add(market.short_open_interest)
            .ok_or(PerpDexError::MathOverflow)?;
        if total_oi > market.max_open_interest {
            return Err(PerpDexError::MaxOpenInterestExceeded.into());
        }
        market.total_volume = market
            .total_volume
            .checked_add(size as u128)
            .ok_or(PerpDexError::MathOverflow)?;
    }

    // ── Update exchange stats ────────────────────────────────────────────
    {
        let mut data = ctx.exchange_state.try_borrow_mut()?;
        let exchange = ExchangeState::load_mut(&mut data)?;
        exchange.total_fees_collected = exchange
            .total_fees_collected
            .checked_add(fee)
            .ok_or(PerpDexError::MathOverflow)?;
        exchange.total_volume = exchange
            .total_volume
            .checked_add(size as u128)
            .ok_or(PerpDexError::MathOverflow)?;
        // M-01: Decrease total_deposits when user balance is reduced by total_cost,
        // so withdraw_fees can correctly compute available protocol revenue.
        exchange.total_deposits = exchange.total_deposits.saturating_sub(total_cost);
    }

    Ok(())
}

// ── 9. Close Position ───────────────────────────────────────────────────────

#[inline(never)]
pub fn process_close_position(
    program_id: &Address,
    accounts: &[AccountView],
    product_id: u32,
    price: u64,
    timestamp: i64,
) -> ProgramResult {
    let ctx = ClosePositionAccounts::parse(program_id, accounts)?;

    // SECURITY: Verify market and position PDA addresses
    let pid_bytes = product_id.to_le_bytes();
    verify_pda(program_id, ctx.market, &[MARKET_SEED, &pid_bytes])?;
    verify_pda(
        program_id,
        ctx.position,
        &[POSITION_SEED, ctx.owner.address().as_ref(), &pid_bytes],
    )?;

    // Verify vault and insurance_fund addresses
    // NOTE: close_position is allowed even when paused (users must be able to exit)
    {
        let data = ctx.exchange_state.try_borrow()?;
        let exchange = ExchangeState::load(&data)?;
        if *ctx.vault.address().as_array() != exchange.vault {
            return Err(ProgramError::InvalidArgument);
        }
        if *ctx.insurance_fund.address().as_array() != exchange.insurance_fund {
            return Err(ProgramError::InvalidArgument);
        }
    }

    // Read market data (no active check — users must be able to close on inactive markets)
    let cum_funding_long: i64;
    let cum_funding_short: i64;
    {
        let data = ctx.market.try_borrow()?;
        let market = Market::load(&data)?;
        cum_funding_long = market.cumulative_funding_rate_long;
        cum_funding_short = market.cumulative_funding_rate_short;
    }

    // Verify Ed25519-signed oracle price
    verify_signed_price(
        ctx.instructions_sysvar,
        ctx.oracle_state,
        product_id,
        price,
        timestamp,
        0, // Ed25519 instruction at index 0
    )?;
    let oracle_price = price;

    // Read position data
    let pos_product_id: u32;
    let pos_entry_price: u64;
    let pos_size: u64;
    let pos_side: Side;
    let pos_margin: u64;
    let pos_entry_funding_rate: i64;
    let pos_owner: [u8; 32];
    {
        let data = ctx.position.try_borrow()?;
        let position = Position::load(&data)?;
        pos_product_id = position.product_id;
        pos_entry_price = position.entry_price;
        pos_size = position.size;
        pos_side = position.side_enum();
        pos_margin = position.margin;
        pos_entry_funding_rate = position.entry_funding_rate;
        pos_owner = position.owner;
    }

    if pos_product_id != product_id {
        return Err(PerpDexError::ProductMismatch.into());
    }
    if pos_owner != *ctx.owner.address().as_array() {
        return Err(PerpDexError::Unauthorized.into());
    }

    // Calculate PnL, funding, fee
    let pnl = calculate_pnl(pos_entry_price, oracle_price, pos_size, pos_side)?;
    let current_cumulative = match pos_side {
        Side::Long => cum_funding_long,
        Side::Short => cum_funding_short,
    };
    let funding_payment =
        calculate_funding_payment(pos_entry_funding_rate, current_cumulative, pos_size)?;
    let fee = pos_size
        .checked_mul(TRADING_FEE_BPS)
        .ok_or(PerpDexError::MathOverflow)?
        .checked_div(BPS_DENOMINATOR)
        .ok_or(PerpDexError::MathOverflow)?;
    let net_pnl = pnl
        .checked_sub(funding_payment)
        .ok_or(PerpDexError::MathOverflow)?
        .checked_sub(fee as i64)
        .ok_or(PerpDexError::MathOverflow)?;
    let margin_i64: i64 = pos_margin
        .try_into()
        .map_err(|_| PerpDexError::MathOverflow)?;
    let payout_i64 = margin_i64
        .checked_add(net_pnl)
        .ok_or(PerpDexError::MathOverflow)?;
    let payout: u64 = if payout_i64 < 0 { 0 } else { payout_i64 as u64 };

    // Check vault balance
    let vault_balance = read_token_amount(ctx.vault)?;
    let actual_payout = payout.min(vault_balance);
    let shortfall = payout.saturating_sub(actual_payout);
    // M-01: Track total credit to user balance for total_deposits update
    let mut total_user_credit = actual_payout;

    // Update user_account
    {
        let mut data = ctx.user_account.try_borrow_mut()?;
        let ua = UserAccount::load_mut(&mut data)?;
        ua.balance = ua
            .balance
            .checked_add(actual_payout)
            .ok_or(PerpDexError::MathOverflow)?;
        ua.total_realized_pnl = ua
            .total_realized_pnl
            .checked_add(net_pnl)
            .ok_or(PerpDexError::MathOverflow)?;
        ua.total_fees_paid = ua
            .total_fees_paid
            .checked_add(fee)
            .ok_or(PerpDexError::MathOverflow)?;
        ua.open_position_count = ua
            .open_position_count
            .checked_sub(1)
            .ok_or(PerpDexError::MathOverflow)?;
    }

    // PDA signer for exchange_state
    let (_, exchange_bump) = find_pda(program_id, &[EXCHANGE_SEED])?;
    let exchange_bump_seed = [exchange_bump];
    let signer_seeds: [Seed; 2] = [Seed::from(EXCHANGE_SEED), Seed::from(&exchange_bump_seed)];

    // Handle bad debt (loss beyond margin) — transfer insurance_fund -> vault
    if payout == 0 {
        // Use i128 to safely compute absolute value (avoids i64::MIN overflow)
        let abs_net_pnl = (net_pnl as i128).unsigned_abs() as u64;
        let loss_beyond_margin = abs_net_pnl.saturating_sub(pos_margin);
        if loss_beyond_margin > 0 {
            let insurance_balance = read_token_amount(ctx.insurance_fund)?;
            let transfer_amount = loss_beyond_margin.min(insurance_balance);
            if transfer_amount > 0 {
                token_transfer_pda_signed(
                    ctx.token_program,
                    ctx.insurance_fund,
                    ctx.vault,
                    ctx.exchange_state,
                    transfer_amount,
                    &signer_seeds,
                )?;
            }
            // L-01: Use actual transfer_amount (not loss_beyond_margin) for bookkeeping,
            // since the transfer is capped by insurance token balance.
            let mut data = ctx.exchange_state.try_borrow_mut()?;
            let exchange = ExchangeState::load_mut(&mut data)?;
            exchange.insurance_balance = exchange.insurance_balance.saturating_sub(transfer_amount);
        }
    }

    // Handle vault shortfall — transfer insurance_fund -> vault
    if shortfall > 0 {
        let insurance_balance = read_token_amount(ctx.insurance_fund)?;
        let transfer_amount = shortfall.min(insurance_balance);
        if transfer_amount > 0 {
            token_transfer_pda_signed(
                ctx.token_program,
                ctx.insurance_fund,
                ctx.vault,
                ctx.exchange_state,
                transfer_amount,
                &signer_seeds,
            )?;
            // H-01: Re-credit user with the insurance-covered shortfall amount.
            // Without this, the user loses funds that insurance actually covered.
            let mut data = ctx.user_account.try_borrow_mut()?;
            let ua = UserAccount::load_mut(&mut data)?;
            ua.balance = ua
                .balance
                .checked_add(transfer_amount)
                .ok_or(PerpDexError::MathOverflow)?;
            total_user_credit += transfer_amount;
        }
        // L-01: Use actual transfer_amount (not shortfall) for bookkeeping,
        // since the transfer is capped by insurance token balance.
        let mut data = ctx.exchange_state.try_borrow_mut()?;
        let exchange = ExchangeState::load_mut(&mut data)?;
        exchange.insurance_balance = exchange.insurance_balance.saturating_sub(transfer_amount);
    }

    // Update market OI
    {
        let mut data = ctx.market.try_borrow_mut()?;
        let market = Market::load_mut(&mut data)?;
        match pos_side {
            Side::Long => {
                market.long_open_interest = market.long_open_interest.saturating_sub(pos_size);
            }
            Side::Short => {
                market.short_open_interest = market.short_open_interest.saturating_sub(pos_size);
            }
        }
        market.total_volume = market
            .total_volume
            .checked_add(pos_size as u128)
            .ok_or(PerpDexError::MathOverflow)?;
    }

    // Update exchange stats
    {
        let mut data = ctx.exchange_state.try_borrow_mut()?;
        let exchange = ExchangeState::load_mut(&mut data)?;
        exchange.total_fees_collected = exchange
            .total_fees_collected
            .checked_add(fee)
            .ok_or(PerpDexError::MathOverflow)?;
        exchange.total_volume = exchange
            .total_volume
            .checked_add(pos_size as u128)
            .ok_or(PerpDexError::MathOverflow)?;
        // M-01: Increase total_deposits when user balance is credited,
        // so withdraw_fees correctly reflects vault obligations to users.
        exchange.total_deposits = exchange
            .total_deposits
            .checked_add(total_user_credit)
            .ok_or(PerpDexError::MathOverflow)?;
    }

    // Close position account (refund rent to owner)
    close_account(ctx.position, ctx.owner)?;

    Ok(())
}

// ── 10. Add Margin ──────────────────────────────────────────────────────────

#[inline(never)]
pub fn process_add_margin(
    program_id: &Address,
    accounts: &[AccountView],
    product_id: u32,
    amount: u64,
) -> ProgramResult {
    let ctx = AddMarginAccounts::parse(program_id, accounts)?;

    // SECURITY: Verify position PDA address
    let pid_bytes = product_id.to_le_bytes();
    verify_pda(
        program_id,
        ctx.position,
        &[POSITION_SEED, ctx.owner.address().as_ref(), &pid_bytes],
    )?;

    if amount == 0 {
        return Err(PerpDexError::InvalidAmount.into());
    }

    // Read position data
    let pos_side: Side;
    let pos_entry_price: u64;
    let pos_size: u64;
    {
        let data = ctx.position.try_borrow()?;
        let position = Position::load(&data)?;
        if position.product_id != product_id {
            return Err(PerpDexError::ProductMismatch.into());
        }
        if position.owner != *ctx.owner.address().as_array() {
            return Err(PerpDexError::Unauthorized.into());
        }
        pos_side = position.side_enum();
        pos_entry_price = position.entry_price;
        pos_size = position.size;
    }

    // Verify and update user_account balance
    {
        let mut data = ctx.user_account.try_borrow_mut()?;
        let ua = UserAccount::load_mut(&mut data)?;
        if ua.owner != *ctx.owner.address().as_array() {
            return Err(PerpDexError::Unauthorized.into());
        }
        if ua.balance < amount {
            return Err(PerpDexError::InsufficientBalance.into());
        }
        ua.balance = ua
            .balance
            .checked_sub(amount)
            .ok_or(PerpDexError::MathOverflow)?;
    }

    // Update position margin and liquidation price
    {
        let mut data = ctx.position.try_borrow_mut()?;
        let position = Position::load_mut(&mut data)?;
        position.margin = position
            .margin
            .checked_add(amount)
            .ok_or(PerpDexError::MathOverflow)?;
        position.liquidation_price = calculate_liquidation_price_from_margin(
            pos_entry_price,
            position.margin,
            pos_size,
            pos_side,
        )?;
        position.last_updated = Clock::get()?.unix_timestamp;
    }

    Ok(())
}

// ── 11. Liquidate ───────────────────────────────────────────────────────────

#[inline(never)]
pub fn process_liquidate(
    program_id: &Address,
    accounts: &[AccountView],
    product_id: u32,
    price: u64,
    timestamp: i64,
) -> ProgramResult {
    let ctx = LiquidateAccounts::parse(program_id, accounts)?;

    // SECURITY: Verify market and position PDA addresses
    let pid_bytes = product_id.to_le_bytes();
    verify_pda(program_id, ctx.market, &[MARKET_SEED, &pid_bytes])?;
    verify_pda(
        program_id,
        ctx.position,
        &[
            POSITION_SEED,
            ctx.position_owner.address().as_ref(),
            &pid_bytes,
        ],
    )?;

    // Verify exchange not paused + vault/insurance addresses (H-4)
    {
        let data = ctx.exchange_state.try_borrow()?;
        let exchange = ExchangeState::load(&data)?;
        if exchange.is_paused() {
            return Err(PerpDexError::ExchangePaused.into());
        }
        if *ctx.vault.address().as_array() != exchange.vault {
            return Err(ProgramError::InvalidArgument);
        }
        if *ctx.insurance_fund.address().as_array() != exchange.insurance_fund {
            return Err(ProgramError::InvalidArgument);
        }
    }

    // Read market data
    let cum_funding_long: i64;
    let cum_funding_short: i64;
    {
        let data = ctx.market.try_borrow()?;
        let market = Market::load(&data)?;
        cum_funding_long = market.cumulative_funding_rate_long;
        cum_funding_short = market.cumulative_funding_rate_short;
    }

    // Verify Ed25519-signed oracle price
    verify_signed_price(
        ctx.instructions_sysvar,
        ctx.oracle_state,
        product_id,
        price,
        timestamp,
        0, // Ed25519 instruction at index 0
    )?;
    let oracle_price = price;

    // Read position data
    let pos_product_id: u32;
    let pos_entry_price: u64;
    let pos_size: u64;
    let pos_side: Side;
    let pos_margin: u64;
    let pos_entry_funding_rate: i64;
    let pos_owner_key: [u8; 32];
    {
        let data = ctx.position.try_borrow()?;
        let position = Position::load(&data)?;
        pos_product_id = position.product_id;
        pos_entry_price = position.entry_price;
        pos_size = position.size;
        pos_side = position.side_enum();
        pos_margin = position.margin;
        pos_entry_funding_rate = position.entry_funding_rate;
        pos_owner_key = position.owner;
    }

    if pos_product_id != product_id {
        return Err(PerpDexError::ProductMismatch.into());
    }

    // Verify position_owner matches
    if pos_owner_key != *ctx.position_owner.address().as_array() {
        return Err(PerpDexError::Unauthorized.into());
    }

    // Calculate PnL, funding, check liquidatable
    let pnl = calculate_pnl(pos_entry_price, oracle_price, pos_size, pos_side)?;
    let current_cumulative = match pos_side {
        Side::Long => cum_funding_long,
        Side::Short => cum_funding_short,
    };
    let funding_payment =
        calculate_funding_payment(pos_entry_funding_rate, current_cumulative, pos_size)?;
    let net_pnl = pnl
        .checked_sub(funding_payment)
        .ok_or(PerpDexError::MathOverflow)?;

    let maintenance_margin = pos_size
        .checked_mul(MAINTENANCE_MARGIN_BPS)
        .ok_or(PerpDexError::MathOverflow)?
        .checked_div(BPS_DENOMINATOR)
        .ok_or(PerpDexError::MathOverflow)?;
    let margin_i64: i64 = pos_margin
        .try_into()
        .map_err(|_| PerpDexError::MathOverflow)?;
    let effective_margin = margin_i64
        .checked_add(net_pnl)
        .ok_or(PerpDexError::MathOverflow)?;

    if effective_margin >= maintenance_margin as i64 {
        return Err(PerpDexError::PositionNotLiquidatable.into());
    }

    // Calculate fees
    let remaining_margin: u64 = if effective_margin > 0 {
        effective_margin as u64
    } else {
        0
    };
    let liquidator_fee = remaining_margin
        .checked_mul(LIQUIDATION_FEE_BPS)
        .ok_or(PerpDexError::MathOverflow)?
        .checked_div(BPS_DENOMINATOR)
        .ok_or(PerpDexError::MathOverflow)?;
    let insurance_fee = remaining_margin
        .checked_mul(LIQUIDATION_INSURANCE_BPS)
        .ok_or(PerpDexError::MathOverflow)?
        .checked_div(BPS_DENOMINATOR)
        .ok_or(PerpDexError::MathOverflow)?;

    // Verify liquidator_account.owner == liquidator signer (C-1)
    {
        let data = ctx.liquidator_account.try_borrow()?;
        let la = UserAccount::load(&data)?;
        if la.owner != *ctx.liquidator.address().as_array() {
            return Err(PerpDexError::Unauthorized.into());
        }
    }

    // Verify position_owner_account.owner == position.owner (C-2)
    {
        let data = ctx.position_owner_account.try_borrow()?;
        let poa = UserAccount::load(&data)?;
        if poa.owner != pos_owner_key {
            return Err(PerpDexError::Unauthorized.into());
        }
    }

    // Credit liquidator
    {
        let mut data = ctx.liquidator_account.try_borrow_mut()?;
        let la = UserAccount::load_mut(&mut data)?;
        la.balance = la
            .balance
            .checked_add(liquidator_fee)
            .ok_or(PerpDexError::MathOverflow)?;
    }

    // PDA signer for exchange_state
    let (_, exchange_bump) = find_pda(program_id, &[EXCHANGE_SEED])?;
    let exchange_bump_seed = [exchange_bump];
    let signer_seeds: [Seed; 2] = [Seed::from(EXCHANGE_SEED), Seed::from(&exchange_bump_seed)];

    // Transfer insurance fee: vault -> insurance_fund (best-effort — cap to vault balance
    // so liquidations don't revert when vault is nearly empty)
    let capped_fee = if insurance_fee > 0 {
        let vault_balance = read_token_amount(ctx.vault)?;
        let cf = insurance_fee.min(vault_balance);
        if cf > 0 {
            token_transfer_pda_signed(
                ctx.token_program,
                ctx.vault,
                ctx.insurance_fund,
                ctx.exchange_state,
                cf,
                &signer_seeds,
            )?;
        }
        cf
    } else {
        0
    };

    // Transfer bad debt: insurance_fund -> vault
    let bad_debt_transferred: u64 = if effective_margin < 0 {
        let bad_debt = (-effective_margin) as u64;
        let insurance_balance = read_token_amount(ctx.insurance_fund)?;
        let transfer_amount = bad_debt.min(insurance_balance);
        if transfer_amount > 0 {
            token_transfer_pda_signed(
                ctx.token_program,
                ctx.insurance_fund,
                ctx.vault,
                ctx.exchange_state,
                transfer_amount,
                &signer_seeds,
            )?;
        }
        // L-01: Return actual transfer_amount, not bad_debt, since transfer is capped
        // by insurance token balance.
        transfer_amount
    } else {
        0
    };

    // Update exchange insurance balance and total_deposits
    {
        let mut data = ctx.exchange_state.try_borrow_mut()?;
        let exchange = ExchangeState::load_mut(&mut data)?;
        // L-01: Use capped_fee (actual transferred) instead of insurance_fee,
        // since the vault->insurance transfer is capped by vault balance.
        exchange.insurance_balance = exchange
            .insurance_balance
            .checked_add(capped_fee)
            .ok_or(PerpDexError::MathOverflow)?;
        if bad_debt_transferred > 0 {
            exchange.insurance_balance = exchange
                .insurance_balance
                .saturating_sub(bad_debt_transferred);
        }
        // M-02: Liquidator fee was credited to liquidator's virtual balance
        // without updating total_deposits. This inflates withdrawable claims
        // beyond what the vault accounts for.
        exchange.total_deposits = exchange
            .total_deposits
            .checked_add(liquidator_fee)
            .ok_or(PerpDexError::MathOverflow)?;
    }

    // Update position owner's account
    {
        let actual_realized_pnl = net_pnl
            .checked_sub(liquidator_fee as i64)
            .ok_or(PerpDexError::MathOverflow)?
            .checked_sub(insurance_fee as i64)
            .ok_or(PerpDexError::MathOverflow)?;
        let owner_realized = margin_i64
            .checked_add(actual_realized_pnl)
            .ok_or(PerpDexError::MathOverflow)?
            .min(0);
        let owner_pnl = owner_realized
            .checked_sub(margin_i64)
            .ok_or(PerpDexError::MathOverflow)?;

        let mut data = ctx.position_owner_account.try_borrow_mut()?;
        let poa = UserAccount::load_mut(&mut data)?;
        poa.open_position_count = poa
            .open_position_count
            .checked_sub(1)
            .ok_or(PerpDexError::MathOverflow)?;
        poa.total_realized_pnl = poa
            .total_realized_pnl
            .checked_add(owner_pnl)
            .ok_or(PerpDexError::MathOverflow)?;
    }

    // Update market OI
    {
        let mut data = ctx.market.try_borrow_mut()?;
        let market = Market::load_mut(&mut data)?;
        match pos_side {
            Side::Long => {
                market.long_open_interest = market.long_open_interest.saturating_sub(pos_size);
            }
            Side::Short => {
                market.short_open_interest = market.short_open_interest.saturating_sub(pos_size);
            }
        }
    }

    // Close position account (refund rent to position_owner)
    close_account(ctx.position, ctx.position_owner)?;

    Ok(())
}

// ── 11b. Liquidate (Keeper-only) ────────────────────────────────────────────
//
// Simplified liquidation for the keeper bot. No liquidator user account
// needed — the keeper must be the oracle authority OR exchange admin.
// Liquidation reward goes entirely to the insurance fund.

#[inline(never)]
pub fn process_liquidate_keeper(
    program_id: &Address,
    accounts: &[AccountView],
    product_id: u32,
    price: u64,
    timestamp: i64,
) -> ProgramResult {
    let ctx = LiquidateKeeperAccounts::parse(program_id, accounts)?;

    // SECURITY: Verify market and position PDA addresses
    let pid_bytes = product_id.to_le_bytes();
    verify_pda(program_id, ctx.market, &[MARKET_SEED, &pid_bytes])?;
    verify_pda(
        program_id,
        ctx.position,
        &[
            POSITION_SEED,
            ctx.position_owner.address().as_ref(),
            &pid_bytes,
        ],
    )?;

    // Verify keeper is oracle authority OR exchange admin
    let keeper_key = *ctx.keeper.address().as_array();
    {
        let oracle_data = ctx.oracle_state.try_borrow()?;
        let os = OracleState::load(&oracle_data)?;
        let is_oracle_authority = keeper_key == os.authority;

        let exchange_data = ctx.exchange_state.try_borrow()?;
        let exchange = ExchangeState::load(&exchange_data)?;
        let is_admin = keeper_key == exchange.admin;

        if !is_oracle_authority && !is_admin {
            return Err(PerpDexError::Unauthorized.into());
        }

        if exchange.is_paused() {
            return Err(PerpDexError::ExchangePaused.into());
        }
        if *ctx.vault.address().as_array() != exchange.vault {
            return Err(ProgramError::InvalidArgument);
        }
        if *ctx.insurance_fund.address().as_array() != exchange.insurance_fund {
            return Err(ProgramError::InvalidArgument);
        }
    }

    // Read market data
    let cum_funding_long: i64;
    let cum_funding_short: i64;
    {
        let data = ctx.market.try_borrow()?;
        let market = Market::load(&data)?;
        cum_funding_long = market.cumulative_funding_rate_long;
        cum_funding_short = market.cumulative_funding_rate_short;
    }

    // Verify Ed25519-signed oracle price
    verify_signed_price(
        ctx.instructions_sysvar,
        ctx.oracle_state,
        product_id,
        price,
        timestamp,
        0,
    )?;
    let oracle_price = price;

    // Read position data
    let pos_product_id: u32;
    let pos_entry_price: u64;
    let pos_size: u64;
    let pos_side: Side;
    let pos_margin: u64;
    let pos_entry_funding_rate: i64;
    let pos_owner_key: [u8; 32];
    {
        let data = ctx.position.try_borrow()?;
        let position = Position::load(&data)?;
        pos_product_id = position.product_id;
        pos_entry_price = position.entry_price;
        pos_size = position.size;
        pos_side = position.side_enum();
        pos_margin = position.margin;
        pos_entry_funding_rate = position.entry_funding_rate;
        pos_owner_key = position.owner;
    }

    if pos_product_id != product_id {
        return Err(PerpDexError::ProductMismatch.into());
    }

    // Verify position_owner matches
    if pos_owner_key != *ctx.position_owner.address().as_array() {
        return Err(PerpDexError::Unauthorized.into());
    }

    // Calculate PnL, funding, check liquidatable
    let pnl = calculate_pnl(pos_entry_price, oracle_price, pos_size, pos_side)?;
    let current_cumulative = match pos_side {
        Side::Long => cum_funding_long,
        Side::Short => cum_funding_short,
    };
    let funding_payment =
        calculate_funding_payment(pos_entry_funding_rate, current_cumulative, pos_size)?;
    let net_pnl = pnl
        .checked_sub(funding_payment)
        .ok_or(PerpDexError::MathOverflow)?;

    let maintenance_margin = pos_size
        .checked_mul(MAINTENANCE_MARGIN_BPS)
        .ok_or(PerpDexError::MathOverflow)?
        .checked_div(BPS_DENOMINATOR)
        .ok_or(PerpDexError::MathOverflow)?;
    let margin_i64: i64 = pos_margin
        .try_into()
        .map_err(|_| PerpDexError::MathOverflow)?;
    let effective_margin = margin_i64
        .checked_add(net_pnl)
        .ok_or(PerpDexError::MathOverflow)?;

    if effective_margin >= maintenance_margin as i64 {
        return Err(PerpDexError::PositionNotLiquidatable.into());
    }

    // Calculate fees — all remaining margin goes to insurance fund
    let remaining_margin: u64 = if effective_margin > 0 {
        effective_margin as u64
    } else {
        0
    };
    // Keeper liquidation: entire remaining margin goes to insurance (no liquidator fee)
    let insurance_fee = remaining_margin;

    // PDA signer for exchange_state
    let (_, exchange_bump) = find_pda(program_id, &[EXCHANGE_SEED])?;
    let exchange_bump_seed = [exchange_bump];
    let signer_seeds: [Seed; 2] = [Seed::from(EXCHANGE_SEED), Seed::from(&exchange_bump_seed)];

    // Transfer insurance fee: vault -> insurance_fund (best-effort — cap to vault balance
    // so liquidations don't revert when vault is nearly empty)
    let capped_fee = if insurance_fee > 0 {
        let vault_balance = read_token_amount(ctx.vault)?;
        let cf = insurance_fee.min(vault_balance);
        if cf > 0 {
            token_transfer_pda_signed(
                ctx.token_program,
                ctx.vault,
                ctx.insurance_fund,
                ctx.exchange_state,
                cf,
                &signer_seeds,
            )?;
        }
        cf
    } else {
        0
    };

    // Transfer bad debt: insurance_fund -> vault
    let bad_debt_transferred: u64 = if effective_margin < 0 {
        let bad_debt = (-effective_margin) as u64;
        let insurance_balance = read_token_amount(ctx.insurance_fund)?;
        let transfer_amount = bad_debt.min(insurance_balance);
        if transfer_amount > 0 {
            token_transfer_pda_signed(
                ctx.token_program,
                ctx.insurance_fund,
                ctx.vault,
                ctx.exchange_state,
                transfer_amount,
                &signer_seeds,
            )?;
        }
        // L-01: Return actual transfer_amount, not bad_debt, since transfer is capped
        // by insurance token balance.
        transfer_amount
    } else {
        0
    };

    // Update exchange insurance balance
    {
        let mut data = ctx.exchange_state.try_borrow_mut()?;
        let exchange = ExchangeState::load_mut(&mut data)?;
        // L-01: Use capped_fee (actual transferred) instead of insurance_fee,
        // since the vault->insurance transfer is capped by vault balance.
        exchange.insurance_balance = exchange
            .insurance_balance
            .checked_add(capped_fee)
            .ok_or(PerpDexError::MathOverflow)?;
        if bad_debt_transferred > 0 {
            exchange.insurance_balance = exchange
                .insurance_balance
                .saturating_sub(bad_debt_transferred);
        }
    }

    // Verify position_owner_account.owner == position.owner (BEFORE mutation)
    {
        let data = ctx.position_owner_account.try_borrow()?;
        let poa = UserAccount::load(&data)?;
        if poa.owner != pos_owner_key {
            return Err(PerpDexError::Unauthorized.into());
        }
    }

    // Update position owner's account
    {
        let actual_realized_pnl = net_pnl
            .checked_sub(insurance_fee as i64)
            .ok_or(PerpDexError::MathOverflow)?;
        let owner_realized = margin_i64
            .checked_add(actual_realized_pnl)
            .ok_or(PerpDexError::MathOverflow)?
            .min(0);
        let owner_pnl = owner_realized
            .checked_sub(margin_i64)
            .ok_or(PerpDexError::MathOverflow)?;

        let mut data = ctx.position_owner_account.try_borrow_mut()?;
        let poa = UserAccount::load_mut(&mut data)?;
        poa.open_position_count = poa
            .open_position_count
            .checked_sub(1)
            .ok_or(PerpDexError::MathOverflow)?;
        poa.total_realized_pnl = poa
            .total_realized_pnl
            .checked_add(owner_pnl)
            .ok_or(PerpDexError::MathOverflow)?;
    }

    // Update market OI
    {
        let mut data = ctx.market.try_borrow_mut()?;
        let market = Market::load_mut(&mut data)?;
        match pos_side {
            Side::Long => {
                market.long_open_interest = market.long_open_interest.saturating_sub(pos_size);
            }
            Side::Short => {
                market.short_open_interest = market.short_open_interest.saturating_sub(pos_size);
            }
        }
    }

    // Close position account (refund rent to position_owner)
    close_account(ctx.position, ctx.position_owner)?;

    Ok(())
}

// ── 12. Update Funding Rate ─────────────────────────────────────────────────

#[inline(never)]
pub fn process_update_funding_rate(
    program_id: &Address,
    accounts: &[AccountView],
    product_id: u32,
) -> ProgramResult {
    let ctx = UpdateFundingRateAccounts::parse(program_id, accounts)?;

    // SECURITY: Verify market PDA address
    let pid_bytes = product_id.to_le_bytes();
    verify_pda(program_id, ctx.market, &[MARKET_SEED, &pid_bytes])?;

    let clock = Clock::get()?;

    // Read, validate, and update market
    {
        let mut data = ctx.market.try_borrow_mut()?;
        let market = Market::load_mut(&mut data)?;

        if market.product_id != product_id {
            return Err(PerpDexError::ProductMismatch.into());
        }
        if market.active == 0 {
            return Err(PerpDexError::MarketInactive.into());
        }

        let time_elapsed = clock
            .unix_timestamp
            .checked_sub(market.last_funding_time)
            .ok_or(PerpDexError::MathOverflow)?;
        if time_elapsed < FUNDING_INTERVAL {
            return Err(PerpDexError::FundingTooEarly.into());
        }

        let (long_oi, short_oi) = (market.long_open_interest, market.short_open_interest);
        let total_oi = long_oi
            .checked_add(short_oi)
            .ok_or(PerpDexError::MathOverflow)?;

        // M-3: Scale imbalance by BPS_DENOMINATOR before dividing by total_oi
        // to prevent truncation to 0 for moderate OI imbalances.
        let funding_rate: i64 = if total_oi == 0 {
            0
        } else {
            let imbalance = (long_oi as i128)
                .checked_sub(short_oi as i128)
                .ok_or(PerpDexError::MathOverflow)?;
            let rate = imbalance
                .checked_mul(BPS_DENOMINATOR as i128)
                .ok_or(PerpDexError::MathOverflow)?
                .checked_mul(FUNDING_RATE_CAP_BPS as i128)
                .ok_or(PerpDexError::MathOverflow)?
                .checked_div(total_oi as i128)
                .ok_or(PerpDexError::MathOverflow)?
                .checked_div(BPS_DENOMINATOR as i128)
                .ok_or(PerpDexError::MathOverflow)?;
            rate.max(-FUNDING_RATE_CAP_BPS as i128)
                .min(FUNDING_RATE_CAP_BPS as i128) as i64
        };

        market.cumulative_funding_rate_long = market
            .cumulative_funding_rate_long
            .checked_add(funding_rate)
            .ok_or(PerpDexError::MathOverflow)?;
        market.cumulative_funding_rate_short = market
            .cumulative_funding_rate_short
            .checked_sub(funding_rate)
            .ok_or(PerpDexError::MathOverflow)?;
        market.last_funding_time = clock.unix_timestamp;
    }

    Ok(())
}

// ── 13. Set Paused ──────────────────────────────────────────────────────────

#[inline(never)]
pub fn process_set_paused(
    _program_id: &Address,
    accounts: &[AccountView],
    paused: bool,
) -> ProgramResult {
    let ctx = AdminOnlyAccounts::parse(_program_id, accounts)?;

    let mut data = ctx.exchange_state.try_borrow_mut()?;
    let exchange = ExchangeState::load_mut(&mut data)?;
    if exchange.admin != *ctx.admin.address().as_array() {
        return Err(PerpDexError::Unauthorized.into());
    }
    exchange.paused = if paused { 1 } else { 0 };

    Ok(())
}

// ── 14. Set Market Active ───────────────────────────────────────────────────

#[inline(never)]
pub fn process_set_market_active(
    program_id: &Address,
    accounts: &[AccountView],
    product_id: u32,
    active: bool,
) -> ProgramResult {
    let ctx = AdminMarketAccounts::parse(program_id, accounts)?;

    // SECURITY: Verify market PDA address
    let pid_bytes = product_id.to_le_bytes();
    verify_pda(program_id, ctx.market, &[MARKET_SEED, &pid_bytes])?;

    // Verify has_one = admin
    {
        let data = ctx.exchange_state.try_borrow()?;
        let exchange = ExchangeState::load(&data)?;
        if exchange.admin != *ctx.admin.address().as_array() {
            return Err(PerpDexError::Unauthorized.into());
        }
    }

    // Update market
    {
        let mut data = ctx.market.try_borrow_mut()?;
        let market = Market::load_mut(&mut data)?;
        if market.product_id != product_id {
            return Err(PerpDexError::ProductMismatch.into());
        }
        market.active = if active { 1 } else { 0 };
    }

    Ok(())
}

// ── 15. Update Market Params ────────────────────────────────────────────────

#[inline(never)]
pub fn process_update_market_params(
    program_id: &Address,
    accounts: &[AccountView],
    product_id: u32,
    max_leverage: u32,
    max_open_interest: u64,
) -> ProgramResult {
    let ctx = AdminMarketAccounts::parse(program_id, accounts)?;

    // SECURITY: Verify market PDA address
    let pid_bytes = product_id.to_le_bytes();
    verify_pda(program_id, ctx.market, &[MARKET_SEED, &pid_bytes])?;

    // Verify has_one = admin
    {
        let data = ctx.exchange_state.try_borrow()?;
        let exchange = ExchangeState::load(&data)?;
        if exchange.admin != *ctx.admin.address().as_array() {
            return Err(PerpDexError::Unauthorized.into());
        }
    }

    if max_leverage == 0 || max_leverage > MAX_LEVERAGE {
        return Err(PerpDexError::InvalidLeverage.into());
    }

    // Update market
    {
        let mut data = ctx.market.try_borrow_mut()?;
        let market = Market::load_mut(&mut data)?;
        if market.product_id != product_id {
            return Err(PerpDexError::ProductMismatch.into());
        }
        market.max_leverage = max_leverage;
        market.max_open_interest = max_open_interest;
    }

    Ok(())
}

// ── 16. Fund Insurance ──────────────────────────────────────────────────────

#[inline(never)]
pub fn process_fund_insurance(
    _program_id: &Address,
    accounts: &[AccountView],
    amount: u64,
) -> ProgramResult {
    let ctx = FundInsuranceAccounts::parse(_program_id, accounts)?;

    if amount == 0 {
        return Err(PerpDexError::InvalidAmount.into());
    }

    // Verify has_one = admin, token accounts, and insurance_fund address
    {
        let data = ctx.exchange_state.try_borrow()?;
        let exchange = ExchangeState::load(&data)?;
        if exchange.admin != *ctx.admin.address().as_array() {
            return Err(PerpDexError::Unauthorized.into());
        }
        if *ctx.insurance_fund.address().as_array() != exchange.insurance_fund {
            return Err(ProgramError::InvalidArgument);
        }

        // Verify admin_token_account owner and mint
        let token_owner = read_token_owner(ctx.admin_token_account)?;
        if token_owner != *ctx.admin.address().as_array() {
            return Err(PerpDexError::Unauthorized.into());
        }
        let token_mint = read_token_mint(ctx.admin_token_account)?;
        if token_mint != exchange.usdc_mint {
            return Err(PerpDexError::InvalidAmount.into());
        }
    }

    // Transfer tokens: admin -> insurance_fund
    token_transfer_user_signed(
        ctx.token_program,
        ctx.admin_token_account,
        ctx.insurance_fund,
        ctx.admin,
        amount,
    )?;

    // Update exchange insurance_balance
    {
        let mut data = ctx.exchange_state.try_borrow_mut()?;
        let exchange = ExchangeState::load_mut(&mut data)?;
        exchange.insurance_balance = exchange
            .insurance_balance
            .checked_add(amount)
            .ok_or(PerpDexError::MathOverflow)?;
    }

    Ok(())
}

// ── 17. Transfer Admin ──────────────────────────────────────────────────────

#[inline(never)]
pub fn process_transfer_admin(
    _program_id: &Address,
    accounts: &[AccountView],
    new_admin: [u8; 32],
) -> ProgramResult {
    let ctx = AdminOnlyAccounts::parse(_program_id, accounts)?;

    if is_zero_pubkey(&new_admin) {
        return Err(PerpDexError::Unauthorized.into());
    }

    let mut data = ctx.exchange_state.try_borrow_mut()?;
    let exchange = ExchangeState::load_mut(&mut data)?;
    if exchange.admin != *ctx.admin.address().as_array() {
        return Err(PerpDexError::Unauthorized.into());
    }
    exchange.pending_admin = new_admin;

    Ok(())
}

// ── 18. Accept Admin ────────────────────────────────────────────────────────

#[inline(never)]
pub fn process_accept_admin(_program_id: &Address, accounts: &[AccountView]) -> ProgramResult {
    let ctx = AcceptAdminAccounts::parse(_program_id, accounts)?;

    let mut data = ctx.exchange_state.try_borrow_mut()?;
    let exchange = ExchangeState::load_mut(&mut data)?;
    if exchange.pending_admin != *ctx.new_admin.address().as_array() {
        return Err(PerpDexError::Unauthorized.into());
    }
    if is_zero_pubkey(&exchange.pending_admin) {
        return Err(PerpDexError::Unauthorized.into());
    }
    exchange.admin = exchange.pending_admin;
    exchange.pending_admin = [0u8; 32];

    Ok(())
}

// ── 19. Withdraw Fees ───────────────────────────────────────────────────────

/// Withdraw accumulated protocol fees to the treasury wallet.
/// Only admin can call this, and funds go to the hardcoded treasury address.
#[inline(never)]
pub fn process_withdraw_fees(
    program_id: &Address,
    accounts: &[AccountView],
    amount: u64,
) -> ProgramResult {
    let ctx = WithdrawFeesAccounts::parse(program_id, accounts)?;

    if amount == 0 {
        return Err(PerpDexError::InvalidAmount.into());
    }

    // Verify has_one = admin and vault address
    {
        let data = ctx.exchange_state.try_borrow()?;
        let exchange = ExchangeState::load(&data)?;
        if exchange.admin != *ctx.admin.address().as_array() {
            return Err(PerpDexError::Unauthorized.into());
        }
        if *ctx.vault.address().as_array() != exchange.vault {
            return Err(ProgramError::InvalidArgument);
        }
    }

    // SECURITY: Verify treasury_token_account is owned by the hardcoded treasury wallet
    // This prevents admin from redirecting fees to arbitrary addresses
    {
        let token_owner = read_token_owner(ctx.treasury_token_account)?;
        if token_owner != TREASURY_WALLET {
            return Err(PerpDexError::Unauthorized.into());
        }
    }

    // Verify amount doesn't exceed collected fees
    {
        let data = ctx.exchange_state.try_borrow()?;
        let exchange = ExchangeState::load(&data)?;
        if amount > exchange.total_fees_collected {
            return Err(PerpDexError::InsufficientBalance.into());
        }
    }

    // Verify vault has enough tokens (prevent overdrowing user deposits)
    {
        let vault_balance = read_token_amount(ctx.vault)?;
        let exchange_data = ctx.exchange_state.try_borrow()?;
        let exchange = ExchangeState::load(&exchange_data)?;
        // Vault must retain at least total_deposits after withdrawal
        if vault_balance.saturating_sub(amount) < exchange.total_deposits {
            return Err(PerpDexError::InsufficientBalance.into());
        }
    }

    // Transfer tokens: vault -> treasury (PDA-signed by exchange_state)
    let (_, exchange_bump) = find_pda(program_id, &[EXCHANGE_SEED])?;
    let exchange_bump_seed = [exchange_bump];
    let signer_seeds: [Seed; 2] = [Seed::from(EXCHANGE_SEED), Seed::from(&exchange_bump_seed)];

    token_transfer_pda_signed(
        ctx.token_program,
        ctx.vault,
        ctx.treasury_token_account,
        ctx.exchange_state,
        amount,
        &signer_seeds,
    )?;

    // Update exchange total_fees_collected (track withdrawn fees)
    {
        let mut data = ctx.exchange_state.try_borrow_mut()?;
        let exchange = ExchangeState::load_mut(&mut data)?;
        exchange.total_fees_collected = exchange.total_fees_collected.saturating_sub(amount);
    }

    Ok(())
}

// ── Close Account Helper ────────────────────────────────────────────────────

/// Close an account: transfer all lamports to the destination and zero the data.
fn close_account(account: &AccountView, destination: &AccountView) -> ProgramResult {
    let account_lamports = account.lamports();
    destination.set_lamports(destination.lamports().saturating_add(account_lamports));
    account.set_lamports(0);
    account.close()
}
