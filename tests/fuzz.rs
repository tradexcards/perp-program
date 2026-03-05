/// Fuzz tests for the TradeX program.
///
/// These tests use pseudo-random inputs to stress test instruction validation,
/// boundary conditions, and error handling across all instruction paths.
///
/// Uses mollusk-svm to execute the compiled .so against randomized payloads.
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

const DISC_PUBLISH_PRICE: [u8; 8] = [0x75, 0x0d, 0x06, 0xab, 0x1d, 0xcc, 0x0b, 0x01];
const DISC_PUBLISH_PRICES_BATCH: [u8; 8] = [0xe7, 0x47, 0xef, 0x79, 0xc4, 0x70, 0x65, 0xe4];
const DISC_OPEN_POSITION: [u8; 8] = [0x87, 0x80, 0x2f, 0x4d, 0x0f, 0x98, 0xf0, 0x31];
const DISC_CLOSE_POSITION: [u8; 8] = [0x7b, 0x86, 0x51, 0x00, 0x31, 0x44, 0x62, 0x62];
const DISC_ADD_MARGIN: [u8; 8] = [0xd3, 0xee, 0xee, 0x5a, 0xdf, 0xe4, 0xe4, 0x4c];
const DISC_DEPOSIT: [u8; 8] = [0xf2, 0x23, 0xc6, 0x89, 0x52, 0xe1, 0xf2, 0xb6];
const DISC_WITHDRAW: [u8; 8] = [0xb7, 0x12, 0x46, 0x9c, 0x94, 0x6d, 0xa1, 0x22];
const DISC_LIQUIDATE: [u8; 8] = [0xdf, 0xb3, 0xe2, 0x7d, 0x30, 0x2e, 0x27, 0x4a];
const DISC_UPDATE_FUNDING_RATE: [u8; 8] = [0xc9, 0xb2, 0x74, 0xd4, 0xa6, 0x90, 0x48, 0xee];
const DISC_CREATE_MARKET: [u8; 8] = [0x67, 0xe2, 0x61, 0xeb, 0xc8, 0xbc, 0xfb, 0xfe];
const DISC_SET_PAUSED: [u8; 8] = [0x5b, 0x3c, 0x7d, 0xc0, 0xb0, 0xe1, 0xa6, 0xda];
const DISC_UPDATE_MARKET_PARAMS: [u8; 8] = [0x46, 0x75, 0xca, 0xbf, 0xcd, 0xae, 0x5c, 0x52];
const DISC_FUND_INSURANCE: [u8; 8] = [0x3a, 0xd8, 0x25, 0xa2, 0xc9, 0x32, 0x14, 0x42];

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
    let pid = program_id();
    Mollusk::new(&pid, "tradex_program")
}

