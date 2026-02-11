use anchor_lang::prelude::*;
use anchor_spl::associated_token::AssociatedToken;
use anchor_spl::token_interface::{TokenInterface, Mint, TokenAccount};

use crate::errors::VaultError;
use crate::state::{Vault, Blacklist};

#[derive(Accounts)]
pub struct Deposit<'info> {
    #[account(mut)]
    pub user: Signer<'info>,
    pub mint: InterfaceAccount<'info, Mint>,
    #[account(
        mut,
        associated_token::mint = mint,
        associated_token::authority = user,
        associated_token::token_program = token_program,
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
        seeds = [b"whitelist".as_ref()],
        bump = whitelist.bump,
    )]
    pub whitelist: Account<'info, Blacklist>,
    #[account(
        mut,
        associated_token::mint = mint,
        associated_token::authority = vault,
        associated_token::token_program = token_program,
    )]
    pub vault_ata: InterfaceAccount<'info, TokenAccount>,

    pub associated_token_program: Program<'info, AssociatedToken>,
    pub token_program: Interface<'info, TokenInterface>,
    pub system_program: Program<'info, System>,
    /// CHECK: The transfer hook program
    pub hook_program: UncheckedAccount<'info>,
}


impl<'info> Deposit<'info> {

    pub fn deposit(&mut self, amount: u64, remaining_accounts: &[AccountInfo<'info>]) -> Result<()> {

        require!(amount > 0, VaultError::InvalidAmount);

        let vault = &mut self.vault;
        vault.balance = vault
            .balance
            .checked_add(amount)
            .ok_or(VaultError::OverflowError)?;

        self.update_ledger(amount)?;

        // Manual instruction building to avoid Anchor CPI scrambling
        let mut accounts = vec![
            anchor_lang::solana_program::instruction::AccountMeta::new(self.user_ata.key(), false),
            anchor_lang::solana_program::instruction::AccountMeta::new_readonly(self.mint.key(), false),
            anchor_lang::solana_program::instruction::AccountMeta::new(self.vault_ata.key(), false),
            anchor_lang::solana_program::instruction::AccountMeta::new_readonly(self.user.key(), true),
        ];

        // Add transfer hook accounts: ValidationAccount, ResolvedAccounts, HookProgram
        for acc in remaining_accounts.iter() {
            accounts.push(anchor_lang::solana_program::instruction::AccountMeta {
                pubkey: acc.key(),
                is_signer: acc.is_signer,
                is_writable: acc.is_writable,
            });
        }
        accounts.push(anchor_lang::solana_program::instruction::AccountMeta::new_readonly(self.hook_program.key(), false));

        let ix = anchor_lang::solana_program::instruction::Instruction {
            program_id: *self.token_program.key,
            accounts,
            data: anchor_spl::token_2022::spl_token_2022::instruction::TokenInstruction::TransferChecked {
                amount,
                decimals: self.mint.decimals,
            }
            .pack(),
        };

        let mut account_infos = vec![
            self.user_ata.to_account_info(),
            self.mint.to_account_info(),
            self.vault_ata.to_account_info(),
            self.user.to_account_info(),
            self.token_program.to_account_info(),
            self.hook_program.to_account_info(),
        ];
        account_infos.extend_from_slice(remaining_accounts);

        anchor_lang::solana_program::program::invoke(&ix, &account_infos)?;

        Ok(())
    }

    fn update_ledger(&mut self, amount: u64) -> Result<()> {
        let whitelist = &mut self.whitelist;
        let user_key = self.user.key();

        if whitelist.blacklisted_users.contains(&user_key) {
            return Err(VaultError::Blacklisted.into());
        }

        if let Some(index) = whitelist.get_index(user_key) {
            whitelist.amount[index] = whitelist.amount[index]
                .checked_add(amount)
                .ok_or(VaultError::OverflowError)?;
        } else {
            whitelist.users.push(user_key);
            whitelist.amount.push(amount);
        }

        Ok(())
    }
}
