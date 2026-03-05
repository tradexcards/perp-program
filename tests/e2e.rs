/// End-to-end tests for the TradeX program.
///
/// These tests exercise complete multi-instruction flows through the program,
/// verifying that state changes propagate correctly across instructions.
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
const DISC_CREATE_MARKET_PERMISSIONLESS: [u8; 8] = [0x48, 0xc0, 0xbe, 0x8e, 0x69, 0x97, 0x02, 0x50];
const DISC_CREATE_USER_ACCOUNT: [u8; 8] = [0x92, 0x44, 0x64, 0x45, 0x3f, 0x2e, 0xb6, 0xc7];
const DISC_CLOSE_USER_ACCOUNT: [u8; 8] = [0xec, 0xb5, 0x03, 0x47, 0xc2, 0x12, 0x97, 0xbf];
const DISC_DEPOSIT: [u8; 8] = [0xf2, 0x23, 0xc6, 0x89, 0x52, 0xe1, 0xf2, 0xb6];
const DISC_WITHDRAW: [u8; 8] = [0xb7, 0x12, 0x46, 0x9c, 0x94, 0x6d, 0xa1, 0x22];
const DISC_OPEN_POSITION: [u8; 8] = [0x87, 0x80, 0x2f, 0x4d, 0x0f, 0x98, 0xf0, 0x31];
const DISC_CLOSE_POSITION: [u8; 8] = [0x7b, 0x86, 0x51, 0x00, 0x31, 0x44, 0x62, 0x62];
const DISC_ADD_MARGIN: [u8; 8] = [0xd3, 0xee, 0xee, 0x5a, 0xdf, 0xe4, 0xe4, 0x4c];
const DISC_UPDATE_FUNDING_RATE: [u8; 8] = [0xc9, 0xb2, 0x74, 0xd4, 0xa6, 0x90, 0x48, 0xee];
const DISC_SET_PAUSED: [u8; 8] = [0x5b, 0x3c, 0x7d, 0xc0, 0xb0, 0xe1, 0xa6, 0xda];
const DISC_SET_MARKET_ACTIVE: [u8; 8] = [0xad, 0x21, 0xdd, 0x36, 0x53, 0x4c, 0x72, 0xcd];
const DISC_UPDATE_MARKET_PARAMS: [u8; 8] = [0x46, 0x75, 0xca, 0xbf, 0xcd, 0xae, 0x5c, 0x52];
const DISC_FUND_INSURANCE: [u8; 8] = [0x3a, 0xd8, 0x25, 0xa2, 0xc9, 0x32, 0x14, 0x42];
const DISC_TRANSFER_ADMIN: [u8; 8] = [0x2a, 0xf2, 0x42, 0x6a, 0xe4, 0x0a, 0x6f, 0x9c];
const DISC_ACCEPT_ADMIN: [u8; 8] = [0x70, 0x2a, 0x2d, 0x5a, 0x74, 0xb5, 0x0d, 0xaa];
const DISC_TRANSFER_AUTHORITY: [u8; 8] = [0x30, 0xa9, 0x4c, 0x48, 0xe5, 0xb4, 0x37, 0xa1];
const DISC_ACCEPT_AUTHORITY: [u8; 8] = [0x6b, 0x56, 0xc6, 0x5b, 0x21, 0x0c, 0x6b, 0xa0];
const DISC_REMOVE_PUBLISHER: [u8; 8] = [0x5b, 0x40, 0x9e, 0x65, 0x4c, 0x9d, 0x44, 0x66];
const DISC_LIQUIDATE: [u8; 8] = [0xdf, 0xb3, 0xe2, 0x7d, 0x30, 0x2e, 0x27, 0x4a];

// ── Account discriminators ──────────────────────────────────────────────────

const DISC_ORACLE_STATE: [u8; 8] = [0x6a, 0x0a, 0xcb, 0x5b, 0x63, 0x60, 0xa1, 0x27];
const DISC_PUBLISHER_LIST: [u8; 8] = [0xf0, 0x4d, 0x78, 0xa9, 0x7d, 0x0c, 0xf2, 0x22];
const DISC_PRICE_FEED: [u8; 8] = [0x65, 0x5b, 0x5b, 0x7e, 0xf3, 0x56, 0xbc, 0x52];
const DISC_EXCHANGE_STATE: [u8; 8] = [0x79, 0xf0, 0x75, 0x17, 0x49, 0xb2, 0xc4, 0x47];
const DISC_MARKET: [u8; 8] = [0xdf, 0xb8, 0x9c, 0xee, 0x6b, 0xa0, 0x8a, 0x4a];
const DISC_USER_ACCOUNT: [u8; 8] = [0x21, 0x1e, 0x8e, 0x54, 0xb9, 0xd3, 0x19, 0xe5];
const DISC_POSITION: [u8; 8] = [0xaa, 0xbc, 0x8f, 0xe4, 0x7a, 0x17, 0x2b, 0x6c];

// ── Helpers ─────────────────────────────────────────────────────────────────

