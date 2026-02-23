/// Stress tests for the TradeX program.
///
/// These tests target boundary conditions, extreme values, arithmetic overflow
/// protection, maximum capacity scenarios, and adversarial inputs.
///
/// Uses mollusk-svm to execute the compiled .so against instruction payloads.
use mollusk_svm::program::keyed_account_for_system_program;
use mollusk_svm::Mollusk;
use mollusk_svm_programs_token::token as spl_token_program;
use solana_account::Account;
use solana_instruction::{AccountMeta, Instruction};
use solana_pubkey::Pubkey;
use solana_sdk_ids::{system_program, sysvar};
use std::str::FromStr;

// ── Program ID ──────────────────────────────────────────────────────────────

fn program_id() -> Pubkey {
    Pubkey::from_str("AzYLMDPsNDvzb8ShhVzNWBVv6k9ogCKpqdjWMhnkATdz").unwrap()
}

fn spl_token_id() -> Pubkey {
    Pubkey::from_str("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA").unwrap()
}

// ── PDA Seeds ───────────────────────────────────────────────────────────────

const ORACLE_SEED: &[u8] = b"oracle";
const PRICE_FEED_SEED: &[u8] = b"price_feed";
const PUBLISHER_LIST_SEED: &[u8] = b"publisher_list";
const EXCHANGE_SEED: &[u8] = b"exchange";
const VAULT_SEED: &[u8] = b"vault";
const MARKET_SEED: &[u8] = b"market";
const USER_ACCOUNT_SEED: &[u8] = b"user_account";
const POSITION_SEED: &[u8] = b"position";
const INSURANCE_SEED: &[u8] = b"insurance";

// ── Discriminators ──────────────────────────────────────────────────────────

const DISC_INITIALIZE_ORACLE: [u8; 8] = [0x90, 0xdf, 0x83, 0x78, 0xc4, 0xfd, 0xb5, 0x63];
const DISC_CREATE_FEED: [u8; 8] = [0xad, 0x56, 0x5f, 0x5e, 0x0d, 0xc1, 0x43, 0xb4];
const DISC_ADD_PUBLISHER: [u8; 8] = [0x26, 0x08, 0xea, 0x2c, 0x4b, 0x1d, 0xa3, 0x92];
const DISC_PUBLISH_PRICE: [u8; 8] = [0x75, 0x0d, 0x06, 0xab, 0x1d, 0xcc, 0x0b, 0x01];
const DISC_PUBLISH_PRICES_BATCH: [u8; 8] = [0xe7, 0x47, 0xef, 0x79, 0xc4, 0x70, 0x65, 0xe4];
const DISC_INITIALIZE_EXCHANGE: [u8; 8] = [0xe0, 0x69, 0x74, 0xa6, 0xe4, 0xcf, 0x60, 0x13];
const DISC_CREATE_MARKET: [u8; 8] = [0x67, 0xe2, 0x61, 0xeb, 0xc8, 0xbc, 0xfb, 0xfe];
const DISC_CREATE_USER_ACCOUNT: [u8; 8] = [0x92, 0x44, 0x64, 0x45, 0x3f, 0x2e, 0xb6, 0xc7];
const DISC_DEPOSIT: [u8; 8] = [0xf2, 0x23, 0xc6, 0x89, 0x52, 0xe1, 0xf2, 0xb6];
const DISC_WITHDRAW: [u8; 8] = [0xb7, 0x12, 0x46, 0x9c, 0x94, 0x6d, 0xa1, 0x22];
const DISC_OPEN_POSITION: [u8; 8] = [0x87, 0x80, 0x2f, 0x4d, 0x0f, 0x98, 0xf0, 0x31];
const DISC_CLOSE_POSITION: [u8; 8] = [0x7b, 0x86, 0x51, 0x00, 0x31, 0x44, 0x62, 0x62];
const DISC_ADD_MARGIN: [u8; 8] = [0xd3, 0xee, 0xee, 0x5a, 0xdf, 0xe4, 0xe4, 0x4c];
const DISC_UPDATE_FUNDING_RATE: [u8; 8] = [0xc9, 0xb2, 0x74, 0xd4, 0xa6, 0x90, 0x48, 0xee];
const DISC_SET_PAUSED: [u8; 8] = [0x5b, 0x3c, 0x7d, 0xc0, 0xb0, 0xe1, 0xa6, 0xda];
const DISC_LIQUIDATE: [u8; 8] = [0xdf, 0xb3, 0xe2, 0x7d, 0x30, 0x2e, 0x27, 0x4a];
const DISC_FUND_INSURANCE: [u8; 8] = [0x3a, 0xd8, 0x25, 0xa2, 0xc9, 0x32, 0x14, 0x42];
const DISC_UPDATE_MARKET_PARAMS: [u8; 8] = [0x46, 0x75, 0xca, 0xbf, 0xcd, 0xae, 0x5c, 0x52];

// Account discriminators
const DISC_ORACLE_STATE: [u8; 8] = [0x6a, 0x0a, 0xcb, 0x5b, 0x63, 0x60, 0xa1, 0x27];
const DISC_PUBLISHER_LIST: [u8; 8] = [0xf0, 0x4d, 0x78, 0xa9, 0x7d, 0x0c, 0xf2, 0x22];
const DISC_PRICE_FEED: [u8; 8] = [0x65, 0x5b, 0x5b, 0x7e, 0xf3, 0x56, 0xbc, 0x52];
const DISC_EXCHANGE_STATE: [u8; 8] = [0x79, 0xf0, 0x75, 0x17, 0x49, 0xb2, 0xc4, 0x47];
const DISC_MARKET: [u8; 8] = [0xdf, 0xb8, 0x9c, 0xee, 0x6b, 0xa0, 0x8a, 0x4a];
const DISC_USER_ACCOUNT: [u8; 8] = [0x21, 0x1e, 0x8e, 0x54, 0xb9, 0xd3, 0x19, 0xe5];
const DISC_POSITION: [u8; 8] = [0xaa, 0xbc, 0x8f, 0xe4, 0x7a, 0x17, 0x2b, 0x6c];

// ── Helpers ─────────────────────────────────────────────────────────────────

fn setup_mollusk() -> Mollusk {
    Mollusk::new(&program_id(), "tradex_program")
}

fn find_pda(seeds: &[&[u8]]) -> (Pubkey, u8) {
    Pubkey::find_program_address(seeds, &program_id())
}

fn build_ix_data(disc: &[u8; 8], extra: &[u8]) -> Vec<u8> {
    let mut data = disc.to_vec();
    data.extend_from_slice(extra);
    data
}

fn system_account(lamports: u64) -> Account {
    Account {
        lamports,
        data: vec![],
        owner: system_program::ID,
        executable: false,
        rent_epoch: 0,
    }
}

fn system_program_account() -> (Pubkey, Account) {
    keyed_account_for_system_program()
}

fn program_owned_account(data: Vec<u8>) -> Account {
    Account {
        lamports: 1_000_000_000,
        data,
        owner: program_id(),
        executable: false,
        rent_epoch: 0,
    }
}

fn token_account_obj(data: Vec<u8>) -> Account {
    Account {
        lamports: 1_000_000_000,
        data,
        owner: spl_token_id(),
        executable: false,
        rent_epoch: 0,
    }
}

fn make_oracle_state(authority: &Pubkey) -> Vec<u8> {
    let mut data = vec![0u8; 92];
    data[..8].copy_from_slice(&DISC_ORACLE_STATE);
    data[8..40].copy_from_slice(authority.as_ref());
    data
}

