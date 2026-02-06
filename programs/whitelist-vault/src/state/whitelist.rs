use anchor_lang::prelude::*;

#[account]
#[derive(InitSpace)]
pub struct Whitelist {
    pub address: Vec<Pubkey>,
    pub amount: Vec<u64>,
    pub bump: u8,
}