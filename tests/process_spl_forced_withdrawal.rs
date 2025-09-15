#[cfg(test)]
mod helpers;
use borsh::BorshDeserialize;
use sha3::{Digest, Keccak256};
use solana_program_test::*;
use solana_sdk::{
    signature::{Keypair, Signer},
    transaction::Transaction,
};

use helpers::tokens_gateway_helper::{
    create_spl_and_mint, fund_account_for_rent_exemption, get_ethereum_signature,
    get_or_create_ata, program_test, TokensGatewayAccounts,
};
use tokens_gateway::{
    core::{instruction as tokens_gateway_instruction, state::SignMessageInfo},
    id as tokens_gateway_id,
    utils::{
        address_derivation::{derive_spl_tokens_vault_data, derive_spl_vault_authority},
        constants::{CHAIN_ID, ROLE_MANAGER_ACCOUNT_SIZE},
    },
};

use twine_chain::{
    core::{
        instruction as twine_chain_instruction,
        state::{RoleType, TransactionType, TwineChainStorage},
    },
    id as twine_chain_id,
    utils::{
        address_derivation::{derive_twine_chain_storage},
        constants::MESSAGE_NONCE_GAP,
    },
};

#[tokio::test]
async fn process_spl_forced_withdrawal() {
    let mut context = program_test().start_with_context().await;
    let accounts = TokensGatewayAccounts::default();
    let payer = Keypair::from_bytes(&context.payer.to_bytes()).unwrap();
    fund_account_for_rent_exemption(
        &mut context,
        &payer,
        &accounts.chain_admin.pubkey(),
        ROLE_MANAGER_ACCOUNT_SIZE,
        844073716442015,
    )
    .await;
    let txn_type: TransactionType = TransactionType::Withdraw;
    let l1_decimals = 9u8;
    let l2_decimals = 18u8;
    let amount = 50000000000u64;
    let l2_amount = "50000000000000000000";
    let chain_admin = &accounts.chain_admin.pubkey();
    let l2_address = "0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266".to_string();
    let batch_number = 1u64;
    let nonce = 1u64;
    let slot_number = 1u64;
    let batch_hash = [1u8; 32];
    let l2_token = "0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266".to_string();
    let data = "".to_string();
    let execution_proof = vec![];
    let mut public_values = vec![];

    let from_twine_address = "0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266".to_string();

    let (spl_token_pubkey, user_token_account) =
        create_spl_and_mint(&mut context, &accounts.chain_admin, 9, 90000000000).await;
    let spl_token_vault = get_or_create_ata(
        &mut context,
        &accounts.chain_admin,
        &derive_spl_vault_authority(&tokens_gateway_id()).0,
        &spl_token_pubkey,
    )
    .await;
    let l1_token = spl_token_pubkey.to_string();

    let sign_info = SignMessageInfo {
        nonce: 1,
        chain_id: 900,
        amount: amount,
        l1_pubkey: user_token_account.to_string(),
        twine_address: l2_address.to_string(),
        l1_token: l1_token.to_string(),
        l2_token: l2_token.clone(),
    };
    let privkey = "0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80";

    let signature = get_ethereum_signature(&sign_info, privkey);
    let spl_token_valut_data_account = derive_spl_tokens_vault_data(&tokens_gateway_id()).0;
    public_values.extend_from_slice(&batch_hash);
    public_values.extend_from_slice(&batch_number.to_be_bytes());
    public_values.extend_from_slice(&txn_type.as_bytes());
    public_values.extend_from_slice(&nonce.to_be_bytes());
    public_values.extend_from_slice(&CHAIN_ID.to_be_bytes());
    public_values.extend_from_slice(&slot_number.to_be_bytes());
    public_values.extend_from_slice(&Keccak256::digest(&data.as_bytes()));
    public_values.extend_from_slice(&user_token_account.to_string().to_lowercase().as_bytes());
    public_values.extend_from_slice(&l2_address.to_lowercase().as_bytes());
    public_values.extend_from_slice(&l1_token.to_lowercase().as_bytes());
    public_values.extend_from_slice(&l2_token.to_lowercase().as_bytes());
    public_values.extend_from_slice(&l2_amount.to_string().as_bytes());
    println!("The public values{:?}", public_values);
    println!("The hex values {:?}", hex::encode(&public_values));

    let mut instructions = vec![];
    instructions.extend(twine_chain_instruction::initialize_twine_chain_role_manager(&chain_admin));
    instructions.extend(twine_chain_instruction::initialize_twine_chain_storage(
        &chain_admin,
    ));
    instructions.extend(twine_chain_instruction::add_role_in_twine_chain(
        &chain_admin,
        &spl_token_valut_data_account,
        RoleType::MessageAppender,
    ));
    instructions.extend(twine_chain_instruction::initialize_message_buffer(
        &chain_admin,
    ));
    instructions
        .extend(tokens_gateway_instruction::initialize_tokens_gateway_role_manager(&chain_admin));
    instructions.extend(tokens_gateway_instruction::initialize_tokens_gateway(
        &chain_admin,
    ));
    instructions.extend(tokens_gateway_instruction::update_gateway_token_mapping(
        l1_token.clone(),
        l2_token.clone(),
        l1_decimals,
        l2_decimals,
        &chain_admin,
    ));

    let genesis_block_hash = [0u8; 32];
    instructions.extend(twine_chain_instruction::initialize_genesis_batch(
        &accounts.chain_admin.pubkey(),
        genesis_block_hash,
    ));

    let batch_number = 1;
    let batch_hash = [5u8; 32];

    let total_msg_handled_on_twine: u64 = 2;

    let mut finalize_public_values = Vec::with_capacity(72);
    finalize_public_values.extend_from_slice(&genesis_block_hash);
    finalize_public_values.extend_from_slice(&batch_hash);
    finalize_public_values.extend_from_slice(&total_msg_handled_on_twine.to_be_bytes());
    finalize_public_values.extend_from_slice(&total_msg_handled_on_twine.to_be_bytes());

    instructions.extend(twine_chain_instruction::commit_and_finalize_batch(
        &accounts.chain_admin.pubkey(),
        batch_number,
        finalize_public_values.clone(),
        finalize_public_values,
    ));
  

    let transaction = Transaction::new_signed_with_payer(
        &instructions,
        Some(&context.payer.pubkey()),
        &[&context.payer, &accounts.chain_admin],
        context.last_blockhash,
    );

    let error = context.banks_client.process_transaction(transaction).await;

    println!("Transaction status: {:?}", error);
    let mut other_instructions = vec![];
    let twine_chain_storage_account = context
        .banks_client
        .get_account(derive_twine_chain_storage(&twine_chain_id()).0)
        .await
        .unwrap()
        .expect("Twine Storage Account Not Found");

    let twine_chain_storage_data: TwineChainStorage =
        TwineChainStorage::deserialize(&mut &twine_chain_storage_account.data[..])
            .expect("Failed to deserialize TwineChainStorage");
    let start_nonce = twine_chain_storage_data.last_copied_message_end_nonce + 1;
    let end_nonce = twine_chain_storage_data.last_copied_message_end_nonce + MESSAGE_NONCE_GAP;
    other_instructions.extend(tokens_gateway_instruction::forced_spl_token_withdrawal(
        &chain_admin,
        &user_token_account,
        &spl_token_pubkey,
        from_twine_address,
        l1_token.clone(),
        l2_token.clone(),
        amount,
        start_nonce,
        end_nonce,
        signature,
    ));
    other_instructions.extend(tokens_gateway_instruction::spl_token_deposit(
        &chain_admin,
        &user_token_account,
        &spl_token_pubkey,
        &spl_token_vault,
        l2_address,
        l1_token.clone(),
        l2_token.clone(),
        amount,
        start_nonce,
        end_nonce,
        hex::decode(data.clone()).unwrap(),
    ));

      other_instructions.extend(tokens_gateway_instruction::process_spl_forced_withdrawal(
        &chain_admin,
        &spl_token_pubkey,
        &spl_token_vault,
        user_token_account,
        nonce,
        public_values,
        execution_proof,
    ));

    let other_transaction = Transaction::new_signed_with_payer(
        &other_instructions,
        Some(&context.payer.pubkey()),
        &[&context.payer, &accounts.chain_admin],
        context.last_blockhash,
    );

    let second_error = context
        .banks_client
        .process_transaction(other_transaction)
        .await;

    println!("Deposit and Forced Withdrawal status: {:?}", second_error);
}