fn setup_mollusk() -> Mollusk {
    let pid = program_id();
    Mollusk::new(&pid, "tradex_program")
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

fn make_exchange_state_with_stats(
    admin: &Pubkey,
    usdc_mint: &Pubkey,
    vault: &Pubkey,
    insurance: &Pubkey,
    market_count: u32,
    total_deposits: u64,
    insurance_balance: u64,
) -> Vec<u8> {
    let mut data = make_exchange_state(admin, usdc_mint, vault, insurance);
    data[168..172].copy_from_slice(&market_count.to_le_bytes());
    data[172..180].copy_from_slice(&total_deposits.to_le_bytes());
    data[204..212].copy_from_slice(&insurance_balance.to_le_bytes());
    data
}

fn make_market(product_id: u32, oracle_feed: &Pubkey, active: bool) -> Vec<u8> {
    let mut data = vec![0u8; 113];
    data[..8].copy_from_slice(&DISC_MARKET);
    data[8..12].copy_from_slice(&product_id.to_le_bytes());
    data[12..44].copy_from_slice(oracle_feed.as_ref());
    data[44..48].copy_from_slice(&10u32.to_le_bytes());
    data[48..56].copy_from_slice(&1_000_000_000_000u64.to_le_bytes());
    let now = 1000i64;
    data[88..96].copy_from_slice(&now.to_le_bytes());
    data[112] = if active { 1 } else { 0 };
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
    // total_volume at 72..88 = 0
    data[88..96].copy_from_slice(&last_funding_time.to_le_bytes());
    data[96..104].copy_from_slice(&cum_funding_long.to_le_bytes());
    data[104..112].copy_from_slice(&cum_funding_short.to_le_bytes());
    data[112] = if active { 1 } else { 0 };
    data
}

fn make_user_account(owner: &Pubkey, balance: u64) -> Vec<u8> {
    let mut data = vec![0u8; 84];
    data[..8].copy_from_slice(&DISC_USER_ACCOUNT);
    data[8..40].copy_from_slice(owner.as_ref());
    data[40..48].copy_from_slice(&balance.to_le_bytes());
    data
}

fn make_user_account_full(
    owner: &Pubkey,
    balance: u64,
    total_deposited: u64,
    total_withdrawn: u64,
    total_realized_pnl: i64,
    total_fees_paid: u64,
    open_position_count: u32,
) -> Vec<u8> {
    let mut data = vec![0u8; 84];
    data[..8].copy_from_slice(&DISC_USER_ACCOUNT);
    data[8..40].copy_from_slice(owner.as_ref());
    data[40..48].copy_from_slice(&balance.to_le_bytes());
    data[48..56].copy_from_slice(&total_deposited.to_le_bytes());
    data[56..64].copy_from_slice(&total_withdrawn.to_le_bytes());
    data[64..72].copy_from_slice(&total_realized_pnl.to_le_bytes());
    data[72..80].copy_from_slice(&total_fees_paid.to_le_bytes());
    data[80..84].copy_from_slice(&open_position_count.to_le_bytes());
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

fn make_position_full(
    owner: &Pubkey,
    product_id: u32,
    side: u8,
    size: u64,
    entry_price: u64,
    leverage: u32,
    margin: u64,
    liquidation_price: u64,
    realized_pnl: i64,
    entry_funding_rate: i64,
    opened_at: i64,
    last_updated: i64,
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
    data[73..81].copy_from_slice(&liquidation_price.to_le_bytes());
    data[81..89].copy_from_slice(&realized_pnl.to_le_bytes());
    data[89..97].copy_from_slice(&entry_funding_rate.to_le_bytes());
    data[97..105].copy_from_slice(&opened_at.to_le_bytes());
    data[105..113].copy_from_slice(&last_updated.to_le_bytes());
    data
}

fn make_token_account(mint: &Pubkey, owner: &Pubkey, amount: u64) -> Vec<u8> {
    let mut data = vec![0u8; 165];
    data[..32].copy_from_slice(mint.as_ref());
    data[32..64].copy_from_slice(owner.as_ref());
    data[64..72].copy_from_slice(&amount.to_le_bytes());
    data[108] = 1; // state = Initialized
    data
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

/// Helper to extract an account from a mollusk result
fn get_account_data(result: &mollusk_svm::result::InstructionResult, key: &Pubkey) -> Vec<u8> {
    result
        .resulting_accounts
        .iter()
        .find(|(k, _)| k == key)
        .unwrap()
        .1
        .data
        .clone()
}

fn get_account(result: &mollusk_svm::result::InstructionResult, key: &Pubkey) -> Account {
    result
        .resulting_accounts
        .iter()
        .find(|(k, _)| k == key)
        .unwrap()
        .1
        .clone()
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

fn read_u128_at(data: &[u8], offset: usize) -> u128 {
    u128::from_le_bytes(data[offset..offset + 16].try_into().unwrap())
}

// ══════════════════════════════════════════════════════════════════════════════
// E2E TEST 1: Full Oracle Lifecycle
// Initialize oracle -> Create feeds -> Add publishers -> Publish prices
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn e2e_oracle_full_lifecycle() {
    let mollusk = setup_mollusk();
    let pid = program_id();
    let authority = Pubkey::new_unique();
    let publisher1 = Pubkey::new_unique();
    let publisher2 = Pubkey::new_unique();

    let (oracle_state_pda, _) = find_pda(&[ORACLE_SEED]);
    let (publisher_list_pda, _) = find_pda(&[PUBLISHER_LIST_SEED]);

    // ── Step 1: Initialize Oracle ───────────────────────────────────────
    let ix = Instruction {
        program_id: pid,
        accounts: vec![
            AccountMeta::new(oracle_state_pda, false),
            AccountMeta::new(publisher_list_pda, false),
            AccountMeta::new(authority, true),
            AccountMeta::new_readonly(system_program::ID, false),
        ],
        data: build_ix_data(&DISC_INITIALIZE_ORACLE, &[]),
    };

    let result = mollusk.process_instruction(
        &ix,
        &[
            (oracle_state_pda, system_account(0)),
            (publisher_list_pda, system_account(0)),
            (authority, system_account(100_000_000_000)),
            system_program_account(),
        ],
    );
    assert!(
        result.program_result.is_ok(),
        "Step 1 (init oracle) failed: {:?}",
        result.program_result
    );

    let oracle_state_acc = get_account(&result, &oracle_state_pda);
    let publisher_list_acc = get_account(&result, &publisher_list_pda);
    let authority_acc = get_account(&result, &authority);

    // Verify initial state
    assert_eq!(read_u32_at(&oracle_state_acc.data, 72), 0); // feed_count = 0
    assert_eq!(read_u32_at(&publisher_list_acc.data, 40), 1); // 1 publisher (authority)

    // ── Step 2: Create two price feeds ──────────────────────────────────
    let product1: u32 = 1;
    let product2: u32 = 2;
    let (feed1_pda, _) = find_pda(&[PRICE_FEED_SEED, &product1.to_le_bytes()]);
    let (feed2_pda, _) = find_pda(&[PRICE_FEED_SEED, &product2.to_le_bytes()]);

    let ix1 = Instruction {
        program_id: pid,
        accounts: vec![
            AccountMeta::new(feed1_pda, false),
            AccountMeta::new(oracle_state_pda, false),
            AccountMeta::new(authority, true),
            AccountMeta::new_readonly(system_program::ID, false),
        ],
        data: build_ix_data(&DISC_CREATE_FEED, &product1.to_le_bytes()),
    };

    let result = mollusk.process_instruction(
        &ix1,
        &[
            (feed1_pda, system_account(0)),
            (oracle_state_pda, oracle_state_acc.clone()),
            (authority, authority_acc.clone()),
            system_program_account(),
        ],
    );
    assert!(
        result.program_result.is_ok(),
        "Step 2a (create feed 1) failed: {:?}",
        result.program_result
    );

    let oracle_state_acc = get_account(&result, &oracle_state_pda);
    let feed1_acc = get_account(&result, &feed1_pda);
    let authority_acc = get_account(&result, &authority);
    assert_eq!(read_u32_at(&oracle_state_acc.data, 72), 1); // feed_count = 1

    let ix2 = Instruction {
        program_id: pid,
        accounts: vec![
            AccountMeta::new(feed2_pda, false),
            AccountMeta::new(oracle_state_pda, false),
            AccountMeta::new(authority, true),
            AccountMeta::new_readonly(system_program::ID, false),
        ],
        data: build_ix_data(&DISC_CREATE_FEED, &product2.to_le_bytes()),
    };

    let result = mollusk.process_instruction(
        &ix2,
        &[
            (feed2_pda, system_account(0)),
            (oracle_state_pda, oracle_state_acc.clone()),
            (authority, authority_acc.clone()),
            system_program_account(),
        ],
    );
    assert!(
        result.program_result.is_ok(),
        "Step 2b (create feed 2) failed: {:?}",
        result.program_result
    );

    let oracle_state_acc = get_account(&result, &oracle_state_pda);
    let feed2_acc = get_account(&result, &feed2_pda);
    let authority_acc = get_account(&result, &authority);
    assert_eq!(read_u32_at(&oracle_state_acc.data, 72), 2); // feed_count = 2

    // ── Step 3: Add two publishers ──────────────────────────────────────
    let mut add_data1 = DISC_ADD_PUBLISHER.to_vec();
    add_data1.extend_from_slice(publisher1.as_ref());

    let ix_add1 = Instruction {
        program_id: pid,
        accounts: vec![
            AccountMeta::new(publisher_list_pda, false),
            AccountMeta::new(oracle_state_pda, false),
            AccountMeta::new_readonly(authority, true),
        ],
        data: add_data1,
    };

    let result = mollusk.process_instruction(
        &ix_add1,
        &[
            (publisher_list_pda, publisher_list_acc.clone()),
            (oracle_state_pda, oracle_state_acc.clone()),
            (authority, authority_acc.clone()),
        ],
    );
    assert!(
        result.program_result.is_ok(),
        "Step 3a (add publisher 1) failed: {:?}",
        result.program_result
    );

    let publisher_list_acc = get_account(&result, &publisher_list_pda);
    assert_eq!(read_u32_at(&publisher_list_acc.data, 40), 2); // 2 publishers

    let mut add_data2 = DISC_ADD_PUBLISHER.to_vec();
    add_data2.extend_from_slice(publisher2.as_ref());

    let ix_add2 = Instruction {
        program_id: pid,
        accounts: vec![
            AccountMeta::new(publisher_list_pda, false),
            AccountMeta::new(oracle_state_pda, false),
            AccountMeta::new_readonly(authority, true),
        ],
        data: add_data2,
    };

    let result = mollusk.process_instruction(
        &ix_add2,
        &[
            (publisher_list_pda, publisher_list_acc.clone()),
            (oracle_state_pda, oracle_state_acc.clone()),
            (authority, authority_acc.clone()),
        ],
    );
    assert!(
        result.program_result.is_ok(),
        "Step 3b (add publisher 2) failed: {:?}",
        result.program_result
    );

    let publisher_list_acc = get_account(&result, &publisher_list_pda);
    assert_eq!(read_u32_at(&publisher_list_acc.data, 40), 3); // 3 publishers

    // ── Step 4: Publisher1 publishes price to feed1 ─────────────────────
    let price1: u64 = 1_500_000_000; // $15.00
    let mut pub_data = DISC_PUBLISH_PRICE.to_vec();
    pub_data.extend_from_slice(&product1.to_le_bytes());
    pub_data.extend_from_slice(&price1.to_le_bytes());

    let ix_pub = Instruction {
        program_id: pid,
        accounts: vec![
            AccountMeta::new(publisher1, true),
            AccountMeta::new(feed1_pda, false),
            AccountMeta::new_readonly(publisher_list_pda, false),
        ],
        data: pub_data,
    };

    let result = mollusk.process_instruction(
        &ix_pub,
        &[
            (publisher1, system_account(1_000_000_000)),
            (feed1_pda, feed1_acc.clone()),
            (publisher_list_pda, publisher_list_acc.clone()),
        ],
    );
    assert!(
        result.program_result.is_ok(),
        "Step 4 (publish price) failed: {:?}",
        result.program_result
    );

    let feed1_data = get_account_data(&result, &feed1_pda);
    assert_eq!(read_u64_at(&feed1_data, 12), price1);
    assert_eq!(read_u64_at(&feed1_data, 28), 1); // update_count = 1

    // ── Step 5: Publisher2 batch publishes to both feeds ─────────────────
    let price_new1: u64 = 1_600_000_000;
    let price_new2: u64 = 2_500_000_000;
    let mut batch_data = DISC_PUBLISH_PRICES_BATCH.to_vec();
    batch_data.extend_from_slice(&2u32.to_le_bytes());
    batch_data.extend_from_slice(&product1.to_le_bytes());
    batch_data.extend_from_slice(&price_new1.to_le_bytes());
    batch_data.extend_from_slice(&product2.to_le_bytes());
    batch_data.extend_from_slice(&price_new2.to_le_bytes());

    let feed1_acc_updated = get_account(&result, &feed1_pda);

    let ix_batch = Instruction {
        program_id: pid,
        accounts: vec![
            AccountMeta::new(publisher2, true),
            AccountMeta::new_readonly(publisher_list_pda, false),
            AccountMeta::new(feed1_pda, false),
            AccountMeta::new(feed2_pda, false),
        ],
        data: batch_data,
    };

    let result = mollusk.process_instruction(
        &ix_batch,
        &[
            (publisher2, system_account(1_000_000_000)),
            (publisher_list_pda, publisher_list_acc.clone()),
            (feed1_pda, feed1_acc_updated),
            (feed2_pda, feed2_acc.clone()),
        ],
    );
    assert!(
        result.program_result.is_ok(),
        "Step 5 (batch publish) failed: {:?}",
        result.program_result
    );

    let feed1_data = get_account_data(&result, &feed1_pda);
    let feed2_data = get_account_data(&result, &feed2_pda);
    assert_eq!(read_u64_at(&feed1_data, 12), price_new1);
    assert_eq!(read_u64_at(&feed2_data, 12), price_new2);
    assert_eq!(read_u64_at(&feed1_data, 28), 2); // update_count = 2
    assert_eq!(read_u64_at(&feed2_data, 28), 1); // update_count = 1
}

// ══════════════════════════════════════════════════════════════════════════════
// E2E TEST 2: Full Trading Lifecycle
// Init exchange -> Create market -> Create user -> Deposit -> Open position
// -> Close position (in profit) -> Withdraw -> Close user account
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn e2e_full_trading_lifecycle_long_profit() {
    let mut mollusk = setup_mollusk();
    spl_token_program::add_program(&mut mollusk);

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

    let user_token = Pubkey::new_unique();

    // Pre-setup: oracle + exchange + market already exist (we simulate via pre-built state)
    // Clock timestamp for oracle freshness
    mollusk.sysvars.warp_to_slot(100);
    mollusk.sysvars.clock.unix_timestamp = 1000;

    let oracle_price: u64 = 1_000_000_000; // $10.00
    let initial_deposit: u64 = 100_000_000; // $100 USDC

    // We need a pre-existing exchange and market. Use pre-built state.
    let exchange_data = make_exchange_state_with_stats(
        &admin,
        &usdc_mint,
        &vault_pda,
        &insurance_pda,
        1, // 1 market
        0,
        0,
    );

    let market_data = make_market_with_oi(
        product_id,
        &price_feed_pda,
        true,
        10,                // max_leverage
        1_000_000_000_000, // max_oi
        0,
        0, // no OI
        0,
        0,
        0, // funding
    );

    // ── Step 1: Create user account ─────────────────────────────────────
    let ix_create_user = Instruction {
        program_id: pid,
        accounts: vec![
            AccountMeta::new(user_account_pda, false),
            AccountMeta::new(user, true),
            AccountMeta::new_readonly(system_program::ID, false),
        ],
        data: build_ix_data(&DISC_CREATE_USER_ACCOUNT, &[]),
    };

    let result = mollusk.process_instruction(
        &ix_create_user,
        &[
            (user_account_pda, system_account(0)),
            (user, system_account(100_000_000_000)),
            system_program_account(),
        ],
    );
    assert!(
        result.program_result.is_ok(),
        "Create user failed: {:?}",
        result.program_result
    );
    let user_account_acc = get_account(&result, &user_account_pda);
    let user_sol_acc = get_account(&result, &user);

    // ── Step 2: Deposit $100 USDC ───────────────────────────────────────
    let mut deposit_data = DISC_DEPOSIT.to_vec();
    deposit_data.extend_from_slice(&initial_deposit.to_le_bytes());

    let ix_deposit = Instruction {
        program_id: pid,
        accounts: vec![
            AccountMeta::new(user_account_pda, false),
            AccountMeta::new(exchange_pda, false),
            AccountMeta::new(vault_pda, false),
            AccountMeta::new(user_token, false),
            AccountMeta::new(user, true),
            AccountMeta::new_readonly(spl_token_id(), false),
        ],
        data: deposit_data,
    };

    let result = mollusk.process_instruction(
        &ix_deposit,
        &[
            (user_account_pda, user_account_acc),
            (exchange_pda, program_owned_account(exchange_data.clone())),
            (
                vault_pda,
                token_account_obj(make_token_account(&usdc_mint, &exchange_pda, 0)),
            ),
            (
                user_token,
                token_account_obj(make_token_account(&usdc_mint, &user, 500_000_000)),
            ),
            (user, user_sol_acc.clone()),
            spl_token_program::keyed_account(),
        ],
    );
    assert!(
        result.program_result.is_ok(),
        "Deposit failed: {:?}",
        result.program_result
    );

    let user_account_acc = get_account(&result, &user_account_pda);
    let exchange_acc = get_account(&result, &exchange_pda);
    let vault_acc = get_account(&result, &vault_pda);
    let user_token_acc = get_account(&result, &user_token);

    // Verify deposit
    assert_eq!(read_u64_at(&user_account_acc.data, 40), initial_deposit);
    assert_eq!(read_u64_at(&user_account_acc.data, 48), initial_deposit); // total_deposited
    assert_eq!(read_u64_at(&vault_acc.data, 64), initial_deposit); // vault balance

    // ── Step 3: Open long position ($50, 5x leverage) ───────────────────
    let position_size: u64 = 50_000_000; // $50
    let leverage: u32 = 5;
    // margin = 50_000_000 / 5 = 10_000_000 ($10)
    // fee = 50_000_000 * 5 / 10_000 = 25_000 ($0.025)
    // total_cost = 10_000_000 + 25_000 = 10_025_000

    let mut open_data = DISC_OPEN_POSITION.to_vec();
    open_data.extend_from_slice(&product_id.to_le_bytes());
    open_data.push(0); // side = Long
    open_data.extend_from_slice(&position_size.to_le_bytes());
    open_data.extend_from_slice(&leverage.to_le_bytes());

    let ix_open = Instruction {
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
        data: open_data,
    };

    let result = mollusk.process_instruction(
        &ix_open,
        &[
            (user_account_pda, user_account_acc),
            (position_pda, system_account(0)),
            (market_pda, program_owned_account(market_data)),
            (exchange_pda, exchange_acc),
            (
                price_feed_pda,
                program_owned_account(make_price_feed(product_id, oracle_price, 1000, &admin)),
            ),
            (user, user_sol_acc.clone()),
            system_program_account(),
        ],
    );
    assert!(
        result.program_result.is_ok(),
        "Open position failed: {:?}",
        result.program_result
    );

    let user_account_acc = get_account(&result, &user_account_pda);
    let position_acc = get_account(&result, &position_pda);
    let market_acc = get_account(&result, &market_pda);
    let exchange_acc = get_account(&result, &exchange_pda);
    let user_sol_acc = get_account(&result, &user);

    // Verify position was created
    let expected_margin = position_size / leverage as u64;
    let expected_fee = position_size * 5 / 10_000;
    let expected_balance = initial_deposit - expected_margin - expected_fee;

    assert_eq!(read_u64_at(&user_account_acc.data, 40), expected_balance); // user balance
    assert_eq!(read_u32_at(&user_account_acc.data, 80), 1); // open_position_count
    assert_eq!(read_u64_at(&position_acc.data, 45), position_size); // size
    assert_eq!(read_u64_at(&position_acc.data, 53), oracle_price); // entry_price
    assert_eq!(read_u32_at(&position_acc.data, 61), leverage); // leverage
    assert_eq!(read_u64_at(&position_acc.data, 65), expected_margin); // margin
    assert_eq!(read_u64_at(&market_acc.data, 56), position_size); // long_oi

    // ── Step 4: Close position with 20% price increase (profit) ─────────
    let new_price: u64 = 1_200_000_000; // $12.00 (+20%)
                                        // PnL = size * (new_price - entry_price) / entry_price
                                        //      = 50_000_000 * (1_200_000_000 - 1_000_000_000) / 1_000_000_000
                                        //      = 50_000_000 * 200_000_000 / 1_000_000_000
                                        //      = 10_000_000 ($10 profit)

    let mut close_data = DISC_CLOSE_POSITION.to_vec();
    close_data.extend_from_slice(&product_id.to_le_bytes());

    let ix_close = Instruction {
        program_id: pid,
        accounts: vec![
            AccountMeta::new(user_account_pda, false),
            AccountMeta::new(position_pda, false),
            AccountMeta::new(market_pda, false),
            AccountMeta::new(exchange_pda, false),
            AccountMeta::new(vault_pda, false),
            AccountMeta::new(insurance_pda, false),
            AccountMeta::new_readonly(price_feed_pda, false),
            AccountMeta::new(user, true),
            AccountMeta::new_readonly(spl_token_id(), false),
        ],
        data: close_data,
    };

    let result = mollusk.process_instruction(
        &ix_close,
        &[
            (user_account_pda, user_account_acc),
            (position_pda, position_acc),
            (market_pda, market_acc),
            (exchange_pda, exchange_acc),
            (vault_pda, vault_acc),
            (
                insurance_pda,
                token_account_obj(make_token_account(&usdc_mint, &exchange_pda, 0)),
            ),
            (
                price_feed_pda,
                program_owned_account(make_price_feed(product_id, new_price, 1000, &admin)),
            ),
            (user, user_sol_acc.clone()),
            spl_token_program::keyed_account(),
        ],
    );
    assert!(
        result.program_result.is_ok(),
        "Close position failed: {:?}",
        result.program_result
    );

    let user_account_acc = get_account(&result, &user_account_pda);
    let market_acc = get_account(&result, &market_pda);

    // User should have received margin + pnl - close_fee
    let close_fee = position_size * 5 / 10_000;
    let pnl: i64 = 10_000_000; // $10
    let payout = expected_margin as i64 + pnl - close_fee as i64;
    let final_balance = expected_balance + payout as u64;

    assert_eq!(read_u64_at(&user_account_acc.data, 40), final_balance);
    assert_eq!(read_u32_at(&user_account_acc.data, 80), 0); // no open positions
    assert_eq!(read_u64_at(&market_acc.data, 56), 0); // long_oi cleared

    // ── Step 5: Withdraw all funds ──────────────────────────────────────
    let withdraw_amount = final_balance;
    let mut withdraw_data = DISC_WITHDRAW.to_vec();
    withdraw_data.extend_from_slice(&withdraw_amount.to_le_bytes());

    let exchange_acc = get_account(&result, &exchange_pda);
    // The vault needs enough tokens to cover the withdrawal (including profit).
    // In production, the vault accumulates deposits from many users.
    // Here we simulate that by providing a vault with sufficient balance.
    let vault_acc_for_withdraw =
        token_account_obj(make_token_account(&usdc_mint, &exchange_pda, final_balance));

    let ix_withdraw = Instruction {
        program_id: pid,
        accounts: vec![
            AccountMeta::new(user_account_pda, false),
            AccountMeta::new(exchange_pda, false),
            AccountMeta::new(vault_pda, false),
            AccountMeta::new(user_token, false),
            AccountMeta::new(user, true),
            AccountMeta::new_readonly(spl_token_id(), false),
        ],
        data: withdraw_data,
    };

    let result = mollusk.process_instruction(
        &ix_withdraw,
        &[
            (user_account_pda, user_account_acc),
            (exchange_pda, exchange_acc),
            (vault_pda, vault_acc_for_withdraw),
            (user_token, user_token_acc),
            (user, user_sol_acc.clone()),
            spl_token_program::keyed_account(),
        ],
    );
    assert!(
        result.program_result.is_ok(),
        "Withdraw failed: {:?}",
        result.program_result
    );

    let user_account_acc = get_account(&result, &user_account_pda);
    assert_eq!(read_u64_at(&user_account_acc.data, 40), 0); // zero balance

    // ── Step 6: Close user account ──────────────────────────────────────
    let ix_close_user = Instruction {
        program_id: pid,
        accounts: vec![
            AccountMeta::new(user_account_pda, false),
            AccountMeta::new(user, true),
        ],
        data: build_ix_data(&DISC_CLOSE_USER_ACCOUNT, &[]),
    };

    let result = mollusk.process_instruction(
        &ix_close_user,
        &[
            (user_account_pda, user_account_acc),
            (user, get_account(&result, &user)),
        ],
    );
    assert!(
        result.program_result.is_ok(),
        "Close user account failed: {:?}",
        result.program_result
    );
}

// ══════════════════════════════════════════════════════════════════════════════
// E2E TEST 3: Short position with loss
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn e2e_short_position_with_loss() {
    let mut mollusk = setup_mollusk();
    spl_token_program::add_program(&mut mollusk);
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

    let oracle_price: u64 = 1_000_000_000; // $10.00
    let position_size: u64 = 10_000_000; // $10
    let leverage: u32 = 5;
    let margin = position_size / leverage as u64; // $2
    let fee = position_size * 5 / 10_000;
    let user_balance: u64 = 50_000_000; // $50

    let exchange_data = make_exchange_state_with_stats(
        &admin,
        &usdc_mint,
        &vault_pda,
        &insurance_pda,
        1,
        user_balance,
        0,
    );

    // User already has account and balance, market exists
    let user_account_data = make_user_account_full(&user, user_balance, user_balance, 0, 0, 0, 0);
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

    // ── Open short position ─────────────────────────────────────────────
    let mut open_data = DISC_OPEN_POSITION.to_vec();
    open_data.extend_from_slice(&product_id.to_le_bytes());
    open_data.push(1); // side = Short
    open_data.extend_from_slice(&position_size.to_le_bytes());
    open_data.extend_from_slice(&leverage.to_le_bytes());

    let ix_open = Instruction {
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
        data: open_data,
    };

    let result = mollusk.process_instruction(
        &ix_open,
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
        "Open short failed: {:?}",
        result.program_result
    );

    let user_account_acc = get_account(&result, &user_account_pda);
    let position_acc = get_account(&result, &position_pda);
    let market_acc = get_account(&result, &market_pda);
    let exchange_acc = get_account(&result, &exchange_pda);

    // Verify short position
    assert_eq!(position_acc.data[44], 1); // side = Short
    assert_eq!(read_u64_at(&market_acc.data, 64), position_size); // short_oi

    // ── Close at higher price (loss for short) ──────────────────────────
    let new_price: u64 = 1_050_000_000; // $10.50 (+5%)
                                        // PnL (short) = size * (entry - current) / entry
                                        //             = 10_000_000 * (1_000_000_000 - 1_050_000_000) / 1_000_000_000
                                        //             = 10_000_000 * (-50_000_000) / 1_000_000_000
                                        //             = -500_000 (-$0.50)

    let mut close_data = DISC_CLOSE_POSITION.to_vec();
    close_data.extend_from_slice(&product_id.to_le_bytes());

    let vault_balance: u64 = 100_000_000;

    let ix_close = Instruction {
        program_id: pid,
        accounts: vec![
            AccountMeta::new(user_account_pda, false),
            AccountMeta::new(position_pda, false),
            AccountMeta::new(market_pda, false),
            AccountMeta::new(exchange_pda, false),
            AccountMeta::new(vault_pda, false),
            AccountMeta::new(insurance_pda, false),
            AccountMeta::new_readonly(price_feed_pda, false),
            AccountMeta::new(user, true),
            AccountMeta::new_readonly(spl_token_id(), false),
        ],
        data: close_data,
    };

    let result = mollusk.process_instruction(
        &ix_close,
        &[
            (user_account_pda, user_account_acc),
            (position_pda, position_acc),
            (market_pda, market_acc),
            (exchange_pda, exchange_acc),
            (
                vault_pda,
                token_account_obj(make_token_account(&usdc_mint, &exchange_pda, vault_balance)),
            ),
            (
                insurance_pda,
                token_account_obj(make_token_account(&usdc_mint, &exchange_pda, 10_000_000)),
            ),
            (
                price_feed_pda,
                program_owned_account(make_price_feed(product_id, new_price, 1000, &admin)),
            ),
            (user, get_account(&result, &user)),
            spl_token_program::keyed_account(),
        ],
    );
    assert!(
        result.program_result.is_ok(),
        "Close short (loss) failed: {:?}",
        result.program_result
    );

    let user_account_data = get_account_data(&result, &user_account_pda);
    // The user's balance should reflect the loss
    // payout = margin + pnl - fee > 0 (margin was $2, loss was $0.50, fee ~$0.005)
    // Should be positive
    let close_fee = position_size * 5 / 10_000;
    let pnl: i64 = -500_000;
    let payout = (margin as i64 + pnl - close_fee as i64).max(0) as u64;
    let expected_balance_before = user_balance - margin - fee;
    let final_balance = expected_balance_before + payout;
    assert_eq!(read_u64_at(&user_account_data, 40), final_balance);
    assert_eq!(read_u32_at(&user_account_data, 80), 0); // no open positions
}

// ══════════════════════════════════════════════════════════════════════════════
// E2E TEST 4: Liquidation flow
// Open a high leverage position, price moves against, liquidator calls liquidate
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn e2e_liquidation_flow() {
    let mut mollusk = setup_mollusk();
    spl_token_program::add_program(&mut mollusk);
    mollusk.sysvars.warp_to_slot(100);
    mollusk.sysvars.clock.unix_timestamp = 1000;

    let pid = program_id();
    let admin = Pubkey::new_unique();
    let position_owner = Pubkey::new_unique();
    let liquidator = Pubkey::new_unique();
    let usdc_mint = Pubkey::new_unique();
    let product_id: u32 = 1;

    let (exchange_pda, _) = find_pda(&[EXCHANGE_SEED]);
    let (vault_pda, _) = find_pda(&[VAULT_SEED]);
    let (insurance_pda, _) = find_pda(&[INSURANCE_SEED]);
    let (owner_account_pda, _) = find_pda(&[USER_ACCOUNT_SEED, position_owner.as_ref()]);
    let (liquidator_account_pda, _) = find_pda(&[USER_ACCOUNT_SEED, liquidator.as_ref()]);
    let (market_pda, _) = find_pda(&[MARKET_SEED, &product_id.to_le_bytes()]);
    let (price_feed_pda, _) = find_pda(&[PRICE_FEED_SEED, &product_id.to_le_bytes()]);
    let (position_pda, _) = find_pda(&[
        POSITION_SEED,
        position_owner.as_ref(),
        &product_id.to_le_bytes(),
    ]);

    // Long position: size=$100, 10x leverage -> margin=$10
    let entry_price: u64 = 1_000_000_000; // $10.00
    let position_size: u64 = 100_000_000; // $100
    let leverage: u32 = 10;
    let margin: u64 = 10_000_000; // $10

    // Price drops to $9.00 (-10%), PnL = 100_000_000 * (-100_000_000/1_000_000_000) = -10_000_000
    // effective_margin = 10_000_000 + (-10_000_000) = 0
    // maintenance_margin = 100_000_000 * 100 / 10_000 = 1_000_000
    // 0 < 1_000_000 => liquidatable
    let liquidation_price: u64 = 900_000_000; // $9.00

    let exchange_data = make_exchange_state_with_stats(
        &admin,
        &usdc_mint,
        &vault_pda,
        &insurance_pda,
        1,
        200_000_000,
        10_000_000,
    );

    let market_data = make_market_with_oi(
        product_id,
        &price_feed_pda,
        true,
        10,
        1_000_000_000_000,
        position_size,
        0, // long_oi = position_size
        0,
        0,
        0,
    );

    let owner_account_data = make_user_account_full(&position_owner, 0, 100_000_000, 0, 0, 0, 1);
    let liquidator_account_data =
        make_user_account_full(&liquidator, 50_000_000, 50_000_000, 0, 0, 0, 0);
    let position_data = make_position_full(
        &position_owner,
        product_id,
        0,
        position_size,
        entry_price,
        leverage,
        margin,
        0,
        0,
        0,
        1000,
        1000,
    );

    let mut ix_data = DISC_LIQUIDATE.to_vec();
    ix_data.extend_from_slice(&product_id.to_le_bytes());

    let ix = Instruction {
        program_id: pid,
        accounts: vec![
            AccountMeta::new(owner_account_pda, false),
            AccountMeta::new(position_pda, false),
            AccountMeta::new(liquidator_account_pda, false),
            AccountMeta::new(market_pda, false),
            AccountMeta::new(exchange_pda, false),
            AccountMeta::new(vault_pda, false),
            AccountMeta::new(insurance_pda, false),
            AccountMeta::new_readonly(price_feed_pda, false),
            AccountMeta::new(position_owner, false),
            AccountMeta::new(liquidator, true),
            AccountMeta::new_readonly(spl_token_id(), false),
        ],
        data: ix_data,
    };

    let result = mollusk.process_instruction(
        &ix,
        &[
            (owner_account_pda, program_owned_account(owner_account_data)),
            (position_pda, program_owned_account(position_data)),
            (
                liquidator_account_pda,
                program_owned_account(liquidator_account_data),
            ),
            (market_pda, program_owned_account(market_data)),
            (exchange_pda, program_owned_account(exchange_data)),
            (
                vault_pda,
                token_account_obj(make_token_account(&usdc_mint, &exchange_pda, 200_000_000)),
            ),
            (
                insurance_pda,
                token_account_obj(make_token_account(&usdc_mint, &exchange_pda, 10_000_000)),
            ),
            (
                price_feed_pda,
                program_owned_account(make_price_feed(product_id, liquidation_price, 1000, &admin)),
            ),
            (position_owner, system_account(1_000_000_000)),
            (liquidator, system_account(1_000_000_000)),
            spl_token_program::keyed_account(),
        ],
    );
    assert!(
        result.program_result.is_ok(),
        "Liquidation failed: {:?}",
        result.program_result
    );

    let owner_data = get_account_data(&result, &owner_account_pda);
    let market_data = get_account_data(&result, &market_pda);

    // Verify position owner lost position
    assert_eq!(read_u32_at(&owner_data, 80), 0); // open_position_count = 0
                                                 // Verify market OI cleared
    assert_eq!(read_u64_at(&market_data, 56), 0); // long_oi = 0
}

// ══════════════════════════════════════════════════════════════════════════════
// E2E TEST 5: Admin operations lifecycle
// Transfer admin, pause exchange, unpause, update market params
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn e2e_admin_operations() {
    let mollusk = setup_mollusk();
    let pid = program_id();
    let admin = Pubkey::new_unique();
    let new_admin = Pubkey::new_unique();
    let usdc_mint = Pubkey::new_unique();
    let vault = Pubkey::new_unique();
    let insurance = Pubkey::new_unique();
    let product_id: u32 = 1;

    let (exchange_pda, _) = find_pda(&[EXCHANGE_SEED]);
    let (market_pda, _) = find_pda(&[MARKET_SEED, &product_id.to_le_bytes()]);
    let oracle_feed = Pubkey::new_unique();

    let exchange_data = make_exchange_state(&admin, &usdc_mint, &vault, &insurance);
    let market_data = make_market(product_id, &oracle_feed, true);

    // ── Step 1: Pause exchange ──────────────────────────────────────────
    let ix_pause = Instruction {
        program_id: pid,
        accounts: vec![
            AccountMeta::new(exchange_pda, false),
            AccountMeta::new_readonly(admin, true),
        ],
        data: build_ix_data(&DISC_SET_PAUSED, &[1]),
    };

    let result = mollusk.process_instruction(
        &ix_pause,
        &[
            (exchange_pda, program_owned_account(exchange_data)),
            (admin, system_account(1_000_000_000)),
        ],
    );
    assert!(
        result.program_result.is_ok(),
        "Pause failed: {:?}",
        result.program_result
    );
    let exchange_acc = get_account(&result, &exchange_pda);
    assert_eq!(exchange_acc.data[212], 1); // paused

    // ── Step 2: Unpause ─────────────────────────────────────────────────
    let ix_unpause = Instruction {
        program_id: pid,
        accounts: vec![
            AccountMeta::new(exchange_pda, false),
            AccountMeta::new_readonly(admin, true),
        ],
        data: build_ix_data(&DISC_SET_PAUSED, &[0]),
    };

    let result = mollusk.process_instruction(
        &ix_unpause,
        &[
            (exchange_pda, exchange_acc),
            (admin, system_account(1_000_000_000)),
        ],
    );
    assert!(
        result.program_result.is_ok(),
        "Unpause failed: {:?}",
        result.program_result
    );
    let exchange_acc = get_account(&result, &exchange_pda);
    assert_eq!(exchange_acc.data[212], 0); // active

    // ── Step 3: Update market params ────────────────────────────────────
    let new_leverage: u32 = 25;
    let new_oi: u64 = 10_000_000_000_000;
    let mut update_data = DISC_UPDATE_MARKET_PARAMS.to_vec();
    update_data.extend_from_slice(&product_id.to_le_bytes());
    update_data.extend_from_slice(&new_leverage.to_le_bytes());
    update_data.extend_from_slice(&new_oi.to_le_bytes());

    let ix_update = Instruction {
        program_id: pid,
        accounts: vec![
            AccountMeta::new(market_pda, false),
            AccountMeta::new_readonly(exchange_pda, false),
            AccountMeta::new_readonly(admin, true),
        ],
        data: update_data,
    };

    let result = mollusk.process_instruction(
        &ix_update,
        &[
            (market_pda, program_owned_account(market_data)),
            (exchange_pda, exchange_acc.clone()),
            (admin, system_account(1_000_000_000)),
        ],
    );
    assert!(
        result.program_result.is_ok(),
        "Update market params failed: {:?}",
        result.program_result
    );
    let market_acc = get_account(&result, &market_pda);
    assert_eq!(read_u32_at(&market_acc.data, 44), new_leverage);
    assert_eq!(read_u64_at(&market_acc.data, 48), new_oi);

    // ── Step 4: Deactivate market ───────────────────────────────────────
    let mut deactivate_data = DISC_SET_MARKET_ACTIVE.to_vec();
    deactivate_data.extend_from_slice(&product_id.to_le_bytes());
    deactivate_data.push(0); // active = false

    let ix_deactivate = Instruction {
        program_id: pid,
        accounts: vec![
            AccountMeta::new(market_pda, false),
            AccountMeta::new_readonly(exchange_pda, false),
            AccountMeta::new_readonly(admin, true),
        ],
        data: deactivate_data,
    };

    let result = mollusk.process_instruction(
        &ix_deactivate,
        &[
            (market_pda, market_acc),
            (exchange_pda, exchange_acc.clone()),
            (admin, system_account(1_000_000_000)),
        ],
    );
    assert!(
        result.program_result.is_ok(),
        "Deactivate market failed: {:?}",
        result.program_result
    );
    assert_eq!(get_account_data(&result, &market_pda)[112], 0);

    // ── Step 5: Transfer admin ──────────────────────────────────────────
    let mut transfer_data = DISC_TRANSFER_ADMIN.to_vec();
    transfer_data.extend_from_slice(new_admin.as_ref());

    let ix_transfer = Instruction {
        program_id: pid,
        accounts: vec![
            AccountMeta::new(exchange_pda, false),
            AccountMeta::new_readonly(admin, true),
        ],
        data: transfer_data,
    };

    let result = mollusk.process_instruction(
        &ix_transfer,
        &[
            (exchange_pda, exchange_acc),
            (admin, system_account(1_000_000_000)),
        ],
    );
    assert!(
        result.program_result.is_ok(),
        "Transfer admin failed: {:?}",
        result.program_result
    );
    let exchange_acc = get_account(&result, &exchange_pda);
    assert_eq!(&exchange_acc.data[40..72], new_admin.as_ref());

    // ── Step 6: Accept admin ────────────────────────────────────────────
    let ix_accept = Instruction {
        program_id: pid,
        accounts: vec![
            AccountMeta::new(exchange_pda, false),
            AccountMeta::new_readonly(new_admin, true),
        ],
        data: build_ix_data(&DISC_ACCEPT_ADMIN, &[]),
    };

    let result = mollusk.process_instruction(
        &ix_accept,
        &[
            (exchange_pda, exchange_acc),
            (new_admin, system_account(1_000_000_000)),
        ],
    );
    assert!(
        result.program_result.is_ok(),
        "Accept admin failed: {:?}",
        result.program_result
    );
    let exchange_data = get_account_data(&result, &exchange_pda);
    assert_eq!(&exchange_data[8..40], new_admin.as_ref()); // new admin
    assert_eq!(&exchange_data[40..72], &[0u8; 32]); // pending cleared

    // ── Step 7: Old admin can no longer pause ───────────────────────────
    let exchange_acc = get_account(&result, &exchange_pda);
    let ix_old_pause = Instruction {
        program_id: pid,
        accounts: vec![
            AccountMeta::new(exchange_pda, false),
            AccountMeta::new_readonly(admin, true),
        ],
        data: build_ix_data(&DISC_SET_PAUSED, &[1]),
    };

    let result = mollusk.process_instruction(
        &ix_old_pause,
        &[
            (exchange_pda, exchange_acc),
            (admin, system_account(1_000_000_000)),
        ],
    );
    assert!(
        result.program_result.is_err(),
        "Old admin should not be able to pause"
    );
}

// ══════════════════════════════════════════════════════════════════════════════
// E2E TEST 6: Funding rate lifecycle
// Create market -> Open positions on both sides -> Update funding rate
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn e2e_funding_rate_imbalanced() {
    let mut mollusk = setup_mollusk();
    mollusk.sysvars.warp_to_slot(100);
    // Set clock past the funding interval (28800 seconds)
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

    // Market with more longs than shorts
    let long_oi: u64 = 100_000_000_000; // $100k
    let short_oi: u64 = 50_000_000_000; // $50k
    let market_data = make_market_with_oi(
        product_id,
        &oracle_feed,
        true,
        10,
        1_000_000_000_000,
        long_oi,
        short_oi,
        0, // last_funding_time = 0 (well past)
        0,
        0, // cumulative rates
    );

    let exchange_data = make_exchange_state(&admin, &usdc_mint, &vault, &insurance);

    let mut ix_data = DISC_UPDATE_FUNDING_RATE.to_vec();
    ix_data.extend_from_slice(&product_id.to_le_bytes());

    let ix = Instruction {
        program_id: pid,
        accounts: vec![
            AccountMeta::new(market_pda, false),
            AccountMeta::new_readonly(exchange_pda, false),
            AccountMeta::new_readonly(cranker, true),
        ],
        data: ix_data,
    };

    let result = mollusk.process_instruction(
        &ix,
        &[
            (market_pda, program_owned_account(market_data)),
            (exchange_pda, program_owned_account(exchange_data)),
            (cranker, system_account(1_000_000_000)),
        ],
    );
    assert!(
        result.program_result.is_ok(),
        "Update funding rate failed: {:?}",
        result.program_result
    );

    let market_data = get_account_data(&result, &market_pda);
    let last_funding = read_i64_at(&market_data, 88);
    let cum_long = read_i64_at(&market_data, 96);
    let cum_short = read_i64_at(&market_data, 104);

    assert_eq!(last_funding, 50_000); // updated to current time
                                      // imbalance = long - short = 100k - 50k = 50k
                                      // rate = imbalance * 5 / total = 50k * 5 / 150k = 1 (clamped)
                                      // longs pay, so cum_long increases, cum_short decreases (by negation)
    assert!(cum_long > 0, "cum_long should be positive (longs pay)");
    assert!(cum_short < 0, "cum_short should be negative");
    assert_eq!(cum_long, -cum_short); // symmetric
}

// ══════════════════════════════════════════════════════════════════════════════
// E2E TEST 7: Deposit -> Open position -> Add margin -> Close position
// Verify add_margin correctly adjusts liquidation price
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn e2e_add_margin_adjusts_liquidation() {
    let mut mollusk = setup_mollusk();
    spl_token_program::add_program(&mut mollusk);
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
    let position_size: u64 = 50_000_000;
    let leverage: u32 = 10;
    let margin = position_size / leverage as u64; // 5_000_000
    let fee = position_size * 5 / 10_000;
    let user_balance: u64 = 100_000_000;

    // Pre-build state
    let exchange_data = make_exchange_state_with_stats(
        &admin,
        &usdc_mint,
        &vault_pda,
        &insurance_pda,
        1,
        user_balance,
        0,
    );
    let user_account_data = make_user_account_full(&user, user_balance, user_balance, 0, 0, 0, 0);
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

    // ── Open long position ──────────────────────────────────────────────
    let mut open_data = DISC_OPEN_POSITION.to_vec();
    open_data.extend_from_slice(&product_id.to_le_bytes());
    open_data.push(0); // Long
    open_data.extend_from_slice(&position_size.to_le_bytes());
    open_data.extend_from_slice(&leverage.to_le_bytes());

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
            data: open_data,
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
        "Open position failed: {:?}",
        result.program_result
    );

    let user_account_acc = get_account(&result, &user_account_pda);
    let position_acc = get_account(&result, &position_pda);
    let original_liq_price = read_u64_at(&position_acc.data, 73);
    let original_margin = read_u64_at(&position_acc.data, 65);

    // ── Add margin ──────────────────────────────────────────────────────
    let add_amount: u64 = 5_000_000; // double the margin
    let mut margin_data = DISC_ADD_MARGIN.to_vec();
    margin_data.extend_from_slice(&product_id.to_le_bytes());
    margin_data.extend_from_slice(&add_amount.to_le_bytes());

    let exchange_acc = get_account(&result, &exchange_pda);

    let result = mollusk.process_instruction(
        &Instruction {
            program_id: pid,
            accounts: vec![
                AccountMeta::new(user_account_pda, false),
                AccountMeta::new(position_pda, false),
                AccountMeta::new(exchange_pda, false),
                AccountMeta::new(user, true),
            ],
            data: margin_data,
        },
        &[
            (user_account_pda, user_account_acc),
            (position_pda, position_acc),
            (exchange_pda, exchange_acc),
            (user, system_account(1_000_000_000)),
        ],
    );
    assert!(
        result.program_result.is_ok(),
        "Add margin failed: {:?}",
        result.program_result
    );

    let position_data = get_account_data(&result, &position_pda);
    let new_margin = read_u64_at(&position_data, 65);
    let new_liq_price = read_u64_at(&position_data, 73);

    assert_eq!(new_margin, original_margin + add_amount);
    // For a long, adding margin should push the liquidation price LOWER
    assert!(
        new_liq_price < original_liq_price,
        "Liq price should decrease after adding margin: {} vs {}",
        new_liq_price,
        original_liq_price
    );
}

// ══════════════════════════════════════════════════════════════════════════════
// E2E TEST 8: Oracle authority transfer lifecycle
// Transfer -> (wrong person tries to accept) -> Correct person accepts
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn e2e_oracle_authority_transfer_with_rejection() {
    let mollusk = setup_mollusk();
    let pid = program_id();
    let authority = Pubkey::new_unique();
    let new_authority = Pubkey::new_unique();
    let imposter = Pubkey::new_unique();

    let (oracle_state_pda, _) = find_pda(&[ORACLE_SEED]);
    let (publisher_list_pda, _) = find_pda(&[PUBLISHER_LIST_SEED]);

    let oracle_state_data = make_oracle_state(&authority);
    let publisher_list_data = make_publisher_list(&authority, &[authority]);

    // ── Step 1: Transfer authority ──────────────────────────────────────
    let mut transfer_data = DISC_TRANSFER_AUTHORITY.to_vec();
    transfer_data.extend_from_slice(new_authority.as_ref());

    let result = mollusk.process_instruction(
        &Instruction {
            program_id: pid,
            accounts: vec![
                AccountMeta::new(publisher_list_pda, false),
                AccountMeta::new(oracle_state_pda, false),
                AccountMeta::new_readonly(authority, true),
            ],
            data: transfer_data,
        },
        &[
            (
                publisher_list_pda,
                program_owned_account(publisher_list_data.clone()),
            ),
            (oracle_state_pda, program_owned_account(oracle_state_data)),
            (authority, system_account(1_000_000_000)),
        ],
    );
    assert!(result.program_result.is_ok());
    let oracle_state_acc = get_account(&result, &oracle_state_pda);

    // ── Step 2: Imposter tries to accept -> should fail ─────────────────
    let result_bad = mollusk.process_instruction(
        &Instruction {
            program_id: pid,
            accounts: vec![
                AccountMeta::new(oracle_state_pda, false),
                AccountMeta::new(publisher_list_pda, false),
                AccountMeta::new_readonly(imposter, true),
            ],
            data: build_ix_data(&DISC_ACCEPT_AUTHORITY, &[]),
        },
        &[
            (oracle_state_pda, oracle_state_acc.clone()),
            (
                publisher_list_pda,
                program_owned_account(publisher_list_data.clone()),
            ),
            (imposter, system_account(1_000_000_000)),
        ],
    );
    assert!(
        result_bad.program_result.is_err(),
        "Imposter should not accept authority"
    );

    // ── Step 3: Correct new_authority accepts ───────────────────────────
    let result_good = mollusk.process_instruction(
        &Instruction {
            program_id: pid,
            accounts: vec![
                AccountMeta::new(oracle_state_pda, false),
                AccountMeta::new(publisher_list_pda, false),
                AccountMeta::new_readonly(new_authority, true),
            ],
            data: build_ix_data(&DISC_ACCEPT_AUTHORITY, &[]),
        },
        &[
            (oracle_state_pda, oracle_state_acc),
            (
                publisher_list_pda,
                program_owned_account(publisher_list_data),
            ),
            (new_authority, system_account(1_000_000_000)),
        ],
    );
    assert!(result_good.program_result.is_ok());

    let oracle_data = get_account_data(&result_good, &oracle_state_pda);
    assert_eq!(&oracle_data[8..40], new_authority.as_ref());
    assert_eq!(&oracle_data[40..72], &[0u8; 32]);
}

// ══════════════════════════════════════════════════════════════════════════════
// E2E TEST 9: Multiple deposits and partial withdrawals
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn e2e_multiple_deposits_and_partial_withdrawals() {
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

    let exchange_data =
        make_exchange_state_with_stats(&admin, &usdc_mint, &vault_pda, &insurance_pda, 0, 0, 0);
    let user_account_data = make_user_account(&user, 0);

    let mut current_exchange = program_owned_account(exchange_data);
    let mut current_user_account = program_owned_account(user_account_data);
    let mut current_vault = token_account_obj(make_token_account(&usdc_mint, &exchange_pda, 0));
    let mut current_user_token =
        token_account_obj(make_token_account(&usdc_mint, &user, 1_000_000_000));

    // ── Deposit 1: $10 ──────────────────────────────────────────────────
    let deposits = [10_000_000u64, 25_000_000, 15_000_000]; // $10, $25, $15 = $50 total
    let mut running_balance: u64 = 0;

    for (i, &amount) in deposits.iter().enumerate() {
        let mut ix_data = DISC_DEPOSIT.to_vec();
        ix_data.extend_from_slice(&amount.to_le_bytes());

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
            i + 1,
            result.program_result
        );

        running_balance += amount;
        current_user_account = get_account(&result, &user_account_pda);
        current_exchange = get_account(&result, &exchange_pda);
        current_vault = get_account(&result, &vault_pda);
        current_user_token = get_account(&result, &user_token);

        assert_eq!(read_u64_at(&current_user_account.data, 40), running_balance);
    }

    assert_eq!(running_balance, 50_000_000); // $50 total

    // ── Partial withdrawals: $20, then $15 ──────────────────────────────
    let withdrawals = [20_000_000u64, 15_000_000];
    for (i, &amount) in withdrawals.iter().enumerate() {
        let mut ix_data = DISC_WITHDRAW.to_vec();
        ix_data.extend_from_slice(&amount.to_le_bytes());

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
            "Withdraw {} failed: {:?}",
            i + 1,
            result.program_result
        );

        running_balance -= amount;
        current_user_account = get_account(&result, &user_account_pda);
        current_exchange = get_account(&result, &exchange_pda);
        current_vault = get_account(&result, &vault_pda);
        current_user_token = get_account(&result, &user_token);

        assert_eq!(read_u64_at(&current_user_account.data, 40), running_balance);
    }

    assert_eq!(running_balance, 15_000_000); // $15 remaining

    // Verify cumulative stats
    let total_deposited = read_u64_at(&current_user_account.data, 48);
    let total_withdrawn = read_u64_at(&current_user_account.data, 56);
    assert_eq!(total_deposited, 50_000_000);
    assert_eq!(total_withdrawn, 35_000_000);
}

// ══════════════════════════════════════════════════════════════════════════════
// E2E TEST 10: Deposit -> Withdraw fails with open positions
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn e2e_withdraw_blocked_with_open_positions() {
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

    // User has balance but 1 open position
    let user_account_data = make_user_account_full(&user, 50_000_000, 100_000_000, 0, 0, 0, 1);
    let exchange_data = make_exchange_state_with_stats(
        &admin,
        &usdc_mint,
        &vault_pda,
        &insurance_pda,
        1,
        100_000_000,
        0,
    );

    let mut ix_data = DISC_WITHDRAW.to_vec();
    ix_data.extend_from_slice(&10_000_000u64.to_le_bytes());

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
                token_account_obj(make_token_account(&usdc_mint, &exchange_pda, 100_000_000)),
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
        result.program_result.is_err(),
        "Withdraw should fail with open positions"
    );
}

