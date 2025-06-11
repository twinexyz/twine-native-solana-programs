#![allow(dead_code)]

use {
    borsh::BorshDeserialize,
    solana_program::{
        borsh1::{get_instance_packed_len, get_packed_len, try_from_slice_unchecked},
        hash::Hash,
        instruction::Instruction,
        program_option::COption,
        program_pack::Pack,
        pubkey::Pubkey,
        stake, system_instruction, system_program,
    },
    solana_program_test::{processor, BanksClient, ProgramTest, ProgramTestContext},
    solana_sdk::{
        account::{Account as SolanaAccount, WritableAccount},
        clock::{Clock, Epoch},
        compute_budget::ComputeBudgetInstruction,
        signature::{Keypair, Signer},
        transaction::Transaction,
        transport::TransportError,
    },
};

pub fn tokens_gateway_program_test() -> ProgramTest {
    let mut program_test = ProgramTest::new("tokens_gateway", id(), processor!(Processor::process));
    program_test.prefer_bpf(false);
    program_test.add_program(
        "spl_token_2022",
        spl_token_2022::id(),
        processor!(spl_token_2022::processor::Processor::process),
    );
    program_test
}
