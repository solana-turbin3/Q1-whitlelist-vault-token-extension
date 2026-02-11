use anchor_lang::prelude::*;

#[derive(Accounts)]
pub struct TransferHook<'info> {
    /// CHECK: source
    pub source_token_ata: UncheckedAccount<'info>,
    /// CHECK: mint
    pub mint: UncheckedAccount<'info>,
    /// CHECK: destination
    pub destination_token_ata: UncheckedAccount<'info>,
    /// CHECK: owner
    pub owner: UncheckedAccount<'info>,
    /// CHECK: ExtraAccountMetaList Account
    pub extra_account_meta_list: UncheckedAccount<'info>,
    /// CHECK: whitelist
    pub whitelist: UncheckedAccount<'info>,
}

impl<'info> TransferHook<'info> {
    pub fn transfer_hook(&mut self, _amount: u64) -> Result<()> {
        msg!("Transfer Hook called!");
        Ok(())
    }
}