fn make_publisher_list(authority: &Pubkey, publishers: &[Pubkey]) -> Vec<u8> {
    let mut data = vec![0u8; 556];
    data[..8].copy_from_slice(&DISC_PUBLISHER_LIST);
    data[8..40].copy_from_slice(authority.as_ref());
    let count = publishers.len() as u32;
    data[40..44].copy_from_slice(&count.to_le_bytes());
    for (i, pk) in publishers.iter().enumerate() {
        let offset = 44 + i * 32;
        data[offset..offset + 32].copy_from_slice(pk.as_ref());
    }
    data
}

fn make_price_feed(product_id: u32, price: u64, last_updated: i64, authority: &Pubkey) -> Vec<u8> {
    let mut data = vec![0u8; 68];
    data[..8].copy_from_slice(&DISC_PRICE_FEED);
    data[8..12].copy_from_slice(&product_id.to_le_bytes());
    data[12..20].copy_from_slice(&price.to_le_bytes());
    data[20..28].copy_from_slice(&last_updated.to_le_bytes());
    data[36..68].copy_from_slice(authority.as_ref());
    data
}

fn make_exchange_state(
    admin: &Pubkey,
    usdc_mint: &Pubkey,
    vault: &Pubkey,
    insurance: &Pubkey,
) -> Vec<u8> {
    let mut data = vec![0u8; 213];
    data[..8].copy_from_slice(&DISC_EXCHANGE_STATE);
    data[8..40].copy_from_slice(admin.as_ref());
    data[72..104].copy_from_slice(usdc_mint.as_ref());
    data[104..136].copy_from_slice(vault.as_ref());
    data[136..168].copy_from_slice(insurance.as_ref());
    data
}

fn make_exchange_state_full(
    admin: &Pubkey,
    usdc_mint: &Pubkey,
    vault: &Pubkey,
    insurance: &Pubkey,
    market_count: u32,
    total_deposits: u64,
    total_volume: u128,
    total_fees: u64,
    insurance_balance: u64,
    paused: bool,
) -> Vec<u8> {
    let mut data = make_exchange_state(admin, usdc_mint, vault, insurance);
    data[168..172].copy_from_slice(&market_count.to_le_bytes());
    data[172..180].copy_from_slice(&total_deposits.to_le_bytes());
    data[180..196].copy_from_slice(&total_volume.to_le_bytes());
    data[196..204].copy_from_slice(&total_fees.to_le_bytes());
    data[204..212].copy_from_slice(&insurance_balance.to_le_bytes());
    data[212] = if paused { 1 } else { 0 };
    data
}

fn make_market_with_oi(
    product_id: u32,
    oracle_feed: &Pubkey,
    active: bool,
    max_leverage: u32,
    max_oi: u64,
    long_oi: u64,
    short_oi: u64,
    last_funding_time: i64,
    cum_funding_long: i64,
    cum_funding_short: i64,
) -> Vec<u8> {
    let mut data = vec![0u8; 113];
    data[..8].copy_from_slice(&DISC_MARKET);
    data[8..12].copy_from_slice(&product_id.to_le_bytes());
    data[12..44].copy_from_slice(oracle_feed.as_ref());
    data[44..48].copy_from_slice(&max_leverage.to_le_bytes());
    data[48..56].copy_from_slice(&max_oi.to_le_bytes());
    data[56..64].copy_from_slice(&long_oi.to_le_bytes());
    data[64..72].copy_from_slice(&short_oi.to_le_bytes());
    data[88..96].copy_from_slice(&last_funding_time.to_le_bytes());
    data[96..104].copy_from_slice(&cum_funding_long.to_le_bytes());
    data[104..112].copy_from_slice(&cum_funding_short.to_le_bytes());
    data[112] = if active { 1 } else { 0 };
    data
}

fn make_user_account(owner: &Pubkey, balance: u64, open_positions: u32) -> Vec<u8> {
    let mut data = vec![0u8; 84];
    data[..8].copy_from_slice(&DISC_USER_ACCOUNT);
    data[8..40].copy_from_slice(owner.as_ref());
    data[40..48].copy_from_slice(&balance.to_le_bytes());
    data[80..84].copy_from_slice(&open_positions.to_le_bytes());
    data
}

fn make_position(
    owner: &Pubkey,
    product_id: u32,
    side: u8,
    size: u64,
    entry_price: u64,
    leverage: u32,
    margin: u64,
) -> Vec<u8> {
    let mut data = vec![0u8; 113];
    data[..8].copy_from_slice(&DISC_POSITION);
    data[8..40].copy_from_slice(owner.as_ref());
    data[40..44].copy_from_slice(&product_id.to_le_bytes());
    data[44] = side;
    data[45..53].copy_from_slice(&size.to_le_bytes());
    data[53..61].copy_from_slice(&entry_price.to_le_bytes());
    data[61..65].copy_from_slice(&leverage.to_le_bytes());
    data[65..73].copy_from_slice(&margin.to_le_bytes());
    data
}

fn make_token_account(mint: &Pubkey, owner: &Pubkey, amount: u64) -> Vec<u8> {
    let mut data = vec![0u8; 165];
    data[..32].copy_from_slice(mint.as_ref());
    data[32..64].copy_from_slice(owner.as_ref());
    data[64..72].copy_from_slice(&amount.to_le_bytes());
    data[108] = 1;
    data
}

fn read_u32_at(data: &[u8], offset: usize) -> u32 {
    u32::from_le_bytes(data[offset..offset + 4].try_into().unwrap())
}

fn read_u64_at(data: &[u8], offset: usize) -> u64 {
    u64::from_le_bytes(data[offset..offset + 8].try_into().unwrap())
}

fn read_i64_at(data: &[u8], offset: usize) -> i64 {
    i64::from_le_bytes(data[offset..offset + 8].try_into().unwrap())
}

// ══════════════════════════════════════════════════════════════════════════════
// STRESS TEST 1: Maximum open interest boundary
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn stress_max_open_interest_boundary() {
    let mut mollusk = setup_mollusk();
    mollusk.sysvars.warp_to_slot(100);
    mollusk.sysvars.clock.unix_timestamp = 1000;

    let pid = program_id();
    let admin = Pubkey::new_unique();
    let user = Pubkey::new_unique();
    let usdc_mint = Pubkey::new_unique();
    let product_id: u32 = 1;

    let (exchange_pda, _) = find_pda(&[EXCHANGE_SEED]);
    let (vault_pda, _) = find_pda(&[VAULT_SEED]);
    let (insurance_pda, _) = find_pda(&[INSURANCE_SEED]);
    let (user_account_pda, _) = find_pda(&[USER_ACCOUNT_SEED, user.as_ref()]);
    let (market_pda, _) = find_pda(&[MARKET_SEED, &product_id.to_le_bytes()]);
    let (price_feed_pda, _) = find_pda(&[PRICE_FEED_SEED, &product_id.to_le_bytes()]);
    let (position_pda, _) = find_pda(&[POSITION_SEED, user.as_ref(), &product_id.to_le_bytes()]);

    let oracle_price: u64 = 1_000_000_000;
    let max_oi: u64 = 50_000_000; // $50
    let position_size: u64 = 50_000_000; // Exactly at max OI

    let exchange_data = make_exchange_state(&admin, &usdc_mint, &vault_pda, &insurance_pda);
    // Market already has some OI, adding this position would exceed max
    let market_data = make_market_with_oi(
        product_id,
        &price_feed_pda,
        true,
        10,
        max_oi,
        1,
        0, // long_oi = 1 (just above zero to trigger boundary)
        0,
        0,
        0,
    );
    let user_account_data = make_user_account(&user, 1_000_000_000, 0);

    let mut ix_data = DISC_OPEN_POSITION.to_vec();
    ix_data.extend_from_slice(&product_id.to_le_bytes());
    ix_data.push(0); // Long
    ix_data.extend_from_slice(&position_size.to_le_bytes());
    ix_data.extend_from_slice(&1u32.to_le_bytes()); // 1x leverage

    let result = mollusk.process_instruction(
        &Instruction {
            program_id: pid,
            accounts: vec![
                AccountMeta::new(user_account_pda, false),
                AccountMeta::new(position_pda, false),
                AccountMeta::new(market_pda, false),
                AccountMeta::new(exchange_pda, false),
                AccountMeta::new_readonly(price_feed_pda, false),
                AccountMeta::new(user, true),
                AccountMeta::new_readonly(system_program::ID, false),
            ],
            data: ix_data,
        },
        &[
            (user_account_pda, program_owned_account(user_account_data)),
            (position_pda, system_account(0)),
            (market_pda, program_owned_account(market_data)),
            (exchange_pda, program_owned_account(exchange_data)),
            (
                price_feed_pda,
                program_owned_account(make_price_feed(product_id, oracle_price, 1000, &admin)),
            ),
            (user, system_account(100_000_000_000)),
            system_program_account(),
        ],
    );
    // total OI = 1 + 50_000_000 = 50_000_001 > 50_000_000, should fail
    assert!(
        result.program_result.is_err(),
        "Exceeding max OI should fail"
    );
}

