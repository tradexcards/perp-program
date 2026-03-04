use pinocchio::{
    account::AccountView,
    cpi::Signer,
    error::ProgramError,
    sysvars::{clock::Clock, instructions::Instructions, Sysvar},
    Address, ProgramResult,
};
use pinocchio_system::instructions::{Allocate, Assign, CreateAccount, Transfer};

use crate::constants::*;
use crate::error::PerpDexError;
use crate::state::OracleState;

#[allow(unused_imports)]
use crate::state::Side;

// ── PDA utilities ────────────────────────────────────────────────────────────

/// Verify that an account address matches an expected PDA. Returns the bump.
pub fn verify_pda(
    program_id: &Address,
    account: &AccountView,
    seeds: &[&[u8]],
) -> Result<u8, ProgramError> {
    // Try bumps from 255 down to 0
    for bump in (0u8..=255).rev() {
        let bump_seed = [bump];
        let mut all_seeds: [&[u8]; 5] = [&[], &[], &[], &[], &[]];
        let seed_count = seeds.len();
        for (i, s) in seeds.iter().enumerate() {
            all_seeds[i] = s;
        }
        all_seeds[seed_count] = &bump_seed;

        let pda = match seed_count {
            1 => Address::create_program_address(&[all_seeds[0], all_seeds[1]], program_id),
            2 => Address::create_program_address(
                &[all_seeds[0], all_seeds[1], all_seeds[2]],
                program_id,
            ),
            3 => Address::create_program_address(
                &[all_seeds[0], all_seeds[1], all_seeds[2], all_seeds[3]],
                program_id,
            ),
            _ => return Err(ProgramError::InvalidArgument),
        };

        if let Ok(addr) = pda {
            if addr == *account.address() {
                return Ok(bump);
            }
        }
    }
    Err(ProgramError::InvalidArgument)
}

/// Find PDA address and bump.
pub fn find_pda(program_id: &Address, seeds: &[&[u8]]) -> Result<(Address, u8), ProgramError> {
    for bump in (0u8..=255).rev() {
        let bump_seed = [bump];
        let mut all_seeds: [&[u8]; 5] = [&[], &[], &[], &[], &[]];
        let seed_count = seeds.len();
        for (i, s) in seeds.iter().enumerate() {
            all_seeds[i] = s;
        }
        all_seeds[seed_count] = &bump_seed;

        let pda = match seed_count {
            1 => Address::create_program_address(&[all_seeds[0], all_seeds[1]], program_id),
            2 => Address::create_program_address(
                &[all_seeds[0], all_seeds[1], all_seeds[2]],
                program_id,
            ),
            3 => Address::create_program_address(
                &[all_seeds[0], all_seeds[1], all_seeds[2], all_seeds[3]],
                program_id,
            ),
            _ => return Err(ProgramError::InvalidArgument),
        };

        if let Ok(addr) = pda {
            return Ok((addr, bump));
        }
    }
    Err(ProgramError::InvalidArgument)
}

/// M-03: Create a PDA account, resilient to prefunding DoS attacks.
/// If the target account already has lamports (prefunded by attacker),
/// falls back to Allocate + Assign + Transfer instead of CreateAccount,
/// which would fail on a prefunded account.
pub fn create_pda_account<'a>(
    from: &'a AccountView,
    to: &'a AccountView,
    lamports: u64,
    space: u64,
    owner: &Address,
    signers: &[Signer],
) -> ProgramResult {
    let existing = to.lamports();
    if existing > 0 {
        // Account was prefunded — CreateAccount would fail, use fallback
        Allocate { account: to, space }.invoke_signed(signers)?;
        Assign { account: to, owner }.invoke_signed(signers)?;
        let needed = lamports.saturating_sub(existing);
        if needed > 0 {
            Transfer {
                from,
                to,
                lamports: needed,
            }
            .invoke()?;
        }
    } else {
        CreateAccount {
            from,
            to,
            lamports,
            space,
            owner,
        }
        .invoke_signed(signers)?;
    }
    Ok(())
}

