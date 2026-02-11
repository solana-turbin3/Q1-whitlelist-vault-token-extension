#[cfg(test)]
mod tests {
    use litesvm::LiteSVM;
    use anchor_lang::prelude::{Pubkey, ToAccountMetas};
    use anchor_lang::InstructionData;
    use solana_sdk::{
        signature::Keypair,
        signer::Signer,
        transaction::Transaction,
    };
    use solana_address::Address;

    const PROGRAM_ID: Pubkey = crate::ID;
    const HOOK_PROGRAM_ID: Pubkey = Pubkey::new_from_array([
        0, 0, 1, 198, 231, 56, 13, 78, 131, 198, 127, 21, 77, 199, 94, 19, 107, 47, 194, 134, 255, 41, 94, 105, 35, 207, 46, 27, 206, 156, 4, 193
    ]);

    fn to_address(p: Pubkey) -> Address {
        Address::new_from_array(p.to_bytes())
    }

    fn to_pubkey(a: Address) -> Pubkey {
        Pubkey::new_from_array(a.to_bytes())
    }

    fn map_ix(ix: anchor_lang::solana_program::instruction::Instruction) -> solana_sdk::instruction::Instruction {
        let accounts: Vec<_> = ix.accounts.into_iter().map(|m| {
            solana_sdk::instruction::AccountMeta {
                pubkey: to_address(m.pubkey),
                is_signer: m.is_signer,
                is_writable: m.is_writable,
            }
        }).collect();
        
        solana_sdk::instruction::Instruction {
            program_id: to_address(ix.program_id),
            accounts,
            data: ix.data,
        }
    }

    fn send_tx(svm: &mut LiteSVM, ixs: &[solana_sdk::instruction::Instruction], payer: &Keypair, signers: &[&Keypair]) {
        let mut tx = Transaction::new_with_payer(ixs, Some(&payer.pubkey()));
        tx.sign(signers, svm.latest_blockhash());
        let result = svm.send_transaction(tx);
        
        if let Err(e) = result {
            panic!("Transaction failed: {:?}", e);
        }
    }

    fn setup() -> (LiteSVM, Keypair) {
        let mut svm = LiteSVM::new();
        let payer = Keypair::new();
        let program_bytes = std::fs::read("../../target/deploy/whitelist_vault.so").expect("Run anchor build first");
        svm.add_program(to_address(PROGRAM_ID), &program_bytes).expect("Failed to add program");
        
        // Note: HOOK_PROGRAM_ID (Memo) is already available in LiteSVM
        let payer_address = payer.pubkey();
        svm.airdrop(&payer_address, 10_000_000_000).unwrap();
        (svm, payer)
    }

    fn vault_pda() -> (Pubkey, u8) {
        Pubkey::find_program_address(&[b"vault"], &PROGRAM_ID)
    }

    fn whitelist_pda() -> (Pubkey, u8) {
        Pubkey::find_program_address(&[b"whitelist"], &PROGRAM_ID)
    }

    fn initialize(svm: &mut LiteSVM, admin: &Keypair, mint: &Keypair) {
        let (vault, _) = vault_pda();
        let (blacklist, _) = whitelist_pda();
        let mint_pubkey = to_pubkey(mint.pubkey());
        let vault_ata = anchor_spl::associated_token::get_associated_token_address_with_program_id(&vault, &mint_pubkey, &spl_token_2022::ID);
        // User HOOK_PROGRAM_ID for the extra accounts seeding
        let (extra_meta, _) = Pubkey::find_program_address(&[b"extra-account-metas", mint_pubkey.as_ref()], &HOOK_PROGRAM_ID);

        // 1. Manually initialize the EXTRA MEATA account state in SVM (since Memo Program has no handler)
        let extra_account_metas = crate::instructions::InitializeExtraAccountMetaList::extra_account_metas().unwrap();
        let space = spl_tlv_account_resolution::state::ExtraAccountMetaList::size_of(extra_account_metas.len()).unwrap();
        let rent = svm.minimum_balance_for_rent_exemption(space);
        
        let mut data = vec![0u8; space];
        spl_tlv_account_resolution::state::ExtraAccountMetaList::init::<spl_transfer_hook_interface::instruction::ExecuteInstruction>(
            &mut data,
            &extra_account_metas,
        ).unwrap();

        svm.set_account(to_address(extra_meta), solana_sdk::account::Account {
            lamports: rent,
            data,
            owner: to_address(HOOK_PROGRAM_ID),
            executable: false,
            rent_epoch: 0,
        }).unwrap();


        // 2. Initialize the MAIN program
        let main_accounts = crate::accounts::Initialize {
            admin: to_pubkey(admin.pubkey()),
            vault,
            whitelist: blacklist,
            mint: mint_pubkey,
            vault_ata,
            associated_token_program: anchor_spl::associated_token::ID,
            system_program: anchor_lang::system_program::ID,
            token_program: spl_token_2022::ID,
            extra_account_meta_list: extra_meta,
        }
        .to_account_metas(None);

        let main_ix = map_ix(anchor_lang::solana_program::instruction::Instruction {
            program_id: PROGRAM_ID, // Call the MAIN
            accounts: main_accounts,
            data: crate::instruction::Initialize { target_hook_id: HOOK_PROGRAM_ID }.data(),
        });

        send_tx(svm, &[main_ix], admin, &[admin, mint]);
    }