fn find_pda(seeds: &[&[u8]]) -> (Pubkey, u8) {
    Pubkey::find_program_address(seeds, &program_id())
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

/// Simple deterministic PRNG for reproducible fuzz runs
struct SimplePrng {
    state: u64,
}

impl SimplePrng {
    fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    fn next_u64(&mut self) -> u64 {
        // xorshift64
        self.state ^= self.state << 13;
        self.state ^= self.state >> 7;
        self.state ^= self.state << 17;
        self.state
    }

    fn next_u32(&mut self) -> u32 {
        self.next_u64() as u32
    }

    fn next_range(&mut self, min: u64, max: u64) -> u64 {
        if min >= max {
            return min;
        }
        min + self.next_u64() % (max - min)
    }

    fn next_bool(&mut self) -> bool {
        self.next_u64() % 2 == 0
    }
}

// ══════════════════════════════════════════════════════════════════════════════
// FUZZ TEST 1: Random publish_price inputs
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn fuzz_publish_price_random_prices() {
    let mollusk = setup_mollusk();
    let pid = program_id();
    let publisher = Pubkey::new_unique();
    let product_id: u32 = 1;

    let (feed_pda, _) = find_pda(&[PRICE_FEED_SEED, &product_id.to_le_bytes()]);
    let (publisher_list_pda, _) = find_pda(&[PUBLISHER_LIST_SEED]);

    let mut rng = SimplePrng::new(42);

    // Test 100 random prices
    let test_prices: Vec<u64> = (0..100).map(|_| rng.next_u64()).collect();
    // Also test boundary values
    let boundary_prices: Vec<u64> = vec![
        0,
        1,
        u64::MAX,
        u64::MAX - 1,
        u64::MAX / 2,
        100_000_000,         // $1
        1_000_000_000,       // $10
        100_000_000_000_000, // $1M
    ];

    for price in test_prices.iter().chain(boundary_prices.iter()) {
        let mut ix_data = DISC_PUBLISH_PRICE.to_vec();
        ix_data.extend_from_slice(&product_id.to_le_bytes());
        ix_data.extend_from_slice(&price.to_le_bytes());

        let ix = Instruction {
            program_id: pid,
            accounts: vec![
                AccountMeta::new(publisher, true),
                AccountMeta::new(feed_pda, false),
                AccountMeta::new_readonly(publisher_list_pda, false),
            ],
            data: ix_data,
        };

        let result = mollusk.process_instruction(
            &ix,
            &[
                (publisher, system_account(1_000_000_000)),
                (
                    feed_pda,
                    program_owned_account(make_price_feed(product_id, 0, 0, &publisher)),
                ),
                (
                    publisher_list_pda,
                    program_owned_account(make_publisher_list(&publisher, &[publisher])),
                ),
            ],
        );

        if *price == 0 {
            // price=0 should always fail
            assert!(result.program_result.is_err(), "Price 0 should fail");
        } else {
            // Non-zero prices should succeed
            assert!(
                result.program_result.is_ok(),
                "Price {} should succeed: {:?}",
                price,
                result.program_result
            );
        }
    }
}

// ══════════════════════════════════════════════════════════════════════════════
// FUZZ TEST 2: Random open_position parameters
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn fuzz_open_position_random_params() {
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
    let max_leverage: u32 = 10;
    let user_balance: u64 = 1_000_000_000_000; // $1M (very large)

    let exchange_data = make_exchange_state(&admin, &usdc_mint, &vault_pda, &insurance_pda);
    let market_data = make_market_with_oi(
        product_id,
        &price_feed_pda,
        true,
        max_leverage,
        1_000_000_000_000_000,
        0,
        0,
        0,
        0,
        0,
    );
    let user_account_data = make_user_account(&user, user_balance, 0);

    struct TestCase {
        side: u8,
        size: u64,
        leverage: u32,
        should_succeed: bool,
        reason: &'static str,
    }

    let test_cases = vec![
        // Valid cases
        TestCase {
            side: 0,
            size: 1_000_000,
            leverage: 1,
            should_succeed: true,
            reason: "min size, 1x",
        },
        TestCase {
            side: 1,
            size: 1_000_000,
            leverage: 1,
            should_succeed: true,
            reason: "min size short",
        },
        TestCase {
            side: 0,
            size: 10_000_000,
            leverage: 10,
            should_succeed: true,
            reason: "max market leverage",
        },
        TestCase {
            side: 0,
            size: 100_000_000,
            leverage: 5,
            should_succeed: true,
            reason: "normal trade",
        },
        // Invalid cases
        TestCase {
            side: 0,
            size: 0,
            leverage: 5,
            should_succeed: false,
            reason: "zero size",
        },
        TestCase {
            side: 0,
            size: 999_999,
            leverage: 5,
            should_succeed: false,
            reason: "below min size",
        },
        TestCase {
            side: 0,
            size: 10_000_000,
            leverage: 0,
            should_succeed: false,
            reason: "zero leverage",
        },
        TestCase {
            side: 0,
            size: 10_000_000,
            leverage: 11,
            should_succeed: false,
            reason: "exceed max leverage",
        },
        TestCase {
            side: 0,
            size: 10_000_000,
            leverage: 51,
            should_succeed: false,
            reason: "way over max leverage",
        },
        TestCase {
            side: 0,
            size: 10_000_000,
            leverage: u32::MAX,
            should_succeed: false,
            reason: "max u32 leverage",
        },
    ];

    for tc in &test_cases {
        let mut ix_data = DISC_OPEN_POSITION.to_vec();
        ix_data.extend_from_slice(&product_id.to_le_bytes());
        ix_data.push(tc.side);
        ix_data.extend_from_slice(&tc.size.to_le_bytes());
        ix_data.extend_from_slice(&tc.leverage.to_le_bytes());

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
                (
                    user_account_pda,
                    program_owned_account(user_account_data.clone()),
                ),
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

        if tc.should_succeed {
            assert!(
                result.program_result.is_ok(),
                "Open position '{}' should succeed: {:?}",
                tc.reason,
                result.program_result
            );
        } else {
            assert!(
                result.program_result.is_err(),
                "Open position '{}' should fail",
                tc.reason
            );
        }
    }
}

// ══════════════════════════════════════════════════════════════════════════════
// FUZZ TEST 3: Random deposit/withdraw amounts
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn fuzz_deposit_withdraw_amounts() {
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
    let user_initial_tokens: u64 = 1_000_000_000; // $1000

    // Test zero deposit
    {
        let mut ix_data = DISC_DEPOSIT.to_vec();
        ix_data.extend_from_slice(&0u64.to_le_bytes());

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
                (
                    user_account_pda,
                    program_owned_account(make_user_account(&user, 0, 0)),
                ),
                (exchange_pda, program_owned_account(exchange_data.clone())),
                (
                    vault_pda,
                    token_account_obj(make_token_account(&usdc_mint, &exchange_pda, 0)),
                ),
                (
                    user_token,
                    token_account_obj(make_token_account(&usdc_mint, &user, user_initial_tokens)),
                ),
                (user, system_account(1_000_000_000)),
                spl_token_program::keyed_account(),
            ],
        );
        assert!(result.program_result.is_err(), "Zero deposit should fail");
    }

    // Test zero withdrawal
    {
        let mut ix_data = DISC_WITHDRAW.to_vec();
        ix_data.extend_from_slice(&0u64.to_le_bytes());

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
                (
                    user_account_pda,
                    program_owned_account(make_user_account(&user, 100_000_000, 0)),
                ),
                (exchange_pda, program_owned_account(exchange_data.clone())),
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
        assert!(result.program_result.is_err(), "Zero withdraw should fail");
    }

    // Test withdraw more than balance
    {
        let mut ix_data = DISC_WITHDRAW.to_vec();
        ix_data.extend_from_slice(&200_000_000u64.to_le_bytes()); // more than 100M balance

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
                (
                    user_account_pda,
                    program_owned_account(make_user_account(&user, 100_000_000, 0)),
                ),
                (exchange_pda, program_owned_account(exchange_data.clone())),
                (
                    vault_pda,
                    token_account_obj(make_token_account(&usdc_mint, &exchange_pda, 200_000_000)),
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
            "Withdraw more than balance should fail"
        );
    }
}

