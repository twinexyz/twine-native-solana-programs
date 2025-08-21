#[cfg(test)]
mod helpers;
use solana_program_test::*;
use solana_sdk::{
    msg, signature::{Keypair, Signer}, transaction::Transaction
};

use helpers::tokens_gateway_helper::{
    fund_account_for_rent_exemption, program_test, TokensGatewayAccounts,
};
use tokens_gateway::{
    core::{instruction as tokens_gateway_instruction,},
    id,
    utils::{
        address_derivation::derive_native_token_vault_data, constants::ROLE_MANAGER_ACCOUNT_SIZE,
    },
};
use twine_chain::core::{instruction as twine_chain_instruction, state::RoleType};

#[tokio::test]
async fn l2_native_withdrawal_finalized_succeed() {
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
    

    let l1_decimals = 9u8;
    let l2_decimals = 18u8;
    let amount = 50000000000u64;
    let l1_amount = "50000000000000000000";
    let chain_admin = &accounts.chain_admin.pubkey();
    let receiver_twine_address = "0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266".to_string();
    let batch_number = 1u64;
    let nonce = 12u64;
    let batch_hash = [1u8; 32];
    let l1_receiver_address = accounts.chain_admin.pubkey();
    let l1_token = "11111111111111111111111111111111".to_string();
    let l2_token = "0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266".to_string();
    let execution_proof = vec![];
    let mut public_values = vec![];
    let native_token_valut_data_account = derive_native_token_vault_data(&id()).0;

    public_values.extend_from_slice(&batch_number.to_be_bytes());
    public_values.extend_from_slice(&nonce.to_be_bytes());
    public_values.extend_from_slice(&batch_hash);
    public_values.extend_from_slice(&l1_receiver_address.to_string().as_bytes());
    public_values.extend_from_slice(&l1_token.as_bytes());
    public_values.extend_from_slice(&l2_token.as_bytes());
    public_values.extend_from_slice(&l1_amount.to_string().as_bytes());

    println!("The public values{:?}",public_values);
    msg!("The hex values {:?}",hex::encode(&public_values));


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
    instructions.extend(tokens_gateway_instruction::native_token_deposit(
        &chain_admin,
        receiver_twine_address.clone(),
        l1_token.clone(),
        l2_token.clone(),
        amount,
        "".to_string(),
    ));
    instructions.extend(tokens_gateway_instruction::execute_l2_native_withdrawal(
        l1_receiver_address,
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