// ── Oracle helpers ───────────────────────────────────────────────────────────

/// Verify an Ed25519-signed oracle price from the Instructions sysvar.
///
/// Scans all instructions in the transaction for an Ed25519SigVerify precompile
/// instruction that was already verified by the runtime.  This is robust to
/// wallet injected instructions (e.g. Phantom inserts ComputeBudget ixs at the
/// front of the transaction, shifting instruction indices).
///
/// Signed message format (20 bytes):
///   [product_id: u32 LE][price: u64 LE][timestamp: i64 LE]
///
/// The caller passes `price`, `timestamp`, and `product_id` from instruction
/// data.  We reconstruct the expected message and compare it against the one
/// in the Ed25519 precompile instruction.
#[inline(never)]
pub fn verify_signed_price(
    instructions_sysvar: &AccountView,
    oracle_state: &AccountView,
    product_id: u32,
    price: u64,
    timestamp: i64,
    _ed25519_ix_index: usize, // ignored — we scan for it
) -> Result<(), ProgramError> {
    if price == 0 {
        return Err(PerpDexError::InvalidOraclePrice.into());
    }

    // Check staleness against on-chain clock
    let clock = Clock::get()?;
    let age = clock
        .unix_timestamp
        .checked_sub(timestamp)
        .ok_or(PerpDexError::MathOverflow)?;
    if age > MAX_ORACLE_PRICE_AGE || age < -30 {
        // Allow up to 30s in the future for clock skew
        return Err(PerpDexError::OraclePriceStale.into());
    }

    // Load the oracle authority from OracleState
    let expected_pubkey: [u8; 32] = {
        let data = oracle_state.try_borrow()?;
        let os = OracleState::load(&data)?;
        os.authority
    };

    // Reconstruct the signed message
    let mut expected_msg = [0u8; SIGNED_PRICE_MSG_LEN];
    expected_msg[0..4].copy_from_slice(&product_id.to_le_bytes());
    expected_msg[4..12].copy_from_slice(&price.to_le_bytes());
    expected_msg[12..20].copy_from_slice(&timestamp.to_le_bytes());

    // Parse the Instructions sysvar and scan for Ed25519 precompile
    let instructions = Instructions::try_from(instructions_sysvar)?;

    // Scan up to 8 instructions (ComputeBudget + Ed25519 + our ix, typically 3-4)
    let max_scan: usize = 8;
    let mut found = false;

    for idx in 0..max_scan {
        let ix = match instructions.load_instruction_at(idx) {
            Ok(ix) => ix,
            Err(_) => break, // no more instructions
        };

        // Skip non-Ed25519 instructions (e.g. ComputeBudget)
        if ix.get_program_id().as_ref() != &ED25519_PROGRAM_ID {
            continue;
        }

        let ix_data = ix.get_instruction_data();

        // Minimum size: 2 (header) + 14 (offsets) = 16
        if ix_data.len() < 16 {
            continue;
        }

        // num_signatures must be >= 1
        if ix_data[0] < 1 {
            continue;
        }

        // Parse Ed25519SignatureOffsets (14 bytes starting at offset 2)
        // Layout: sig_offset(u16), sig_ix_idx(u16), pk_offset(u16), pk_ix_idx(u16),
        //         msg_offset(u16), msg_size(u16), msg_ix_idx(u16)
        let offsets = &ix_data[2..16];
        let pk_offset = u16::from_le_bytes([offsets[4], offsets[5]]) as usize;
        let pk_ix_idx = u16::from_le_bytes([offsets[6], offsets[7]]);
        let msg_offset = u16::from_le_bytes([offsets[8], offsets[9]]) as usize;
        let msg_size = u16::from_le_bytes([offsets[10], offsets[11]]) as usize;
        let msg_ix_idx = u16::from_le_bytes([offsets[12], offsets[13]]);

        // Both pubkey and message must reference this same instruction (0xFFFF)
        if pk_ix_idx != 0xFFFF || msg_ix_idx != 0xFFFF {
            continue;
        }

        // Verify pubkey matches oracle authority
        if pk_offset + 32 > ix_data.len() {
            continue;
        }
        if &ix_data[pk_offset..pk_offset + 32] != &expected_pubkey {
            // Wrong authority — skip (could be a different Ed25519 ix)
            continue;
        }

        // Verify message matches our expected signed price message
        if msg_size != SIGNED_PRICE_MSG_LEN {
            continue;
        }
        if msg_offset + msg_size > ix_data.len() {
            continue;
        }
        if &ix_data[msg_offset..msg_offset + msg_size] != &expected_msg {
            return Err(PerpDexError::InvalidSignedPrice.into());
        }

        // All checks passed — Ed25519 instruction is valid
        found = true;
        break;
    }

    if !found {
        return Err(PerpDexError::InvalidEd25519Instruction.into());
    }

    Ok(())
}

