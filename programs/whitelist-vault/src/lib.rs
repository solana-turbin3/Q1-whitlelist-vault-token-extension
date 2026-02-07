use anchor_lang::prelude::*;
use anchor_lang::solana_program::msg;


declare_id!("2Ze9h7UzmTccSf5F6oYYrxxM6biDMDPUWh2B1iKwubEg");


#[program]
pub mod whitelist_transfer_hook {
    use super::*;

    pub fn add_to_whitelist(ctx: Context<AddToWhitelist>, address: Pubkey) -> Result<()> {
        ctx.accounts
            .add_to_whitelist(address, ctx.bumps.whitelist_entry)
    }

    pub fn remove_from_whitelist(ctx: Context<RemoveFromWhitelist>, address: Pubkey) -> Result<()> {
        ctx.accounts.remove_from_whitelist(address)
    }




    pub fn deposit(ctx: Context<Deposit>, amount: u64) -> Result<()> {
        ctx.accounts
            .deposit(amount)
    }

    pub fn withdraw(ctx: Context<Withdraw>, amount: u64) -> Result<()> {
        ctx.accounts.withdraw(amount)
    }


    #[instruction(discriminator = ExecuteInstruction::SPL_DISCRIMINATOR_SLICE)]
    pub fn transfer_hook(ctx: Context<TransferHook>, amount: u64) -> Result<()> {
        // Call the transfer hook logic
        ctx.accounts.transfer_hook(amount)
    }
}

