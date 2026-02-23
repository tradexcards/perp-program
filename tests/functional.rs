use mollusk_svm::program::keyed_account_for_system_program;
/// Functional tests for all 31 instructions in the Pinocchio program.
///
/// Uses mollusk-svm to execute the compiled .so against instruction payloads.
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

// ── Discriminators (from lib.rs) ────────────────────────────────────────────

const DISC_INITIALIZE_ORACLE: [u8; 8] = [0x90, 0xdf, 0x83, 0x78, 0xc4, 0xfd, 0xb5, 0x63];
const DISC_CREATE_FEED: [u8; 8] = [0xad, 0x56, 0x5f, 0x5e, 0x0d, 0xc1, 0x43, 0xb4];
const DISC_ADD_PUBLISHER: [u8; 8] = [0x26, 0x08, 0xea, 0x2c, 0x4b, 0x1d, 0xa3, 0x92];
const DISC_REMOVE_PUBLISHER: [u8; 8] = [0x5b, 0x40, 0x9e, 0x65, 0x4c, 0x9d, 0x44, 0x66];
const DISC_TRANSFER_AUTHORITY: [u8; 8] = [0x30, 0xa9, 0x4c, 0x48, 0xe5, 0xb4, 0x37, 0xa1];
const DISC_ACCEPT_AUTHORITY: [u8; 8] = [0x6b, 0x56, 0xc6, 0x5b, 0x21, 0x0c, 0x6b, 0xa0];
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

/// Build instruction data from a discriminator + appended data.
fn build_ix_data(disc: &[u8; 8], extra: &[u8]) -> Vec<u8> {
    let mut data = disc.to_vec();
    data.extend_from_slice(extra);
    data
}

/// Build a funded system account.
fn system_account(lamports: u64) -> Account {
    Account {
        lamports,
        data: vec![],
        owner: system_program::ID,
        executable: false,
        rent_epoch: 0,
    }
}

/// Get the system program account for CPI tests.
fn system_program_account() -> (Pubkey, Account) {
    keyed_account_for_system_program()
}

/// Build an oracle state account with given authority.
fn make_oracle_state(authority: &Pubkey) -> Vec<u8> {
    let mut data = vec![0u8; 92]; // 8 disc + 84 body
    data[..8].copy_from_slice(&DISC_ORACLE_STATE);
    data[8..40].copy_from_slice(authority.as_ref());
    // pending_authority = [0; 32] at offset 40..72
    // feed_count = 0 at offset 72..76
    data
}

/// Build a publisher list account with given authority and publishers.
fn make_publisher_list(authority: &Pubkey, publishers: &[Pubkey]) -> Vec<u8> {
    let mut data = vec![0u8; 556]; // max size
    data[..8].copy_from_slice(&DISC_PUBLISHER_LIST);
    data[8..40].copy_from_slice(authority.as_ref()); // authority
    let count = publishers.len() as u32;
    data[40..44].copy_from_slice(&count.to_le_bytes()); // len
    for (i, pk) in publishers.iter().enumerate() {
        let offset = 44 + i * 32;
        data[offset..offset + 32].copy_from_slice(pk.as_ref());
    }
    data
}

/// Build a price feed account.
fn make_price_feed(product_id: u32, price: u64, last_updated: i64, authority: &Pubkey) -> Vec<u8> {
    let mut data = vec![0u8; 68]; // 8 disc + 60 body
    data[..8].copy_from_slice(&DISC_PRICE_FEED);
    data[8..12].copy_from_slice(&product_id.to_le_bytes());
    data[12..20].copy_from_slice(&price.to_le_bytes());
    data[20..28].copy_from_slice(&last_updated.to_le_bytes());
    // update_count at 28..36 = 0
    data[36..68].copy_from_slice(authority.as_ref());
    data
}

/// Build an exchange state account.
fn make_exchange_state(
    admin: &Pubkey,
    usdc_mint: &Pubkey,
    vault: &Pubkey,
    insurance: &Pubkey,
) -> Vec<u8> {
    let mut data = vec![0u8; 213]; // 8 disc + 205 body
    data[..8].copy_from_slice(&DISC_EXCHANGE_STATE);
    data[8..40].copy_from_slice(admin.as_ref()); // admin
                                                 // pending_admin at 40..72 = [0; 32]
    data[72..104].copy_from_slice(usdc_mint.as_ref()); // usdc_mint
    data[104..136].copy_from_slice(vault.as_ref()); // vault
    data[136..168].copy_from_slice(insurance.as_ref()); // insurance_fund
                                                        // market_count at 168..172 = 0
                                                        // total_deposits at 172..180 = 0
                                                        // total_volume at 180..196 = 0
                                                        // total_fees_collected at 196..204 = 0
                                                        // insurance_balance at 204..212 = 0
                                                        // paused at 212 = 0
    data
}

/// Build a market account.
fn make_market(product_id: u32, oracle_feed: &Pubkey, active: bool) -> Vec<u8> {
    let mut data = vec![0u8; 113]; // 8 disc + 105 body
    data[..8].copy_from_slice(&DISC_MARKET);
    data[8..12].copy_from_slice(&product_id.to_le_bytes());
    data[12..44].copy_from_slice(oracle_feed.as_ref()); // oracle_price_feed
    data[44..48].copy_from_slice(&10u32.to_le_bytes()); // max_leverage = 10
    data[48..56].copy_from_slice(&1_000_000_000_000u64.to_le_bytes()); // max_open_interest
                                                                       // long/short_open_interest = 0
                                                                       // total_volume = 0
    let now = 1000i64;
    data[88..96].copy_from_slice(&now.to_le_bytes()); // last_funding_time
                                                      // cumulative_funding_rate_long/short = 0
    data[112] = if active { 1 } else { 0 };
    data
}

/// Build a user account.
fn make_user_account(owner: &Pubkey, balance: u64) -> Vec<u8> {
    let mut data = vec![0u8; 84]; // 8 disc + 76 body
    data[..8].copy_from_slice(&DISC_USER_ACCOUNT);
    data[8..40].copy_from_slice(owner.as_ref());
    data[40..48].copy_from_slice(&balance.to_le_bytes());
    data
}

/// Build a position account.
fn make_position(
    owner: &Pubkey,
    product_id: u32,
    side: u8,
    size: u64,
    entry_price: u64,
    leverage: u32,
    margin: u64,
) -> Vec<u8> {
    let mut data = vec![0u8; 113]; // 8 disc + 105 body
    data[..8].copy_from_slice(&DISC_POSITION);
    data[8..40].copy_from_slice(owner.as_ref());
    data[40..44].copy_from_slice(&product_id.to_le_bytes());
    data[44] = side;
    data[45..53].copy_from_slice(&size.to_le_bytes());
    data[53..61].copy_from_slice(&entry_price.to_le_bytes());
    data[61..65].copy_from_slice(&leverage.to_le_bytes());
    data[65..73].copy_from_slice(&margin.to_le_bytes());
    // liq_price, realized_pnl, entry_funding_rate, opened_at, last_updated = 0
    data
}

