use anchor_lang::prelude::*;
use anchor_spl::token::{TokenAccount, Token2022};
use anchor_spl::token_interface::{
    Mint, 
    TokenInterface,
};
use crate::state::vault::Vault;
use crate::state::whitelist::Whitelist;

#[derive(Accounts)]
pub struct Initialize <'info> {
    
    pub admin: Signer<'info>,
    #[account(
        init,
        seeds = [b"vault".as_ref()],
        bump,
        payer = admin,
        space = 8 + Vault::INIT_SPACE, // Account discriminator + Pubkey + u8
    )]
    pub vault: Account<'info, Vault>,

    #[
        account(
            init,
            seeds = [b"whitelist".as_ref()],
            bump,
            payer = admin,
            space = 8 + Whitelist::INIT_SPACE, // Account discriminator + Vec<Pubkey> + u8
        )
    ]
    pub whitelist: Account<'info, Whitelist>,
    #[account(
        init,
        token::mint = mint,
        token::authority = vault,
    )]
    pub vault_ata: InterfaceAccount<'info, TokenAccount>,
    pub mint : InterfaceAccount<'info, Mint>,
    #[account(
        token::mint = mint,
    )]
    pub destination_token: InterfaceAccount<'info, TokenAccount>,
    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token2022>,


}

impl<'info> Initialize<'info> {
    pub fn initialize_vault(&mut self, bump: u8) -> Result<()> {
        let vault = &mut self.vault;
        vault.address = self.vault.key();
        vault.bump = bump;

        Ok(())
    }

    pub fn initialize_whitelist(&mut self, bump: u8) -> Result<()> {
        let whitelist = &mut self.whitelist;
        whitelist.address = self.whitelist.key();
        whitelist.bump = bump;

        Ok(())
    }
}