// ── Math helpers ─────────────────────────────────────────────────────────────

/// Calculate PnL for a position.
pub fn calculate_pnl(
    entry_price: u64,
    current_price: u64,
    size: u64,
    side: Side,
) -> Result<i64, ProgramError> {
    let (entry, current, sz) = (entry_price as i128, current_price as i128, size as i128);
    let price_diff = match side {
        Side::Long => current
            .checked_sub(entry)
            .ok_or(PerpDexError::MathOverflow)?,
        Side::Short => entry
            .checked_sub(current)
            .ok_or(PerpDexError::MathOverflow)?,
    };
    // M-2: Improved precision — scale up before dividing to avoid truncation
    // for small positions on expensive assets (e.g., $1 position on $500 card).
    let pnl = sz
        .checked_mul(price_diff)
        .ok_or(PerpDexError::MathOverflow)?
        .checked_mul(crate::constants::PRICE_SCALE as i128)
        .ok_or(PerpDexError::MathOverflow)?
        .checked_div(entry)
        .ok_or(PerpDexError::MathOverflow)?
        .checked_div(crate::constants::PRICE_SCALE as i128)
        .ok_or(PerpDexError::MathOverflow)?;
    if pnl < i64::MIN as i128 || pnl > i64::MAX as i128 {
        return Err(PerpDexError::MathOverflow.into());
    }
    Ok(pnl as i64)
}

/// Calculate liquidation price from leverage (for new positions).
pub fn calculate_liquidation_price(
    entry_price: u64,
    leverage: u32,
    side: Side,
) -> Result<u64, ProgramError> {
    let (entry, lev, mm, bps) = (
        entry_price as u128,
        leverage as u128,
        MAINTENANCE_MARGIN_BPS as u128,
        BPS_DENOMINATOR as u128,
    );
    match side {
        Side::Long => {
            let num = lev
                .checked_mul(bps)
                .ok_or(PerpDexError::MathOverflow)?
                .checked_sub(bps)
                .ok_or(PerpDexError::MathOverflow)?
                .checked_add(mm.checked_mul(lev).ok_or(PerpDexError::MathOverflow)?)
                .ok_or(PerpDexError::MathOverflow)?;
            let den = lev.checked_mul(bps).ok_or(PerpDexError::MathOverflow)?;
            Ok(entry
                .checked_mul(num)
                .ok_or(PerpDexError::MathOverflow)?
                .checked_div(den)
                .ok_or(PerpDexError::MathOverflow)? as u64)
        }
        Side::Short => {
            let num = lev
                .checked_mul(bps)
                .ok_or(PerpDexError::MathOverflow)?
                .checked_add(bps)
                .ok_or(PerpDexError::MathOverflow)?
                .checked_sub(mm.checked_mul(lev).ok_or(PerpDexError::MathOverflow)?)
                .ok_or(PerpDexError::MathOverflow)?;
            let den = lev.checked_mul(bps).ok_or(PerpDexError::MathOverflow)?;
            Ok(entry
                .checked_mul(num)
                .ok_or(PerpDexError::MathOverflow)?
                .checked_div(den)
                .ok_or(PerpDexError::MathOverflow)? as u64)
        }
    }
}

