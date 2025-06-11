#![cfg(feature = "test-sbf")]
use {
    solana_program::{
        borsh1::{get_instance_packed_len, get_packed_len, try_from_slice_unchecked},
        hash::Hash,
        instruction::{AccountMeta, Instruction},
        program_pack::Pack,
        pubkey::Pubkey,
        stake, system_instruction, sysvar,
    },
    solana_program_test::{processor, BanksClient, ProgramTest, ProgramTestContext},
    solana_sdk::{
        instruction::InstructionError,
        signature::{Keypair, Signer},
        transaction::{Transaction, TransactionError},
        transport::TransportError,
    },
};
pub async fn tokens_gateway_setup(
    token_program_id: Pubkey,
)-> (){
    let mut context = program_test().start_with_context().await;
}