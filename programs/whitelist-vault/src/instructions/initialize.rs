use anchor_lang::prelude::*;
use anchor_spl::associated_token::AssociatedToken;
use anchor_spl::token_2022::Token2022;
use anchor_spl::token_interface::{Mint, TokenAccount};
use spl_tlv_account_resolution::state::ExtraAccountMetaList;
use spl_transfer_hook_interface::instruction::ExecuteInstruction;

use crate::errors::VaultError;
use crate::instructions::InitializeExtraAccountMetaList;
use crate::state::{Vault, Blacklist};

#[derive(Accounts)]
#[instruction(target_hook_id: Pubkey)]
pub struct Initialize<'info> {
    
    #[account(mut)]
    pub admin: Signer<'info>,
    #[account(
        init,
        seeds = [b"vault".as_ref()],
        bump,
        payer = admin,
        space = Vault::DISCRIMINATOR.len() + Vault::INIT_SPACE,
    )]
    pub vault: Account<'info, Vault>,

    #[account(
        init,
        seeds = [b"whitelist".as_ref()],
        bump,
        payer = admin,
        space = Blacklist::DISCRIMINATOR.len() + Blacklist::INIT_SPACE,
    )]
    pub whitelist: Account<'info, Blacklist>,

    #[account(
        init,
        payer = admin,
        mint::decimals = 9,
        mint::authority = admin,
        mint::token_program = token_program,
        extensions::transfer_hook::authority = admin,
        // We will set program_id manually in the handler to allow dynamic configuration
    )]
    pub mint: InterfaceAccount<'info, Mint>,

    #[account(
        init,
        associated_token::mint = mint,
        associated_token::authority = vault,
        payer = admin,
    )]
    pub vault_ata: InterfaceAccount<'info, TokenAccount>,

    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token2022>,

    /// CHECK: This account is manually initialized in the handler to allow for dynamic hook programs.
    #[account(mut)]
    pub extra_account_meta_list: AccountInfo<'info>,


}

impl<'info> Initialize<'info> {
    pub fn initialize(&mut self, bumps: InitializeBumps, target_hook_id: Pubkey) -> Result<()> {
        self.initialize_vault(bumps.vault)?;
        self.initialize_whitelist(bumps.whitelist)?;
        self.initialize_transfer_hook(target_hook_id)?;
        Ok(())
    }
    

    fn initialize_vault(&mut self, bump: u8) -> Result<()> {
        let vault = &mut self.vault;
        vault.admin = self.admin.key();
        vault.mint = self.mint.key();
        vault.bump = bump;
        vault.balance = 0;

        Ok(())
    }

    fn initialize_whitelist(&mut self, bump: u8) -> Result<()> {
        let whitelist = &mut self.whitelist;
        whitelist.vault = self.vault.key();
        whitelist.blacklisted_users = Vec::new();
        whitelist.users = Vec::new();
        whitelist.amount = Vec::new();
        whitelist.bump = bump;

        Ok(())
    }

