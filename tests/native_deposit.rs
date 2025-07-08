#[cfg(test)]
mod helpers;

use borsh::BorshDeserialize;
use solana_program_test::*;
use solana_sdk::{
    signature::{Keypair, Signer},
    transaction::Transaction,
};

use helpers::tokens_gateway_helper::{
    fund_account_for_rent_exemption, program_test, TokensGatewayAccounts,
};
use tokens_gateway::{
    core::instruction as tokens_gateway_instruction,
    id as tokens_gateway_id,
    utils::address_derivation::{derive_native_token_vault_data},
    utils::constants::ROLE_MANAGER_ACCOUNT_SIZE,
};
use twine_chain::{
    core::{
        instruction as twine_chain_instruction,
        state::{DepositMessagesBuffer, RoleType},
    },
    id as twine_chain_id,
    utils::address_derivation::derive_deposit_message_buffer,
};

#[tokio::test]
async fn native_token_deposit_succeed() {
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
    let l1_token = "11111111111111111111111111111111".to_string();
    let l2_token = "0x1234567890abcdef1234567890abcdef12345678".to_string();
    let l1_decimals = 9u8;
    let l2_decimals = 18u8;
    let amount = 1u64;
    let chain_admin = &accounts.chain_admin.pubkey();
    let receiver_twine_address = "0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266".to_string();
    let data = "".to_string();
    let native_token_valut_data_account = derive_native_token_vault_data(&tokens_gateway_id()).0;

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
        receiver_twine_address,
        l1_token.clone(),
        l2_token.clone(),
        amount,
        data,
    ));

    let transaction = Transaction::new_signed_with_payer(
        &instructions,
        Some(&context.payer.pubkey()),
        &[&context.payer, &accounts.chain_admin],
        context.last_blockhash,
    );

    let error = context.banks_client.process_transaction(transaction).await;

    println!("Transaction status: {:?}", error);
    let deposit_message_buffer_account = context
        .banks_client
        .get_account(derive_deposit_message_buffer(&twine_chain_id()).0)
        .await
        .unwrap()
        .expect("Deposit Message Account Not Found");
    let deposit_message_buffer_data: DepositMessagesBuffer =
        DepositMessagesBuffer::deserialize(&mut &deposit_message_buffer_account.data[..])
            .expect("Failed to deserialize Deposit Message Buffer Data");
    assert!(
        deposit_message_buffer_data.deposit_nonce == 1,
        "Deposit not successfull"
    );
}