    fn create_user_ata(svm: &mut LiteSVM, user: &Keypair, mint: &Keypair) {
        let mint_pubkey = to_pubkey(mint.pubkey());
        let user_pubkey = to_pubkey(user.pubkey());
        
        let ix = spl_associated_token_account::instruction::create_associated_token_account(
            &user_pubkey,
            &user_pubkey,
            &mint_pubkey,
            &spl_token_2022::ID,
        );
        
        let mapped_ix = solana_sdk::instruction::Instruction {
            program_id: to_address(ix.program_id),
            accounts: ix.accounts.into_iter().map(|a| solana_sdk::instruction::AccountMeta {
                pubkey: to_address(a.pubkey),
                is_signer: a.is_signer,
                is_writable: a.is_writable,
            }).collect(),
            data: ix.data,
        };
        
        send_tx(svm, &[mapped_ix], user, &[user]);
    }

    fn mint_tokens_to(svm: &mut LiteSVM, mint: &Keypair, mint_authority: &Keypair, to: &Keypair, amount: u64) {
        let mint_pubkey = to_pubkey(mint.pubkey());
        let to_user_pubkey = to_pubkey(to.pubkey());
        let mint_auth_pubkey = to_pubkey(mint_authority.pubkey());
        let to_ata = anchor_spl::associated_token::get_associated_token_address_with_program_id(&to_user_pubkey, &mint_pubkey, &spl_token_2022::ID);
        
        let ix = spl_token_2022::instruction::mint_to(
            &spl_token_2022::ID,
            &mint_pubkey,
            &to_ata,
            &mint_auth_pubkey,
            &[],
            amount,
        ).unwrap();
        
        let mapped_ix = solana_sdk::instruction::Instruction {
            program_id: to_address(ix.program_id),
            accounts: ix.accounts.into_iter().map(|a| solana_sdk::instruction::AccountMeta {
                pubkey: to_address(a.pubkey),
                is_signer: a.is_signer,
                is_writable: a.is_writable,
            }).collect(),
            data: ix.data,
        };
        
        send_tx(svm, &[mapped_ix], mint_authority, &[mint_authority]);
    }

    fn deposit(svm: &mut LiteSVM, user: &Keypair, mint: &Keypair, amount: u64) {
        let (vault, _) = vault_pda();
        let (whitelist, _) = whitelist_pda();
        let mint_pubkey = to_pubkey(mint.pubkey());
        let user_pubkey = to_pubkey(user.pubkey());
        let user_ata = anchor_spl::associated_token::get_associated_token_address_with_program_id(&user_pubkey, &mint_pubkey, &spl_token_2022::ID);
        let vault_ata = anchor_spl::associated_token::get_associated_token_address_with_program_id(&vault, &mint_pubkey, &spl_token_2022::ID);
        let (extra_meta, _) = Pubkey::find_program_address(&[b"extra-account-metas", mint_pubkey.as_ref()], &HOOK_PROGRAM_ID);

        let mut accounts = crate::accounts::Deposit {
            user: user_pubkey,
            mint: mint_pubkey,
            user_ata,
            vault,
            whitelist,
            vault_ata,
            associated_token_program: anchor_spl::associated_token::ID,
            token_program: spl_token_2022::ID,
            system_program: anchor_lang::system_program::ID,
            hook_program: HOOK_PROGRAM_ID,
        }
        .to_account_metas(None);

        // Add transfer hook accounts required by Token2022
        // Specific order: ExtraMeta (Validation), Whitelist (Resolved)
        accounts.push(anchor_lang::prelude::AccountMeta::new_readonly(extra_meta, false));
        accounts.push(anchor_lang::prelude::AccountMeta::new(whitelist, false));

        let ix = map_ix(anchor_lang::solana_program::instruction::Instruction {
            program_id: PROGRAM_ID,
            accounts,
            data: crate::instruction::Deposit { amount }.data(),
        });

        send_tx(svm, &[ix], user, &[user]);
    }