// ══════════════════════════════════════════════════════════════════════════════
// STRESS TEST 2: Exactly at max OI boundary (should succeed)
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn stress_exact_max_oi_should_succeed() {
    let mut mollusk = setup_mollusk();
    mollusk.sysvars.warp_to_slot(100);
    mollusk.sysvars.clock.unix_timestamp = 1000;

    let pid = program_id();
    let admin = Pubkey::new_unique();
    let user = Pubkey::new_unique();
    let usdc_mint = Pubkey::new_unique();
    let product_id: u32 = 1;

    let (exchange_pda, _) = find_pda(&[EXCHANGE_SEED]);
    let (vault_pda, _) = find_pda(&[VAULT_SEED]);
    let (insurance_pda, _) = find_pda(&[INSURANCE_SEED]);
    let (user_account_pda, _) = find_pda(&[USER_ACCOUNT_SEED, user.as_ref()]);
    let (market_pda, _) = find_pda(&[MARKET_SEED, &product_id.to_le_bytes()]);
    let (price_feed_pda, _) = find_pda(&[PRICE_FEED_SEED, &product_id.to_le_bytes()]);
    let (position_pda, _) = find_pda(&[POSITION_SEED, user.as_ref(), &product_id.to_le_bytes()]);

    let oracle_price: u64 = 1_000_000_000;
    let position_size: u64 = 10_000_000; // $10
    let max_oi: u64 = position_size; // exact match

    let exchange_data = make_exchange_state(&admin, &usdc_mint, &vault_pda, &insurance_pda);
    let market_data =
        make_market_with_oi(product_id, &price_feed_pda, true, 10, max_oi, 0, 0, 0, 0, 0);
    let user_account_data = make_user_account(&user, 1_000_000_000, 0);

    let mut ix_data = DISC_OPEN_POSITION.to_vec();
    ix_data.extend_from_slice(&product_id.to_le_bytes());
    ix_data.push(0);
    ix_data.extend_from_slice(&position_size.to_le_bytes());
    ix_data.extend_from_slice(&1u32.to_le_bytes());

    let result = mollusk.process_instruction(
        &Instruction {
            program_id: pid,
            accounts: vec![
                AccountMeta::new(user_account_pda, false),
                AccountMeta::new(position_pda, false),
                AccountMeta::new(market_pda, false),
                AccountMeta::new(exchange_pda, false),
                AccountMeta::new_readonly(price_feed_pda, false),
                AccountMeta::new(user, true),
                AccountMeta::new_readonly(system_program::ID, false),
            ],
            data: ix_data,
        },
        &[
            (user_account_pda, program_owned_account(user_account_data)),
            (position_pda, system_account(0)),
            (market_pda, program_owned_account(market_data)),
            (exchange_pda, program_owned_account(exchange_data)),
            (
                price_feed_pda,
                program_owned_account(make_price_feed(product_id, oracle_price, 1000, &admin)),
            ),
            (user, system_account(100_000_000_000)),
            system_program_account(),
        ],
    );
    // total OI = 10_000_000 == max_oi, should succeed
    assert!(
        result.program_result.is_ok(),
        "Exact max OI should succeed: {:?}",
        result.program_result
    );
}

// ══════════════════════════════════════════════════════════════════════════════
// STRESS TEST 3: Large u64 values in position operations
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn stress_large_position_values() {
    let mut mollusk = setup_mollusk();
    mollusk.sysvars.warp_to_slot(100);
    mollusk.sysvars.clock.unix_timestamp = 1000;

    let pid = program_id();
    let admin = Pubkey::new_unique();
    let user = Pubkey::new_unique();
    let usdc_mint = Pubkey::new_unique();
    let product_id: u32 = 1;

    let (exchange_pda, _) = find_pda(&[EXCHANGE_SEED]);
    let (vault_pda, _) = find_pda(&[VAULT_SEED]);
    let (insurance_pda, _) = find_pda(&[INSURANCE_SEED]);
    let (user_account_pda, _) = find_pda(&[USER_ACCOUNT_SEED, user.as_ref()]);
    let (market_pda, _) = find_pda(&[MARKET_SEED, &product_id.to_le_bytes()]);
    let (price_feed_pda, _) = find_pda(&[PRICE_FEED_SEED, &product_id.to_le_bytes()]);
    let (position_pda, _) = find_pda(&[POSITION_SEED, user.as_ref(), &product_id.to_le_bytes()]);

    // Very large position size that could cause overflow
    let oracle_price: u64 = 1_000_000_000;
    let huge_size: u64 = u64::MAX / 2; // near max

    let exchange_data = make_exchange_state(&admin, &usdc_mint, &vault_pda, &insurance_pda);
    let market_data = make_market_with_oi(
        product_id,
        &price_feed_pda,
        true,
        10,
        u64::MAX,
        0,
        0,
        0,
        0,
        0,
    );
    let user_account_data = make_user_account(&user, u64::MAX / 2, 0);

    let mut ix_data = DISC_OPEN_POSITION.to_vec();
    ix_data.extend_from_slice(&product_id.to_le_bytes());
    ix_data.push(0);
    ix_data.extend_from_slice(&huge_size.to_le_bytes());
    ix_data.extend_from_slice(&1u32.to_le_bytes());

    let result = mollusk.process_instruction(
        &Instruction {
            program_id: pid,
            accounts: vec![
                AccountMeta::new(user_account_pda, false),
                AccountMeta::new(position_pda, false),
                AccountMeta::new(market_pda, false),
                AccountMeta::new(exchange_pda, false),
                AccountMeta::new_readonly(price_feed_pda, false),
                AccountMeta::new(user, true),
                AccountMeta::new_readonly(system_program::ID, false),
            ],
            data: ix_data,
        },
        &[
            (user_account_pda, program_owned_account(user_account_data)),
            (position_pda, system_account(0)),
            (market_pda, program_owned_account(market_data)),
            (exchange_pda, program_owned_account(exchange_data)),
            (
                price_feed_pda,
                program_owned_account(make_price_feed(product_id, oracle_price, 1000, &admin)),
            ),
            (user, system_account(100_000_000_000)),
            system_program_account(),
        ],
    );
    // Should handle gracefully - either succeed or fail with overflow, NOT panic
    let _ = result.program_result;
}

