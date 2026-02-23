/// Declarative macro framework for account validation.
///
/// Replaces manual account extraction + validation in every handler with a
/// single declarative block that specifies account types, ordering, ownership
/// checks, PDA verification, and signer requirements.
///
/// Account types:
///   - `signer`        — must be a signer
///   - `signer_mut`    — must be a signer + writable (for payers)
///   - `account`       — owned by this program, discriminator checked via load()
///   - `account_mut`   — same, writable
///   - `pda`           — owned by this program, PDA address verified
///   - `pda_mut`       — same, writable
///   - `init_pda`      — will be created via CreateAccount CPI (no ownership check)
///   - `token`         — SPL Token account (owned by token program)
///   - `token_mut`     — same, writable
///   - `program`       — program account (system, token, etc.) — no checks
///   - `sysvar`        — sysvar account — no checks
///   - `unchecked`     — no checks at all (e.g., oracle AccountInfo in Anchor)
///   - `unchecked_mut` — writable, no ownership checks
///
/// This module uses `macro_rules!` (no proc-macro crate needed).

/// Parse and validate instruction accounts from the accounts slice.
///
/// Usage:
/// ```ignore
/// instruction_accounts! {
///     pub struct MyAccounts<'a> {
///         // Fields are extracted in ORDER (index 0, 1, 2, ...)
///         pub oracle_state:   account,      // owned_by(program_id), load discriminator
///         pub publisher_list: account_mut,   // same + writable
///         pub authority:      signer,        // is_signer() check
///         pub system_program: program,       // no checks
///     }
/// }
/// ```
///
/// Generates:
/// - A struct `MyAccounts<'a>` with `&'a AccountView` fields
/// - A `MyAccounts::parse(program_id, accounts) -> Result<Self, ProgramError>` method
#[macro_export]
macro_rules! instruction_accounts {
    (
        pub struct $name:ident<'a> {
            $(
                pub $field:ident : $kind:ident $( ( $($extra:tt)* ) )?,
            )*
        }
    ) => {
        pub struct $name<'a> {
            $(
                pub $field: &'a ::pinocchio::account::AccountView,
            )*
        }

        impl<'a> $name<'a> {
            #[inline(never)]
            pub fn parse(
                program_id: &::pinocchio::Address,
                accounts: &'a [::pinocchio::account::AccountView],
            ) -> Result<Self, ::pinocchio::error::ProgramError> {
                let mut _idx: usize = 0;

                // Count required accounts at compile time
                let _required = instruction_accounts!(@count $($field),*);

                if accounts.len() < _required {
                    return Err(::pinocchio::error::ProgramError::NotEnoughAccountKeys);
                }

                $(
                    let $field = &accounts[_idx];
                    instruction_accounts!(@validate $kind, $field, program_id $(, $($extra)* )?);
                    _idx += 1;
                )*

                Ok(Self {
                    $( $field, )*
                })
            }
        }
    };

    // ── Count helper ────────────────────────────────────────────────────────
    (@count $($field:ident),*) => {
        {
            let mut count = 0usize;
            $( let _ = stringify!($field); count += 1; )*
            count
        }
    };

    // ── Validation rules per account type ────────────────────────────────────

    // signer: must be a signer
    (@validate signer, $acc:ident, $pid:ident) => {
        if !$acc.is_signer() {
            return Err(::pinocchio::error::ProgramError::MissingRequiredSignature);
        }
    };

    // signer_mut: must be a signer (writable checked by runtime)
    (@validate signer_mut, $acc:ident, $pid:ident) => {
        if !$acc.is_signer() {
            return Err(::pinocchio::error::ProgramError::MissingRequiredSignature);
        }
    };

    // account: owned by this program
    (@validate account, $acc:ident, $pid:ident) => {
        if !$acc.owned_by($pid) {
            return Err(::pinocchio::error::ProgramError::IllegalOwner);
        }
    };

    // account_mut: owned by this program
    (@validate account_mut, $acc:ident, $pid:ident) => {
        if !$acc.owned_by($pid) {
            return Err(::pinocchio::error::ProgramError::IllegalOwner);
        }
    };

    // pda: owned by this program + PDA address verified
    // Usage: pda(SEED1, SEED2, ...)
    (@validate pda, $acc:ident, $pid:ident, $($seed:expr),+ ) => {
        if !$acc.owned_by($pid) {
            return Err(::pinocchio::error::ProgramError::IllegalOwner);
        }
        $crate::helpers::verify_pda($pid, $acc, &[$($seed),+])?;
    };

    // pda_mut: owned by this program + PDA address verified
    (@validate pda_mut, $acc:ident, $pid:ident, $($seed:expr),+ ) => {
        if !$acc.owned_by($pid) {
            return Err(::pinocchio::error::ProgramError::IllegalOwner);
        }
        $crate::helpers::verify_pda($pid, $acc, &[$($seed),+])?;
    };

    // init_pda: will be created — no ownership check (account doesn't exist yet)
    (@validate init_pda, $acc:ident, $pid:ident) => {
        // No validation — account is about to be created via CreateAccount CPI
    };

    // token: SPL Token account (owned by token program)
    (@validate token, $acc:ident, $pid:ident) => {
        if !$acc.owned_by(&::pinocchio::Address::from($crate::constants::TOKEN_PROGRAM_ID)) {
            return Err(::pinocchio::error::ProgramError::IllegalOwner);
        }
    };

    // token_mut: SPL Token account, writable (owned by token program)
    (@validate token_mut, $acc:ident, $pid:ident) => {
        if !$acc.owned_by(&::pinocchio::Address::from($crate::constants::TOKEN_PROGRAM_ID)) {
            return Err(::pinocchio::error::ProgramError::IllegalOwner);
        }
    };

    // program: program account — validated against known program IDs
    (@validate program, $acc:ident, $pid:ident) => {
        // Validate program account is a known program (system, token, etc.)
        // This prevents substitution attacks where an attacker passes a rogue program.
        let addr = $acc.address();
        let is_known = addr == &::pinocchio::Address::from($crate::constants::TOKEN_PROGRAM_ID)
            || addr == &::pinocchio::Address::from($crate::constants::SYSTEM_PROGRAM_ID)
            || addr == $pid; // Allow our own program ID
        if !is_known {
            return Err(::pinocchio::error::ProgramError::IncorrectProgramId);
        }
    };

    // sysvar: sysvar account — no checks
    (@validate sysvar, $acc:ident, $pid:ident) => {
        // No validation for sysvar accounts
    };

    // instructions_sysvar: Instructions sysvar — validates address
    (@validate instructions_sysvar, $acc:ident, $pid:ident) => {
        if $acc.address() != &::pinocchio::sysvars::instructions::INSTRUCTIONS_ID {
            return Err(::pinocchio::error::ProgramError::UnsupportedSysvar);
        }
    };

    // unchecked: no checks (CHECK in Anchor)
    (@validate unchecked, $acc:ident, $pid:ident) => {
        // No validation — caller is responsible
    };

    // unchecked_mut: writable, no ownership checks
    (@validate unchecked_mut, $acc:ident, $pid:ident) => {
        // No validation — caller is responsible
    };
}
