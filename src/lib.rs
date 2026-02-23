//! Pinocchio entrypoint + instruction dispatcher.
//!
//! Instruction format (Anchor-compatible):
//!   [8-byte discriminator][...instruction data]
//!
//! Discriminators are SHA-256("global:<snake_case_fn_name>")[0..8].

use pinocchio::{error::ProgramError, program_entrypoint, ProgramResult};

#[macro_use]
mod accounts;
mod constants;
mod error;
mod helpers;
mod oracle;
mod perpdex;
mod state;
mod token_cpi;

program_entrypoint!(process_instruction);

// ── Anchor-compatible discriminators ─────────────────────────────────────────

// Oracle instructions (minimal — only authority management)
const DISC_INITIALIZE_ORACLE: [u8; 8] = [0x90, 0xdf, 0x83, 0x78, 0xc4, 0xfd, 0xb5, 0x63];
const DISC_TRANSFER_AUTHORITY: [u8; 8] = [0x30, 0xa9, 0x4c, 0x48, 0xe5, 0xb4, 0x37, 0xa1];
const DISC_ACCEPT_AUTHORITY: [u8; 8] = [0x6b, 0x56, 0xc6, 0x5b, 0x21, 0x0c, 0x6b, 0xa0];

// PerpDex instructions
const DISC_INITIALIZE_EXCHANGE: [u8; 8] = [0xe0, 0x69, 0x74, 0xa6, 0xe4, 0xcf, 0x60, 0x13];
const DISC_CREATE_MARKET: [u8; 8] = [0x67, 0xe2, 0x61, 0xeb, 0xc8, 0xbc, 0xfb, 0xfe];
const DISC_CREATE_MARKET_PERMISSIONLESS: [u8; 8] = [0x48, 0xc0, 0xbe, 0x8e, 0x69, 0x97, 0x02, 0x50];
const DISC_CREATE_USER_ACCOUNT: [u8; 8] = [0x92, 0x44, 0x64, 0x45, 0x3f, 0x2e, 0xb6, 0xc7];
const DISC_CLOSE_USER_ACCOUNT: [u8; 8] = [0xec, 0xb5, 0x03, 0x47, 0xc2, 0x12, 0x97, 0xbf];
const DISC_DEPOSIT: [u8; 8] = [0xf2, 0x23, 0xc6, 0x89, 0x52, 0xe1, 0xf2, 0xb6];
const DISC_WITHDRAW: [u8; 8] = [0xb7, 0x12, 0x46, 0x9c, 0x94, 0x6d, 0xa1, 0x22];
const DISC_OPEN_POSITION: [u8; 8] = [0x87, 0x80, 0x2f, 0x4d, 0x0f, 0x98, 0xf0, 0x31];
const DISC_CLOSE_POSITION: [u8; 8] = [0x7b, 0x86, 0x51, 0x00, 0x31, 0x44, 0x62, 0x62];
const DISC_ADD_MARGIN: [u8; 8] = [0xd3, 0xee, 0xee, 0x5a, 0xdf, 0xe4, 0xe4, 0x4c];
const DISC_LIQUIDATE: [u8; 8] = [0xdf, 0xb3, 0xe2, 0x7d, 0x30, 0x2e, 0x27, 0x4a];
const DISC_UPDATE_FUNDING_RATE: [u8; 8] = [0xc9, 0xb2, 0x74, 0xd4, 0xa6, 0x90, 0x48, 0xee];
const DISC_SET_PAUSED: [u8; 8] = [0x5b, 0x3c, 0x7d, 0xc0, 0xb0, 0xe1, 0xa6, 0xda];
const DISC_SET_MARKET_ACTIVE: [u8; 8] = [0xad, 0x21, 0xdd, 0x36, 0x53, 0x4c, 0x72, 0xcd];
const DISC_UPDATE_MARKET_PARAMS: [u8; 8] = [0x46, 0x75, 0xca, 0xbf, 0xcd, 0xae, 0x5c, 0x52];
const DISC_FUND_INSURANCE: [u8; 8] = [0x3a, 0xd8, 0x25, 0xa2, 0xc9, 0x32, 0x14, 0x42];
const DISC_TRANSFER_ADMIN: [u8; 8] = [0x2a, 0xf2, 0x42, 0x6a, 0xe4, 0x0a, 0x6f, 0x9c];
const DISC_ACCEPT_ADMIN: [u8; 8] = [0x70, 0x2a, 0x2d, 0x5a, 0x74, 0xb5, 0x0d, 0xaa];
const DISC_WITHDRAW_FEES: [u8; 8] = [0xc6, 0xd4, 0xab, 0x6d, 0x90, 0xd7, 0xae, 0x59];
const DISC_LIQUIDATE_KEEPER: [u8; 8] = [0xc5, 0x66, 0x9e, 0x95, 0xca, 0xd8, 0x49, 0x36];