// ══════════════════════════════════════════════════════════════════════════════
// STRESS TEST 4: Rapid consecutive price updates
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn stress_rapid_price_updates() {
    let mollusk = setup_mollusk();
    let pid = program_id();
    let publisher = Pubkey::new_unique();
    let product_id: u32 = 1;

    let (feed_pda, _) = find_pda(&[PRICE_FEED_SEED, &product_id.to_le_bytes()]);
    let (publisher_list_pda, _) = find_pda(&[PUBLISHER_LIST_SEED]);

    let publisher_list = program_owned_account(make_publisher_list(&publisher, &[publisher]));
    let mut current_feed = program_owned_account(make_price_feed(product_id, 0, 0, &publisher));

    // Rapidly update price 100 times
    for i in 1..=100u64 {
        let price = i * 100_000_000; // $1, $2, $3, ...

        let mut ix_data = DISC_PUBLISH_PRICE.to_vec();
        ix_data.extend_from_slice(&product_id.to_le_bytes());
        ix_data.extend_from_slice(&price.to_le_bytes());

        let result = mollusk.process_instruction(
            &Instruction {
                program_id: pid,
                accounts: vec![
                    AccountMeta::new(publisher, true),
                    AccountMeta::new(feed_pda, false),
                    AccountMeta::new_readonly(publisher_list_pda, false),
                ],
                data: ix_data,
            },
            &[
                (publisher, system_account(1_000_000_000)),
                (feed_pda, current_feed.clone()),
                (publisher_list_pda, publisher_list.clone()),
            ],
        );
        assert!(
            result.program_result.is_ok(),
            "Price update {} failed: {:?}",
            i,
            result.program_result
        );

        current_feed = result
            .resulting_accounts
            .iter()
            .find(|(k, _)| *k == feed_pda)
            .unwrap()
            .1
            .clone();

        // Verify update_count incremented correctly
        let update_count = read_u64_at(&current_feed.data, 28);
        assert_eq!(update_count, i);
    }

    // Verify final state
    let final_price = read_u64_at(&current_feed.data, 12);
    assert_eq!(final_price, 100 * 100_000_000); // $100
}

// ══════════════════════════════════════════════════════════════════════════════
// STRESS TEST 5: Multiple consecutive deposits building up balance
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn stress_many_small_deposits() {
    let mut mollusk = setup_mollusk();
    spl_token_program::add_program(&mut mollusk);

    let pid = program_id();
    let admin = Pubkey::new_unique();
    let user = Pubkey::new_unique();
    let usdc_mint = Pubkey::new_unique();

    let (exchange_pda, _) = find_pda(&[EXCHANGE_SEED]);
    let (vault_pda, _) = find_pda(&[VAULT_SEED]);
    let (insurance_pda, _) = find_pda(&[INSURANCE_SEED]);
    let (user_account_pda, _) = find_pda(&[USER_ACCOUNT_SEED, user.as_ref()]);
    let user_token = Pubkey::new_unique();

    let exchange_data = make_exchange_state(&admin, &usdc_mint, &vault_pda, &insurance_pda);
    let mut current_user_account = program_owned_account(make_user_account(&user, 0, 0));
    let mut current_exchange = program_owned_account(exchange_data);
    let mut current_vault = token_account_obj(make_token_account(&usdc_mint, &exchange_pda, 0));
    let mut current_user_token =
        token_account_obj(make_token_account(&usdc_mint, &user, 10_000_000_000));

    let deposit_amount: u64 = 1_000_000; // $1
    let num_deposits = 50;

    for i in 0..num_deposits {
        let mut ix_data = DISC_DEPOSIT.to_vec();
        ix_data.extend_from_slice(&deposit_amount.to_le_bytes());

        let result = mollusk.process_instruction(
            &Instruction {
                program_id: pid,
                accounts: vec![
                    AccountMeta::new(user_account_pda, false),
                    AccountMeta::new(exchange_pda, false),
                    AccountMeta::new(vault_pda, false),
                    AccountMeta::new(user_token, false),
                    AccountMeta::new(user, true),
                    AccountMeta::new_readonly(spl_token_id(), false),
                ],
                data: ix_data,
            },
            &[
                (user_account_pda, current_user_account.clone()),
                (exchange_pda, current_exchange.clone()),
                (vault_pda, current_vault.clone()),
                (user_token, current_user_token.clone()),
                (user, system_account(1_000_000_000)),
                spl_token_program::keyed_account(),
            ],
        );
        assert!(
            result.program_result.is_ok(),
            "Deposit {} failed: {:?}",
            i,
            result.program_result
        );

        current_user_account = result
            .resulting_accounts
            .iter()
            .find(|(k, _)| *k == user_account_pda)
            .unwrap()
            .1
            .clone();
        current_exchange = result
            .resulting_accounts
            .iter()
            .find(|(k, _)| *k == exchange_pda)
            .unwrap()
            .1
            .clone();
        current_vault = result
            .resulting_accounts
            .iter()
            .find(|(k, _)| *k == vault_pda)
            .unwrap()
            .1
            .clone();
        current_user_token = result
            .resulting_accounts
            .iter()
            .find(|(k, _)| *k == user_token)
            .unwrap()
            .1
            .clone();
    }

    // Verify accumulated balance
    let final_balance = read_u64_at(&current_user_account.data, 40);
    assert_eq!(final_balance, deposit_amount * num_deposits);

    let vault_balance = read_u64_at(&current_vault.data, 64);
    assert_eq!(vault_balance, deposit_amount * num_deposits);
}

