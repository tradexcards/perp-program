use pinocchio::error::ProgramError;

/// Custom error codes (base = 6000 + offset).
#[repr(u32)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PerpDexError {
    // Oracle errors (kept sparse for backward compat with frontend error maps)
    OracleUnauthorized = 6006,

    // PerpDex errors
    InvalidAmount = 6012,
    InsufficientBalance = 6013,
    ExchangePaused = 6014,
    InvalidLeverage = 6015,
    PositionTooSmall = 6016,
    ProductMismatch = 6017,
    MarketInactive = 6018,
    MaxOpenInterestExceeded = 6019,
    PositionNotLiquidatable = 6020,
    InvalidOraclePrice = 6021,
    MathOverflow = 6022,
    FundingTooEarly = 6023,
    Unauthorized = 6024,
    TooManyMarkets = 6025,
    HasBalance = 6026,
    HasOpenPositions = 6027,
    OraclePriceStale = 6030,
    PositionTooLarge = 6031,
    InvalidTokenAccountOwner = 6032,

    // Ed25519 signed price errors
    InvalidEd25519Instruction = 6035,
    InvalidSignedPrice = 6036,

    // M-05: Price deviation circuit breaker
    PriceDeviationExceeded = 6040,

    // L-04: Token mint mismatch
    MintMismatch = 6041,

    // L-05: Position already exists
    PositionAlreadyExists = 6042,
}

impl From<PerpDexError> for ProgramError {
    fn from(e: PerpDexError) -> Self {
        ProgramError::Custom(e as u32)
    }
}