fn process_instruction(
    program_id: &pinocchio::Address,
    accounts: &[pinocchio::account::AccountView],
    instruction_data: &[u8],
) -> ProgramResult {
    if instruction_data.len() < 8 {
        return Err(ProgramError::InvalidInstructionData);
    }

    let disc: [u8; 8] = instruction_data[..8]
        .try_into()
        .map_err(|_| ProgramError::InvalidInstructionData)?;
    let data = &instruction_data[8..];

    match disc {
        // ── Oracle (authority management only) ──────────────────────────
        DISC_INITIALIZE_ORACLE => oracle::process_initialize_oracle(program_id, accounts),
        DISC_TRANSFER_AUTHORITY => {
            let new_authority = helpers::read_pubkey(data)?;
            oracle::process_transfer_authority(program_id, accounts, new_authority)
        }
        DISC_ACCEPT_AUTHORITY => oracle::process_accept_authority(program_id, accounts),

        // ── PerpDex ──────────────────────────────────────────────────────
        DISC_INITIALIZE_EXCHANGE => perpdex::process_initialize_exchange(program_id, accounts),
        DISC_CREATE_MARKET => {
            if data.len() < 16 {
                return Err(ProgramError::InvalidInstructionData);
            }
            let product_id = helpers::read_u32(data)?;
            let max_leverage = helpers::read_u32(&data[4..])?;
            let max_open_interest = helpers::read_u64(&data[8..])?;
            perpdex::process_create_market(
                program_id,
                accounts,
                product_id,
                max_leverage,
                max_open_interest,
            )
        }
        DISC_CREATE_MARKET_PERMISSIONLESS => {
            let product_id = helpers::read_u32(data)?;
            perpdex::process_create_market_permissionless(program_id, accounts, product_id)
        }
        DISC_CREATE_USER_ACCOUNT => perpdex::process_create_user_account(program_id, accounts),
        DISC_CLOSE_USER_ACCOUNT => perpdex::process_close_user_account(program_id, accounts),
        DISC_DEPOSIT => {
            let amount = helpers::read_u64(data)?;
            perpdex::process_deposit(program_id, accounts, amount)
        }
        DISC_WITHDRAW => {
            let amount = helpers::read_u64(data)?;
            perpdex::process_withdraw(program_id, accounts, amount)
        }
        // open_position data: [product_id:4][side:1][size:8][leverage:4][price:8][timestamp:8] = 33 bytes
        DISC_OPEN_POSITION => {
            if data.len() < 33 {
                return Err(ProgramError::InvalidInstructionData);
            }
            let product_id = helpers::read_u32(data)?;
            let side = state::Side::from_u8(data[4])?;
            let size = helpers::read_u64(&data[5..])?;
            let leverage = helpers::read_u32(&data[13..])?;
            let price = helpers::read_u64(&data[17..])?;
            let timestamp = helpers::read_i64(&data[25..])?;
            perpdex::process_open_position(
                program_id, accounts, product_id, side, size, leverage, price, timestamp,
            )
        }
        // close_position data: [product_id:4][price:8][timestamp:8] = 20 bytes
        DISC_CLOSE_POSITION => {
            if data.len() < 20 {
                return Err(ProgramError::InvalidInstructionData);
            }
            let product_id = helpers::read_u32(data)?;
            let price = helpers::read_u64(&data[4..])?;
            let timestamp = helpers::read_i64(&data[12..])?;
            perpdex::process_close_position(program_id, accounts, product_id, price, timestamp)
        }
        DISC_ADD_MARGIN => {
            if data.len() < 12 {
                return Err(ProgramError::InvalidInstructionData);
            }
            let product_id = helpers::read_u32(data)?;
            let amount = helpers::read_u64(&data[4..])?;
            perpdex::process_add_margin(program_id, accounts, product_id, amount)
        }
        // liquidate data: [product_id:4][price:8][timestamp:8] = 20 bytes
        DISC_LIQUIDATE => {
            if data.len() < 20 {
                return Err(ProgramError::InvalidInstructionData);
            }
            let product_id = helpers::read_u32(data)?;
            let price = helpers::read_u64(&data[4..])?;
            let timestamp = helpers::read_i64(&data[12..])?;
            perpdex::process_liquidate(program_id, accounts, product_id, price, timestamp)
        }
        DISC_UPDATE_FUNDING_RATE => {
            let product_id = helpers::read_u32(data)?;
            perpdex::process_update_funding_rate(program_id, accounts, product_id)
        }
        DISC_SET_PAUSED => {
            if data.is_empty() {
                return Err(ProgramError::InvalidInstructionData);
            }
            let paused = data[0] != 0;
            perpdex::process_set_paused(program_id, accounts, paused)
        }
        DISC_SET_MARKET_ACTIVE => {
            if data.len() < 5 {
                return Err(ProgramError::InvalidInstructionData);
            }
            let product_id = helpers::read_u32(data)?;
            let active = data[4] != 0;
            perpdex::process_set_market_active(program_id, accounts, product_id, active)
        }
        DISC_UPDATE_MARKET_PARAMS => {
            if data.len() < 16 {
                return Err(ProgramError::InvalidInstructionData);
            }
            let product_id = helpers::read_u32(data)?;
            let max_leverage = helpers::read_u32(&data[4..])?;
            let max_open_interest = helpers::read_u64(&data[8..])?;
            perpdex::process_update_market_params(
                program_id,
                accounts,
                product_id,
                max_leverage,
                max_open_interest,
            )
        }
        DISC_FUND_INSURANCE => {
            let amount = helpers::read_u64(data)?;
            perpdex::process_fund_insurance(program_id, accounts, amount)
        }
        DISC_TRANSFER_ADMIN => {
            let new_admin = helpers::read_pubkey(data)?;
            perpdex::process_transfer_admin(program_id, accounts, new_admin)
        }
        DISC_ACCEPT_ADMIN => perpdex::process_accept_admin(program_id, accounts),
        DISC_WITHDRAW_FEES => {
            let amount = helpers::read_u64(data)?;
            perpdex::process_withdraw_fees(program_id, accounts, amount)
        }

        // liquidate_keeper data: [product_id:4][price:8][timestamp:8] = 20 bytes
        DISC_LIQUIDATE_KEEPER => {
            if data.len() < 20 {
                return Err(ProgramError::InvalidInstructionData);
            }
            let product_id = helpers::read_u32(data)?;
            let price = helpers::read_u64(&data[4..])?;
            let timestamp = helpers::read_i64(&data[12..])?;
            perpdex::process_liquidate_keeper(program_id, accounts, product_id, price, timestamp)
        }

        _ => Err(ProgramError::InvalidInstructionData),
    }
}
