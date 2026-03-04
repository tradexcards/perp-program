/// Zero-copy account state definitions.
///
/// Layout is Anchor-compatible: every account is prefixed by an 8-byte discriminator
/// (SHA-256("account:<StructName>")[..8]).  We store the discriminator constants here
/// and the raw struct layouts start *after* the discriminator in on-chain data.
///
/// All structs use `#[repr(C, packed)]` for deterministic layout and zero-copy access.
use pinocchio::error::ProgramError;

// ── Anchor discriminators ────────────────────────────────────────────────────
//
// Generated via: sha256("account:StructName")[0..8]
// We keep them so we can validate existing data and remain wire-compatible with
// the Anchor program's IDL (clients already compute these).

pub const DISC_ORACLE_STATE: [u8; 8] = [0x6a, 0x0a, 0xcb, 0x5b, 0x63, 0x60, 0xa1, 0x27];
pub const DISC_EXCHANGE_STATE: [u8; 8] = [0x79, 0xf0, 0x75, 0x17, 0x49, 0xb2, 0xc4, 0x47];
pub const DISC_MARKET: [u8; 8] = [0xdf, 0xb8, 0x9c, 0xee, 0x6b, 0xa0, 0x8a, 0x4a];
pub const DISC_USER_ACCOUNT: [u8; 8] = [0x21, 0x1e, 0x8e, 0x54, 0xb9, 0xd3, 0x19, 0xe5];
pub const DISC_POSITION: [u8; 8] = [0xaa, 0xbc, 0x8f, 0xe4, 0x7a, 0x17, 0x2b, 0x6c];

// ── Side enum ────────────────────────────────────────────────────────────────

#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Side {
    Long = 0,
    Short = 1,
}

impl Side {
    pub fn from_u8(v: u8) -> Result<Self, ProgramError> {
        match v {
            0 => Ok(Side::Long),
            1 => Ok(Side::Short),
            _ => Err(ProgramError::InvalidInstructionData),
        }
    }

    pub fn to_u8(self) -> u8 {
        self as u8
    }
}

// ── OracleState (84 bytes data, 92 on-chain) ────────────────────────────────

#[repr(C, packed)]
pub struct OracleState {
    pub authority: [u8; 32],         // Pubkey
    pub pending_authority: [u8; 32], // Pubkey
    pub feed_count: u32,
    pub last_update_slot: u64, // dead field, kept for layout compat
    pub last_update_ts: i64,   // dead field, kept for layout compat
}

impl OracleState {
    pub const SIZE: usize = core::mem::size_of::<Self>(); // 84
    pub const ACCOUNT_SIZE: usize = 8 + Self::SIZE; // 92

    pub fn load(data: &[u8]) -> Result<&Self, ProgramError> {
        if data.len() < Self::ACCOUNT_SIZE {
            return Err(ProgramError::AccountDataTooSmall);
        }
        if data[..8] != DISC_ORACLE_STATE {
            return Err(ProgramError::InvalidAccountData);
        }
        Ok(unsafe { &*(data.as_ptr().add(8) as *const Self) })
    }

    pub fn load_mut(data: &mut [u8]) -> Result<&mut Self, ProgramError> {
        if data.len() < Self::ACCOUNT_SIZE {
            return Err(ProgramError::AccountDataTooSmall);
        }
        if data[..8] != DISC_ORACLE_STATE {
            return Err(ProgramError::InvalidAccountData);
        }
        Ok(unsafe { &mut *(data.as_mut_ptr().add(8) as *mut Self) })
    }

    pub fn init(data: &mut [u8]) -> Result<&mut Self, ProgramError> {
        if data.len() < Self::ACCOUNT_SIZE {
            return Err(ProgramError::AccountDataTooSmall);
        }
        data[..8].copy_from_slice(&DISC_ORACLE_STATE);
        // CreateAccount syscall already zeros the data region
        Ok(unsafe { &mut *(data.as_mut_ptr().add(8) as *mut Self) })
    }
}

// ── ExchangeState (205 bytes data, 213 on-chain) ────────────────────────────

#[repr(C, packed)]
pub struct ExchangeState {
    pub admin: [u8; 32],
    pub pending_admin: [u8; 32],
    pub usdc_mint: [u8; 32],
    pub vault: [u8; 32],
    pub insurance_fund: [u8; 32],
    pub market_count: u32,
    pub total_deposits: u64,
    pub total_volume: u128,
    pub total_fees_collected: u64,
    pub insurance_balance: u64,
    pub paused: u8, // 0 = false, 1 = true (bool as u8 for packed layout)
}

impl ExchangeState {
    pub const SIZE: usize = core::mem::size_of::<Self>(); // 205
    pub const ACCOUNT_SIZE: usize = 8 + Self::SIZE; // 213

    pub fn load(data: &[u8]) -> Result<&Self, ProgramError> {
        if data.len() < Self::ACCOUNT_SIZE {
            return Err(ProgramError::AccountDataTooSmall);
        }
        if data[..8] != DISC_EXCHANGE_STATE {
            return Err(ProgramError::InvalidAccountData);
        }
        Ok(unsafe { &*(data.as_ptr().add(8) as *const Self) })
    }