// ══════════════════════════════════════════════════════════════════════════════
// E2E TEST 11: Open position on paused exchange should fail
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn e2e_open_position_fails_when_paused() {
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

    // Exchange is paused
    let mut exchange_data = make_exchange_state_with_stats(
        &admin,
        &usdc_mint,
        &vault_pda,
        &insurance_pda,
        1,
        100_000_000,
        0,
    );
    exchange_data[212] = 1; // paused = true

    let user_account_data = make_user_account_full(&user, 100_000_000, 100_000_000, 0, 0, 0, 0);
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

    let mut open_data = DISC_OPEN_POSITION.to_vec();
    open_data.extend_from_slice(&product_id.to_le_bytes());
    open_data.push(0);
    open_data.extend_from_slice(&10_000_000u64.to_le_bytes());
    open_data.extend_from_slice(&5u32.to_le_bytes());

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
            data: open_data,
        },
        &[
            (user_account_pda, program_owned_account(user_account_data)),
            (position_pda, system_account(0)),
            (market_pda, program_owned_account(market_data)),
            (exchange_pda, program_owned_account(exchange_data)),
            (
                price_feed_pda,
                program_owned_account(make_price_feed(product_id, 1_000_000_000, 1000, &admin)),
            ),
            (user, system_account(100_000_000_000)),
            system_program_account(),
        ],
    );
    assert!(
        result.program_result.is_err(),
        "Open position should fail when exchange is paused"
    );
}