// ══════════════════════════════════════════════════════════════════════════════
// FUZZ TEST 4: Invalid discriminators
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn fuzz_invalid_discriminators() {
    let mollusk = setup_mollusk();
    let pid = program_id();
    let user = Pubkey::new_unique();

    let mut rng = SimplePrng::new(12345);

    // Send 50 random 8-byte discriminators
    for _ in 0..50 {
        let mut disc = [0u8; 8];
        for b in disc.iter_mut() {
            *b = rng.next_u64() as u8;
        }

        let ix = Instruction {
            program_id: pid,
            accounts: vec![AccountMeta::new(user, true)],
            data: disc.to_vec(),
        };

        let result = mollusk.process_instruction(&ix, &[(user, system_account(1_000_000_000))]);
        // Should fail with unknown discriminator (unless we happen to hit a valid one)
        // We just verify the program doesn't panic/crash
        let _ = result.program_result;
    }
}

// ══════════════════════════════════════════════════════════════════════════════
// FUZZ TEST 5: Truncated instruction data
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn fuzz_truncated_instruction_data() {
    let mollusk = setup_mollusk();
    let pid = program_id();
    let user = Pubkey::new_unique();

    // Discriminators that require additional data
    let discs_with_data = vec![
        DISC_PUBLISH_PRICE,       // needs product_id + price
        DISC_OPEN_POSITION,       // needs product_id + side + size + leverage
        DISC_DEPOSIT,             // needs amount
        DISC_WITHDRAW,            // needs amount
        DISC_ADD_MARGIN,          // needs product_id + amount
        DISC_UPDATE_FUNDING_RATE, // needs product_id
    ];

    for disc in &discs_with_data {
        // Test with just the discriminator (no extra data)
        let ix = Instruction {
            program_id: pid,
            accounts: vec![AccountMeta::new(user, true)],
            data: disc.to_vec(),
        };

        let result = mollusk.process_instruction(&ix, &[(user, system_account(1_000_000_000))]);
        // Should fail due to insufficient data, but should NOT panic
        assert!(
            result.program_result.is_err(),
            "Truncated data for disc {:?} should fail",
            disc
        );

        // Test with partial data
        let mut partial = disc.to_vec();
        partial.push(0); // just 1 extra byte
        let ix = Instruction {
            program_id: pid,
            accounts: vec![AccountMeta::new(user, true)],
            data: partial,
        };

        let result = mollusk.process_instruction(&ix, &[(user, system_account(1_000_000_000))]);
        assert!(result.program_result.is_err());
    }
}

// ══════════════════════════════════════════════════════════════════════════════
// FUZZ TEST 6: Empty instruction data
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn fuzz_empty_instruction_data() {
    let mollusk = setup_mollusk();
    let pid = program_id();
    let user = Pubkey::new_unique();

    // Completely empty data
    let ix = Instruction {
        program_id: pid,
        accounts: vec![AccountMeta::new(user, true)],
        data: vec![],
    };

    let result = mollusk.process_instruction(&ix, &[(user, system_account(1_000_000_000))]);
    assert!(
        result.program_result.is_err(),
        "Empty instruction data should fail"
    );

    // Data shorter than discriminator
    for len in 1..8 {
        let ix = Instruction {
            program_id: pid,
            accounts: vec![AccountMeta::new(user, true)],
            data: vec![0u8; len],
        };

        let result = mollusk.process_instruction(&ix, &[(user, system_account(1_000_000_000))]);
        assert!(
            result.program_result.is_err(),
            "Data of length {} should fail",
            len
        );
    }
}