// ══════════════════════════════════════════════════════════════════════════════
// STRESS TEST 6: Funding rate with extreme OI imbalance
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn stress_funding_rate_extreme_imbalance() {
    let mut mollusk = setup_mollusk();
    mollusk.sysvars.warp_to_slot(100);
    mollusk.sysvars.clock.unix_timestamp = 50_000;

    let pid = program_id();
    let cranker = Pubkey::new_unique();
    let admin = Pubkey::new_unique();
    let usdc_mint = Pubkey::new_unique();
    let vault = Pubkey::new_unique();
    let insurance = Pubkey::new_unique();
    let product_id: u32 = 1;

    let (exchange_pda, _) = find_pda(&[EXCHANGE_SEED]);
    let (market_pda, _) = find_pda(&[MARKET_SEED, &product_id.to_le_bytes()]);
    let oracle_feed = Pubkey::new_unique();

    // Test 1: All longs, zero shorts
    let market_data = make_market_with_oi(
        product_id,
        &oracle_feed,
        true,
        10,
        u64::MAX,
        1_000_000_000_000,
        0, // all longs
        0,
        0,
        0,
    );
    let exchange_data = make_exchange_state(&admin, &usdc_mint, &vault, &insurance);

    let mut ix_data = DISC_UPDATE_FUNDING_RATE.to_vec();
    ix_data.extend_from_slice(&product_id.to_le_bytes());

    let result = mollusk.process_instruction(
        &Instruction {
            program_id: pid,
            accounts: vec![
                AccountMeta::new(market_pda, false),
                AccountMeta::new_readonly(exchange_pda, false),
                AccountMeta::new_readonly(cranker, true),
            ],
            data: ix_data.clone(),
        },
        &[
            (market_pda, program_owned_account(market_data)),
            (exchange_pda, program_owned_account(exchange_data.clone())),
            (cranker, system_account(1_000_000_000)),
        ],
    );
    assert!(
        result.program_result.is_ok(),
        "All-longs funding rate failed: {:?}",
        result.program_result
    );

    let market_data_result = &result
        .resulting_accounts
        .iter()
        .find(|(k, _)| *k == market_pda)
        .unwrap()
        .1
        .data;
    let cum_long = read_i64_at(market_data_result, 96);
    let cum_short = read_i64_at(market_data_result, 104);
    // With 100% longs, rate should be capped at +5 BPS
    assert_eq!(cum_long, 5); // max cap
    assert_eq!(cum_short, -5);

    // Test 2: All shorts, zero longs
    let market_data2 = make_market_with_oi(
        product_id,
        &oracle_feed,
        true,
        10,
        u64::MAX,
        0,
        1_000_000_000_000, // all shorts
        0,
        0,
        0,
    );

    let result2 = mollusk.process_instruction(
        &Instruction {
            program_id: pid,
            accounts: vec![
                AccountMeta::new(market_pda, false),
                AccountMeta::new_readonly(exchange_pda, false),
                AccountMeta::new_readonly(cranker, true),
            ],
            data: ix_data.clone(),
        },
        &[
            (market_pda, program_owned_account(market_data2)),
            (exchange_pda, program_owned_account(exchange_data.clone())),
            (cranker, system_account(1_000_000_000)),
        ],
    );
    assert!(
        result2.program_result.is_ok(),
        "All-shorts funding rate failed: {:?}",
        result2.program_result
    );

    let market_data_result2 = &result2
        .resulting_accounts
        .iter()
        .find(|(k, _)| *k == market_pda)
        .unwrap()
        .1
        .data;
    let cum_long2 = read_i64_at(market_data_result2, 96);
    let cum_short2 = read_i64_at(market_data_result2, 104);
    assert_eq!(cum_long2, -5); // min cap
    assert_eq!(cum_short2, 5);

    // Test 3: Zero OI on both sides
    let market_data3 = make_market_with_oi(
        product_id,
        &oracle_feed,
        true,
        10,
        u64::MAX,
        0,
        0, // no OI
        0,
        0,
        0,
    );

    let result3 = mollusk.process_instruction(
        &Instruction {
            program_id: pid,
            accounts: vec![
                AccountMeta::new(market_pda, false),
                AccountMeta::new_readonly(exchange_pda, false),
                AccountMeta::new_readonly(cranker, true),
            ],
            data: ix_data,
        },
        &[
            (market_pda, program_owned_account(market_data3)),
            (exchange_pda, program_owned_account(exchange_data)),
            (cranker, system_account(1_000_000_000)),
        ],
    );
    assert!(
        result3.program_result.is_ok(),
        "Zero OI funding rate failed: {:?}",
        result3.program_result
    );

    let market_data_result3 = &result3
        .resulting_accounts
        .iter()
        .find(|(k, _)| *k == market_pda)
        .unwrap()
        .1
        .data;
    assert_eq!(read_i64_at(market_data_result3, 96), 0); // no change
    assert_eq!(read_i64_at(market_data_result3, 104), 0);
}

// ══════════════════════════════════════════════════════════════════════════════
// STRESS TEST 7: Position at minimum size boundary
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn stress_minimum_position_size() {
    let mut mollusk = setup_mollusk();
    mollusk.sysvars.warp_to_slot(100);
    mollusk.sysvars.clock.unix_timestamp = 1000;

    let pid = program_id();
    let admin = Pubkey::new_unique();
    let user = Pubkey::new_unique();
    let usdc_mint = Pubkey::new_unique();
    let product_id: u32 = 1;

    let (exchange_pda, _) = find_pda(&[EXCHANGE_SEED]);
    let (vault_pda, _) = find_pda(&[VAULT_SEED]);
    let (insurance_pda, _) = find_pda(&[INSURANCE_SEED]);
    let (user_account_pda, _) = find_pda(&[USER_ACCOUNT_SEED, user.as_ref()]);
    let (market_pda, _) = find_pda(&[MARKET_SEED, &product_id.to_le_bytes()]);
    let (price_feed_pda, _) = find_pda(&[PRICE_FEED_SEED, &product_id.to_le_bytes()]);
    let (position_pda, _) = find_pda(&[POSITION_SEED, user.as_ref(), &product_id.to_le_bytes()]);

    let oracle_price: u64 = 1_000_000_000;
    let exchange_data = make_exchange_state(&admin, &usdc_mint, &vault_pda, &insurance_pda);
    let market_data = make_market_with_oi(
        product_id,
        &price_feed_pda,
        true,
        10,
        1_000_000_000_000,
        0,
        0,
        0,
        0,
        0,
    );

    // Test just below minimum ($1 = 1_000_000)
    let below_min: u64 = 999_999;
    let user_account_data = make_user_account(&user, 100_000_000, 0);

    let mut ix_data = DISC_OPEN_POSITION.to_vec();
    ix_data.extend_from_slice(&product_id.to_le_bytes());
    ix_data.push(0);
    ix_data.extend_from_slice(&below_min.to_le_bytes());
    ix_data.extend_from_slice(&1u32.to_le_bytes());

    let result = mollusk.process_instruction(
        &Instruction {
            program_id: pid,
            accounts: vec![
                AccountMeta::new(user_account_pda, false),
                AccountMeta::new(position_pda, false),
                AccountMeta::new(market_pda, false),
                AccountMeta::new(exchange_pda, false),
                AccountMeta::new_readonly(price_feed_pda, false),
                AccountMeta::new(user, true),
                AccountMeta::new_readonly(system_program::ID, false),
            ],
            data: ix_data,
        },
        &[
            (user_account_pda, program_owned_account(user_account_data)),
            (position_pda, system_account(0)),
            (market_pda, program_owned_account(market_data.clone())),
            (exchange_pda, program_owned_account(exchange_data.clone())),
            (
                price_feed_pda,
                program_owned_account(make_price_feed(product_id, oracle_price, 1000, &admin)),
            ),
            (user, system_account(100_000_000_000)),
            system_program_account(),
        ],
    );
    assert!(
        result.program_result.is_err(),
        "Below min position size should fail"
    );

    // Test exactly at minimum ($1 = 1_000_000)
    let exact_min: u64 = 1_000_000;
    let user_account_data = make_user_account(&user, 100_000_000, 0);

    let mut ix_data = DISC_OPEN_POSITION.to_vec();
    ix_data.extend_from_slice(&product_id.to_le_bytes());
    ix_data.push(0);
    ix_data.extend_from_slice(&exact_min.to_le_bytes());
    ix_data.extend_from_slice(&1u32.to_le_bytes());

    let result = mollusk.process_instruction(
        &Instruction {
            program_id: pid,
            accounts: vec![
                AccountMeta::new(user_account_pda, false),
                AccountMeta::new(position_pda, false),
                AccountMeta::new(market_pda, false),
                AccountMeta::new(exchange_pda, false),
                AccountMeta::new_readonly(price_feed_pda, false),
                AccountMeta::new(user, true),
                AccountMeta::new_readonly(system_program::ID, false),
            ],
            data: ix_data,
        },
        &[
            (user_account_pda, program_owned_account(user_account_data)),
            (position_pda, system_account(0)),
            (market_pda, program_owned_account(market_data)),
            (exchange_pda, program_owned_account(exchange_data)),
            (
                price_feed_pda,
                program_owned_account(make_price_feed(product_id, oracle_price, 1000, &admin)),
            ),
            (user, system_account(100_000_000_000)),
            system_program_account(),
        ],
    );
    assert!(
        result.program_result.is_ok(),
        "Exact min position size should succeed: {:?}",
        result.program_result
    );
}