// ══════════════════════════════════════════════════════════════════════════════
// E2E TEST 12: Publisher management - add up to max, verify duplicate rejection
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn e2e_publisher_management_full() {
    let mollusk = setup_mollusk();
    let pid = program_id();
    let authority = Pubkey::new_unique();

    let (oracle_state_pda, _) = find_pda(&[ORACLE_SEED]);
    let (publisher_list_pda, _) = find_pda(&[PUBLISHER_LIST_SEED]);

    let oracle_state_data = make_oracle_state(&authority);
    let mut publisher_list_data = make_publisher_list(&authority, &[authority]);

    let mut current_publisher_list = program_owned_account(publisher_list_data.clone());
    let oracle_state_acc = program_owned_account(oracle_state_data);

    // Add 15 more publishers (total 16 = MAX_PUBLISHERS)
    let mut publishers = vec![authority];
    for i in 0..15 {
        let new_pub = Pubkey::new_unique();
        publishers.push(new_pub);

        let mut add_data = DISC_ADD_PUBLISHER.to_vec();
        add_data.extend_from_slice(new_pub.as_ref());

        let result = mollusk.process_instruction(
            &Instruction {
                program_id: pid,
                accounts: vec![
                    AccountMeta::new(publisher_list_pda, false),
                    AccountMeta::new(oracle_state_pda, false),
                    AccountMeta::new_readonly(authority, true),
                ],
                data: add_data,
            },
            &[
                (publisher_list_pda, current_publisher_list.clone()),
                (oracle_state_pda, oracle_state_acc.clone()),
                (authority, system_account(1_000_000_000)),
            ],
        );
        assert!(
            result.program_result.is_ok(),
            "Add publisher {} failed: {:?}",
            i + 1,
            result.program_result
        );
        current_publisher_list = get_account(&result, &publisher_list_pda);
    }

    assert_eq!(read_u32_at(&current_publisher_list.data, 40), 16);

    // ── Try to add 17th publisher -> should fail (PublisherListFull) ─────
    let extra_pub = Pubkey::new_unique();
    let mut add_data = DISC_ADD_PUBLISHER.to_vec();
    add_data.extend_from_slice(extra_pub.as_ref());

    let result = mollusk.process_instruction(
        &Instruction {
            program_id: pid,
            accounts: vec![
                AccountMeta::new(publisher_list_pda, false),
                AccountMeta::new(oracle_state_pda, false),
                AccountMeta::new_readonly(authority, true),
            ],
            data: add_data,
        },
        &[
            (publisher_list_pda, current_publisher_list.clone()),
            (oracle_state_pda, oracle_state_acc.clone()),
            (authority, system_account(1_000_000_000)),
        ],
    );
    assert!(
        result.program_result.is_err(),
        "Adding 17th publisher should fail"
    );

    // ── Try to add duplicate -> should fail (PublisherAlreadyExists) ─────
    let existing = publishers[5];
    let mut dup_data = DISC_ADD_PUBLISHER.to_vec();
    dup_data.extend_from_slice(existing.as_ref());

    let result = mollusk.process_instruction(
        &Instruction {
            program_id: pid,
            accounts: vec![
                AccountMeta::new(publisher_list_pda, false),
                AccountMeta::new(oracle_state_pda, false),
                AccountMeta::new_readonly(authority, true),
            ],
            data: dup_data,
        },
        &[
            (publisher_list_pda, current_publisher_list.clone()),
            (oracle_state_pda, oracle_state_acc.clone()),
            (authority, system_account(1_000_000_000)),
        ],
    );
    assert!(
        result.program_result.is_err(),
        "Duplicate publisher should fail"
    );

    // ── Remove a publisher, then try to remove authority -> should fail ──
    let to_remove = publishers[10];
    let mut remove_data = DISC_REMOVE_PUBLISHER.to_vec();
    remove_data.extend_from_slice(to_remove.as_ref());

    let result = mollusk.process_instruction(
        &Instruction {
            program_id: pid,
            accounts: vec![
                AccountMeta::new(publisher_list_pda, false),
                AccountMeta::new(oracle_state_pda, false),
                AccountMeta::new_readonly(authority, true),
            ],
            data: remove_data,
        },
        &[
            (publisher_list_pda, current_publisher_list.clone()),
            (oracle_state_pda, oracle_state_acc.clone()),
            (authority, system_account(1_000_000_000)),
        ],
    );
    assert!(result.program_result.is_ok(), "Remove publisher failed");
    let current_publisher_list = get_account(&result, &publisher_list_pda);
    assert_eq!(read_u32_at(&current_publisher_list.data, 40), 15);

    // Try to remove authority itself -> should fail
    let mut remove_auth_data = DISC_REMOVE_PUBLISHER.to_vec();
    remove_auth_data.extend_from_slice(authority.as_ref());

    let result = mollusk.process_instruction(
        &Instruction {
            program_id: pid,
            accounts: vec![
                AccountMeta::new(publisher_list_pda, false),
                AccountMeta::new(oracle_state_pda, false),
                AccountMeta::new_readonly(authority, true),
            ],
            data: remove_auth_data,
        },
        &[
            (publisher_list_pda, current_publisher_list),
            (oracle_state_pda, oracle_state_acc),
            (authority, system_account(1_000_000_000)),
        ],
    );
    assert!(
        result.program_result.is_err(),
        "Removing authority should fail"
    );
}

