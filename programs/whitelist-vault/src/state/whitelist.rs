use anchor_lang::prelude::*;

#[account]
#[derive(InitSpace)]
pub struct Blacklist {
    pub vault: Pubkey,
    #[max_len(100)]
    pub blacklisted_users: Vec<Pubkey>,
    #[max_len(100)]
    pub users: Vec<Pubkey>,
    #[max_len(100)]
    pub amount: Vec<u64>,
    pub bump: u8,
}



impl Blacklist {
    pub fn get_index(&self, key: Pubkey) -> Option<usize> {
        self.users.iter().position(|address| *address == key)
    }
}
