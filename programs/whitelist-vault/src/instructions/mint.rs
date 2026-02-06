use anchor_lang::prelude::*;
use anchor_lang::solana_program::{program::invoke_signed, system_instruction};
use anchor_spl::token_2022::{self, spl_token_2022, Token2022};
use anchor_spl::token_2022_extensions;
use spl_token_2022::extension::{AccountType, ExtensionType};

#[derive(Accounts)]
#[instruction(seed: Pubkey)]
pub struct CreateMint<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,

    /// CHECK: Created + initialized by this instruction.
    #[account(
        mut,
        seeds = [b"mint", payer.key().as_ref(), seed.as_ref()],
        bump
    )]
    pub mint: UncheckedAccount<'info>,

    pub token_program: Program<'info, Token2022>,
    pub system_program: Program<'info, System>,
}

impl<'info> CreateMint<'info> {
    pub fn create_mint(
        &self,
        seed: Pubkey,
        decimals: u8,
        extension_types: Vec<u16>,
        mint_bump: u8,
    ) -> Result<()> {
        // Convert and validate extension types.
        let mut extensions: Vec<ExtensionType> = Vec::with_capacity(extension_types.len());
        for raw in extension_types {
            let ext = ExtensionType::try_from(raw)
                .map_err(|_| error!(crate::ErrorCode::InvalidExtensionType))?;

            // This instruction only supports MINT extensions.
            require!(
                ext.get_account_type() == AccountType::Mint,
                crate::ErrorCode::InvalidMintExtension
            );

            // For now, only TransferHook is supported.
            require!(
                ext == ExtensionType::TransferHook,
                crate::ErrorCode::UnsupportedMintExtension
            );

            if !extensions.contains(&ext) {
                extensions.push(ext);
            }
        }

        // Calculate mint account size with the requested extensions.
        let mint_space = ExtensionType::try_calculate_account_len::<spl_token_2022::state::Mint>(
            &extensions,
        )
        .map_err(|_| error!(crate::ErrorCode::InvalidExtensionTypes))?;

        let lamports = Rent::get()?.minimum_balance(mint_space);

        let payer_key = self.payer.key();
        let signer_seeds: &[&[&[u8]]] = &[&[b"mint", payer_key.as_ref(), seed.as_ref(), &[mint_bump]]];

        // Create the mint account owned by the Token-2022 program.
        let create_ix = system_instruction::create_account(
            &self.payer.key(),
            &self.mint.key(),
            lamports,
            mint_space as u64,
            &self.token_program.key(),
        );
        invoke_signed(
            &create_ix,
            &[
                self.payer.to_account_info(),
                self.mint.to_account_info(),
                self.system_program.to_account_info(),
            ],
            signer_seeds,
        )?;

        // Initialize extensions (must happen before initialize_mint2).
        if extensions.contains(&ExtensionType::TransferHook) {
            let cpi_accounts = token_2022_extensions::TransferHookInitialize {
                token_program_id: self.token_program.to_account_info(),
                mint: self.mint.to_account_info(),
            };
            let cpi_ctx = CpiContext::new(self.token_program.to_account_info(), cpi_accounts)
                .with_signer(signer_seeds);

            // Set this program as the transfer-hook program.
            token_2022_extensions::transfer_hook_initialize(
                cpi_ctx,
                Some(self.payer.key()),
                Some(crate::ID),
            )?;
        }

        // Initialize the mint itself.
        let cpi_accounts = token_2022::InitializeMint2 {
            mint: self.mint.to_account_info(),
        };
        let cpi_ctx = CpiContext::new(self.token_program.to_account_info(), cpi_accounts);
        token_2022::initialize_mint2(cpi_ctx, decimals, &self.payer.key(), None)
    }
}
