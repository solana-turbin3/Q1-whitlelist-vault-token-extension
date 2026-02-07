use anchor_lang::prelude::*;

#[error_code]
pub enum VaultError {
    #[msg("Invalid vault account")]
    InvalidVaultAccount,
    #[msg("Invalid whitelist account")]
    InvalidWhitelistAccount,
    #[msg("Invalid admin")]
    InvalidAdmin,
    #[msg("Invalid mint")]
    InvalidMint,
    #[msg("Invalid amount")]
    InvalidAmount,
    #[msg("Overflow error")]
    OverflowError,
}