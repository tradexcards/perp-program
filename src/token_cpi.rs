/// SPL Token CPI helpers using pinocchio-token.
use pinocchio::{
    account::AccountView, cpi::Seed, cpi::Signer, error::ProgramError, Address, ProgramResult,
};
use pinocchio_token::instructions::Transfer;

/// Transfer tokens using user-signed authority (e.g., deposit, fund_insurance).
pub fn token_transfer_user_signed(
    _token_program: &AccountView,
    from: &AccountView,
    to: &AccountView,
    authority: &AccountView,
    amount: u64,
) -> ProgramResult {
    Transfer {
        from,
        to,
        authority,
        amount,
    }
    .invoke()
}

/// Transfer tokens using PDA-signed authority (e.g., withdraw, insurance operations).
pub fn token_transfer_pda_signed(
    _token_program: &AccountView,
    from: &AccountView,
    to: &AccountView,
    authority: &AccountView,
    amount: u64,
    signer_seeds: &[Seed],
) -> ProgramResult {
    let signer = Signer::from(signer_seeds);
    Transfer {
        from,
        to,
        authority,
        amount,
    }
    .invoke_signed(&[signer])
}

/// Read the `amount` field from an SPL Token account.
/// Token account layout: [mint: 32][owner: 32][amount: u64 LE][...rest]
pub fn read_token_amount(account: &AccountView) -> Result<u64, ProgramError> {
    let data = account.try_borrow()?;
    if data.len() < 72 {
        return Err(ProgramError::InvalidAccountData);
    }
    Ok(u64::from_le_bytes(
        data[64..72]
            .try_into()
            .map_err(|_| ProgramError::InvalidAccountData)?,
    ))
}

/// Read the `owner` field from an SPL Token account.
pub fn read_token_owner(account: &AccountView) -> Result<[u8; 32], ProgramError> {
    let data = account.try_borrow()?;
    if data.len() < 64 {
        return Err(ProgramError::InvalidAccountData);
    }
    let mut owner = [0u8; 32];
    owner.copy_from_slice(&data[32..64]);
    Ok(owner)
}

/// Read the `mint` field from an SPL Token account.
pub fn read_token_mint(account: &AccountView) -> Result<[u8; 32], ProgramError> {
    let data = account.try_borrow()?;
    if data.len() < 32 {
        return Err(ProgramError::InvalidAccountData);
    }
    let mut mint = [0u8; 32];
    mint.copy_from_slice(&data[..32]);
    Ok(mint)
}

/// Initialize a new SPL Token account via CPI.
///
/// Account list for SPL Token InitializeAccount:
///   0. [writable] The token account to initialize
///   1. [] The mint
///   2. [] The owner (read-only, just provides the pubkey)
///   3. [] Rent sysvar
pub fn initialize_token_account(
    _token_program: &AccountView,
    account: &AccountView,
    mint: &AccountView,
    owner: &AccountView,
    rent_sysvar: &AccountView,
) -> ProgramResult {
    use pinocchio::cpi;
    use pinocchio::instruction::{InstructionAccount, InstructionView};

    let data = [1u8]; // InitializeAccount instruction index

    let account_metas = [
        InstructionAccount::writable(account.address()),
        InstructionAccount::readonly(mint.address()),
        InstructionAccount::readonly(owner.address()),
        InstructionAccount::readonly(rent_sysvar.address()),
    ];

    let token_program_id = Address::from(crate::constants::TOKEN_PROGRAM_ID);
    let ix = InstructionView {
        program_id: &token_program_id,
        data: &data,
        accounts: &account_metas,
    };

    cpi::invoke::<4>(&ix, &[account, mint, owner, rent_sysvar])
}