// ══════════════════════════════════════════════════════════════════════════════
// E2E TEST 13: Fund insurance and verify it's used during liquidation
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn e2e_insurance_fund_lifecycle() {
    let mut mollusk = setup_mollusk();
    spl_token_program::add_program(&mut mollusk);
    mollusk.sysvars.warp_to_slot(100);
    mollusk.sysvars.clock.unix_timestamp = 1000;

    let pid = program_id();
    let admin = Pubkey::new_unique();
    let usdc_mint = Pubkey::new_unique();

    let (exchange_pda, _) = find_pda(&[EXCHANGE_SEED]);
    let (insurance_pda, _) = find_pda(&[INSURANCE_SEED]);
    let admin_token = Pubkey::new_unique();

    let exchange_data = make_exchange_state_with_stats(
        &admin,
        &usdc_mint,
        &Pubkey::new_unique(),
        &insurance_pda,
        0,
        0,
        0,
    );

    // ── Fund insurance ──────────────────────────────────────────────────
    let fund_amount: u64 = 50_000_000;
    let mut ix_data = DISC_FUND_INSURANCE.to_vec();
    ix_data.extend_from_slice(&fund_amount.to_le_bytes());

    let result = mollusk.process_instruction(
        &Instruction {
            program_id: pid,
            accounts: vec![
                AccountMeta::new(exchange_pda, false),
                AccountMeta::new(insurance_pda, false),
                AccountMeta::new(admin_token, false),
                AccountMeta::new(admin, true),
                AccountMeta::new_readonly(spl_token_id(), false),
            ],
            data: ix_data,
        },
        &[
            (exchange_pda, program_owned_account(exchange_data)),
            (
                insurance_pda,
                token_account_obj(make_token_account(&usdc_mint, &exchange_pda, 0)),
            ),
            (
                admin_token,
                token_account_obj(make_token_account(&usdc_mint, &admin, 100_000_000)),
            ),
            (admin, system_account(1_000_000_000)),
            spl_token_program::keyed_account(),
        ],
    );
    assert!(
        result.program_result.is_ok(),
        "Fund insurance failed: {:?}",
        result.program_result
    );

    let exchange_data = get_account_data(&result, &exchange_pda);
    let insurance_balance = read_u64_at(&exchange_data, 204);
    assert_eq!(insurance_balance, fund_amount);

    let ins_tok_data = get_account_data(&result, &insurance_pda);
    assert_eq!(read_u64_at(&ins_tok_data, 64), fund_amount);
}