    pub fn load_mut(data: &mut [u8]) -> Result<&mut Self, ProgramError> {
        if data.len() < Self::ACCOUNT_SIZE {
            return Err(ProgramError::AccountDataTooSmall);
        }
        if data[..8] != DISC_EXCHANGE_STATE {
            return Err(ProgramError::InvalidAccountData);
        }
        Ok(unsafe { &mut *(data.as_mut_ptr().add(8) as *mut Self) })
    }

    pub fn init(data: &mut [u8]) -> Result<&mut Self, ProgramError> {
        if data.len() < Self::ACCOUNT_SIZE {
            return Err(ProgramError::AccountDataTooSmall);
        }
        data[..8].copy_from_slice(&DISC_EXCHANGE_STATE);
        // CreateAccount syscall already zeros the data region
        Ok(unsafe { &mut *(data.as_mut_ptr().add(8) as *mut Self) })
    }

    pub fn is_paused(&self) -> bool {
        self.paused != 0
    }
}

// ── Market (105 bytes core, extended to 121 bytes with M-05 fields) ──────────
//
// On-chain sizes:
//   Legacy:   113 bytes (8 disc + 105 core)
//   Extended: 129 bytes (8 disc + 105 core + 8 last_oracle_price + 8 max_price_deviation_bps)
//
// load/load_mut accept both sizes for backward compatibility.

#[repr(C, packed)]
pub struct Market {
    pub product_id: u32,
    pub oracle_price_feed: [u8; 32],
    pub max_leverage: u32,
    pub max_open_interest: u64,
    pub long_open_interest: u64,
    pub short_open_interest: u64,
    pub total_volume: u128,
    pub last_funding_time: i64,
    pub cumulative_funding_rate_long: i64,
    pub cumulative_funding_rate_short: i64,
    pub active: u8, // 0 = false, 1 = true
}

impl Market {
    pub const SIZE: usize = core::mem::size_of::<Self>(); // 105
    pub const ACCOUNT_SIZE: usize = 8 + Self::SIZE; // 113 (legacy)
    pub const EXTENDED_ACCOUNT_SIZE: usize = Self::ACCOUNT_SIZE + 16; // 129

    // Extension field offsets (relative to start of account data)
    const EXT_LAST_ORACLE_PRICE: usize = Self::ACCOUNT_SIZE; // byte 113
    const EXT_MAX_DEVIATION_BPS: usize = Self::ACCOUNT_SIZE + 8; // byte 121

    pub fn load(data: &[u8]) -> Result<&Self, ProgramError> {
        // Accept both legacy (113) and extended (129) sizes
        if data.len() < Self::ACCOUNT_SIZE {
            return Err(ProgramError::AccountDataTooSmall);
        }
        if data[..8] != DISC_MARKET {
            return Err(ProgramError::InvalidAccountData);
        }
        Ok(unsafe { &*(data.as_ptr().add(8) as *const Self) })
    }

    pub fn load_mut(data: &mut [u8]) -> Result<&mut Self, ProgramError> {
        // Accept both legacy (113) and extended (129) sizes
        if data.len() < Self::ACCOUNT_SIZE {
            return Err(ProgramError::AccountDataTooSmall);
        }
        if data[..8] != DISC_MARKET {
            return Err(ProgramError::InvalidAccountData);
        }
        Ok(unsafe { &mut *(data.as_mut_ptr().add(8) as *mut Self) })
    }

    pub fn init(data: &mut [u8]) -> Result<&mut Self, ProgramError> {
        if data.len() < Self::ACCOUNT_SIZE {
            return Err(ProgramError::AccountDataTooSmall);
        }
        data[..8].copy_from_slice(&DISC_MARKET);
        // CreateAccount syscall already zeros the data region
        Ok(unsafe { &mut *(data.as_mut_ptr().add(8) as *mut Self) })
    }

    pub fn is_active(&self) -> bool {
        self.active != 0
    }

    /// M-05: Read last_oracle_price from extension data. Returns 0 for legacy accounts.
    pub fn last_oracle_price(data: &[u8]) -> u64 {
        if data.len() >= Self::EXTENDED_ACCOUNT_SIZE {
            u64::from_le_bytes(
                data[Self::EXT_LAST_ORACLE_PRICE..Self::EXT_LAST_ORACLE_PRICE + 8]
                    .try_into()
                    .unwrap(),
            )
        } else {
            0
        }
    }

    /// M-05: Read max_price_deviation_bps from extension data. Returns 0 for legacy accounts (disabled).
    pub fn max_price_deviation_bps(data: &[u8]) -> u64 {
        if data.len() >= Self::EXTENDED_ACCOUNT_SIZE {
            u64::from_le_bytes(
                data[Self::EXT_MAX_DEVIATION_BPS..Self::EXT_MAX_DEVIATION_BPS + 8]
                    .try_into()
                    .unwrap(),
            )
        } else {
            0
        }
    }

