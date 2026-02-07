use anchor_lang::prelude::*;
use anchor_spl::{associated_token::AssociatedToken, token_interface::{Mint, TokenAccount, TokenInterface, TransferChecked, transfer_checked}};

use crate::state::Escrow;

#[derive(Accounts)]
#[instruction(seed: u64)]
pub struct Deposit<'info> {
    #[account(mut)]
    pub user: Signer<'info>,
    pub mint: InterfaceAccount<'info, Mint>,
    #[account(
        mut,
        associated_token::mint = mint,
        associated_token::authority = user,
        
    )]
    pub user_ata: InterfaceAccount<'info, TokenAccount>,

    #[account(
        mut,
        seeds = [b"vault".as_ref()],
        bump = vault.bump,
    )]
    pub vault: Account<'info, Vault>,
    #[account(
        mut,
        associated_token::mint = mint,
        associated_token::authority = vault,
    )]
    pub vault_ata: InterfaceAccount<'info, TokenAccount>,

    pub associated_token_program: Program<'info, AssociatedToken>,
    pub token_program: Program<'info, Token2022>,
    pub system_program: Program<'info, System>,
}


impl<'info> Deposit<'info> {

    pub fn deposit(&mut self, amount: u64) -> Result<()> {

        require!(amount > 0, ErrorCode::InvalidDepositAmount);

        vault.balance = vault.balance
        .checked_add(amount)
        .ok_or(ErrorCode::Overflow)?;

        update_whitelisted_amounts(amount)?;

        let cpi_accounts = TransferChecked {
            from: self.user_ata.to_account_info(),
            to: self.vault_ata.to_account_info(),
            authority: self.user.to_account_info(),
            mint: self.mint.to_account_info(),
        };
        
        let cpi_program = self.token_program.to_account_info();
        let cpi_ctx = CpiContext::new(cpi_program, cpi_accounts);
        transfer_checked(cpi_ctx, amount, self.mint.decimals)?;

        Ok(())
    }

    fn update_whitelisted_amounts(&mut self, amount: u64) -> Result<()> {
        let mut current_amount = whitelist.
        amount[whitelist.get_index(self.user.key())];

        current_amount = current_amount
        .checked_add(amount)
        .ok_or(ErrorCode::Overflow)?;

        Ok(())
    }
}