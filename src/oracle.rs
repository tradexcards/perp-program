use pinocchio::cpi::Seed;
use pinocchio::{account::AccountView, cpi::Signer, error::ProgramError, Address, ProgramResult};
// M-03: PDA creation uses create_pda_account from helpers (resilient to prefunding DoS)

use crate::constants::*;
use crate::error::PerpDexError;
use crate::helpers::*;
use crate::state::*;

// ── Rent helper ──────────────────────────────────────────────────────────────

/// Approximate rent-exempt minimum: 890_880 + 6960 * account_size.
#[inline]
pub fn rent_exempt_minimum(account_size: usize) -> u64 {
    890_880u64 + (account_size as u64) * 6960
}

// ══════════════════════════════════════════════════════════════════════════════
// Account context structs
// ══════════════════════════════════════════════════════════════════════════════

// InitializeOracle: oracle_state, authority, system_program
instruction_accounts! {
    pub struct InitializeOracleAccounts<'a> {
        pub oracle_state:   init_pda,
        pub authority:      signer_mut,
        pub system_program: program,
    }
}

// TransferAuthority: oracle_state, authority
instruction_accounts! {
    pub struct ManageOracleAccounts<'a> {
        pub oracle_state:   pda_mut(ORACLE_SEED),
        pub authority:      signer,
    }
}

// AcceptAuthority: oracle_state, new_authority
instruction_accounts! {
    pub struct AcceptAuthorityAccounts<'a> {
        pub oracle_state:   pda_mut(ORACLE_SEED),
        pub new_authority:  signer,
    }
}

// ══════════════════════════════════════════════════════════════════════════════
// Instruction handlers
// ══════════════════════════════════════════════════════════════════════════════

// ── 1. Initialize Oracle ─────────────────────────────────────────────────────

#[inline(never)]
pub fn process_initialize_oracle(program_id: &Address, accounts: &[AccountView]) -> ProgramResult {
    let ctx = InitializeOracleAccounts::parse(program_id, accounts)?;

    // ── Create oracle_state PDA ──────────────────────────────────────────
    let (oracle_pda, oracle_bump) = find_pda(program_id, &[ORACLE_SEED])?;
    if oracle_pda != *ctx.oracle_state.address() {
        return Err(ProgramError::InvalidArgument);
    }

    let oracle_bump_seed = [oracle_bump];
    let oracle_seeds: [Seed; 2] = [Seed::from(ORACLE_SEED), Seed::from(&oracle_bump_seed)];
    let oracle_signer = Signer::from(&oracle_seeds);

    create_pda_account(
        ctx.authority,
        ctx.oracle_state,
        rent_exempt_minimum(OracleState::ACCOUNT_SIZE),
        OracleState::ACCOUNT_SIZE as u64,
        program_id,
        &[oracle_signer],
    )?;

    // ── Init oracle_state ────────────────────────────────────────────────
    {
        let mut data = ctx.oracle_state.try_borrow_mut()?;
        let oracle_state = OracleState::init(&mut data)?;
        oracle_state.authority = *ctx.authority.address().as_array();
    }

    Ok(())
}

// ── 2. Transfer Authority ────────────────────────────────────────────────────

#[inline(never)]
pub fn process_transfer_authority(
    program_id: &Address,
    accounts: &[AccountView],
    new_authority: [u8; 32],
) -> ProgramResult {
    let ctx = ManageOracleAccounts::parse(program_id, accounts)?;

    if is_zero_pubkey(&new_authority) {
        return Err(PerpDexError::OracleUnauthorized.into());
    }

    {
        let mut data = ctx.oracle_state.try_borrow_mut()?;
        let oracle_state = OracleState::load_mut(&mut data)?;
        if oracle_state.authority != *ctx.authority.address().as_array() {
            return Err(PerpDexError::OracleUnauthorized.into());
        }
        oracle_state.pending_authority = new_authority;
    }

    Ok(())
}

// ── 3. Accept Authority ──────────────────────────────────────────────────────

#[inline(never)]
pub fn process_accept_authority(program_id: &Address, accounts: &[AccountView]) -> ProgramResult {
    let ctx = AcceptAuthorityAccounts::parse(program_id, accounts)?;

    let new_authority_key = *ctx.new_authority.address().as_array();

    {
        let mut data = ctx.oracle_state.try_borrow_mut()?;
        let oracle_state = OracleState::load_mut(&mut data)?;
        if is_zero_pubkey(&oracle_state.pending_authority) {
            return Err(PerpDexError::OracleUnauthorized.into());
        }
        if oracle_state.pending_authority != new_authority_key {
            return Err(PerpDexError::OracleUnauthorized.into());
        }
        oracle_state.authority = new_authority_key;
        oracle_state.pending_authority = [0u8; 32];
    }

    Ok(())
}
