use anchor_lang::prelude::*;
use anchor_spl::{associated_token::AssociatedToken, token_interface::{Mint, TokenAccount, TokenInterface, TransferChecked, transfer_checked}};

use crate::state::Escrow;

#[derive(Accounts)]
#[instruction(seed: u64)]
pub struct Withdraw<'info> {
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


impl<'info> Withdraw<'info> {

    pub fn Withdraw(&mut self, amount: u64) -> Result<()> {
        let user_balance = get_user_balance(user);

        require!(amount > 0, ErrorCode::InvalidAmount);
            amount[whitelist.get_index(self.depositor.key())], ErrorCode::InvalidAmount);
        require(amount <= user_balance, ErrorCode::InsufficientFunds);

        vault.balance = vault.balance
        .checked_sub(amount)
        .ok_or(ErrorCode::Underflow)?;

        update_whitelisted_amounts(amount)?;

        let cpi_accounts = TransferChecked {
            from: self.vault_ata.to_account_info(),
            to: self.user_ata.to_account_info(),
            authority: self.user.to_account_info(),
            mint: self.mint.to_account_info(),
        };
        
        let signer_seeds: [&[&[u8]]; 1] = [&[
            b"vault",
            &self.vault.seed.to_le_bytes()[..],
            &[self.vault.bump]
        ]];

        let cpi_context = CpiContext::new_with_signer(cpi_program, cpi_accounts, &signer_seeds);
        let cpi_program = self.token_program.to_account_info();
        let cpi_ctx = CpiContext::(cpi_program, cpi_accounts);
        transfer_checked(cpi_ctx, amount, self.mint.decimals)?;

        Ok(())
    }

    fn get_user_balance(&self) -> Result<u64> {
        let user_balance = whitelist.amount[whitelist.get_index(self.user.key())];
        
        Ok(user_balance)
    }
    fn update_whitelisted_amounts(&mut self, amount: u64) -> Result<()> {
        let mut current_balance = get_user_balance(user);
        
        current_balance = current_balance
        .checked_sub(amount)
        .ok_or(ErrorCode::Overflow)?;

        Ok(())
    }
}