/// Build a minimal SPL token account.
fn make_token_account(mint: &Pubkey, owner: &Pubkey, amount: u64) -> Vec<u8> {
    let mut data = vec![0u8; 165];
    data[..32].copy_from_slice(mint.as_ref());
    data[32..64].copy_from_slice(owner.as_ref());
    data[64..72].copy_from_slice(&amount.to_le_bytes());
    // state = Initialized (byte 108)
    data[108] = 1;
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

// ══════════════════════════════════════════════════════════════════════════════
// ORACLE TESTS
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_initialize_oracle() {
    let mollusk = setup_mollusk();
    let pid = program_id();
    let authority = Pubkey::new_unique();

    let (oracle_state_pda, _) = find_pda(&[ORACLE_SEED]);
    let (publisher_list_pda, _) = find_pda(&[PUBLISHER_LIST_SEED]);

    let ix = Instruction {
        program_id: pid,
        accounts: vec![
            // Anchor order: oracle_state, publisher_list, authority, system_program
            AccountMeta::new(oracle_state_pda, false),
            AccountMeta::new(publisher_list_pda, false),
            AccountMeta::new(authority, true),
            AccountMeta::new_readonly(system_program::ID, false),
        ],
        data: build_ix_data(&DISC_INITIALIZE_ORACLE, &[]),
    };

    let accounts: Vec<(Pubkey, Account)> = vec![
        (oracle_state_pda, system_account(0)),
        (publisher_list_pda, system_account(0)),
        (authority, system_account(10_000_000_000)),
        system_program_account(),
    ];

    let result = mollusk.process_instruction(&ix, &accounts);
    assert!(
        result.program_result.is_ok(),
        "initialize_oracle failed: {:?}",
        result.program_result
    );

    // Verify oracle_state was created with correct authority
    let oracle_account = result
        .resulting_accounts
        .iter()
        .find(|(k, _)| *k == oracle_state_pda)
        .unwrap();
    assert_eq!(oracle_account.1.owner, pid);
    assert_eq!(&oracle_account.1.data[..8], &DISC_ORACLE_STATE);
    assert_eq!(&oracle_account.1.data[8..40], authority.as_ref());

    // Verify publisher_list was created
    let pl_account = result
        .resulting_accounts
        .iter()
        .find(|(k, _)| *k == publisher_list_pda)
        .unwrap();
    assert_eq!(pl_account.1.owner, pid);
    assert_eq!(&pl_account.1.data[..8], &DISC_PUBLISHER_LIST);
    // authority in publisher_list
    assert_eq!(&pl_account.1.data[8..40], authority.as_ref());
    // count = 1 (authority is first publisher)
    let count = u32::from_le_bytes(pl_account.1.data[40..44].try_into().unwrap());
    assert_eq!(count, 1);
    // first publisher = authority
    assert_eq!(&pl_account.1.data[44..76], authority.as_ref());
}

#[test]
fn test_create_feed() {
    let mollusk = setup_mollusk();
    let pid = program_id();
    let authority = Pubkey::new_unique();
    let product_id: u32 = 1;

    let (oracle_state_pda, _) = find_pda(&[ORACLE_SEED]);
    let (price_feed_pda, _) = find_pda(&[PRICE_FEED_SEED, &product_id.to_le_bytes()]);

    let ix = Instruction {
        program_id: pid,
        accounts: vec![
            // Anchor order: price_feed, oracle_state, authority, system_program
            AccountMeta::new(price_feed_pda, false),
            AccountMeta::new(oracle_state_pda, false),
            AccountMeta::new(authority, true),
            AccountMeta::new_readonly(system_program::ID, false),
        ],
        data: build_ix_data(&DISC_CREATE_FEED, &product_id.to_le_bytes()),
    };

    let accounts: Vec<(Pubkey, Account)> = vec![
        (price_feed_pda, system_account(0)),
        (
            oracle_state_pda,
            program_owned_account(make_oracle_state(&authority)),
        ),
        (authority, system_account(10_000_000_000)),
        system_program_account(),
    ];

    let result = mollusk.process_instruction(&ix, &accounts);
    assert!(
        result.program_result.is_ok(),
        "create_feed failed: {:?}",
        result.program_result
    );

    // Verify price_feed was created
    let feed_account = result
        .resulting_accounts
        .iter()
        .find(|(k, _)| *k == price_feed_pda)
        .unwrap();
    assert_eq!(feed_account.1.owner, pid);
    assert_eq!(&feed_account.1.data[..8], &DISC_PRICE_FEED);
    let stored_pid = u32::from_le_bytes(feed_account.1.data[8..12].try_into().unwrap());
    assert_eq!(stored_pid, product_id);
}

#[test]
fn test_add_and_remove_publisher() {
    let mollusk = setup_mollusk();
    let pid = program_id();
    let authority = Pubkey::new_unique();
    let new_publisher = Pubkey::new_unique();

    let (oracle_state_pda, _) = find_pda(&[ORACLE_SEED]);
    let (publisher_list_pda, _) = find_pda(&[PUBLISHER_LIST_SEED]);

    // ── Add Publisher ────────────────────────────────────────────────
    let mut add_data = DISC_ADD_PUBLISHER.to_vec();
    add_data.extend_from_slice(new_publisher.as_ref());

    let ix_add = Instruction {
        program_id: pid,
        accounts: vec![
            // Anchor order: publisher_list, oracle_state, authority
            AccountMeta::new(publisher_list_pda, false),
            AccountMeta::new(oracle_state_pda, false),
            AccountMeta::new_readonly(authority, true),
        ],
        data: add_data,
    };

    let accounts: Vec<(Pubkey, Account)> = vec![
        (
            publisher_list_pda,
            program_owned_account(make_publisher_list(&authority, &[authority])),
        ),
        (
            oracle_state_pda,
            program_owned_account(make_oracle_state(&authority)),
        ),
        (authority, system_account(1_000_000_000)),
    ];

    let result = mollusk.process_instruction(&ix_add, &accounts);
    assert!(
        result.program_result.is_ok(),
        "add_publisher failed: {:?}",
        result.program_result
    );

    // Verify count increased to 2
    let pl = result
        .resulting_accounts
        .iter()
        .find(|(k, _)| *k == publisher_list_pda)
        .unwrap();
    let count = u32::from_le_bytes(pl.1.data[40..44].try_into().unwrap());
    assert_eq!(count, 2);

    // ── Remove Publisher ─────────────────────────────────────────────
    let mut remove_data = DISC_REMOVE_PUBLISHER.to_vec();
    remove_data.extend_from_slice(new_publisher.as_ref());

    let ix_remove = Instruction {
        program_id: pid,
        accounts: vec![
            AccountMeta::new(publisher_list_pda, false),
            AccountMeta::new(oracle_state_pda, false),
            AccountMeta::new_readonly(authority, true),
        ],
        data: remove_data,
    };

    // Use the updated publisher list from the add result
    let updated_pl_account = Account {
        lamports: pl.1.lamports,
        data: pl.1.data.clone(),
        owner: pid,
        executable: false,
        rent_epoch: 0,
    };

    let accounts2: Vec<(Pubkey, Account)> = vec![
        (publisher_list_pda, updated_pl_account),
        (
            oracle_state_pda,
            program_owned_account(make_oracle_state(&authority)),
        ),
        (authority, system_account(1_000_000_000)),
    ];

    let result2 = mollusk.process_instruction(&ix_remove, &accounts2);
    assert!(
        result2.program_result.is_ok(),
        "remove_publisher failed: {:?}",
        result2.program_result
    );

    let pl2 = result2
        .resulting_accounts
        .iter()
        .find(|(k, _)| *k == publisher_list_pda)
        .unwrap();
    let count2 = u32::from_le_bytes(pl2.1.data[40..44].try_into().unwrap());
    assert_eq!(count2, 1);
}

#[test]
fn test_transfer_and_accept_authority() {
    let mollusk = setup_mollusk();
    let pid = program_id();
    let authority = Pubkey::new_unique();
    let new_authority = Pubkey::new_unique();

    let (oracle_state_pda, _) = find_pda(&[ORACLE_SEED]);
    let (publisher_list_pda, _) = find_pda(&[PUBLISHER_LIST_SEED]);

    // ── Transfer Authority ───────────────────────────────────────────
    let mut transfer_data = DISC_TRANSFER_AUTHORITY.to_vec();
    transfer_data.extend_from_slice(new_authority.as_ref());

    let ix_transfer = Instruction {
        program_id: pid,
        accounts: vec![
            // Anchor order: publisher_list, oracle_state, authority
            AccountMeta::new(publisher_list_pda, false),
            AccountMeta::new(oracle_state_pda, false),
            AccountMeta::new_readonly(authority, true),
        ],
        data: transfer_data,
    };

    let accounts: Vec<(Pubkey, Account)> = vec![
        (
            publisher_list_pda,
            program_owned_account(make_publisher_list(&authority, &[authority])),
        ),
        (
            oracle_state_pda,
            program_owned_account(make_oracle_state(&authority)),
        ),
        (authority, system_account(1_000_000_000)),
    ];

    let result = mollusk.process_instruction(&ix_transfer, &accounts);
    assert!(
        result.program_result.is_ok(),
        "transfer_authority failed: {:?}",
        result.program_result
    );

    // Verify pending_authority was set
    let oracle = result
        .resulting_accounts
        .iter()
        .find(|(k, _)| *k == oracle_state_pda)
        .unwrap();
    assert_eq!(&oracle.1.data[40..72], new_authority.as_ref());

    // ── Accept Authority ─────────────────────────────────────────────
    let ix_accept = Instruction {
        program_id: pid,
        accounts: vec![
            // Anchor order: oracle_state, publisher_list, new_authority
            AccountMeta::new(oracle_state_pda, false),
            AccountMeta::new(publisher_list_pda, false),
            AccountMeta::new_readonly(new_authority, true),
        ],
        data: build_ix_data(&DISC_ACCEPT_AUTHORITY, &[]),
    };

    let updated_oracle = Account {
        lamports: oracle.1.lamports,
        data: oracle.1.data.clone(),
        owner: pid,
        executable: false,
        rent_epoch: 0,
    };

    let accounts2: Vec<(Pubkey, Account)> = vec![
        (oracle_state_pda, updated_oracle),
        (
            publisher_list_pda,
            program_owned_account(make_publisher_list(&authority, &[authority])),
        ),
        (new_authority, system_account(1_000_000_000)),
    ];

    let result2 = mollusk.process_instruction(&ix_accept, &accounts2);
    assert!(
        result2.program_result.is_ok(),
        "accept_authority failed: {:?}",
        result2.program_result
    );

    // Verify authority changed
    let oracle2 = result2
        .resulting_accounts
        .iter()
        .find(|(k, _)| *k == oracle_state_pda)
        .unwrap();
    assert_eq!(&oracle2.1.data[8..40], new_authority.as_ref());
    // pending_authority should be zeroed
    assert_eq!(&oracle2.1.data[40..72], &[0u8; 32]);
}

#[test]
fn test_publish_price() {
    let mollusk = setup_mollusk();
    let pid = program_id();
    let publisher = Pubkey::new_unique();
    let product_id: u32 = 1;
    let price: u64 = 1_050_000_000; // $10.50

    let (price_feed_pda, _) = find_pda(&[PRICE_FEED_SEED, &product_id.to_le_bytes()]);
    let (publisher_list_pda, _) = find_pda(&[PUBLISHER_LIST_SEED]);

    let mut ix_data = DISC_PUBLISH_PRICE.to_vec();
    ix_data.extend_from_slice(&product_id.to_le_bytes());
    ix_data.extend_from_slice(&price.to_le_bytes());

    let ix = Instruction {
        program_id: pid,
        accounts: vec![
            // Anchor order: publisher, price_feed, publisher_list
            AccountMeta::new(publisher, true),
            AccountMeta::new(price_feed_pda, false),
            AccountMeta::new_readonly(publisher_list_pda, false),
        ],
        data: ix_data,
    };

    let accounts: Vec<(Pubkey, Account)> = vec![
        (publisher, system_account(1_000_000_000)),
        (
            price_feed_pda,
            program_owned_account(make_price_feed(product_id, 0, 0, &publisher)),
        ),
        (
            publisher_list_pda,
            program_owned_account(make_publisher_list(&publisher, &[publisher])),
        ),
    ];

    let result = mollusk.process_instruction(&ix, &accounts);
    assert!(
        result.program_result.is_ok(),
        "publish_price failed: {:?}",
        result.program_result
    );

    // Verify price was updated
    let feed = result
        .resulting_accounts
        .iter()
        .find(|(k, _)| *k == price_feed_pda)
        .unwrap();
    let stored_price = u64::from_le_bytes(feed.1.data[12..20].try_into().unwrap());
    assert_eq!(stored_price, price);
}

#[test]
fn test_publish_prices_batch() {
    let mollusk = setup_mollusk();
    let pid = program_id();
    let publisher = Pubkey::new_unique();

    let product1: u32 = 1;
    let product2: u32 = 2;
    let price1: u64 = 1_000_000_000;
    let price2: u64 = 2_000_000_000;

    let (feed_pda1, _) = find_pda(&[PRICE_FEED_SEED, &product1.to_le_bytes()]);
    let (feed_pda2, _) = find_pda(&[PRICE_FEED_SEED, &product2.to_le_bytes()]);
    let (publisher_list_pda, _) = find_pda(&[PUBLISHER_LIST_SEED]);

    // Batch data: [4-byte count][N * (4-byte product_id + 8-byte price)]
    let mut ix_data = DISC_PUBLISH_PRICES_BATCH.to_vec();
    ix_data.extend_from_slice(&2u32.to_le_bytes()); // count = 2
    ix_data.extend_from_slice(&product1.to_le_bytes());
    ix_data.extend_from_slice(&price1.to_le_bytes());
    ix_data.extend_from_slice(&product2.to_le_bytes());
    ix_data.extend_from_slice(&price2.to_le_bytes());

    let ix = Instruction {
        program_id: pid,
        accounts: vec![
            // Anchor order: publisher, publisher_list, ...remaining feeds
            AccountMeta::new(publisher, true),
            AccountMeta::new_readonly(publisher_list_pda, false),
            AccountMeta::new(feed_pda1, false),
            AccountMeta::new(feed_pda2, false),
        ],
        data: ix_data,
    };

    let accounts: Vec<(Pubkey, Account)> = vec![
        (publisher, system_account(1_000_000_000)),
        (
            publisher_list_pda,
            program_owned_account(make_publisher_list(&publisher, &[publisher])),
        ),
        (
            feed_pda1,
            program_owned_account(make_price_feed(product1, 0, 0, &publisher)),
        ),
        (
            feed_pda2,
            program_owned_account(make_price_feed(product2, 0, 0, &publisher)),
        ),
    ];

    let result = mollusk.process_instruction(&ix, &accounts);
    assert!(
        result.program_result.is_ok(),
        "publish_prices_batch failed: {:?}",
        result.program_result
    );

    // Verify both feeds updated
    let f1 = result
        .resulting_accounts
        .iter()
        .find(|(k, _)| *k == feed_pda1)
        .unwrap();
    let f2 = result
        .resulting_accounts
        .iter()
        .find(|(k, _)| *k == feed_pda2)
        .unwrap();
    assert_eq!(
        u64::from_le_bytes(f1.1.data[12..20].try_into().unwrap()),
        price1
    );
    assert_eq!(
        u64::from_le_bytes(f2.1.data[12..20].try_into().unwrap()),
        price2
    );
}

// ══════════════════════════════════════════════════════════════════════════════
// PERPDEX TESTS
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_initialize_exchange() {
    let mut mollusk = setup_mollusk();
    // Load the SPL Token program so CPI calls to it work
    spl_token_program::add_program(&mut mollusk);

    let pid = program_id();
    let admin = Pubkey::new_unique();
    let usdc_mint = Pubkey::new_unique();

    let (exchange_pda, _) = find_pda(&[EXCHANGE_SEED]);
    let (vault_pda, _) = find_pda(&[VAULT_SEED]);
    let (insurance_pda, _) = find_pda(&[INSURANCE_SEED]);

    let ix = Instruction {
        program_id: pid,
        accounts: vec![
            // Anchor order: exchange_state, usdc_mint, vault, insurance_fund, admin, system, token, rent
            AccountMeta::new(exchange_pda, false),
            AccountMeta::new_readonly(usdc_mint, false),
            AccountMeta::new(vault_pda, false),
            AccountMeta::new(insurance_pda, false),
            AccountMeta::new(admin, true),
            AccountMeta::new_readonly(system_program::ID, false),
            AccountMeta::new_readonly(spl_token_id(), false),
            AccountMeta::new_readonly(sysvar::rent::ID, false),
        ],
        data: build_ix_data(&DISC_INITIALIZE_EXCHANGE, &[]),
    };

    // Mint account needs to be a valid SPL Mint (82 bytes)
    let mut mint_data = vec![0u8; 82];
    mint_data[44] = 6; // decimals = 6
    mint_data[45] = 1; // is_initialized = true

    let accounts: Vec<(Pubkey, Account)> = vec![
        (exchange_pda, system_account(0)),
        (
            usdc_mint,
            Account {
                lamports: 1_000_000_000,
                data: mint_data,
                owner: spl_token_id(),
                executable: false,
                rent_epoch: 0,
            },
        ),
        (vault_pda, system_account(0)),
        (insurance_pda, system_account(0)),
        (admin, system_account(100_000_000_000)),
        system_program_account(),
        spl_token_program::keyed_account(),
        mollusk.sysvars.keyed_account_for_rent_sysvar(),
    ];

    let result = mollusk.process_instruction(&ix, &accounts);
    assert!(
        result.program_result.is_ok(),
        "initialize_exchange failed: {:?}",
        result.program_result
    );

    // Verify exchange_state was created
    let exchange = result
        .resulting_accounts
        .iter()
        .find(|(k, _)| *k == exchange_pda)
        .unwrap();
    assert_eq!(exchange.1.owner, pid);
    assert_eq!(&exchange.1.data[..8], &DISC_EXCHANGE_STATE);
    assert_eq!(&exchange.1.data[8..40], admin.as_ref());

    // Verify vault was created as a token account
    let vault = result
        .resulting_accounts
        .iter()
        .find(|(k, _)| *k == vault_pda)
        .unwrap();
    assert_eq!(vault.1.owner, spl_token_id());
    assert_eq!(vault.1.data.len(), 165);

    // Verify insurance_fund was created as a token account
    let insurance = result
        .resulting_accounts
        .iter()
        .find(|(k, _)| *k == insurance_pda)
        .unwrap();
    assert_eq!(insurance.1.owner, spl_token_id());
    assert_eq!(insurance.1.data.len(), 165);
}

#[test]
fn test_create_market() {
    let mollusk = setup_mollusk();
    let pid = program_id();
    let admin = Pubkey::new_unique();
    let product_id: u32 = 1;
    let max_leverage: u32 = 10;
    let max_oi: u64 = 1_000_000_000_000;

    let usdc_mint = Pubkey::new_unique();
    let vault = Pubkey::new_unique();
    let insurance = Pubkey::new_unique();

    let (exchange_pda, _) = find_pda(&[EXCHANGE_SEED]);
    let (market_pda, _) = find_pda(&[MARKET_SEED, &product_id.to_le_bytes()]);

    // Create a valid price feed for the oracle
    let (price_feed_pda, _) = find_pda(&[PRICE_FEED_SEED, &product_id.to_le_bytes()]);

    let mut ix_data = DISC_CREATE_MARKET.to_vec();
    ix_data.extend_from_slice(&product_id.to_le_bytes());
    ix_data.extend_from_slice(&max_leverage.to_le_bytes());
    ix_data.extend_from_slice(&max_oi.to_le_bytes());

    let ix = Instruction {
        program_id: pid,
        accounts: vec![
            // Anchor order: market, exchange_state, oracle_price_feed, admin, system_program
            AccountMeta::new(market_pda, false),
            AccountMeta::new(exchange_pda, false),
            AccountMeta::new_readonly(price_feed_pda, false),
            AccountMeta::new(admin, true),
            AccountMeta::new_readonly(system_program::ID, false),
        ],
        data: ix_data,
    };

    let accounts: Vec<(Pubkey, Account)> = vec![
        (market_pda, system_account(0)),
        (
            exchange_pda,
            program_owned_account(make_exchange_state(&admin, &usdc_mint, &vault, &insurance)),
        ),
        (
            price_feed_pda,
            program_owned_account(make_price_feed(product_id, 1_000_000_000, 1000, &admin)),
        ),
        (admin, system_account(10_000_000_000)),
        system_program_account(),
    ];

    let result = mollusk.process_instruction(&ix, &accounts);
    assert!(
        result.program_result.is_ok(),
        "create_market failed: {:?}",
        result.program_result
    );

    let market = result
        .resulting_accounts
        .iter()
        .find(|(k, _)| *k == market_pda)
        .unwrap();
    assert_eq!(market.1.owner, pid);
    assert_eq!(&market.1.data[..8], &DISC_MARKET);
    let stored_pid = u32::from_le_bytes(market.1.data[8..12].try_into().unwrap());
    assert_eq!(stored_pid, product_id);
}

#[test]
fn test_create_market_permissionless() {
    let mollusk = setup_mollusk();
    let pid = program_id();
    let payer = Pubkey::new_unique();
    let admin = Pubkey::new_unique();
    let product_id: u32 = 2;

    let usdc_mint = Pubkey::new_unique();
    let vault = Pubkey::new_unique();
    let insurance = Pubkey::new_unique();

    let (exchange_pda, _) = find_pda(&[EXCHANGE_SEED]);
    let (market_pda, _) = find_pda(&[MARKET_SEED, &product_id.to_le_bytes()]);
    let (price_feed_pda, _) = find_pda(&[PRICE_FEED_SEED, &product_id.to_le_bytes()]);

    let mut ix_data = DISC_CREATE_MARKET_PERMISSIONLESS.to_vec();
    ix_data.extend_from_slice(&product_id.to_le_bytes());

    let ix = Instruction {
        program_id: pid,
        accounts: vec![
            // Anchor order: market, exchange_state, oracle_price_feed, payer, system_program
            AccountMeta::new(market_pda, false),
            AccountMeta::new(exchange_pda, false),
            AccountMeta::new_readonly(price_feed_pda, false),
            AccountMeta::new(payer, true),
            AccountMeta::new_readonly(system_program::ID, false),
        ],
        data: ix_data,
    };

    let accounts: Vec<(Pubkey, Account)> = vec![
        (market_pda, system_account(0)),
        (
            exchange_pda,
            program_owned_account(make_exchange_state(&admin, &usdc_mint, &vault, &insurance)),
        ),
        (
            price_feed_pda,
            program_owned_account(make_price_feed(product_id, 1_000_000_000, 1000, &admin)),
        ),
        (payer, system_account(10_000_000_000)),
        system_program_account(),
    ];

    let result = mollusk.process_instruction(&ix, &accounts);
    assert!(
        result.program_result.is_ok(),
        "create_market_permissionless failed: {:?}",
        result.program_result
    );
}

#[test]
fn test_create_user_account() {
    let mollusk = setup_mollusk();
    let pid = program_id();
    let owner = Pubkey::new_unique();

    let (user_account_pda, _) = find_pda(&[USER_ACCOUNT_SEED, owner.as_ref()]);

    let ix = Instruction {
        program_id: pid,
        accounts: vec![
            // Anchor order: user_account, owner, system_program
            AccountMeta::new(user_account_pda, false),
            AccountMeta::new(owner, true),
            AccountMeta::new_readonly(system_program::ID, false),
        ],
        data: build_ix_data(&DISC_CREATE_USER_ACCOUNT, &[]),
    };

    let accounts: Vec<(Pubkey, Account)> = vec![
        (user_account_pda, system_account(0)),
        (owner, system_account(10_000_000_000)),
        system_program_account(),
    ];

    let result = mollusk.process_instruction(&ix, &accounts);
    assert!(
        result.program_result.is_ok(),
        "create_user_account failed: {:?}",
        result.program_result
    );

    let ua = result
        .resulting_accounts
        .iter()
        .find(|(k, _)| *k == user_account_pda)
        .unwrap();
    assert_eq!(ua.1.owner, pid);
    assert_eq!(&ua.1.data[..8], &DISC_USER_ACCOUNT);
    assert_eq!(&ua.1.data[8..40], owner.as_ref());
}

#[test]
fn test_close_user_account() {
    let mollusk = setup_mollusk();
    let pid = program_id();
    let owner = Pubkey::new_unique();
    let (user_account_pda, _) = find_pda(&[USER_ACCOUNT_SEED, owner.as_ref()]);

    let ix = Instruction {
        program_id: pid,
        accounts: vec![
            // Anchor order: user_account, owner
            AccountMeta::new(user_account_pda, false),
            AccountMeta::new(owner, true),
        ],
        data: build_ix_data(&DISC_CLOSE_USER_ACCOUNT, &[]),
    };

    // balance = 0, open_position_count = 0 → should succeed
    let accounts: Vec<(Pubkey, Account)> = vec![
        (
            user_account_pda,
            program_owned_account(make_user_account(&owner, 0)),
        ),
        (owner, system_account(1_000_000_000)),
    ];

    let result = mollusk.process_instruction(&ix, &accounts);
    assert!(
        result.program_result.is_ok(),
        "close_user_account failed: {:?}",
        result.program_result
    );
}

#[test]
fn test_set_paused() {
    let mollusk = setup_mollusk();
    let pid = program_id();
    let admin = Pubkey::new_unique();
    let usdc_mint = Pubkey::new_unique();
    let vault = Pubkey::new_unique();
    let insurance = Pubkey::new_unique();

    let (exchange_pda, _) = find_pda(&[EXCHANGE_SEED]);

    // Set paused = true
    let ix = Instruction {
        program_id: pid,
        accounts: vec![
            // Anchor order: exchange_state, admin
            AccountMeta::new(exchange_pda, false),
            AccountMeta::new_readonly(admin, true),
        ],
        data: build_ix_data(&DISC_SET_PAUSED, &[1u8]), // paused = true
    };

    let accounts: Vec<(Pubkey, Account)> = vec![
        (
            exchange_pda,
            program_owned_account(make_exchange_state(&admin, &usdc_mint, &vault, &insurance)),
        ),
        (admin, system_account(1_000_000_000)),
    ];

    let result = mollusk.process_instruction(&ix, &accounts);
    assert!(
        result.program_result.is_ok(),
        "set_paused failed: {:?}",
        result.program_result
    );

    let exchange = result
        .resulting_accounts
        .iter()
        .find(|(k, _)| *k == exchange_pda)
        .unwrap();
    assert_eq!(exchange.1.data[212], 1); // paused = true
}

#[test]
fn test_transfer_and_accept_admin() {
    let mollusk = setup_mollusk();
    let pid = program_id();
    let admin = Pubkey::new_unique();
    let new_admin = Pubkey::new_unique();
    let usdc_mint = Pubkey::new_unique();
    let vault = Pubkey::new_unique();
    let insurance = Pubkey::new_unique();

    let (exchange_pda, _) = find_pda(&[EXCHANGE_SEED]);

    // ── Transfer Admin ───────────────────────────────────────────────
    let mut transfer_data = DISC_TRANSFER_ADMIN.to_vec();
    transfer_data.extend_from_slice(new_admin.as_ref());

    let ix_transfer = Instruction {
        program_id: pid,
        accounts: vec![
            // Anchor order: exchange_state, admin
            AccountMeta::new(exchange_pda, false),
            AccountMeta::new_readonly(admin, true),
        ],
        data: transfer_data,
    };

    let accounts: Vec<(Pubkey, Account)> = vec![
        (
            exchange_pda,
            program_owned_account(make_exchange_state(&admin, &usdc_mint, &vault, &insurance)),
        ),
        (admin, system_account(1_000_000_000)),
    ];

    let result = mollusk.process_instruction(&ix_transfer, &accounts);
    assert!(
        result.program_result.is_ok(),
        "transfer_admin failed: {:?}",
        result.program_result
    );

    let exchange = result
        .resulting_accounts
        .iter()
        .find(|(k, _)| *k == exchange_pda)
        .unwrap();
    assert_eq!(&exchange.1.data[40..72], new_admin.as_ref()); // pending_admin

    // ── Accept Admin ─────────────────────────────────────────────────
    let ix_accept = Instruction {
        program_id: pid,
        accounts: vec![
            // Anchor order: exchange_state, new_admin
            AccountMeta::new(exchange_pda, false),
            AccountMeta::new_readonly(new_admin, true),
        ],
        data: build_ix_data(&DISC_ACCEPT_ADMIN, &[]),
    };

    let updated_exchange = Account {
        lamports: exchange.1.lamports,
        data: exchange.1.data.clone(),
        owner: pid,
        executable: false,
        rent_epoch: 0,
    };

    let accounts2: Vec<(Pubkey, Account)> = vec![
        (exchange_pda, updated_exchange),
        (new_admin, system_account(1_000_000_000)),
    ];

    let result2 = mollusk.process_instruction(&ix_accept, &accounts2);
    assert!(
        result2.program_result.is_ok(),
        "accept_admin failed: {:?}",
        result2.program_result
    );

    let exchange2 = result2
        .resulting_accounts
        .iter()
        .find(|(k, _)| *k == exchange_pda)
        .unwrap();
    assert_eq!(&exchange2.1.data[8..40], new_admin.as_ref()); // admin = new_admin
    assert_eq!(&exchange2.1.data[40..72], &[0u8; 32]); // pending_admin zeroed
}

#[test]
fn test_set_market_active() {
    let mollusk = setup_mollusk();
    let pid = program_id();
    let admin = Pubkey::new_unique();
    let product_id: u32 = 1;
    let usdc_mint = Pubkey::new_unique();
    let vault = Pubkey::new_unique();
    let insurance = Pubkey::new_unique();

    let (exchange_pda, _) = find_pda(&[EXCHANGE_SEED]);
    let (market_pda, _) = find_pda(&[MARKET_SEED, &product_id.to_le_bytes()]);
    let oracle_feed = Pubkey::new_unique();

    let mut ix_data = DISC_SET_MARKET_ACTIVE.to_vec();
    ix_data.extend_from_slice(&product_id.to_le_bytes());
    ix_data.push(0); // active = false

    let ix = Instruction {
        program_id: pid,
        accounts: vec![
            // Anchor order: market, exchange_state, admin
            AccountMeta::new(market_pda, false),
            AccountMeta::new_readonly(exchange_pda, false),
            AccountMeta::new_readonly(admin, true),
        ],
        data: ix_data,
    };

    let accounts: Vec<(Pubkey, Account)> = vec![
        (
            market_pda,
            program_owned_account(make_market(product_id, &oracle_feed, true)),
        ),
        (
            exchange_pda,
            program_owned_account(make_exchange_state(&admin, &usdc_mint, &vault, &insurance)),
        ),
        (admin, system_account(1_000_000_000)),
    ];

    let result = mollusk.process_instruction(&ix, &accounts);
    assert!(
        result.program_result.is_ok(),
        "set_market_active failed: {:?}",
        result.program_result
    );

    let market = result
        .resulting_accounts
        .iter()
        .find(|(k, _)| *k == market_pda)
        .unwrap();
    assert_eq!(market.1.data[112], 0); // active = false
}

#[test]
fn test_update_market_params() {
    let mollusk = setup_mollusk();
    let pid = program_id();
    let admin = Pubkey::new_unique();
    let product_id: u32 = 1;
    let new_leverage: u32 = 20;
    let new_oi: u64 = 5_000_000_000_000;
    let usdc_mint = Pubkey::new_unique();
    let vault = Pubkey::new_unique();
    let insurance = Pubkey::new_unique();

    let (exchange_pda, _) = find_pda(&[EXCHANGE_SEED]);
    let (market_pda, _) = find_pda(&[MARKET_SEED, &product_id.to_le_bytes()]);
    let oracle_feed = Pubkey::new_unique();

    let mut ix_data = DISC_UPDATE_MARKET_PARAMS.to_vec();
    ix_data.extend_from_slice(&product_id.to_le_bytes());
    ix_data.extend_from_slice(&new_leverage.to_le_bytes());
    ix_data.extend_from_slice(&new_oi.to_le_bytes());

    let ix = Instruction {
        program_id: pid,
        accounts: vec![
            // Anchor order: market, exchange_state, admin
            AccountMeta::new(market_pda, false),
            AccountMeta::new_readonly(exchange_pda, false),
            AccountMeta::new_readonly(admin, true),
        ],
        data: ix_data,
    };

    let accounts: Vec<(Pubkey, Account)> = vec![
        (
            market_pda,
            program_owned_account(make_market(product_id, &oracle_feed, true)),
        ),
        (
            exchange_pda,
            program_owned_account(make_exchange_state(&admin, &usdc_mint, &vault, &insurance)),
        ),
        (admin, system_account(1_000_000_000)),
    ];

    let result = mollusk.process_instruction(&ix, &accounts);
    assert!(
        result.program_result.is_ok(),
        "update_market_params failed: {:?}",
        result.program_result
    );

    let market = result
        .resulting_accounts
        .iter()
        .find(|(k, _)| *k == market_pda)
        .unwrap();
    let leverage = u32::from_le_bytes(market.1.data[44..48].try_into().unwrap());
    let oi = u64::from_le_bytes(market.1.data[48..56].try_into().unwrap());
    assert_eq!(leverage, new_leverage);
    assert_eq!(oi, new_oi);
}

#[test]
fn test_update_funding_rate() {
    let mut mollusk = setup_mollusk();
    let pid = program_id();
    let cranker = Pubkey::new_unique();
    let product_id: u32 = 1;
    let admin = Pubkey::new_unique();
    let usdc_mint = Pubkey::new_unique();
    let vault = Pubkey::new_unique();
    let insurance = Pubkey::new_unique();

    let (exchange_pda, _) = find_pda(&[EXCHANGE_SEED]);
    let (market_pda, _) = find_pda(&[MARKET_SEED, &product_id.to_le_bytes()]);
    let oracle_feed = Pubkey::new_unique();

    // Make market with last_funding_time far enough in the past (>= 28800s)
    let mut market_data = make_market(product_id, &oracle_feed, true);
    // Set last_funding_time to 0 so the 8-hour interval has passed
    let far_past: i64 = 0;
    market_data[88..96].copy_from_slice(&far_past.to_le_bytes());

    // Set clock unix_timestamp to 30000 (> 28800s funding interval)
    mollusk.sysvars.warp_to_slot(100);
    mollusk.sysvars.clock.unix_timestamp = 30_000;

    let mut ix_data = DISC_UPDATE_FUNDING_RATE.to_vec();
    ix_data.extend_from_slice(&product_id.to_le_bytes());

    let ix = Instruction {
        program_id: pid,
        accounts: vec![
            // Anchor order: market, exchange_state, cranker
            AccountMeta::new(market_pda, false),
            AccountMeta::new_readonly(exchange_pda, false),
            AccountMeta::new_readonly(cranker, true),
        ],
        data: ix_data,
    };

    let accounts: Vec<(Pubkey, Account)> = vec![
        (market_pda, program_owned_account(market_data)),
        (
            exchange_pda,
            program_owned_account(make_exchange_state(&admin, &usdc_mint, &vault, &insurance)),
        ),
        (cranker, system_account(1_000_000_000)),
    ];

    let result = mollusk.process_instruction(&ix, &accounts);
    assert!(
        result.program_result.is_ok(),
        "update_funding_rate failed: {:?}",
        result.program_result
    );

    // last_funding_time should be updated
    let market = result
        .resulting_accounts
        .iter()
        .find(|(k, _)| *k == market_pda)
        .unwrap();
    let new_funding_time = i64::from_le_bytes(market.1.data[88..96].try_into().unwrap());
    assert!(new_funding_time > 0);
}

#[test]
fn test_add_margin() {
    let mollusk = setup_mollusk();
    let pid = program_id();
    let owner = Pubkey::new_unique();
    let product_id: u32 = 1;
    let add_amount: u64 = 500_000; // $0.50

    let (user_account_pda, _) = find_pda(&[USER_ACCOUNT_SEED, owner.as_ref()]);
    let (position_pda, _) = find_pda(&[POSITION_SEED, owner.as_ref(), &product_id.to_le_bytes()]);

    let mut ix_data = DISC_ADD_MARGIN.to_vec();
    ix_data.extend_from_slice(&product_id.to_le_bytes());
    ix_data.extend_from_slice(&add_amount.to_le_bytes());

    let ix = Instruction {
        program_id: pid,
        accounts: vec![
            // Anchor order: user_account, position, owner
            AccountMeta::new(user_account_pda, false),
            AccountMeta::new(position_pda, false),
            AccountMeta::new(owner, true),
        ],
        data: ix_data,
    };

    let initial_balance: u64 = 10_000_000; // $10
    let initial_margin: u64 = 1_000_000; // $1

    let accounts: Vec<(Pubkey, Account)> = vec![
        (
            user_account_pda,
            program_owned_account(make_user_account(&owner, initial_balance)),
        ),
        (
            position_pda,
            program_owned_account(make_position(
                &owner,
                product_id,
                0,
                10_000_000,
                1_000_000_000,
                10,
                initial_margin,
            )),
        ),
        (owner, system_account(1_000_000_000)),
    ];

    let result = mollusk.process_instruction(&ix, &accounts);
    assert!(
        result.program_result.is_ok(),
        "add_margin failed: {:?}",
        result.program_result
    );

    // Verify balance decreased and margin increased
    let ua = result
        .resulting_accounts
        .iter()
        .find(|(k, _)| *k == user_account_pda)
        .unwrap();
    let new_balance = u64::from_le_bytes(ua.1.data[40..48].try_into().unwrap());
    assert_eq!(new_balance, initial_balance - add_amount);

    let pos = result
        .resulting_accounts
        .iter()
        .find(|(k, _)| *k == position_pda)
        .unwrap();
    let new_margin = u64::from_le_bytes(pos.1.data[65..73].try_into().unwrap());
    assert_eq!(new_margin, initial_margin + add_amount);
}

// ══════════════════════════════════════════════════════════════════════════════
// NEGATIVE TESTS (authorization, validation)
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_publish_price_unauthorized_publisher() {
    let mollusk = setup_mollusk();
    let pid = program_id();
    let authorized_publisher = Pubkey::new_unique();
    let unauthorized = Pubkey::new_unique();
    let product_id: u32 = 1;

    let (price_feed_pda, _) = find_pda(&[PRICE_FEED_SEED, &product_id.to_le_bytes()]);
    let (publisher_list_pda, _) = find_pda(&[PUBLISHER_LIST_SEED]);

    let mut ix_data = DISC_PUBLISH_PRICE.to_vec();
    ix_data.extend_from_slice(&product_id.to_le_bytes());
    ix_data.extend_from_slice(&1_000_000_000u64.to_le_bytes());

    let ix = Instruction {
        program_id: pid,
        accounts: vec![
            AccountMeta::new(unauthorized, true),
            AccountMeta::new(price_feed_pda, false),
            AccountMeta::new_readonly(publisher_list_pda, false),
        ],
        data: ix_data,
    };

    let accounts: Vec<(Pubkey, Account)> = vec![
        (unauthorized, system_account(1_000_000_000)),
        (
            price_feed_pda,
            program_owned_account(make_price_feed(product_id, 0, 0, &authorized_publisher)),
        ),
        (
            publisher_list_pda,
            program_owned_account(make_publisher_list(
                &authorized_publisher,
                &[authorized_publisher],
            )),
        ),
    ];

    let result = mollusk.process_instruction(&ix, &accounts);
    assert!(
        result.program_result.is_err(),
        "publish_price should fail for unauthorized publisher"
    );
}

#[test]
fn test_set_paused_wrong_admin() {
    let mollusk = setup_mollusk();
    let pid = program_id();
    let admin = Pubkey::new_unique();
    let wrong_admin = Pubkey::new_unique();
    let usdc_mint = Pubkey::new_unique();
    let vault = Pubkey::new_unique();
    let insurance = Pubkey::new_unique();

    let (exchange_pda, _) = find_pda(&[EXCHANGE_SEED]);

    let ix = Instruction {
        program_id: pid,
        accounts: vec![
            AccountMeta::new(exchange_pda, false),
            AccountMeta::new_readonly(wrong_admin, true),
        ],
        data: build_ix_data(&DISC_SET_PAUSED, &[1u8]),
    };

    let accounts: Vec<(Pubkey, Account)> = vec![
        (
            exchange_pda,
            program_owned_account(make_exchange_state(&admin, &usdc_mint, &vault, &insurance)),
        ),
        (wrong_admin, system_account(1_000_000_000)),
    ];

    let result = mollusk.process_instruction(&ix, &accounts);
    assert!(
        result.program_result.is_err(),
        "set_paused should fail with wrong admin"
    );
}

#[test]
fn test_close_user_account_with_balance() {
    let mollusk = setup_mollusk();
    let pid = program_id();
    let owner = Pubkey::new_unique();
    let (user_account_pda, _) = find_pda(&[USER_ACCOUNT_SEED, owner.as_ref()]);

    let ix = Instruction {
        program_id: pid,
        accounts: vec![
            AccountMeta::new(user_account_pda, false),
            AccountMeta::new(owner, true),
        ],
        data: build_ix_data(&DISC_CLOSE_USER_ACCOUNT, &[]),
    };

    // Has balance → should fail
    let accounts: Vec<(Pubkey, Account)> = vec![
        (
            user_account_pda,
            program_owned_account(make_user_account(&owner, 1_000_000)),
        ),
        (owner, system_account(1_000_000_000)),
    ];

    let result = mollusk.process_instruction(&ix, &accounts);
    assert!(
        result.program_result.is_err(),
        "close_user_account should fail with non-zero balance"
    );
}

#[test]
fn test_transfer_authority_to_default_rejected() {
    let mollusk = setup_mollusk();
    let pid = program_id();
    let authority = Pubkey::new_unique();

    let (oracle_state_pda, _) = find_pda(&[ORACLE_SEED]);
    let (publisher_list_pda, _) = find_pda(&[PUBLISHER_LIST_SEED]);

    // Try to transfer to Pubkey::default() → should fail
    let mut transfer_data = DISC_TRANSFER_AUTHORITY.to_vec();
    transfer_data.extend_from_slice(&[0u8; 32]); // Pubkey::default()

    let ix = Instruction {
        program_id: pid,
        accounts: vec![
            AccountMeta::new(publisher_list_pda, false),
            AccountMeta::new(oracle_state_pda, false),
            AccountMeta::new_readonly(authority, true),
        ],
        data: transfer_data,
    };

    let accounts: Vec<(Pubkey, Account)> = vec![
        (
            publisher_list_pda,
            program_owned_account(make_publisher_list(&authority, &[authority])),
        ),
        (
            oracle_state_pda,
            program_owned_account(make_oracle_state(&authority)),
        ),
        (authority, system_account(1_000_000_000)),
    ];

    let result = mollusk.process_instruction(&ix, &accounts);
    assert!(
        result.program_result.is_err(),
        "transfer_authority to default should fail"
    );
}

#[test]
fn test_publish_price_zero_rejected() {
    let mollusk = setup_mollusk();
    let pid = program_id();
    let publisher = Pubkey::new_unique();
    let product_id: u32 = 1;

    let (price_feed_pda, _) = find_pda(&[PRICE_FEED_SEED, &product_id.to_le_bytes()]);
    let (publisher_list_pda, _) = find_pda(&[PUBLISHER_LIST_SEED]);

    let mut ix_data = DISC_PUBLISH_PRICE.to_vec();
    ix_data.extend_from_slice(&product_id.to_le_bytes());
    ix_data.extend_from_slice(&0u64.to_le_bytes()); // price = 0

    let ix = Instruction {
        program_id: pid,
        accounts: vec![
            AccountMeta::new(publisher, true),
            AccountMeta::new(price_feed_pda, false),
            AccountMeta::new_readonly(publisher_list_pda, false),
        ],
        data: ix_data,
    };

    let accounts: Vec<(Pubkey, Account)> = vec![
        (publisher, system_account(1_000_000_000)),
        (
            price_feed_pda,
            program_owned_account(make_price_feed(product_id, 0, 0, &publisher)),
        ),
        (
            publisher_list_pda,
            program_owned_account(make_publisher_list(&publisher, &[publisher])),
        ),
    ];

    let result = mollusk.process_instruction(&ix, &accounts);
    assert!(
        result.program_result.is_err(),
        "publish_price with price=0 should fail"
    );
}

// ══════════════════════════════════════════════════════════════════════════════
// TOKEN CPI TESTS (deposit, withdraw, fund_insurance)
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_deposit() {
    let mut mollusk = setup_mollusk();
    spl_token_program::add_program(&mut mollusk);

    let pid = program_id();
    let owner = Pubkey::new_unique();
    let admin = Pubkey::new_unique();
    let usdc_mint = Pubkey::new_unique();
    let deposit_amount: u64 = 5_000_000; // $5

    let (exchange_pda, _) = find_pda(&[EXCHANGE_SEED]);
    let (vault_pda, _) = find_pda(&[VAULT_SEED]);
    let (user_account_pda, _) = find_pda(&[USER_ACCOUNT_SEED, owner.as_ref()]);

    let user_token = Pubkey::new_unique();

    let mut ix_data = DISC_DEPOSIT.to_vec();
    ix_data.extend_from_slice(&deposit_amount.to_le_bytes());

    let ix = Instruction {
        program_id: pid,
        accounts: vec![
            // Anchor order: user_account, exchange_state, vault, user_token_account, owner, token_program
            AccountMeta::new(user_account_pda, false),
            AccountMeta::new(exchange_pda, false),
            AccountMeta::new(vault_pda, false),
            AccountMeta::new(user_token, false),
            AccountMeta::new(owner, true),
            AccountMeta::new_readonly(spl_token_id(), false),
        ],
        data: ix_data,
    };

    let accounts: Vec<(Pubkey, Account)> = vec![
        (
            user_account_pda,
            program_owned_account(make_user_account(&owner, 0)),
        ),
        (
            exchange_pda,
            program_owned_account(make_exchange_state(
                &admin,
                &usdc_mint,
                &vault_pda,
                &Pubkey::new_unique(),
            )),
        ),
        (
            vault_pda,
            token_account_obj(make_token_account(&usdc_mint, &exchange_pda, 0)),
        ),
        (
            user_token,
            token_account_obj(make_token_account(&usdc_mint, &owner, 100_000_000)),
        ),
        (owner, system_account(1_000_000_000)),
        spl_token_program::keyed_account(),
    ];

    let result = mollusk.process_instruction(&ix, &accounts);
    assert!(
        result.program_result.is_ok(),
        "deposit failed: {:?}",
        result.program_result
    );

    // Verify user_account balance increased
    let ua = result
        .resulting_accounts
        .iter()
        .find(|(k, _)| *k == user_account_pda)
        .unwrap();
    let new_balance = u64::from_le_bytes(ua.1.data[40..48].try_into().unwrap());
    assert_eq!(new_balance, deposit_amount);

    // Verify user token account decreased
    let user_tok = result
        .resulting_accounts
        .iter()
        .find(|(k, _)| *k == user_token)
        .unwrap();
    let user_tok_amount = u64::from_le_bytes(user_tok.1.data[64..72].try_into().unwrap());
    assert_eq!(user_tok_amount, 100_000_000 - deposit_amount);

    // Verify vault token account increased
    let vault_tok = result
        .resulting_accounts
        .iter()
        .find(|(k, _)| *k == vault_pda)
        .unwrap();
    let vault_amount = u64::from_le_bytes(vault_tok.1.data[64..72].try_into().unwrap());
    assert_eq!(vault_amount, deposit_amount);
}

#[test]
fn test_withdraw() {
    let mut mollusk = setup_mollusk();
    spl_token_program::add_program(&mut mollusk);

    let pid = program_id();
    let owner = Pubkey::new_unique();
    let admin = Pubkey::new_unique();
    let usdc_mint = Pubkey::new_unique();
    let withdraw_amount: u64 = 3_000_000; // $3

    let (exchange_pda, _) = find_pda(&[EXCHANGE_SEED]);
    let (vault_pda, _) = find_pda(&[VAULT_SEED]);
    let (user_account_pda, _) = find_pda(&[USER_ACCOUNT_SEED, owner.as_ref()]);

    let user_token = Pubkey::new_unique();

    let mut ix_data = DISC_WITHDRAW.to_vec();
    ix_data.extend_from_slice(&withdraw_amount.to_le_bytes());

    let ix = Instruction {
        program_id: pid,
        accounts: vec![
            // Anchor order: user_account, exchange_state, vault, user_token_account, owner, token_program
            AccountMeta::new(user_account_pda, false),
            AccountMeta::new(exchange_pda, false),
            AccountMeta::new(vault_pda, false),
            AccountMeta::new(user_token, false),
            AccountMeta::new(owner, true),
            AccountMeta::new_readonly(spl_token_id(), false),
        ],
        data: ix_data,
    };

    // User has balance, no open positions, vault has tokens
    let accounts: Vec<(Pubkey, Account)> = vec![
        (
            user_account_pda,
            program_owned_account(make_user_account(&owner, 10_000_000)), // $10 balance
        ),
        (
            exchange_pda,
            program_owned_account(make_exchange_state(
                &admin,
                &usdc_mint,
                &vault_pda,
                &Pubkey::new_unique(),
            )),
        ),
        (
            vault_pda,
            token_account_obj(make_token_account(&usdc_mint, &exchange_pda, 50_000_000)), // vault has $50
        ),
        (
            user_token,
            token_account_obj(make_token_account(&usdc_mint, &owner, 0)),
        ),
        (owner, system_account(1_000_000_000)),
        spl_token_program::keyed_account(),
    ];

    let result = mollusk.process_instruction(&ix, &accounts);
    assert!(
        result.program_result.is_ok(),
        "withdraw failed: {:?}",
        result.program_result
    );

    // Verify user_account balance decreased
    let ua = result
        .resulting_accounts
        .iter()
        .find(|(k, _)| *k == user_account_pda)
        .unwrap();
    let new_balance = u64::from_le_bytes(ua.1.data[40..48].try_into().unwrap());
    assert_eq!(new_balance, 10_000_000 - withdraw_amount);

    // Verify user token account increased
    let user_tok = result
        .resulting_accounts
        .iter()
        .find(|(k, _)| *k == user_token)
        .unwrap();
    let user_tok_amount = u64::from_le_bytes(user_tok.1.data[64..72].try_into().unwrap());
    assert_eq!(user_tok_amount, withdraw_amount);

    // Verify vault decreased
    let vault_tok = result
        .resulting_accounts
        .iter()
        .find(|(k, _)| *k == vault_pda)
        .unwrap();
    let vault_amount = u64::from_le_bytes(vault_tok.1.data[64..72].try_into().unwrap());
    assert_eq!(vault_amount, 50_000_000 - withdraw_amount);
}

#[test]
fn test_fund_insurance() {
    let mut mollusk = setup_mollusk();
    spl_token_program::add_program(&mut mollusk);

    let pid = program_id();
    let admin = Pubkey::new_unique();
    let usdc_mint = Pubkey::new_unique();
    let fund_amount: u64 = 10_000_000; // $10

    let (exchange_pda, _) = find_pda(&[EXCHANGE_SEED]);
    let (insurance_pda, _) = find_pda(&[INSURANCE_SEED]);

    let admin_token = Pubkey::new_unique();

    let mut ix_data = DISC_FUND_INSURANCE.to_vec();
    ix_data.extend_from_slice(&fund_amount.to_le_bytes());

    let ix = Instruction {
        program_id: pid,
        accounts: vec![
            // Anchor order: exchange_state, insurance_fund, admin_token_account, admin, token_program
            AccountMeta::new(exchange_pda, false),
            AccountMeta::new(insurance_pda, false),
            AccountMeta::new(admin_token, false),
            AccountMeta::new(admin, true),
            AccountMeta::new_readonly(spl_token_id(), false),
        ],
        data: ix_data,
    };

    let accounts: Vec<(Pubkey, Account)> = vec![
        (
            exchange_pda,
            program_owned_account(make_exchange_state(
                &admin,
                &usdc_mint,
                &Pubkey::new_unique(),
                &insurance_pda,
            )),
        ),
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
    ];

    let result = mollusk.process_instruction(&ix, &accounts);
    assert!(
        result.program_result.is_ok(),
        "fund_insurance failed: {:?}",
        result.program_result
    );

    // Verify insurance fund received tokens
    let ins_tok = result
        .resulting_accounts
        .iter()
        .find(|(k, _)| *k == insurance_pda)
        .unwrap();
    let ins_amount = u64::from_le_bytes(ins_tok.1.data[64..72].try_into().unwrap());
    assert_eq!(ins_amount, fund_amount);

    // Verify admin token account decreased
    let admin_tok = result
        .resulting_accounts
        .iter()
        .find(|(k, _)| *k == admin_token)
        .unwrap();
    let admin_tok_amount = u64::from_le_bytes(admin_tok.1.data[64..72].try_into().unwrap());
    assert_eq!(admin_tok_amount, 100_000_000 - fund_amount);

    // Verify exchange insurance_balance increased
    let exchange = result
        .resulting_accounts
        .iter()
        .find(|(k, _)| *k == exchange_pda)
        .unwrap();
    let ins_balance = u64::from_le_bytes(exchange.1.data[204..212].try_into().unwrap());
    assert_eq!(ins_balance, fund_amount);
}