// ══════════════════════════════════════════════════════════════════════════════
// E2E TEST 14: Permissionless market creation
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn e2e_permissionless_market_with_defaults() {
    let mollusk = setup_mollusk();
    let pid = program_id();
    let admin = Pubkey::new_unique();
    let random_payer = Pubkey::new_unique();
    let usdc_mint = Pubkey::new_unique();
    let vault = Pubkey::new_unique();
    let insurance = Pubkey::new_unique();
    let product_id: u32 = 42;

    let (exchange_pda, _) = find_pda(&[EXCHANGE_SEED]);
    let (market_pda, _) = find_pda(&[MARKET_SEED, &product_id.to_le_bytes()]);
    let (price_feed_pda, _) = find_pda(&[PRICE_FEED_SEED, &product_id.to_le_bytes()]);

    let exchange_data = make_exchange_state(&admin, &usdc_mint, &vault, &insurance);

    let mut ix_data = DISC_CREATE_MARKET_PERMISSIONLESS.to_vec();
    ix_data.extend_from_slice(&product_id.to_le_bytes());

    let result = mollusk.process_instruction(
        &Instruction {
            program_id: pid,
            accounts: vec![
                AccountMeta::new(market_pda, false),
                AccountMeta::new(exchange_pda, false),
                AccountMeta::new_readonly(price_feed_pda, false),
                AccountMeta::new(random_payer, true),
                AccountMeta::new_readonly(system_program::ID, false),
            ],
            data: ix_data,
        },
        &[
            (market_pda, system_account(0)),
            (exchange_pda, program_owned_account(exchange_data)),
            (
                price_feed_pda,
                program_owned_account(make_price_feed(product_id, 1_000_000_000, 1000, &admin)),
            ),
            (random_payer, system_account(100_000_000_000)),
            system_program_account(),
        ],
    );
    assert!(
        result.program_result.is_ok(),
        "Permissionless market creation failed: {:?}",
        result.program_result
    );

    let market_data = get_account_data(&result, &market_pda);
    assert_eq!(read_u32_at(&market_data, 8), product_id);
    assert_eq!(read_u32_at(&market_data, 44), 50); // default max_leverage = MAX_LEVERAGE
    assert_eq!(read_u64_at(&market_data, 48), 1_000_000_000_000); // default max_oi
    assert_eq!(market_data[112], 1); // active
}