// ══════════════════════════════════════════════════════════════════════════════
// FUZZ TEST 7: Batch publish with various counts
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn fuzz_batch_publish_counts() {
    let mollusk = setup_mollusk();
    let pid = program_id();
    let publisher = Pubkey::new_unique();
    let (publisher_list_pda, _) = find_pda(&[PUBLISHER_LIST_SEED]);

    let publisher_list = make_publisher_list(&publisher, &[publisher]);

    struct BatchCase {
        count: u32,
        actual_feeds: usize,
        should_succeed: bool,
        reason: &'static str,
    }

    let test_cases = vec![
        BatchCase {
            count: 0,
            actual_feeds: 0,
            should_succeed: false,
            reason: "empty batch",
        },
        BatchCase {
            count: 11,
            actual_feeds: 11,
            should_succeed: false,
            reason: "batch too large (11)",
        },
        BatchCase {
            count: 100,
            actual_feeds: 100,
            should_succeed: false,
            reason: "batch way too large",
        },
        BatchCase {
            count: u32::MAX,
            actual_feeds: 0,
            should_succeed: false,
            reason: "max u32 count",
        },
        BatchCase {
            count: 2,
            actual_feeds: 1,
            should_succeed: false,
            reason: "count mismatch (2 vs 1)",
        },
    ];

    for tc in &test_cases {
        let mut ix_data = DISC_PUBLISH_PRICES_BATCH.to_vec();
        ix_data.extend_from_slice(&tc.count.to_le_bytes());

        // Add data for as many feeds as the count says
        let data_count = (tc.count as usize).min(20); // cap to prevent huge allocations
        for i in 0..data_count {
            ix_data.extend_from_slice(&(i as u32 + 1).to_le_bytes()); // product_id
            ix_data.extend_from_slice(&1_000_000_000u64.to_le_bytes()); // price
        }

        let mut accounts_meta = vec![
            AccountMeta::new(publisher, true),
            AccountMeta::new_readonly(publisher_list_pda, false),
        ];
        let mut accounts_data: Vec<(Pubkey, Account)> = vec![
            (publisher, system_account(1_000_000_000)),
            (
                publisher_list_pda,
                program_owned_account(publisher_list.clone()),
            ),
        ];

        for i in 0..tc.actual_feeds {
            let pid_val = (i as u32) + 1;
            let (feed_pda, _) = find_pda(&[PRICE_FEED_SEED, &pid_val.to_le_bytes()]);
            accounts_meta.push(AccountMeta::new(feed_pda, false));
            accounts_data.push((
                feed_pda,
                program_owned_account(make_price_feed(pid_val, 0, 0, &publisher)),
            ));
        }

        let ix = Instruction {
            program_id: pid,
            accounts: accounts_meta,
            data: ix_data,
        };

        let result = mollusk.process_instruction(&ix, &accounts_data);

        if tc.should_succeed {
            assert!(
                result.program_result.is_ok(),
                "Batch '{}' should succeed: {:?}",
                tc.reason,
                result.program_result
            );
        } else {
            assert!(
                result.program_result.is_err(),
                "Batch '{}' should fail",
                tc.reason
            );
        }
    }
}

// ══════════════════════════════════════════════════════════════════════════════
// FUZZ TEST 8: create_market with invalid leverage values
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn fuzz_create_market_leverage_values() {
    let mollusk = setup_mollusk();
    let pid = program_id();
    let admin = Pubkey::new_unique();
    let usdc_mint = Pubkey::new_unique();
    let vault = Pubkey::new_unique();
    let insurance = Pubkey::new_unique();

    let (exchange_pda, _) = find_pda(&[EXCHANGE_SEED]);
    let exchange_data = make_exchange_state(&admin, &usdc_mint, &vault, &insurance);

    struct LeverageCase {
        leverage: u32,
        max_oi: u64,
        should_succeed: bool,
        reason: &'static str,
    }

    let cases = vec![
        LeverageCase {
            leverage: 0,
            max_oi: 1_000_000,
            should_succeed: false,
            reason: "zero leverage",
        },
        LeverageCase {
            leverage: 1,
            max_oi: 1_000_000,
            should_succeed: true,
            reason: "1x leverage",
        },
        LeverageCase {
            leverage: 20,
            max_oi: 1_000_000,
            should_succeed: true,
            reason: "max leverage (20)",
        },
        LeverageCase {
            leverage: 21,
            max_oi: 1_000_000,
            should_succeed: false,
            reason: "exceed max leverage",
        },
        LeverageCase {
            leverage: u32::MAX,
            max_oi: 1_000_000,
            should_succeed: false,
            reason: "u32::MAX leverage",
        },
        LeverageCase {
            leverage: 10,
            max_oi: 0,
            should_succeed: false,
            reason: "zero max_oi",
        },
        LeverageCase {
            leverage: 10,
            max_oi: 1,
            should_succeed: true,
            reason: "min max_oi",
        },
        LeverageCase {
            leverage: 10,
            max_oi: u64::MAX,
            should_succeed: true,
            reason: "max max_oi",
        },
    ];

    for tc in &cases {
        let product_id: u32 = tc.leverage.wrapping_add(tc.max_oi as u32).wrapping_add(1);
        let (market_pda, _) = find_pda(&[MARKET_SEED, &product_id.to_le_bytes()]);
        let (price_feed_pda, _) = find_pda(&[PRICE_FEED_SEED, &product_id.to_le_bytes()]);

        let mut ix_data = DISC_CREATE_MARKET.to_vec();
        ix_data.extend_from_slice(&product_id.to_le_bytes());
        ix_data.extend_from_slice(&tc.leverage.to_le_bytes());
        ix_data.extend_from_slice(&tc.max_oi.to_le_bytes());

        let result = mollusk.process_instruction(
            &Instruction {
                program_id: pid,
                accounts: vec![
                    AccountMeta::new(market_pda, false),
                    AccountMeta::new(exchange_pda, false),
                    AccountMeta::new_readonly(price_feed_pda, false),
                    AccountMeta::new(admin, true),
                    AccountMeta::new_readonly(system_program::ID, false),
                ],
                data: ix_data,
            },
            &[
                (market_pda, system_account(0)),
                (exchange_pda, program_owned_account(exchange_data.clone())),
                (
                    price_feed_pda,
                    program_owned_account(make_price_feed(product_id, 1_000_000_000, 1000, &admin)),
                ),
                (admin, system_account(100_000_000_000)),
                system_program_account(),
            ],
        );

        if tc.should_succeed {
            assert!(
                result.program_result.is_ok(),
                "Create market '{}' should succeed: {:?}",
                tc.reason,
                result.program_result
            );
        } else {
            assert!(
                result.program_result.is_err(),
                "Create market '{}' should fail",
                tc.reason
            );
        }
    }
}

