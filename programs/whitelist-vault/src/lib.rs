#![allow(unexpected_cfgs)]
use anchor_lang::prelude::*;
use crate::instructions::*;
use spl_discriminator::discriminator::SplDiscriminate;
use spl_transfer_hook_interface::instruction::ExecuteInstruction;

pub mod errors;
pub mod instructions;
pub mod state;
pub mod tests;


declare_id!("2Ze9h7UzmTccSf5F6oYYrxxM6biDMDPUWh2B1iKwubEg");


#[program]
pub mod whitelist_transfer_hook {
    use super::*;
    
    pub fn initialize(ctx: Context<Initialize>, target_hook_id: Pubkey) -> Result<()> {
        ctx.accounts.initialize(ctx.bumps, target_hook_id)
    }

    pub fn initialize_extra_meta(ctx: Context<InitializeExtraMeta>, target_hook_id: Pubkey) -> Result<()> {
        ctx.accounts.initialize_extra_meta(target_hook_id)
    }


    pub fn deposit<'info>(ctx: Context<'_, '_, '_, 'info, Deposit<'info>>, amount: u64) -> Result<()> {
        ctx.accounts
            .deposit(amount, ctx.remaining_accounts)
    }

    pub fn withdraw<'info>(ctx: Context<'_, '_, '_, 'info, Withdraw<'info>>, amount: u64) -> Result<()> {
        ctx.accounts.withdraw(amount, ctx.remaining_accounts)
    }

    pub fn blacklist_user(ctx: Context<BlacklistUser>, user: Pubkey, add: bool) -> Result<()> {
        instructions::blacklist_user(ctx, user, add)
    }


    #[instruction(discriminator = ExecuteInstruction::SPL_DISCRIMINATOR_SLICE)]
    pub fn transfer_hook(ctx: Context<TransferHook>, amount: u64) -> Result<()> {
        // Call the transfer hook logic
        ctx.accounts.transfer_hook(amount)
    }
}