    /// M-05: Write last_oracle_price to extension data. No-op for legacy accounts.
    pub fn set_last_oracle_price(data: &mut [u8], price: u64) {
        if data.len() >= Self::EXTENDED_ACCOUNT_SIZE {
            data[Self::EXT_LAST_ORACLE_PRICE..Self::EXT_LAST_ORACLE_PRICE + 8]
                .copy_from_slice(&price.to_le_bytes());
        }
    }

    /// M-05: Write max_price_deviation_bps to extension data. No-op for legacy accounts.
    pub fn set_max_price_deviation_bps(data: &mut [u8], bps: u64) {
        if data.len() >= Self::EXTENDED_ACCOUNT_SIZE {
            data[Self::EXT_MAX_DEVIATION_BPS..Self::EXT_MAX_DEVIATION_BPS + 8]
                .copy_from_slice(&bps.to_le_bytes());
        }
    }
}

// ── UserAccount (76 bytes data, 84 on-chain) ────────────────────────────────

#[repr(C, packed)]
pub struct UserAccount {
    pub owner: [u8; 32],
    pub balance: u64,
    pub total_deposited: u64,
    pub total_withdrawn: u64,
    pub total_realized_pnl: i64,
    pub total_fees_paid: u64,
    pub open_position_count: u32,
}

impl UserAccount {
    pub const SIZE: usize = core::mem::size_of::<Self>(); // 76
    pub const ACCOUNT_SIZE: usize = 8 + Self::SIZE; // 84

    pub fn load(data: &[u8]) -> Result<&Self, ProgramError> {
        if data.len() < Self::ACCOUNT_SIZE {
            return Err(ProgramError::AccountDataTooSmall);
        }
        if data[..8] != DISC_USER_ACCOUNT {
            return Err(ProgramError::InvalidAccountData);
        }
        Ok(unsafe { &*(data.as_ptr().add(8) as *const Self) })
    }

    pub fn load_mut(data: &mut [u8]) -> Result<&mut Self, ProgramError> {
        if data.len() < Self::ACCOUNT_SIZE {
            return Err(ProgramError::AccountDataTooSmall);
        }
        if data[..8] != DISC_USER_ACCOUNT {
            return Err(ProgramError::InvalidAccountData);
        }
        Ok(unsafe { &mut *(data.as_mut_ptr().add(8) as *mut Self) })
    }

    pub fn init(data: &mut [u8]) -> Result<&mut Self, ProgramError> {
        if data.len() < Self::ACCOUNT_SIZE {
            return Err(ProgramError::AccountDataTooSmall);
        }
        data[..8].copy_from_slice(&DISC_USER_ACCOUNT);
        // CreateAccount syscall already zeros the data region
        Ok(unsafe { &mut *(data.as_mut_ptr().add(8) as *mut Self) })
    }
}

// ── Position (105 bytes data, 113 on-chain) ──────────────────────────────────

#[repr(C, packed)]
pub struct Position {
    pub owner: [u8; 32],
    pub product_id: u32,
    pub side: u8, // 0 = Long, 1 = Short
    pub size: u64,
    pub entry_price: u64,
    pub leverage: u32,
    pub margin: u64,
    pub liquidation_price: u64,
    pub realized_pnl: i64,
    pub entry_funding_rate: i64,
    pub opened_at: i64,
    pub last_updated: i64,
}

impl Position {
    pub const SIZE: usize = core::mem::size_of::<Self>(); // 105
    pub const ACCOUNT_SIZE: usize = 8 + Self::SIZE; // 113

    pub fn load(data: &[u8]) -> Result<&Self, ProgramError> {
        if data.len() < Self::ACCOUNT_SIZE {
            return Err(ProgramError::AccountDataTooSmall);
        }
        if data[..8] != DISC_POSITION {
            return Err(ProgramError::InvalidAccountData);
        }
        Ok(unsafe { &*(data.as_ptr().add(8) as *const Self) })
    }

    pub fn load_mut(data: &mut [u8]) -> Result<&mut Self, ProgramError> {
        if data.len() < Self::ACCOUNT_SIZE {
            return Err(ProgramError::AccountDataTooSmall);
        }
        if data[..8] != DISC_POSITION {
            return Err(ProgramError::InvalidAccountData);
        }
        Ok(unsafe { &mut *(data.as_mut_ptr().add(8) as *mut Self) })
    }

    pub fn init(data: &mut [u8]) -> Result<&mut Self, ProgramError> {
        if data.len() < Self::ACCOUNT_SIZE {
            return Err(ProgramError::AccountDataTooSmall);
        }
        data[..8].copy_from_slice(&DISC_POSITION);
        // CreateAccount syscall already zeros the data region
        Ok(unsafe { &mut *(data.as_mut_ptr().add(8) as *mut Self) })
    }

    pub fn side_enum(&self) -> Side {
        if self.side == 0 {
            Side::Long
        } else {
            Side::Short
        }
    }
}