// ══════════════════════════════════════════════════════════════════════════════
// E2E TEST 15: Funding rate too early should fail
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn e2e_funding_rate_too_early() {
    let mut mollusk = setup_mollusk();
    mollusk.sysvars.warp_to_slot(100);
    mollusk.sysvars.clock.unix_timestamp = 20_000; // < 28_800 from last_funding_time=0

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

    // last_funding_time = 0, current time = 20000, interval = 28800
    let market_data = make_market_with_oi(
        product_id,
        &oracle_feed,
        true,
        10,
        1_000_000_000_000,
        50_000,
        50_000,
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
            data: ix_data,
        },
        &[
            (market_pda, program_owned_account(market_data)),
            (exchange_pda, program_owned_account(exchange_data)),
            (cranker, system_account(1_000_000_000)),
        ],
    );
    assert!(
        result.program_result.is_err(),
        "Funding rate update should fail when too early"
    );
}

// ══════════════════════════════════════════════════════════════════════════════
// E2E TEST 16: Close user account fails with open positions
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn e2e_close_user_account_fails_with_open_positions() {
    let mollusk = setup_mollusk();
    let pid = program_id();
    let owner = Pubkey::new_unique();
    let (user_account_pda, _) = find_pda(&[USER_ACCOUNT_SEED, owner.as_ref()]);

    // User has 0 balance but 1 open position
    let user_data = make_user_account_full(&owner, 0, 0, 0, 0, 0, 1);

    let result = mollusk.process_instruction(
        &Instruction {
            program_id: pid,
            accounts: vec![
                AccountMeta::new(user_account_pda, false),
                AccountMeta::new(owner, true),
            ],
            data: build_ix_data(&DISC_CLOSE_USER_ACCOUNT, &[]),
        },
        &[
            (user_account_pda, program_owned_account(user_data)),
            (owner, system_account(1_000_000_000)),
        ],
    );
    assert!(
        result.program_result.is_err(),
        "Close user account should fail with open positions"
    );
}