    fn withdraw(svm: &mut LiteSVM, user: &Keypair, mint: &Keypair, amount: u64) {
        let (vault, _) = vault_pda();
        let (whitelist, _) = whitelist_pda();
        let mint_pubkey = to_pubkey(mint.pubkey());
        let user_pubkey = to_pubkey(user.pubkey());
        let user_ata = anchor_spl::associated_token::get_associated_token_address_with_program_id(&user_pubkey, &mint_pubkey, &spl_token_2022::ID);
        let vault_ata = anchor_spl::associated_token::get_associated_token_address_with_program_id(&vault, &mint_pubkey, &spl_token_2022::ID);
        let (extra_meta, _) = Pubkey::find_program_address(&[b"extra-account-metas", mint_pubkey.as_ref()], &HOOK_PROGRAM_ID);

        let mut accounts = crate::accounts::Withdraw {
            user: user_pubkey,
            mint: mint_pubkey,
            user_ata,
            vault,
            whitelist,
            vault_ata,
            associated_token_program: anchor_spl::associated_token::ID,
            token_program: spl_token_2022::ID,
            system_program: anchor_lang::system_program::ID,
            hook_program: HOOK_PROGRAM_ID,
        }
        .to_account_metas(None);

        // Add transfer hook accounts required by Token2022
        // Specific order: ExtraMeta (Validation), Whitelist (Resolved)
        accounts.push(anchor_lang::prelude::AccountMeta::new_readonly(extra_meta, false));
        accounts.push(anchor_lang::prelude::AccountMeta::new(whitelist, false));

        let ix = map_ix(anchor_lang::solana_program::instruction::Instruction {
            program_id: PROGRAM_ID,
            accounts,
            data: crate::instruction::Withdraw { amount }.data(),
        });

        send_tx(svm, &[ix], user, &[user]);
    }

    fn blacklist_user_ix(svm: &mut LiteSVM, admin: &Keypair, user_to_blacklist: &Keypair, add: bool) {
        let (vault, _) = vault_pda();
        let (blacklist, _) = whitelist_pda();
        let user_pubkey = to_pubkey(user_to_blacklist.pubkey());

        let accounts = crate::accounts::BlacklistUser {
            admin: to_pubkey(admin.pubkey()),
            blacklist,
            vault,
            vault_data: vault,
        }
        .to_account_metas(None);

        let ix = map_ix(anchor_lang::solana_program::instruction::Instruction {
            program_id: PROGRAM_ID,
            accounts,
            data: crate::instruction::BlacklistUser { user: user_pubkey, add }.data(),
        });

        send_tx(svm, &[ix], admin, &[admin]);
    }

    fn transfer_tokens(svm: &mut LiteSVM, from: &Keypair, to_user_pubkey: &Pubkey, mint: &Keypair, amount: u64) {
        let mint_pubkey = to_pubkey(mint.pubkey());
        let from_pubkey = to_pubkey(from.pubkey());
        let from_ata = anchor_spl::associated_token::get_associated_token_address_with_program_id(&from_pubkey, &mint_pubkey, &spl_token_2022::ID);
        let to_ata = anchor_spl::associated_token::get_associated_token_address_with_program_id(to_user_pubkey, &mint_pubkey, &spl_token_2022::ID);
        let (extra_meta, _) = Pubkey::find_program_address(&[b"extra-account-metas", mint_pubkey.as_ref()], &HOOK_PROGRAM_ID);
        let (whitelist, _) = whitelist_pda();

        let mut accounts = vec![
            solana_sdk::instruction::AccountMeta::new(to_address(from_ata), false),
            solana_sdk::instruction::AccountMeta::new_readonly(to_address(mint_pubkey), false),
            solana_sdk::instruction::AccountMeta::new(to_address(to_ata), false),
            solana_sdk::instruction::AccountMeta::new_readonly(to_address(from_pubkey), true),
        ];

        // Add transfer hook accounts
        accounts.push(solana_sdk::instruction::AccountMeta::new_readonly(to_address(extra_meta), false));
        accounts.push(solana_sdk::instruction::AccountMeta::new_readonly(to_address(whitelist), false));
        accounts.push(solana_sdk::instruction::AccountMeta::new_readonly(to_address(HOOK_PROGRAM_ID), false));

        let ix = solana_sdk::instruction::Instruction {
            program_id: to_address(spl_token_2022::ID),
            accounts,
            data: spl_token_2022::instruction::TokenInstruction::TransferChecked {
                amount,
                decimals: 9,
            }.pack(),
        };

        send_tx(svm, &[ix], from, &[from]);
    }

