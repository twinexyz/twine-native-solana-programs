mod helpers;
use borsh::BorshDeserialize;
use solana_program_test::*;
use solana_sdk::{
    signature::{Keypair, Signer},
    transaction::Transaction,
};

use helpers::tokens_gateway_helper::{
    create_spl_and_mint, fund_account_for_rent_exemption, get_or_create_ata, program_test,
    TokensGatewayAccounts,
};
use tokens_gateway::{
    core::instruction as tokens_gateway_instruction,
    utils::{
        address_derivation::{derive_spl_vault_authority,derive_spl_tokens_vault_data},
        constants::ROLE_MANAGER_ACCOUNT_SIZE,
    },
    id as tokens_gateway_id,
};
use twine_chain::{
    core::{
        instruction as twine_chain_instruction,
        state::{MessagesBuffer, RoleType},
    },
    id as twine_chain_id,
    utils::address_derivation::derive_messages_buffer,
};


#[tokio::test]
async fn spl_token_deposit_succeed() {
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
    let l2_token = "0x1234567890abcdef1234567890abcdef12345678".to_string();
    let l1_decimals = 9u8;
    let l2_decimals = 18u8;
    let amount = 2u64;
    let chain_admin = &accounts.chain_admin.pubkey();
    let receiver_twine_address = "0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266".to_string();
    let (spl_token_pubkey, user_token_account) =
        create_spl_and_mint(&mut context, &accounts.chain_admin, 9, 100).await;
    let spl_token_vault = get_or_create_ata(
        &mut context,
        &accounts.chain_admin,
        &derive_spl_vault_authority(&tokens_gateway_id()).0,
        &spl_token_pubkey,
    )
    .await;
    let l1_token = spl_token_pubkey.to_string();
    let data = "".to_string();
    let spl_token_valut_data_account = derive_spl_tokens_vault_data(&tokens_gateway_id()).0;

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
    instructions.extend(tokens_gateway_instruction::spl_token_deposit(
        &chain_admin,
        &user_token_account,
        &spl_token_pubkey,
        &spl_token_vault,
        receiver_twine_address,
        l1_token.clone(),
        l2_token.clone(),
        amount,
        data
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
        .get_account(derive_messages_buffer(&twine_chain_id()).0)
        .await
        .unwrap()
        .expect("Deposit Message Account Not Found");
    let deposit_message_buffer_data: MessagesBuffer =
        MessagesBuffer::deserialize(&mut &deposit_message_buffer_account.data[..])
            .expect("Failed to deserialize Deposit Message Buffer Data");
    assert!(
        deposit_message_buffer_data.message_nonce == 1,
        "Deposit not successfull"
    );
}