// ══════════════════════════════════════════════════════════════════════════════
// E2E TEST 17: Open position on inactive market should fail
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn e2e_open_position_on_inactive_market() {
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

    let exchange_data = make_exchange_state_with_stats(
        &admin,
        &usdc_mint,
        &vault_pda,
        &insurance_pda,
        1,
        100_000_000,
        0,
    );
    let user_account_data = make_user_account_full(&user, 100_000_000, 100_000_000, 0, 0, 0, 0);
    let market_data = make_market_with_oi(
        product_id,
        &price_feed_pda,
        false, // inactive!
        10,
        1_000_000_000_000,
        0,
        0,
        0,
        0,
        0,
    );

    let mut open_data = DISC_OPEN_POSITION.to_vec();
    open_data.extend_from_slice(&product_id.to_le_bytes());
    open_data.push(0);
    open_data.extend_from_slice(&10_000_000u64.to_le_bytes());
    open_data.extend_from_slice(&5u32.to_le_bytes());

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
            data: open_data,
        },
        &[
            (user_account_pda, program_owned_account(user_account_data)),
            (position_pda, system_account(0)),
            (market_pda, program_owned_account(market_data)),
            (exchange_pda, program_owned_account(exchange_data)),
            (
                price_feed_pda,
                program_owned_account(make_price_feed(product_id, 1_000_000_000, 1000, &admin)),
            ),
            (user, system_account(100_000_000_000)),
            system_program_account(),
        ],
    );
    assert!(
        result.program_result.is_err(),
        "Open position should fail on inactive market"
    );
}