// ══════════════════════════════════════════════════════════════════════════════
// FUZZ TEST 9: add_margin with various amounts
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn fuzz_add_margin_amounts() {
    let mut mollusk = setup_mollusk();
    mollusk.sysvars.warp_to_slot(100);
    mollusk.sysvars.clock.unix_timestamp = 1000;

    let pid = program_id();
    let admin = Pubkey::new_unique();
    let user = Pubkey::new_unique();
    let usdc_mint = Pubkey::new_unique();
    let product_id: u32 = 1;

    let (user_account_pda, _) = find_pda(&[USER_ACCOUNT_SEED, user.as_ref()]);
    let (position_pda, _) = find_pda(&[POSITION_SEED, user.as_ref(), &product_id.to_le_bytes()]);
    let (exchange_pda, _) = find_pda(&[EXCHANGE_SEED]);
    let (vault_pda, _) = find_pda(&[VAULT_SEED]);
    let (insurance_pda, _) = find_pda(&[INSURANCE_SEED]);

    let user_balance: u64 = 100_000_000;
    let initial_margin: u64 = 10_000_000;

    struct MarginCase {
        amount: u64,
        should_succeed: bool,
        reason: &'static str,
    }

    let cases = vec![
        MarginCase {
            amount: 0,
            should_succeed: false,
            reason: "zero amount",
        },
        MarginCase {
            amount: 1,
            should_succeed: true,
            reason: "1 lamport",
        },
        MarginCase {
            amount: user_balance,
            should_succeed: true,
            reason: "entire balance",
        },
        MarginCase {
            amount: user_balance + 1,
            should_succeed: false,
            reason: "exceed balance",
        },
        MarginCase {
            amount: u64::MAX,
            should_succeed: false,
            reason: "u64::MAX",
        },
        MarginCase {
            amount: 50_000_000,
            should_succeed: true,
            reason: "half balance",
        },
    ];

    for tc in &cases {
        let mut ix_data = DISC_ADD_MARGIN.to_vec();
        ix_data.extend_from_slice(&product_id.to_le_bytes());
        ix_data.extend_from_slice(&tc.amount.to_le_bytes());

        // Set total_deposits to match user balance
        let mut exchange_data = make_exchange_state(&admin, &usdc_mint, &vault_pda, &insurance_pda);
        exchange_data[172..180].copy_from_slice(&user_balance.to_le_bytes());

        let result = mollusk.process_instruction(
            &Instruction {
                program_id: pid,
                accounts: vec![
                    AccountMeta::new(user_account_pda, false),
                    AccountMeta::new(position_pda, false),
                    AccountMeta::new(exchange_pda, false),
                    AccountMeta::new(user, true),
                ],
                data: ix_data,
            },
            &[
                (
                    user_account_pda,
                    program_owned_account(make_user_account(&user, user_balance, 1)),
                ),
                (
                    position_pda,
                    program_owned_account(make_position(
                        &user,
                        product_id,
                        0,
                        50_000_000,
                        1_000_000_000,
                        5,
                        initial_margin,
                    )),
                ),
                (exchange_pda, program_owned_account(exchange_data)),
                (user, system_account(1_000_000_000)),
            ],
        );

        if tc.should_succeed {
            assert!(
                result.program_result.is_ok(),
                "Add margin '{}' should succeed: {:?}",
                tc.reason,
                result.program_result
            );
        } else {
            assert!(
                result.program_result.is_err(),
                "Add margin '{}' should fail",
                tc.reason
            );
        }
    }
}