    fn initialize_transfer_hook(&mut self, target_hook_id: Pubkey) -> Result<()> {

        msg!("Initializing Transfer Hook with program: {}...", target_hook_id);

        let extra_meta_key = self.extra_account_meta_list.key();
        let mint_key = self.mint.key();
        
        // Derive the expected PDA and bump
        let (expected_extra_meta, bump) = Pubkey::find_program_address(
            &[b"extra-account-metas", mint_key.as_ref()],
            &target_hook_id,
        );

        require_keys_eq!(
            extra_meta_key,
            expected_extra_meta,
            VaultError::InvalidExtraAccountMeta
        );

        // If the account is not initialized, create it
        if self.extra_account_meta_list.data_is_empty() {
            let space = ExtraAccountMetaList::size_of(
                InitializeExtraAccountMetaList::extra_account_metas()?.len()
            ).unwrap();
            let rent = Rent::get()?;
            let lamports = rent.minimum_balance(space);

            let seeds: &[&[u8]] = &[
                b"extra-account-metas",
                mint_key.as_ref(),
                &[bump],
            ];

            // If we are initializing for a DIFFERENT program, we might not be able to sign for it!
            // Wait! A program can ONLY sign for PDAs derived from ITS OWN Program ID.
            
            if target_hook_id == crate::ID {
                anchor_lang::solana_program::program::invoke_signed(
                    &anchor_lang::solana_program::system_instruction::create_account(
                        &self.admin.key(),
                        &extra_meta_key,
                        lamports,
                        space as u64,
                        &crate::ID,
                    ),
                    &[
                        self.admin.to_account_info(),
                        self.extra_account_meta_list.to_account_info(),
                        self.system_program.to_account_info(),
                    ],
                    &[seeds],
                )?;
            } else {
                // In the test case (Twin Program), the Twin program must initialize its own ExtraMeta.
                // This means the test should call initialize on the Twin program, 
                // and then we can just use the account here.
                msg!("Extra account meta list must be pre-initialized for foreign programs");
            }
        }

        // Manually set the transfer hook program ID in the mint extension
        let mint_info = self.mint.to_account_info();
        anchor_spl::token_2022::spl_token_2022::extension::transfer_hook::instruction::initialize(
            &anchor_spl::token_2022::spl_token_2022::ID,
            &mint_info.key(),
            Some(self.admin.key()),
            Some(target_hook_id),
        )?;

        // Get the extra account metas for the transfer hook
        let extra_account_metas = InitializeExtraAccountMetaList::extra_account_metas()?;

        msg!("Extra Account Metas: {:?}", extra_account_metas);
        msg!("Extra Account Metas Length: {}", extra_account_metas.len());

        // initialize ExtraAccountMetaList account with extra accounts
        if target_hook_id == crate::ID {
            ExtraAccountMetaList::init::<ExecuteInstruction>(
                &mut self.extra_account_meta_list.try_borrow_mut_data()?,
                &extra_account_metas,
            )
            .unwrap();
        } else {
            msg!("Skipping metadata initialization for foreign program: {}", target_hook_id);
        }

        Ok(())
    }
}

#[derive(Accounts)]
#[instruction(target_hook_id: Pubkey)]
pub struct InitializeExtraMeta<'info> {
    #[account(mut)]
    pub admin: Signer<'info>,
    /// CHECK: The mint
    pub mint: AccountInfo<'info>,
    /// CHECK: This account is manually initialized in the handler.
    #[account(mut)]
    pub extra_account_meta_list: AccountInfo<'info>,
    pub system_program: Program<'info, System>,
}

impl<'info> InitializeExtraMeta<'info> {
    pub fn initialize_extra_meta(&mut self, target_hook_id: Pubkey) -> Result<()> {
        msg!("Initializing Extra Meta for program: {}...", target_hook_id);

        let extra_meta_key = self.extra_account_meta_list.key();
        let mint_key = self.mint.key();
        
        let (expected_extra_meta, bump) = Pubkey::find_program_address(
            &[b"extra-account-metas", mint_key.as_ref()],
            &target_hook_id,
        );

        require_keys_eq!(
            extra_meta_key,
            expected_extra_meta,
            VaultError::InvalidExtraAccountMeta
        );

        if self.extra_account_meta_list.data_is_empty() {
            let space = ExtraAccountMetaList::size_of(
                InitializeExtraAccountMetaList::extra_account_metas()?.len()
            ).unwrap();
            let rent = Rent::get()?;
            let lamports = rent.minimum_balance(space);

            let seeds: &[&[u8]] = &[
                b"extra-account-metas",
                mint_key.as_ref(),
                &[bump],
            ];

            // Only allow the program to sign for its own PDAs
            anchor_lang::solana_program::program::invoke_signed(
                &anchor_lang::solana_program::system_instruction::create_account(
                    &self.admin.key(),
                    &extra_meta_key,
                    lamports,
                    space as u64,
                    &crate::ID,
                ),
                &[
                    self.admin.to_account_info(),
                    self.extra_account_meta_list.to_account_info(),
                    self.system_program.to_account_info(),
                ],
                &[seeds],
            )?;

            // Initialize the content
            let extra_account_metas = InitializeExtraAccountMetaList::extra_account_metas()?;
            ExtraAccountMetaList::init::<ExecuteInstruction>(
                &mut self.extra_account_meta_list.try_borrow_mut_data()?,
                &extra_account_metas,
            ).unwrap();
        }

        Ok(())
    }
}
