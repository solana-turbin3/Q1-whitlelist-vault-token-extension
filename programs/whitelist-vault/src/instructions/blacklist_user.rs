use anchor_lang::prelude::*;
use crate::state::Blacklist;
use crate::errors::VaultError;

#[derive(Accounts)]
pub struct BlacklistUser<'info> {
    #[account(mut)]
    pub admin: Signer<'info>,
    #[account(
        mut,
        seeds = [b"whitelist".as_ref()],
        bump = blacklist.bump,
        constraint = blacklist.vault.key() == vault.key(),
    )]
    pub blacklist: Account<'info, Blacklist>,
    /// CHECK: The vault PDA, used for constraint check
    #[account(seeds = [b"vault".as_ref()], bump)]
    pub vault: UncheckedAccount<'info>,
    /// CHECK: The admin set in vault must match the signer
    #[account(
        constraint = vault_data.admin == admin.key() @ VaultError::InvalidAdmin
    )]
    pub vault_data: Account<'info, crate::state::Vault>,
}

pub fn blacklist_user(ctx: Context<BlacklistUser>, user: Pubkey, add: bool) -> Result<()> {
    let blacklist = &mut ctx.accounts.blacklist;
    
    if add {
        if !blacklist.blacklisted_users.contains(&user) {
            blacklist.blacklisted_users.push(user);
        }
    } else {
        if let Some(index) = blacklist.blacklisted_users.iter().position(|&x| x == user) {
            blacklist.blacklisted_users.remove(index);
        }
    }
    
    Ok(())
}