// ══════════════════════════════════════════════════════════════════════════════
// STRESS TEST 8: Corrupted account discriminators
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn stress_corrupted_discriminators() {
    let mollusk = setup_mollusk();
    let pid = program_id();
    let publisher = Pubkey::new_unique();
    let product_id: u32 = 1;

    let (feed_pda, _) = find_pda(&[PRICE_FEED_SEED, &product_id.to_le_bytes()]);
    let (publisher_list_pda, _) = find_pda(&[PUBLISHER_LIST_SEED]);

    // Corrupt the price feed discriminator
    let mut bad_feed = make_price_feed(product_id, 1_000_000_000, 1000, &publisher);
    bad_feed[0] = 0xFF; // corrupt first byte of discriminator

    let mut ix_data = DISC_PUBLISH_PRICE.to_vec();
    ix_data.extend_from_slice(&product_id.to_le_bytes());
    ix_data.extend_from_slice(&1_000_000_000u64.to_le_bytes());

    let result = mollusk.process_instruction(
        &Instruction {
            program_id: pid,
            accounts: vec![
                AccountMeta::new(publisher, true),
                AccountMeta::new(feed_pda, false),
                AccountMeta::new_readonly(publisher_list_pda, false),
            ],
            data: ix_data,
        },
        &[
            (publisher, system_account(1_000_000_000)),
            (feed_pda, program_owned_account(bad_feed)),
            (
                publisher_list_pda,
                program_owned_account(make_publisher_list(&publisher, &[publisher])),
            ),
        ],
    );
    assert!(
        result.program_result.is_err(),
        "Corrupted discriminator should fail"
    );

    // Corrupt publisher list discriminator
    let mut bad_pl = make_publisher_list(&publisher, &[publisher]);
    bad_pl[3] = 0xFF;

    let mut ix_data = DISC_PUBLISH_PRICE.to_vec();
    ix_data.extend_from_slice(&product_id.to_le_bytes());
    ix_data.extend_from_slice(&1_000_000_000u64.to_le_bytes());

    let result = mollusk.process_instruction(
        &Instruction {
            program_id: pid,
            accounts: vec![
                AccountMeta::new(publisher, true),
                AccountMeta::new(feed_pda, false),
                AccountMeta::new_readonly(publisher_list_pda, false),
            ],
            data: ix_data,
        },
        &[
            (publisher, system_account(1_000_000_000)),
            (
                feed_pda,
                program_owned_account(make_price_feed(product_id, 0, 0, &publisher)),
            ),
            (publisher_list_pda, program_owned_account(bad_pl)),
        ],
    );
    assert!(
        result.program_result.is_err(),
        "Corrupted publisher list discriminator should fail"
    );
}

// ══════════════════════════════════════════════════════════════════════════════
// STRESS TEST 9: Account with insufficient data
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn stress_undersized_account_data() {
    let mollusk = setup_mollusk();
    let pid = program_id();
    let publisher = Pubkey::new_unique();
    let product_id: u32 = 1;

    let (feed_pda, _) = find_pda(&[PRICE_FEED_SEED, &product_id.to_le_bytes()]);
    let (publisher_list_pda, _) = find_pda(&[PUBLISHER_LIST_SEED]);

    // Price feed with too little data
    let short_data = vec![0u8; 10]; // way too short (needs 68)
    let mut ix_data = DISC_PUBLISH_PRICE.to_vec();
    ix_data.extend_from_slice(&product_id.to_le_bytes());
    ix_data.extend_from_slice(&1_000_000_000u64.to_le_bytes());

    let result = mollusk.process_instruction(
        &Instruction {
            program_id: pid,
            accounts: vec![
                AccountMeta::new(publisher, true),
                AccountMeta::new(feed_pda, false),
                AccountMeta::new_readonly(publisher_list_pda, false),
            ],
            data: ix_data,
        },
        &[
            (publisher, system_account(1_000_000_000)),
            (feed_pda, program_owned_account(short_data)),
            (
                publisher_list_pda,
                program_owned_account(make_publisher_list(&publisher, &[publisher])),
            ),
        ],
    );
    assert!(
        result.program_result.is_err(),
        "Undersized account should fail"
    );
}

// ══════════════════════════════════════════════════════════════════════════════
// STRESS TEST 10: Repeated batch publish at maximum (10 feeds)
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn stress_max_batch_publish() {
    let mollusk = setup_mollusk();
    let pid = program_id();
    let publisher = Pubkey::new_unique();
    let (publisher_list_pda, _) = find_pda(&[PUBLISHER_LIST_SEED]);

    let publisher_list = program_owned_account(make_publisher_list(&publisher, &[publisher]));

    let count: u32 = 10; // maximum batch size

    let mut ix_data = DISC_PUBLISH_PRICES_BATCH.to_vec();
    ix_data.extend_from_slice(&count.to_le_bytes());

    let mut accounts_meta = vec![
        AccountMeta::new(publisher, true),
        AccountMeta::new_readonly(publisher_list_pda, false),
    ];
    let mut accounts_data: Vec<(Pubkey, Account)> = vec![
        (publisher, system_account(1_000_000_000)),
        (publisher_list_pda, publisher_list),
    ];

    for i in 1..=count {
        let pid_val = i;
        ix_data.extend_from_slice(&pid_val.to_le_bytes());
        ix_data.extend_from_slice(&((i as u64) * 1_000_000_000).to_le_bytes());

        let (feed_pda, _) = find_pda(&[PRICE_FEED_SEED, &pid_val.to_le_bytes()]);
        accounts_meta.push(AccountMeta::new(feed_pda, false));
        accounts_data.push((
            feed_pda,
            program_owned_account(make_price_feed(pid_val, 0, 0, &publisher)),
        ));
    }

    let result = mollusk.process_instruction(
        &Instruction {
            program_id: pid,
            accounts: accounts_meta,
            data: ix_data,
        },
        &accounts_data,
    );
    assert!(
        result.program_result.is_ok(),
        "Max batch publish (10) failed: {:?}",
        result.program_result
    );
}

// ══════════════════════════════════════════════════════════════════════════════
// STRESS TEST 11: Exchange state with near-max cumulative values
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn stress_near_max_exchange_state_values() {
    let mut mollusk = setup_mollusk();
    spl_token_program::add_program(&mut mollusk);

    let pid = program_id();
    let admin = Pubkey::new_unique();
    let user = Pubkey::new_unique();
    let usdc_mint = Pubkey::new_unique();

    let (exchange_pda, _) = find_pda(&[EXCHANGE_SEED]);
    let (vault_pda, _) = find_pda(&[VAULT_SEED]);
    let (insurance_pda, _) = find_pda(&[INSURANCE_SEED]);
    let (user_account_pda, _) = find_pda(&[USER_ACCOUNT_SEED, user.as_ref()]);
    let user_token = Pubkey::new_unique();

    // Exchange state with near-max total_deposits - can we still deposit?
    let exchange_data = make_exchange_state_full(
        &admin,
        &usdc_mint,
        &vault_pda,
        &insurance_pda,
        199,                       // near max markets
        u64::MAX - 1_000_000,      // near max total_deposits
        u128::MAX - 1_000_000_000, // near max total_volume
        u64::MAX - 1_000_000,      // near max total_fees
        0,                         // insurance
        false,                     // not paused
    );

    let user_account_data = make_user_account(&user, 0, 0);
    let deposit_amount: u64 = 1_000_000;

    let mut ix_data = DISC_DEPOSIT.to_vec();
    ix_data.extend_from_slice(&deposit_amount.to_le_bytes());

    let result = mollusk.process_instruction(
        &Instruction {
            program_id: pid,
            accounts: vec![
                AccountMeta::new(user_account_pda, false),
                AccountMeta::new(exchange_pda, false),
                AccountMeta::new(vault_pda, false),
                AccountMeta::new(user_token, false),
                AccountMeta::new(user, true),
                AccountMeta::new_readonly(spl_token_id(), false),
            ],
            data: ix_data,
        },
        &[
            (user_account_pda, program_owned_account(user_account_data)),
            (exchange_pda, program_owned_account(exchange_data)),
            (
                vault_pda,
                token_account_obj(make_token_account(&usdc_mint, &exchange_pda, 0)),
            ),
            (
                user_token,
                token_account_obj(make_token_account(&usdc_mint, &user, 100_000_000)),
            ),
            (user, system_account(1_000_000_000)),
            spl_token_program::keyed_account(),
        ],
    );
    // Should succeed since we're still just below max
    assert!(
        result.program_result.is_ok(),
        "Near-max deposit should still succeed: {:?}",
        result.program_result
    );
}