// ══════════════════════════════════════════════════════════════════════════════
// FUZZ TEST 10: Unauthorized access attempts
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn fuzz_unauthorized_access_all_instructions() {
    let mut mollusk = setup_mollusk();
    spl_token_program::add_program(&mut mollusk);
    mollusk.sysvars.warp_to_slot(100);
    mollusk.sysvars.clock.unix_timestamp = 1000;

    let pid = program_id();
    let admin = Pubkey::new_unique();
    let wrong_user = Pubkey::new_unique();
    let real_user = Pubkey::new_unique();
    let usdc_mint = Pubkey::new_unique();
    let product_id: u32 = 1;

    let (exchange_pda, _) = find_pda(&[EXCHANGE_SEED]);
    let (vault_pda, _) = find_pda(&[VAULT_SEED]);
    let (insurance_pda, _) = find_pda(&[INSURANCE_SEED]);
    let (user_account_pda, _) = find_pda(&[USER_ACCOUNT_SEED, real_user.as_ref()]);
    let (market_pda, _) = find_pda(&[MARKET_SEED, &product_id.to_le_bytes()]);
    let (price_feed_pda, _) = find_pda(&[PRICE_FEED_SEED, &product_id.to_le_bytes()]);
    let user_token = Pubkey::new_unique();

    let exchange_data = make_exchange_state(&admin, &usdc_mint, &vault_pda, &insurance_pda);
    let user_account_data = make_user_account(&real_user, 100_000_000, 0);
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

    // Wrong user tries to deposit to someone else's account
    {
        let mut ix_data = DISC_DEPOSIT.to_vec();
        ix_data.extend_from_slice(&10_000_000u64.to_le_bytes());

        let result = mollusk.process_instruction(
            &Instruction {
                program_id: pid,
                accounts: vec![
                    AccountMeta::new(user_account_pda, false),
                    AccountMeta::new(exchange_pda, false),
                    AccountMeta::new(vault_pda, false),
                    AccountMeta::new(user_token, false),
                    AccountMeta::new(wrong_user, true), // wrong user!
                    AccountMeta::new_readonly(spl_token_id(), false),
                ],
                data: ix_data,
            },
            &[
                (
                    user_account_pda,
                    program_owned_account(user_account_data.clone()),
                ),
                (exchange_pda, program_owned_account(exchange_data.clone())),
                (
                    vault_pda,
                    token_account_obj(make_token_account(&usdc_mint, &exchange_pda, 0)),
                ),
                (
                    user_token,
                    token_account_obj(make_token_account(&usdc_mint, &wrong_user, 100_000_000)),
                ),
                (wrong_user, system_account(1_000_000_000)),
                spl_token_program::keyed_account(),
            ],
        );
        assert!(
            result.program_result.is_err(),
            "Wrong user deposit should fail"
        );
    }

    // Wrong user tries to close someone else's account
    {
        let result = mollusk.process_instruction(
            &Instruction {
                program_id: pid,
                accounts: vec![
                    AccountMeta::new(user_account_pda, false),
                    AccountMeta::new(wrong_user, true),
                ],
                data: DISC_CLOSE_POSITION
                    .to_vec()
                    .into_iter()
                    .chain(product_id.to_le_bytes().into_iter())
                    .collect(),
            },
            &[
                (
                    user_account_pda,
                    program_owned_account(make_user_account(&real_user, 0, 0)),
                ),
                (wrong_user, system_account(1_000_000_000)),
            ],
        );
        // This uses close_user_account discriminator check
    }

    // Non-admin tries to set paused
    {
        let result = mollusk.process_instruction(
            &Instruction {
                program_id: pid,
                accounts: vec![
                    AccountMeta::new(exchange_pda, false),
                    AccountMeta::new_readonly(wrong_user, true),
                ],
                data: [DISC_SET_PAUSED.to_vec(), vec![1u8]].concat(),
            },
            &[
                (exchange_pda, program_owned_account(exchange_data.clone())),
                (wrong_user, system_account(1_000_000_000)),
            ],
        );
        assert!(
            result.program_result.is_err(),
            "Non-admin set_paused should fail"
        );
    }

    // Non-admin tries to update market params
    {
        let mut ix_data = DISC_UPDATE_MARKET_PARAMS.to_vec();
        ix_data.extend_from_slice(&product_id.to_le_bytes());
        ix_data.extend_from_slice(&20u32.to_le_bytes());
        ix_data.extend_from_slice(&1_000_000u64.to_le_bytes());

        let result = mollusk.process_instruction(
            &Instruction {
                program_id: pid,
                accounts: vec![
                    AccountMeta::new(market_pda, false),
                    AccountMeta::new_readonly(exchange_pda, false),
                    AccountMeta::new_readonly(wrong_user, true),
                ],
                data: ix_data,
            },
            &[
                (market_pda, program_owned_account(market_data.clone())),
                (exchange_pda, program_owned_account(exchange_data.clone())),
                (wrong_user, system_account(1_000_000_000)),
            ],
        );
        assert!(
            result.program_result.is_err(),
            "Non-admin update_market_params should fail"
        );
    }

    // Non-admin tries to fund insurance
    {
        let mut ix_data = DISC_FUND_INSURANCE.to_vec();
        ix_data.extend_from_slice(&10_000_000u64.to_le_bytes());

        let result = mollusk.process_instruction(
            &Instruction {
                program_id: pid,
                accounts: vec![
                    AccountMeta::new(exchange_pda, false),
                    AccountMeta::new(insurance_pda, false),
                    AccountMeta::new(user_token, false),
                    AccountMeta::new(wrong_user, true),
                    AccountMeta::new_readonly(spl_token_id(), false),
                ],
                data: ix_data,
            },
            &[
                (exchange_pda, program_owned_account(exchange_data.clone())),
                (
                    insurance_pda,
                    token_account_obj(make_token_account(&usdc_mint, &exchange_pda, 0)),
                ),
                (
                    user_token,
                    token_account_obj(make_token_account(&usdc_mint, &wrong_user, 100_000_000)),
                ),
                (wrong_user, system_account(1_000_000_000)),
                spl_token_program::keyed_account(),
            ],
        );
        assert!(
            result.program_result.is_err(),
            "Non-admin fund_insurance should fail"
        );
    }
}

