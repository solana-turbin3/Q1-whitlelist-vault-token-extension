use anchor_lang::prelude::*;

declare_id!("2Ze9h7UzmTccSf5F6oYYrxxM6biDMDPUWh2B1iKwubEg");

#[program]
pub mod whitelist_vault {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        msg!("Greetings from: {:?}", ctx.program_id);
        Ok(())
    }
}