// ══════════════════════════════════════════════════════════════════════════════
// STRESS TEST 12: Oracle price at exact staleness boundary
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn stress_oracle_staleness_boundary() {
    let mut mollusk = setup_mollusk();
    mollusk.sysvars.warp_to_slot(100);
    mollusk.sysvars.clock.unix_timestamp = 10_000;

    let pid = program_id();
    let admin = Pubkey::new_unique();
    let user = Pubkey::new_unique();
    let usdc_mint = Pubkey::new_unique();
    let product_id: u32 = 1;

    let (exchange_pda, _) = find_pda(&[EXCHANGE_SEED]);
    let (vault_pda, _) = find_pda(&[VAULT_SEED]);
    let (insurance_pda, _) = find_pda(&[INSURANCE_SEED]);
    let (user_account_pda, _) = find_pda(&[USER_ACCOUNT_SEED, user.as_ref()]);
    let (market_pda, _) = find_pda(&[MARKET_SEED, &product_id.to_le_bytes()]);
    let (price_feed_pda, _) = find_pda(&[PRICE_FEED_SEED, &product_id.to_le_bytes()]);
    let (position_pda, _) = find_pda(&[POSITION_SEED, user.as_ref(), &product_id.to_le_bytes()]);

    let exchange_data = make_exchange_state(&admin, &usdc_mint, &vault_pda, &insurance_pda);
    let market_data = make_market_with_oi(
        product_id,
        &price_feed_pda,
        true,
        10,
        1_000_000_000_000,
        0,
        0,
        0,
        0,
        0,
    );
    let user_account_data = make_user_account(&user, 100_000_000, 0);

    let make_open_ix = |product_id: u32| {
        let mut data = DISC_OPEN_POSITION.to_vec();
        data.extend_from_slice(&product_id.to_le_bytes());
        data.push(0);
        data.extend_from_slice(&1_000_000u64.to_le_bytes());
        data.extend_from_slice(&1u32.to_le_bytes());
        Instruction {
            program_id: pid,
            accounts: vec![
                AccountMeta::new(user_account_pda, false),
                AccountMeta::new(position_pda, false),
                AccountMeta::new(market_pda, false),
                AccountMeta::new(exchange_pda, false),
                AccountMeta::new_readonly(price_feed_pda, false),
                AccountMeta::new(user, true),
                AccountMeta::new_readonly(system_program::ID, false),
            ],
            data,
        }
    };

    // Exactly at boundary: age = 600 seconds (MAX_ORACLE_PRICE_AGE)
    // current_time = 10000, last_updated = 9400 -> age = 600
    // MAX_ORACLE_PRICE_AGE is 120 seconds (reduced from 600 for security)
    let just_at_boundary: i64 = 10_000 - 120;
    let result = mollusk.process_instruction(
        &make_open_ix(product_id),
        &[
            (
                user_account_pda,
                program_owned_account(user_account_data.clone()),
            ),
            (position_pda, system_account(0)),
            (market_pda, program_owned_account(market_data.clone())),
            (exchange_pda, program_owned_account(exchange_data.clone())),
            (
                price_feed_pda,
                program_owned_account(make_price_feed(
                    product_id,
                    1_000_000_000,
                    just_at_boundary,
                    &admin,
                )),
            ),
            (user, system_account(100_000_000_000)),
            system_program_account(),
        ],
    );
    assert!(
        result.program_result.is_ok(),
        "Oracle at exactly MAX_AGE should still succeed: {:?}",
        result.program_result
    );

    // Just past boundary: age = 121 seconds (MAX_ORACLE_PRICE_AGE + 1)
    let just_past_boundary: i64 = 10_000 - 121;
    let result = mollusk.process_instruction(
        &make_open_ix(product_id),
        &[
            (user_account_pda, program_owned_account(user_account_data)),
            (position_pda, system_account(0)),
            (market_pda, program_owned_account(market_data)),
            (exchange_pda, program_owned_account(exchange_data)),
            (
                price_feed_pda,
                program_owned_account(make_price_feed(
                    product_id,
                    1_000_000_000,
                    just_past_boundary,
                    &admin,
                )),
            ),
            (user, system_account(100_000_000_000)),
            system_program_account(),
        ],
    );
    assert!(
        result.program_result.is_err(),
        "Oracle past MAX_AGE should fail"
    );
}

// ══════════════════════════════════════════════════════════════════════════════
// STRESS TEST 13: Funding rate with consecutive updates
// Verify cumulative rates accumulate correctly over multiple intervals
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn stress_consecutive_funding_updates() {
    let mut mollusk = setup_mollusk();
    let pid = program_id();
    let cranker = Pubkey::new_unique();
    let admin = Pubkey::new_unique();
    let usdc_mint = Pubkey::new_unique();
    let vault = Pubkey::new_unique();
    let insurance = Pubkey::new_unique();
    let product_id: u32 = 1;

    let (exchange_pda, _) = find_pda(&[EXCHANGE_SEED]);
    let (market_pda, _) = find_pda(&[MARKET_SEED, &product_id.to_le_bytes()]);
    let oracle_feed = Pubkey::new_unique();

    let exchange_data =
        program_owned_account(make_exchange_state(&admin, &usdc_mint, &vault, &insurance));

    // Balanced OI -> funding rate should be 0
    let market_data = make_market_with_oi(
        product_id,
        &oracle_feed,
        true,
        10,
        u64::MAX,
        100_000_000,
        100_000_000, // equal OI
        0,
        0,
        0,
    );
    let mut current_market = program_owned_account(market_data);

    // Run 5 consecutive funding updates with 28800s intervals
    for i in 1..=5u64 {
        let timestamp = (i * 28_800) as i64 + 1;
        mollusk.sysvars.warp_to_slot(i * 100);
        mollusk.sysvars.clock.unix_timestamp = timestamp;

        let mut ix_data = DISC_UPDATE_FUNDING_RATE.to_vec();
        ix_data.extend_from_slice(&product_id.to_le_bytes());

        let result = mollusk.process_instruction(
            &Instruction {
                program_id: pid,
                accounts: vec![
                    AccountMeta::new(market_pda, false),
                    AccountMeta::new_readonly(exchange_pda, false),
                    AccountMeta::new_readonly(cranker, true),
                ],
                data: ix_data,
            },
            &[
                (market_pda, current_market.clone()),
                (exchange_pda, exchange_data.clone()),
                (cranker, system_account(1_000_000_000)),
            ],
        );
        assert!(
            result.program_result.is_ok(),
            "Funding update {} failed: {:?}",
            i,
            result.program_result
        );
        current_market = result
            .resulting_accounts
            .iter()
            .find(|(k, _)| *k == market_pda)
            .unwrap()
            .1
            .clone();
    }

    // With balanced OI, cumulative rates should remain at 0
    assert_eq!(read_i64_at(&current_market.data, 96), 0); // cum_long
    assert_eq!(read_i64_at(&current_market.data, 104), 0); // cum_short
}

