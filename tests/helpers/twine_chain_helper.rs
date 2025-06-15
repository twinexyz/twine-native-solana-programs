#![allow(dead_code)]

use {
    dirs,
    solana_program::{pubkey::Pubkey, system_program},
    solana_program_test::{processor, ProgramTest, ProgramTestContext},
    solana_sdk::{
        signature::{read_keypair_file, Keypair, Signer},
        system_instruction,
        transaction::Transaction,
    },
    twine_chain::{core::processor, id, utils::address_derivation::derive_role_manager},
};

pub fn program_test() -> ProgramTest {
    let mut program_test = ProgramTest::default();
    program_test.add_program(
        "twine_chain",
        id(),
        processor!(processor::process_instruction),
    );
    program_test.prefer_bpf(false);
    program_test
}

#[derive(Debug, PartialEq)]
pub struct TwineChainAccounts {
    pub role_manager: Pubkey,
    pub chain_admin: Keypair,
    pub system_program: Pubkey,
}

fn get_default_keypair() -> Keypair {
    let mut keypair_path = dirs::home_dir().expect("Could not get home directory");
    keypair_path.push(".config/solana/id.json");
    read_keypair_file(keypair_path).expect("Failed to read default keypar file")
}

pub async fn fund_account_for_rent_exemption(
    context: &mut ProgramTestContext,
    payer: &dyn Signer,
    recipient: &Pubkey,
    space: usize,
    buffer_lamports: u64,
) {
    let rent = context.banks_client.get_rent().await.unwrap();
    let min_balance = rent.minimum_balance(space);
    let fund_amount = min_balance + buffer_lamports;

    let tx = Transaction::new_signed_with_payer(
        &[system_instruction::transfer(
            &payer.pubkey(),
            recipient,
            fund_amount,
        )],
        Some(&payer.pubkey()),
        &[payer],
        context.last_blockhash,
    );
    context.banks_client.process_transaction(tx).await.unwrap();
}

impl Default for TwineChainAccounts {
    fn default() -> Self {
        let chain_admin = get_default_keypair();
        Self {
            role_manager: derive_role_manager(&id()).0,
            chain_admin,
            system_program: system_program::id(),
        }
    }
}