// ══════════════════════════════════════════════════════════════════════════════
// FUZZ TEST 11: Non-liquidatable position should fail
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn fuzz_liquidate_healthy_position() {
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

    let entry_price: u64 = 1_000_000_000;
    let current_price: u64 = 990_000_000; // only 1% drop, position should still be healthy with 10x
    let position_size: u64 = 100_000_000;
    let margin: u64 = 10_000_000;

    let exchange_data = make_exchange_state(&admin, &usdc_mint, &vault_pda, &insurance_pda);
    let market_data = make_market_with_oi(
        product_id,
        &price_feed_pda,
        true,
        10,
        1_000_000_000_000,
        position_size,
        0,
        0,
        0,
        0,
    );
    let owner_account_data = make_user_account(&position_owner, 0, 1);
    let liquidator_account_data = make_user_account(&liquidator, 50_000_000, 0);
    let position_data = make_position(
        &position_owner,
        product_id,
        0,
        position_size,
        entry_price,
        10,
        margin,
    );

    let mut ix_data = DISC_LIQUIDATE.to_vec();
    ix_data.extend_from_slice(&product_id.to_le_bytes());

    let result = mollusk.process_instruction(
        &Instruction {
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
        },
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
                program_owned_account(make_price_feed(product_id, current_price, 1000, &admin)),
            ),
            (position_owner, system_account(1_000_000_000)),
            (liquidator, system_account(1_000_000_000)),
            spl_token_program::keyed_account(),
        ],
    );
    assert!(
        result.program_result.is_err(),
        "Healthy position should not be liquidatable"
    );
}

// ══════════════════════════════════════════════════════════════════════════════
// FUZZ TEST 12: Publish price with wrong product_id
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn fuzz_publish_price_wrong_product_id() {
    let mollusk = setup_mollusk();
    let pid = program_id();
    let publisher = Pubkey::new_unique();
    let actual_product_id: u32 = 1;
    let wrong_product_id: u32 = 99;

    let (feed_pda, _) = find_pda(&[PRICE_FEED_SEED, &actual_product_id.to_le_bytes()]);
    let (publisher_list_pda, _) = find_pda(&[PUBLISHER_LIST_SEED]);

    let mut ix_data = DISC_PUBLISH_PRICE.to_vec();
    ix_data.extend_from_slice(&wrong_product_id.to_le_bytes()); // wrong!
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
                program_owned_account(make_price_feed(actual_product_id, 0, 0, &publisher)),
            ),
            (
                publisher_list_pda,
                program_owned_account(make_publisher_list(&publisher, &[publisher])),
            ),
        ],
    );
    assert!(
        result.program_result.is_err(),
        "Wrong product_id should fail"
    );
}

// ══════════════════════════════════════════════════════════════════════════════
// FUZZ TEST 13: Random oracle price values for position operations
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn fuzz_oracle_price_extremes() {
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

    // Extreme price values
    let extreme_prices: Vec<u64> = vec![
        0,           // zero (should fail)
        1,           // minimum possible
        100,         // very tiny
        100_000_000, // $1
        u64::MAX,    // maximum u64
        u64::MAX / 2,
        u64::MAX / 100,
    ];

    for price in &extreme_prices {
        let mut ix_data = DISC_OPEN_POSITION.to_vec();
        ix_data.extend_from_slice(&product_id.to_le_bytes());
        ix_data.push(0); // Long
        ix_data.extend_from_slice(&1_000_000u64.to_le_bytes()); // min size
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
                (
                    user_account_pda,
                    program_owned_account(user_account_data.clone()),
                ),
                (position_pda, system_account(0)),
                (market_pda, program_owned_account(market_data.clone())),
                (exchange_pda, program_owned_account(exchange_data.clone())),
                (
                    price_feed_pda,
                    program_owned_account(make_price_feed(product_id, *price, 1000, &admin)),
                ),
                (user, system_account(100_000_000_000)),
                system_program_account(),
            ],
        );

        if *price == 0 {
            assert!(
                result.program_result.is_err(),
                "Zero oracle price should fail"
            );
        }
        // For non-zero: it may succeed or fail depending on math, but should not panic
    }
}

