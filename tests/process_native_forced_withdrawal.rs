#[cfg(test)]
mod helpers;
use sha3::{Digest, Keccak256};
use solana_program_test::*;
use solana_sdk::{
    msg,
    signature::{Keypair, Signer},
    transaction::Transaction,
};

use helpers::tokens_gateway_helper::{
    fund_account_for_rent_exemption, program_test,get_ethereum_signature, TokensGatewayAccounts,
};
use tokens_gateway::{
    core::{instruction as tokens_gateway_instruction,state::SignMessageInfo},
    id,
    utils::{
        address_derivation::derive_native_token_vault_data,
        constants::{CHAIN_ID, ROLE_MANAGER_ACCOUNT_SIZE},
    },
};
use twine_chain::core::state::TransactionType;
use twine_chain::core::{instruction as twine_chain_instruction, state::RoleType};

#[tokio::test]
async fn process_native_forced_withdrawal() {
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
    let receiver_twine_address = "0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266".to_string();
    let l2_amount = "50000000000000000000";
    let slot_number = 1u64;
    let chain_admin = &accounts.chain_admin.pubkey();
    let batch_number = 1u64;
    let nonce = 1u64;
    let batch_hash = [1u8; 32];
    let l1_address = accounts.chain_admin.pubkey();
    let l2_address = "0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266".to_string();
    let l1_token = "11111111111111111111111111111111".to_string();
    let l2_token = "0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266".to_string();
    let data = "".to_string();
    let execution_proof = vec![];
    let mut public_values = vec![];
    let native_token_valut_data_account = derive_native_token_vault_data(&id()).0;
    let sign_info = SignMessageInfo {
        nonce: 1,
        chain_id: 900,
        amount: amount,
        from_twine_address: l2_address.to_string(),
        to_l1_pubkey: chain_admin.to_string(),
        l1_token: l1_token.to_string(),
        l2_token: l2_token.clone(),
    };
    let privkey = "0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80";
        let signature = get_ethereum_signature(&sign_info, privkey);

    public_values.extend_from_slice(&batch_hash);
    public_values.extend_from_slice(&batch_number.to_be_bytes());
    public_values.extend_from_slice(&txn_type.as_bytes());
    public_values.extend_from_slice(&nonce.to_be_bytes());
    public_values.extend_from_slice(&CHAIN_ID.to_be_bytes());
    public_values.extend_from_slice(&slot_number.to_be_bytes());
    public_values.extend_from_slice(&Keccak256::digest(&data.as_bytes()));
    public_values.extend_from_slice(&l1_address.to_string().to_lowercase().as_bytes());
    public_values.extend_from_slice(&l2_address.to_lowercase().as_bytes());
    public_values.extend_from_slice(&l1_token.to_lowercase().as_bytes());
    public_values.extend_from_slice(&l2_token.to_lowercase().as_bytes());
    public_values.extend_from_slice(&l2_amount.to_string().as_bytes());

    println!("The public values{:?}", public_values);
    msg!("The hex values {:?}", hex::encode(&public_values));

    let mut instructions = vec![];
    instructions.extend(twine_chain_instruction::initialize_twine_chain_role_manager(&chain_admin));
    instructions.extend(twine_chain_instruction::initialize_twine_chain_storage(
        &chain_admin,
    ));
    instructions.extend(twine_chain_instruction::initialize_message_buffer(
        &chain_admin,
    ));
    instructions.extend(twine_chain_instruction::add_role_in_twine_chain(
        &chain_admin,
        &native_token_valut_data_account,
        RoleType::MessageAppender,
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

    instructions.extend(tokens_gateway_instruction::forced_native_token_withdrawal(
        &chain_admin,
        l2_address.clone(),
        chain_admin.to_string(),
        l1_token.clone(),
        l2_token.clone(),
        amount,
        signature,
    ));

    instructions.extend(tokens_gateway_instruction::native_token_deposit(
        &chain_admin,
        receiver_twine_address.clone(),
        l1_token.clone(),
        l2_token.clone(),
        amount,
        hex::decode(data.clone()).unwrap(),
    ));

    let genesis_block_hash = [0u8; 32];
    instructions.extend(twine_chain_instruction::initialize_genesis_batch(
        &accounts.chain_admin.pubkey(),
        genesis_block_hash,
    ));
    let batch_number = 1;
    let batch_hash = [5u8; 32];

    instructions.extend(twine_chain_instruction::commit_batch(
        &accounts.chain_admin.pubkey(),
        batch_number,
        batch_hash,
    ));
    let total_msg_handled_on_twine: u64 = 2;

    let mut finalize_public_values = Vec::with_capacity(72);
    finalize_public_values.extend_from_slice(&genesis_block_hash);
    finalize_public_values.extend_from_slice(&batch_hash);
    finalize_public_values.extend_from_slice(&total_msg_handled_on_twine.to_be_bytes());
    finalize_public_values.extend_from_slice(&total_msg_handled_on_twine.to_be_bytes());

    instructions.extend(twine_chain_instruction::finalize_batch(
        &accounts.chain_admin.pubkey(),
        batch_number,
        finalize_public_values.clone(),
        finalize_public_values,
    ));

    instructions.extend(tokens_gateway_instruction::process_native_forced_withdrawal(
        l1_address,
        nonce,
        public_values,
        execution_proof,
    ));

    let transaction = Transaction::new_signed_with_payer(
        &instructions,
        Some(&context.payer.pubkey()),
        &[&context.payer, &accounts.chain_admin],
        context.last_blockhash,
    );

    let error = context.banks_client.process_transaction(transaction).await;

    println!("Transaction status: {:?}", error);
}
