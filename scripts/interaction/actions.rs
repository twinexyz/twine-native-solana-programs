use crate::action_commands::Commands;
use crate::operations::oapp_init_config::init_config;
use crate::operations::oapp_init_nonce::init_nonce;
use crate::operations::oapp_init_receive_library::init_receive_library;
use crate::operations::oapp_init_send_library::init_send_library;
use crate::operations::oapp_initialize_store::initialize_store;
use crate::operations::oapp_set_config::set_config;
use crate::operations::oapp_set_send_library::set_send_library;
use crate::operations::{
    copy_messages_buffer::copy_messages_buffer,
    create_spl_token::create_spl_token,
    deposit_native_token::native_token_deposit,
    deposit_spl_token::spl_token_deposit,
    execute_native_l2_withdrawal::execute_native_l2_withdrawal,
    execute_spl_l2_withdrawal::execute_spl_l2_withdrawal,
    forced_native_withdrawal::forced_native_withdrawal,
    forced_spl_withdrawal::forced_spl_withdrawal,
    get_all_pdas::{get_all_pdas, get_batch_pda},
    get_associated_token_account::get_associated_token_account,
    get_pdas_data::{
        get_detailed_messages_buffer_data, get_message_replicator_data, get_messages_buffer_data,
        get_tokens_gateway_role_manager_data, get_tokens_mapping_data,
        get_twine_chain_role_manager_data, get_twine_chain_storage_data,
    },
    initialize_programs::initialize_twine_solana_programs,
    process_native_l1_forced_withdrawal::process_native_l1_forced_withdrawal,
    process_native_l1_refund::process_native_l1_refund,
    process_spl_l1_forced_withdrawal::process_spl_l1_forced_withdrawal,
    process_spl_l1_refund::process_spl_l1_refund,
    role_operations_tokens_gateway::{add_role_in_tokens_gateway, remove_role_in_tokens_gateway},
    role_operations_twine_chain::{add_role_in_twine_chain, remove_role_in_twine_chain},
    token_mapping::{remove_token_mapping, update_token_mapping},
};

