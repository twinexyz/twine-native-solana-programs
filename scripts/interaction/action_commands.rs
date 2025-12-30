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
    RemoveTokenMapping {
        l1_token: String,
        l2_token: String,
    },
    DepositNativeToken {
        l1_token: String,
        l2_token: String,
        receiver_twine_address: String,
        amount: u64,
        data: String,
    },
    DepositSplToken {
        l1_token: Pubkey,
        l2_token: String,
        receiver_twine_address: String,
        user_token_account: Pubkey,
        amount: u64,
        data: String,
    },
    ForcedNativeWithdrawal {
        l1_token: String,
        l2_token: String,
        from_twine_address: String,
        l1_receiver: String,
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
    ExecuteNativeL2Withdrawal {
        receiver: Pubkey,
        public_values: String,
        proof: String,
    },
    ExecuteSplL2Withdrawal {
        spl_token_pubkey: Pubkey,
        l1_receiver_address: Pubkey,
        public_values: String,
        execution_proof: String,
    },
    ProcessNativeRefund {
        receiver: Pubkey,
        public_values: String,
        proof: String,
    },
    ProcessSplRefund {
        l1_token: Pubkey,
        l1_receiver_address: Pubkey,
        public_values: String,
        proof: String,
    },
    ProcessNativeForcedWithdrawal {
        receiver: Pubkey,
        public_values: String,
        proof: String,
    },
    ProcessSplForcedWithdrawal {
        l1_token: Pubkey,
        l1_receiver_address: Pubkey,
        public_values: String,
        proof: String,
    },

    GetBatchPda {
        batch_number: u64,
    },
    GetAssociatedTokenAccount {
        wallet_address: Pubkey,
        spl_token_pubkey: Pubkey,
    },
    CreateSplToken {},
    GetAllPdas {},
    GetMessagesBufferData {},
    GetDetailedMessagesBufferData {},
    GetTokensMappingData {},
    GetTwineChainStorageData {},
    GetTwineChainRoleManagerData {},
    GetTokensGatewayRoleManagerData {},
    GetMessageReplicatorData {
        start_nonce: u64,
        end_nonce: u64,
    },
    AddRoleInTwineChain {
        role_type: String,
        user_pubkey: Pubkey,
    },
    RemoveRoleInTwineChain {
        role_type: String,
        user_pubkey: Pubkey,
    },
    AddRoleInTokensGateway {
        role_type: String,
        user_pubkey: Pubkey,
    },
    RemoveRoleInTokensGateway {
        role_type: String,
        user_pubkey: Pubkey,
    },
    CopyMessagesBuffer {
        start_nonce: u64,
        end_nonce: u64,
    },

    // Layer Zero Methods:
    LzDepositNativeToken {
        l1_token: String,
        l2_token: String,
        receiver_twine_address: String,
        amount: u64,
        data: String,
    },
    LzDepositSplToken {
        l1_token: Pubkey,
        l2_token: String,
        receiver_twine_address: String,
        user_token_account: Pubkey,
        amount: u64,
        data: String,
    },
    LzForcedNativeWithdrawal {
        l1_token: String,
        l2_token: String,
        from_twine_address: String,
        l1_receiver: String,
        privkey: String,
        amount: u64,
    },
    LzForcedSplWithdrawal {
        l1_token: Pubkey,
        l2_token: String,
        from_twine_address: String,
        privkey: String,
        user_token_account: Pubkey,
        amount: u64,
    },
    SetLayerZeroInfo {
        dst_eid: u32,
        dst_oapp_address: String,
    },

    // OAPP Commands:
    InitializeStore {},
    InitSendLibrary {},
    InitReceiveLibrary {},
    InitNonce {
        remote_oapp: String,
    },
    InitConfig {},
    SetSendLibrary {},
    SetConfig {},
    SendMessage {},
}