/// Calculate liquidation price from absolute margin (for add_margin).
pub fn calculate_liquidation_price_from_margin(
    entry_price: u64,
    margin: u64,
    size: u64,
    side: Side,
) -> Result<u64, ProgramError> {
    let (entry, m, sz, mm, bps) = (
        entry_price as u128,
        margin as u128,
        size as u128,
        MAINTENANCE_MARGIN_BPS as u128,
        BPS_DENOMINATOR as u128,
    );
    let maintenance = sz
        .checked_mul(mm)
        .ok_or(PerpDexError::MathOverflow)?
        .checked_div(bps)
        .ok_or(PerpDexError::MathOverflow)?;
    let max_loss = m.saturating_sub(maintenance);
    match side {
        Side::Long => {
            let num = sz.saturating_sub(max_loss);
            Ok(entry
                .checked_mul(num)
                .ok_or(PerpDexError::MathOverflow)?
                .checked_div(sz)
                .ok_or(PerpDexError::MathOverflow)? as u64)
        }
        Side::Short => {
            let num = sz.checked_add(max_loss).ok_or(PerpDexError::MathOverflow)?;
            Ok(entry
                .checked_mul(num)
                .ok_or(PerpDexError::MathOverflow)?
                .checked_div(sz)
                .ok_or(PerpDexError::MathOverflow)? as u64)
        }
    }
}

/// Calculate funding payment owed.
pub fn calculate_funding_payment(
    entry_funding_rate: i64,
    current_cumulative_rate: i64,
    size: u64,
) -> Result<i64, ProgramError> {
    let rate_diff = current_cumulative_rate
        .checked_sub(entry_funding_rate)
        .ok_or(PerpDexError::MathOverflow)?;
    let payment = (size as i128)
        .checked_mul(rate_diff as i128)
        .ok_or(PerpDexError::MathOverflow)?
        .checked_div(BPS_DENOMINATOR as i128)
        .ok_or(PerpDexError::MathOverflow)?;
    if payment < i64::MIN as i128 || payment > i64::MAX as i128 {
        return Err(PerpDexError::MathOverflow.into());
    }
    Ok(payment as i64)
}

// ── Utility ──────────────────────────────────────────────────────────────────

/// Read a u32 from little-endian bytes.
pub fn read_u32(data: &[u8]) -> Result<u32, ProgramError> {
    if data.len() < 4 {
        return Err(ProgramError::InvalidInstructionData);
    }
    Ok(u32::from_le_bytes(
        data[..4]
            .try_into()
            .map_err(|_| ProgramError::InvalidInstructionData)?,
    ))
}

/// Read a u64 from little-endian bytes.
pub fn read_u64(data: &[u8]) -> Result<u64, ProgramError> {
    if data.len() < 8 {
        return Err(ProgramError::InvalidInstructionData);
    }
    Ok(u64::from_le_bytes(
        data[..8]
            .try_into()
            .map_err(|_| ProgramError::InvalidInstructionData)?,
    ))
}

/// Read an i64 from little-endian bytes.
pub fn read_i64(data: &[u8]) -> Result<i64, ProgramError> {
    if data.len() < 8 {
        return Err(ProgramError::InvalidInstructionData);
    }
    Ok(i64::from_le_bytes(
        data[..8]
            .try_into()
            .map_err(|_| ProgramError::InvalidInstructionData)?,
    ))
}

/// Read a Pubkey (32 bytes) from data.
pub fn read_pubkey(data: &[u8]) -> Result<[u8; 32], ProgramError> {
    if data.len() < 32 {
        return Err(ProgramError::InvalidInstructionData);
    }
    let mut buf = [0u8; 32];
    buf.copy_from_slice(&data[..32]);
    Ok(buf)
}

/// Check if 32-byte array is all zeros (Pubkey::default()).
pub fn is_zero_pubkey(key: &[u8; 32]) -> bool {
    key.iter().all(|&b| b == 0)
}