pub fn handle_command(command: Commands) -> anyhow::Result<()> {
    match command {
        Commands::InitializePrograms {} => {
            let result = initialize_twine_solana_programs();
            println!("Result: {:?}", result);
        }
        Commands::CreateSplToken {} => {
            let result = create_spl_token();
            println!("Result: {:?}", result);
        }
        Commands::AddRoleInTwineChain {
            role_type,
            user_pubkey,
        } => {
            let result = add_role_in_twine_chain(role_type, user_pubkey);
            println!("Result: {:?}", result);
        }
        Commands::RemoveRoleInTwineChain {
            role_type,
            user_pubkey,
        } => {
            let result = remove_role_in_twine_chain(role_type, user_pubkey);
            println!("Result: {:?}", result);
        }
        Commands::AddRoleInTokensGateway {
            role_type,
            user_pubkey,
        } => {
            let result = add_role_in_tokens_gateway(role_type, user_pubkey);
            println!("Result: {:?}", result);
        }
        Commands::RemoveRoleInTokensGateway {
            role_type,
            user_pubkey,
        } => {
            let result = remove_role_in_tokens_gateway(role_type, user_pubkey);
            println!("Result: {:?}", result);
        }
        Commands::GetAssociatedTokenAccount {
            wallet_address,
            spl_token_pubkey,
        } => {
            let result = get_associated_token_account(wallet_address, spl_token_pubkey);
            println!("Result: {:?}", result);
        }
        Commands::GetAllPdas {} => {
            let result = get_all_pdas();
            println!("Result: {:?}", result);
        }
        Commands::GetBatchPda { batch_number } => {
            let result = get_batch_pda(batch_number);
            println!("Result: {:?}", result);
        }
        Commands::GetMessagesBufferData {} => {
            let result = get_messages_buffer_data();
            println!("Result: {:?}", result);
        }
        Commands::GetDetailedMessagesBufferData {} => {
            let result = get_detailed_messages_buffer_data();
            println!("Result: {:?}", result);
        }
        Commands::GetTokensMappingData {} => {
            let result = get_tokens_mapping_data();
            println!("Result: {:?}", result);
        }
        Commands::GetTwineChainStorageData {} => {
            let result = get_twine_chain_storage_data();
            println!("Result: {:?}", result);
        }
        Commands::GetTwineChainRoleManagerData {} => {
            let result = get_twine_chain_role_manager_data();
            println!("Result: {:?}", result);
        }
        Commands::GetTokensGatewayRoleManagerData {} => {
            let result = get_tokens_gateway_role_manager_data();
            println!("Result: {:?}", result);
        }
        Commands::GetMessageReplicatorData {
            start_nonce,
            end_nonce,
        } => {
            let result = get_message_replicator_data(start_nonce, end_nonce);
            println!("Result: {:?}", result);
        }
        Commands::CopyMessagesBuffer {
            start_nonce,
            end_nonce,
        } => {
            let result = copy_messages_buffer(start_nonce, end_nonce);
            println!("Result: {:?}", result);
        }
        Commands::TokenMapping {
            l1_token,
            l2_token,
            l1_decimals,
            l2_decimals,
        } => {
            let result = update_token_mapping(l1_token, l2_token, l1_decimals, l2_decimals);
            println!("Result: {:?}", result);
        }
        Commands::RemoveTokenMapping { l1_token, l2_token } => {
            let result = remove_token_mapping(l1_token, l2_token);
            println!("Result: {:?}", result);
        }

        Commands::DepositNativeToken {
            l1_token,
            l2_token,
            receiver_twine_address,
            amount,
            data,
        } => {
            let result =
                native_token_deposit(l1_token, l2_token, receiver_twine_address, amount, data);
            println!("Result: {:?}", result);
        }
        Commands::DepositSplToken {
            l1_token,
            l2_token,
            receiver_twine_address,
            user_token_account,
            amount,
            data,
        } => {
            let result = spl_token_deposit(
                l1_token,
                l2_token,
                receiver_twine_address,
                user_token_account,
                amount,
                data,
            );
            println!("Result: {:?}", result);
        }
        Commands::ForcedNativeWithdrawal {
            l1_token,
            l2_token,
            from_twine_address,
            l1_receiver,
            privkey,
            amount,
        } => {
            let result = forced_native_withdrawal(
                l1_token,
                l2_token,
                from_twine_address,
                l1_receiver,
                privkey,
                amount,
            );
            println!("Result: {:?}", result);
        }
        Commands::ForcedSplWithdrawal {
            l1_token,
            l2_token,
            from_twine_address,
            privkey,
            user_token_account,
            amount,
        } => {
            let result = forced_spl_withdrawal(
                l1_token,
                l2_token,
                from_twine_address,
                privkey,
                user_token_account,
                amount,
            );
            println!("Result: {:?}", result);
        }
        Commands::ExecuteNativeL2Withdrawal {
            receiver,
            public_values,
            proof,
        } => match execute_native_l2_withdrawal(receiver, public_values, proof) {
            Ok(_) => {
                println!("L2 originated withdrawal successful");
            }
            Err(e) => {
                eprintln!("Error: {:?}", e);
            }
        },
        Commands::ExecuteSplL2Withdrawal {
            spl_token_pubkey,
            l1_receiver_address,
            public_values,
            execution_proof,
        } => match execute_spl_l2_withdrawal(
            spl_token_pubkey,
            l1_receiver_address,
            public_values,
            execution_proof,
        ) {
            Ok(_) => {
                println!("L2 originated withdrawal successful");
            }
            Err(e) => {
                eprintln!("Error: {:?}", e);
            }
        },
        Commands::ProcessNativeRefund {
            receiver,
            public_values,
            proof,
        } => match process_native_l1_refund(receiver, public_values, proof) {
            Ok(_) => {
                println!("refund successfull");
            }
            Err(e) => {
                eprintln!("Error: {:?}", e);
            }
        },
        Commands::ProcessSplRefund {
            l1_token,
            l1_receiver_address,
            public_values,
            proof,
        } => match process_spl_l1_refund(l1_token, l1_receiver_address, public_values, proof) {
            Ok(_) => {
                println!("refund successfull");
            }
            Err(e) => {
                eprintln!("Error: {:?}", e);
            }
        },
        Commands::ProcessNativeForcedWithdrawal {
            receiver,
            public_values,
            proof,
        } => match process_native_l1_forced_withdrawal(receiver, public_values, proof) {
            Ok(_) => {
                println!("forced withdrawal successfull");
            }
            Err(e) => {
                eprintln!("Error: {:?}", e);
            }
        },
        Commands::ProcessSplForcedWithdrawal {
            l1_token,
            l1_receiver_address,
            public_values,
            proof,
        } => match process_spl_l1_forced_withdrawal(
            l1_token,
            l1_receiver_address,
            public_values,
            proof,
        ) {
            Ok(_) => {
                println!("refund successfull");
            }
            Err(e) => {
                eprintln!("Error: {:?}", e);
            }
        },
        Commands::InitializeStore {} => {
            let result = initialize_store();
            println!("Result: {:?}", result);
        }
        Commands::InitSendLibrary {} => {
            let result = init_send_library();
            println!("Result: {:?}", result);
        }
        Commands::InitReceiveLibrary {} => {
            let result = init_receive_library();
            println!("Result: {:?}", result);
        }
        Commands::InitNonce { remote_oapp } => {
            let result = init_nonce(remote_oapp);
            println!("Result: {:?}", result);
        }
        Commands::InitConfig {} => {
            let result = init_config();
            println!("Result: {:?}", result);
        }
        Commands::SetSendLibrary {} => {
            let result = set_send_library();
            println!("Result: {:?}", result);
        }
        Commands::SetConfig {} => {
            let result = set_config();
            println!("Result: {:?}", result);
        }
    }
    Ok(())
}