// ══════════════════════════════════════════════════════════════════════════════
// FUZZ TEST 14: Stale oracle price should be rejected
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn fuzz_stale_oracle_price() {
    let mut mollusk = setup_mollusk();
    mollusk.sysvars.warp_to_slot(100);
    mollusk.sysvars.clock.unix_timestamp = 10_000; // current time

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

    // Price last updated at timestamp 1 (> 600 seconds ago, stale)
    let stale_time: i64 = 1;

    let mut ix_data = DISC_OPEN_POSITION.to_vec();
    ix_data.extend_from_slice(&product_id.to_le_bytes());
    ix_data.push(0);
    ix_data.extend_from_slice(&10_000_000u64.to_le_bytes());
    ix_data.extend_from_slice(&5u32.to_le_bytes());

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
                program_owned_account(make_price_feed(
                    product_id,
                    1_000_000_000,
                    stale_time,
                    &admin,
                )),
            ),
            (user, system_account(100_000_000_000)),
            system_program_account(),
        ],
    );
    assert!(
        result.program_result.is_err(),
        "Stale oracle price should be rejected"
    );
}

// ══════════════════════════════════════════════════════════════════════════════
// FUZZ TEST 15: Randomized open/close position cycle
// Test many random combinations of price changes to verify no panics
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn fuzz_random_open_close_cycles() {
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

    let entry_price: u64 = 1_000_000_000; // $10
    let position_size: u64 = 10_000_000; // $10
    let leverage: u32 = 2;

    let mut rng = SimplePrng::new(99999);

    // Price change percentages from -50% to +100%
    let price_changes: Vec<i64> =
        vec![-40, -30, -20, -10, -5, -1, 0, 1, 5, 10, 20, 30, 50, 80, 100];

    for &pct_change in &price_changes {
        for side in [0u8, 1u8] {
            let new_price = ((entry_price as i128) * (100 + pct_change as i128) / 100) as u64;
            if new_price == 0 {
                continue;
            }

            let margin = position_size / leverage as u64;
            let fee = position_size * 5 / 10_000;
            let user_balance = 100_000_000u64;

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
            let user_account_data = make_user_account(&user, user_balance, 0);

            // Open
            let mut open_data = DISC_OPEN_POSITION.to_vec();
            open_data.extend_from_slice(&product_id.to_le_bytes());
            open_data.push(side);
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
                        program_owned_account(make_price_feed(
                            product_id,
                            entry_price,
                            1000,
                            &admin,
                        )),
                    ),
                    (user, system_account(100_000_000_000)),
                    system_program_account(),
                ],
            );

            if result.program_result.is_err() {
                continue; // Some combos may fail, that's OK for fuzzing
            }

            // Close at new price
            let mut close_data = DISC_CLOSE_POSITION.to_vec();
            close_data.extend_from_slice(&product_id.to_le_bytes());

            let user_account_acc = result
                .resulting_accounts
                .iter()
                .find(|(k, _)| *k == user_account_pda)
                .unwrap()
                .1
                .clone();
            let position_acc = result
                .resulting_accounts
                .iter()
                .find(|(k, _)| *k == position_pda)
                .unwrap()
                .1
                .clone();
            let market_acc = result
                .resulting_accounts
                .iter()
                .find(|(k, _)| *k == market_pda)
                .unwrap()
                .1
                .clone();
            let exchange_acc = result
                .resulting_accounts
                .iter()
                .find(|(k, _)| *k == exchange_pda)
                .unwrap()
                .1
                .clone();

            let close_result = mollusk.process_instruction(
                &Instruction {
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
                },
                &[
                    (user_account_pda, user_account_acc),
                    (position_pda, position_acc),
                    (market_pda, market_acc),
                    (exchange_pda, exchange_acc),
                    (
                        vault_pda,
                        token_account_obj(make_token_account(
                            &usdc_mint,
                            &exchange_pda,
                            500_000_000,
                        )),
                    ),
                    (
                        insurance_pda,
                        token_account_obj(make_token_account(
                            &usdc_mint,
                            &exchange_pda,
                            50_000_000,
                        )),
                    ),
                    (
                        price_feed_pda,
                        program_owned_account(make_price_feed(product_id, new_price, 1000, &admin)),
                    ),
                    (user, system_account(100_000_000_000)),
                    spl_token_program::keyed_account(),
                ],
            );
            // We don't assert success - some extreme price changes may cause issues
            // The key check is that the program doesn't panic
            let _ = close_result.program_result;
        }
    }
}
