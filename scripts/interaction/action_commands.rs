use clap::{command, Parser, Subcommand};
use solana_sdk::pubkey::Pubkey;

#[derive(Parser, Debug)]
#[command(name = "Twine Solana Programs CLI")]
#[command(about = "A CLI to interact with Twine solana programs", long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    InitializePrograms {},
    TokenMapping {
        l1_token: String,
        l2_token: String,
        l1_decimals: u8,
        l2_decimals: u8,
    },
    DepositNativeToken {
        l1_token: String,
        l2_token: String,
        receiver_twine_address: String,
        amount: u64,
        data: String
    },
    DepositSplToken {
        l1_token: Pubkey,
        l2_token: String,
        receiver_twine_address: String,
        user_token_account: Pubkey,
        amount: u64,
        data: String
    },
    ForcedNativeWithdrawal {
        l1_token: String,
        l2_token: String,
        from_twine_address: String,
        privkey: String,
        amount: u64,
    },
    ForcedSplWithdrawal {
        l1_token: Pubkey,
        l2_token: String,
        from_twine_address: String,
        privkey: String,
        user_token_account: Pubkey,
        amount: u64,
    },
    GetBatchPda{batch_number:u64},
    CreateSplToken {},
    GetAllPdas{},
    // GetMessagesBufferData{}
}