    #[test]
    fn user_to_user_transfer_allowed() {
        let (mut svm, admin) = setup();
        let mint = Keypair::new();
        initialize(&mut svm, &admin, &mint);

        let user_a = Keypair::new();
        let user_b = Keypair::new();
        
        svm.airdrop(&user_a.pubkey(), 10_000_000_000).unwrap();
        svm.airdrop(&user_b.pubkey(), 10_000_000_000).unwrap();
        
        create_user_ata(&mut svm, &user_a, &mint);
        create_user_ata(&mut svm, &user_b, &mint);
        
        mint_tokens_to(&mut svm, &mint, &admin, &user_a, 1_000_000);
        
        transfer_tokens(&mut svm, &user_a, &to_pubkey(user_b.pubkey()), &mint, 400_000);
        
        // No assertion needed if it doesn't panic, but let's be sure
        // We can't easily check balance here without more helpers, 
        // but send_tx panics on failure.
    }

    #[test]
    fn test_deposit_success() {
        let (mut svm, admin) = setup();
        let mint = Keypair::new();
        initialize(&mut svm, &admin, &mint);

        let user = Keypair::new();
        svm.airdrop(&user.pubkey(), 10_000_000_000).unwrap();
        
        create_user_ata(&mut svm, &user, &mint);
        mint_tokens_to(&mut svm, &mint, &admin, &user, 1_000_000);
        
        deposit(&mut svm, &user, &mint, 500_000);
    }

    #[test]
    fn test_withdraw_success() {
        let (mut svm, admin) = setup();
        let mint = Keypair::new();
        initialize(&mut svm, &admin, &mint);

        let user = Keypair::new();
        svm.airdrop(&user.pubkey(), 10_000_000_000).unwrap();
        
        create_user_ata(&mut svm, &user, &mint);
        mint_tokens_to(&mut svm, &mint, &admin, &user, 1_000_000);
        
        deposit(&mut svm, &user, &mint, 500_000);
        withdraw(&mut svm, &user, &mint, 200_000);
    }

    #[test]
    #[should_panic(expected = "Blacklisted")]
    fn test_blacklisted_user_cannot_deposit() {
        let (mut svm, admin) = setup();
        let mint = Keypair::new();
        initialize(&mut svm, &admin, &mint);

        let user = Keypair::new();
        svm.airdrop(&user.pubkey(), 10_000_000_000).unwrap();
        
        blacklist_user_ix(&mut svm, &admin, &user, true);
        
        create_user_ata(&mut svm, &user, &mint);
        mint_tokens_to(&mut svm, &mint, &admin, &user, 1_000_000);
        
        deposit(&mut svm, &user, &mint, 500_000);
    }

    #[test]
    #[should_panic(expected = "Blacklisted")]
    fn test_blacklisted_user_cannot_withdraw() {
        let (mut svm, admin) = setup();
        let mint = Keypair::new();
        initialize(&mut svm, &admin, &mint);

        let user = Keypair::new();
        svm.airdrop(&user.pubkey(), 10_000_000_000).unwrap();
        
        create_user_ata(&mut svm, &user, &mint);
        mint_tokens_to(&mut svm, &mint, &admin, &user, 1_000_000);
        
        deposit(&mut svm, &user, &mint, 500_000);
        
        blacklist_user_ix(&mut svm, &admin, &user, true);
        
        withdraw(&mut svm, &user, &mint, 200_000);
    }
}
