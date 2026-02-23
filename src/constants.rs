/// SPL Token Program ID.
/// TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA
pub const TOKEN_PROGRAM_ID: [u8; 32] = [
    0x06, 0xdd, 0xf6, 0xe1, 0xd7, 0x65, 0xa1, 0x93, 0xd9, 0xcb, 0xe1, 0x46, 0xce, 0xeb, 0x79, 0xac,
    0x1c, 0xb4, 0x85, 0xed, 0x5f, 0x5b, 0x37, 0x91, 0x3a, 0x8c, 0xf5, 0x85, 0x7e, 0xff, 0x00, 0xa9,
];

/// System Program ID.
pub const SYSTEM_PROGRAM_ID: [u8; 32] = [0u8; 32];

/// Ed25519 precompile program ID.
/// Ed25519SigVerify111111111111111111111111111
pub const ED25519_PROGRAM_ID: [u8; 32] = [
    0x03, 0x7d, 0x46, 0xd6, 0x7c, 0x93, 0xfb, 0xbe, 0x12, 0xf9, 0x42, 0x8f, 0x83, 0x8d, 0x40, 0xff,
    0x05, 0x70, 0x74, 0x49, 0x27, 0xf4, 0x8a, 0x64, 0xfc, 0xca, 0x70, 0x44, 0x80, 0x00, 0x00, 0x00,
];

/// Size of the signed oracle price message: product_id(4) + price(8) + timestamp(8) = 20 bytes.
pub const SIGNED_PRICE_MSG_LEN: usize = 20;

// ── PDA Seeds ────────────────────────────────────────────────────────────────

pub const ORACLE_SEED: &[u8] = b"oracle";

pub const EXCHANGE_SEED: &[u8] = b"exchange";
pub const VAULT_SEED: &[u8] = b"vault";
pub const MARKET_SEED: &[u8] = b"market";
pub const USER_ACCOUNT_SEED: &[u8] = b"user_account";
pub const POSITION_SEED: &[u8] = b"position";
pub const INSURANCE_SEED: &[u8] = b"insurance";

// ── Oracle Constants ─────────────────────────────────────────────────────────

/// Maximum age of an oracle price before it's considered stale (2 minutes).
pub const MAX_ORACLE_PRICE_AGE: i64 = 15;

// ── PerpDex Constants ────────────────────────────────────────────────────────

pub const PRICE_SCALE: u64 = 100_000_000; // 10^8
pub const MAX_LEVERAGE: u32 = 50;
pub const MIN_POSITION_SIZE: u64 = 1_000_000; // $1
pub const MAINTENANCE_MARGIN_BPS: u64 = 100; // 1%
pub const LIQUIDATION_FEE_BPS: u64 = 50; // 0.5%
pub const LIQUIDATION_INSURANCE_BPS: u64 = 50; // 0.5%
pub const TRADING_FEE_BPS: u64 = 5; // 0.05%
pub const FUNDING_INTERVAL: i64 = 28_800; // 8 hours
pub const FUNDING_RATE_CAP_BPS: i64 = 5; // 0.05%
pub const BPS_DENOMINATOR: u64 = 10_000;
pub const MAX_MARKETS: u32 = 100_000;

/// Maximum position size per user ($100k in USDC decimals).
pub const MAX_POSITION_SIZE_PER_USER: u64 = 100_000_000_000;

/// Treasury wallet for protocol fee withdrawals.
/// Cv1x1DF9KKdNHSPYwCgcAipo3sYKcW4CpC3rRodwDABF
pub const TREASURY_WALLET: [u8; 32] = [
    0xb1, 0x05, 0x69, 0x4e, 0xf0, 0x0d, 0x87, 0x45, 0x31, 0xb9, 0x63, 0x65, 0xb3, 0xa1, 0x07, 0x9e,
    0x28, 0x22, 0x8e, 0x55, 0x70, 0xa8, 0x86, 0x67, 0xa8, 0x6e, 0x5d, 0xd0, 0xe4, 0xc0, 0x92, 0xd6,
];