// ══════════════════════════════════════════════════════════════════════════════
// STRESS TEST 14: Withdraw exact balance
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn stress_withdraw_exact_balance() {
    let mut mollusk = setup_mollusk();
    spl_token_program::add_program(&mut mollusk);

    let pid = program_id();
    let admin = Pubkey::new_unique();
    let user = Pubkey::new_unique();
    let usdc_mint = Pubkey::new_unique();

    let (exchange_pda, _) = find_pda(&[EXCHANGE_SEED]);
    let (vault_pda, _) = find_pda(&[VAULT_SEED]);
    let (insurance_pda, _) = find_pda(&[INSURANCE_SEED]);
    let (user_account_pda, _) = find_pda(&[USER_ACCOUNT_SEED, user.as_ref()]);
    let user_token = Pubkey::new_unique();

    let balance: u64 = 42_123_456; // odd amount

    let exchange_data = make_exchange_state(&admin, &usdc_mint, &vault_pda, &insurance_pda);
    let user_account_data = make_user_account(&user, balance, 0);

    let mut ix_data = DISC_WITHDRAW.to_vec();
    ix_data.extend_from_slice(&balance.to_le_bytes()); // exact balance

    let result = mollusk.process_instruction(
        &Instruction {
            program_id: pid,
            accounts: vec![
                AccountMeta::new(user_account_pda, false),
                AccountMeta::new(exchange_pda, false),
                AccountMeta::new(vault_pda, false),
                AccountMeta::new(user_token, false),
                AccountMeta::new(user, true),
                AccountMeta::new_readonly(spl_token_id(), false),
            ],
            data: ix_data,
        },
        &[
            (user_account_pda, program_owned_account(user_account_data)),
            (exchange_pda, program_owned_account(exchange_data)),
            (
                vault_pda,
                token_account_obj(make_token_account(
                    &usdc_mint,
                    &exchange_pda,
                    balance + 10_000_000,
                )),
            ),
            (
                user_token,
                token_account_obj(make_token_account(&usdc_mint, &user, 0)),
            ),
            (user, system_account(1_000_000_000)),
            spl_token_program::keyed_account(),
        ],
    );
    assert!(
        result.program_result.is_ok(),
        "Withdraw exact balance should succeed: {:?}",
        result.program_result
    );

    let user_data = &result
        .resulting_accounts
        .iter()
        .find(|(k, _)| *k == user_account_pda)
        .unwrap()
        .1
        .data;
    assert_eq!(read_u64_at(user_data, 40), 0); // zero balance
}

// ══════════════════════════════════════════════════════════════════════════════
// STRESS TEST 15: Multiple feed creations with sequential product IDs
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn stress_sequential_feed_creation() {
    let mollusk = setup_mollusk();
    let pid = program_id();
    let authority = Pubkey::new_unique();

    let (oracle_state_pda, _) = find_pda(&[ORACLE_SEED]);
    let mut oracle_state_acc = program_owned_account(make_oracle_state(&authority));

    // Create 20 feeds sequentially
    for i in 1..=20u32 {
        let (feed_pda, _) = find_pda(&[PRICE_FEED_SEED, &i.to_le_bytes()]);

        let result = mollusk.process_instruction(
            &Instruction {
                program_id: pid,
                accounts: vec![
                    AccountMeta::new(feed_pda, false),
                    AccountMeta::new(oracle_state_pda, false),
                    AccountMeta::new(authority, true),
                    AccountMeta::new_readonly(system_program::ID, false),
                ],
                data: build_ix_data(&DISC_CREATE_FEED, &i.to_le_bytes()),
            },
            &[
                (feed_pda, system_account(0)),
                (oracle_state_pda, oracle_state_acc.clone()),
                (authority, system_account(100_000_000_000)),
                system_program_account(),
            ],
        );
        assert!(
            result.program_result.is_ok(),
            "Create feed {} failed: {:?}",
            i,
            result.program_result
        );

        oracle_state_acc = result
            .resulting_accounts
            .iter()
            .find(|(k, _)| *k == oracle_state_pda)
            .unwrap()
            .1
            .clone();

        // Verify feed count incremented
        assert_eq!(read_u32_at(&oracle_state_acc.data, 72), i);
    }
}

// ══════════════════════════════════════════════════════════════════════════════
// STRESS TEST 16: Insufficient balance edge cases for open position
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn stress_insufficient_balance_for_open_position() {
    let mut mollusk = setup_mollusk();
    mollusk.sysvars.warp_to_slot(100);
    mollusk.sysvars.clock.unix_timestamp = 1000;

    let pid = program_id();
    let admin = Pubkey::new_unique();
    let user = Pubkey::new_unique();
    let usdc_mint = Pubkey::new_unique();
    let product_id: u32 = 1;

    let (exchange_pda, _) = find_pda(&[EXCHANGE_SEED]);
    let (vault_pda, _) = find_pda(&[VAULT_SEED]);
    let (insurance_pda, _) = find_pda(&[INSURANCE_SEED]);
    let (user_account_pda, _) = find_pda(&[USER_ACCOUNT_SEED, user.as_ref()]);
    let (market_pda, _) = find_pda(&[MARKET_SEED, &product_id.to_le_bytes()]);
    let (price_feed_pda, _) = find_pda(&[PRICE_FEED_SEED, &product_id.to_le_bytes()]);
    let (position_pda, _) = find_pda(&[POSITION_SEED, user.as_ref(), &product_id.to_le_bytes()]);

    let oracle_price: u64 = 1_000_000_000;
    let exchange_data = make_exchange_state(&admin, &usdc_mint, &vault_pda, &insurance_pda);
    let market_data = make_market_with_oi(
        product_id,
        &price_feed_pda,
        true,
        10,
        1_000_000_000_000,
        0,
        0,
        0,
        0,
        0,
    );

    // Position size = $10 with 5x leverage -> margin = $2, fee = $0.005, total = $2.005
    // User has exactly $2 -> not enough (need margin + fee)
    let position_size: u64 = 10_000_000;
    let leverage: u32 = 5;
    let user_balance: u64 = 2_000_000; // exactly margin, no room for fee

    let user_account_data = make_user_account(&user, user_balance, 0);

    let mut ix_data = DISC_OPEN_POSITION.to_vec();
    ix_data.extend_from_slice(&product_id.to_le_bytes());
    ix_data.push(0);
    ix_data.extend_from_slice(&position_size.to_le_bytes());
    ix_data.extend_from_slice(&leverage.to_le_bytes());

    let result = mollusk.process_instruction(
        &Instruction {
            program_id: pid,
            accounts: vec![
                AccountMeta::new(user_account_pda, false),
                AccountMeta::new(position_pda, false),
                AccountMeta::new(market_pda, false),
                AccountMeta::new(exchange_pda, false),
                AccountMeta::new_readonly(price_feed_pda, false),
                AccountMeta::new(user, true),
                AccountMeta::new_readonly(system_program::ID, false),
            ],
            data: ix_data,
        },
        &[
            (user_account_pda, program_owned_account(user_account_data)),
            (position_pda, system_account(0)),
            (market_pda, program_owned_account(market_data)),
            (exchange_pda, program_owned_account(exchange_data)),
            (
                price_feed_pda,
                program_owned_account(make_price_feed(product_id, oracle_price, 1000, &admin)),
            ),
            (user, system_account(100_000_000_000)),
            system_program_account(),
        ],
    );
    // margin = 2_000_000, fee = 10_000_000 * 5 / 10_000 = 5_000
    // total = 2_005_000 > 2_000_000 balance
    assert!(
        result.program_result.is_err(),
        "Insufficient balance should fail (need margin + fee)"
    );
}
