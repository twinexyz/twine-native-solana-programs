use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::{
    account_info::AccountInfo,
    instruction::{AccountMeta, Instruction},
    msg,
    program_error::ProgramError,
    pubkey::Pubkey,
    system_program,
};
use twine_chain::{
    core::state::TwineChainStorage,
    utils::{
        address_derivation::{
            derive_detailed_messages_buffer, derive_execution_message_buffer,
            derive_messages_buffer, derive_messages_replicator, derive_twine_chain_role_manager,
            derive_twine_chain_storage,
        },
        constants::MESSAGE_NONCE_GAP,
    },
    ID as twine_chain_id,
};

use super::state::RoleType;

use crate::{
    utils::{
        address_derivation::{
            derive_executed_payouts_pda, derive_executed_withdrawals_pda,
            derive_gateway_role_manager, derive_native_token_vault, derive_native_token_vault_data,
            derive_spl_tokens_vault_data, derive_spl_vault_authority,
            derive_token_decimal_mappings,
        },
        batch_range_provider::batch_range_provider,
    },
    ID as tokens_gateway_ID,
};

#[derive(BorshSerialize, BorshDeserialize, Debug)]
pub enum GatewayInstruction {
    InitializeTokensGatewayRoleManager,
    InitializeTokensGateway,
    UpdateTokenMapping {
        l1_token: String,
        l2_token: String,
        l1_decimals: u8,
        l2_decimals: u8,
    },
    SetGatewayRoleChainAdmin {
        new_admin: Pubkey,
    },
    AddRoleInGateway {
        address: Pubkey,
        role: RoleType,
    },
    RemoveRoleInGateway {
        address: Pubkey,
        role: RoleType,
    },
    NativeTokenDeposit {
        receiver_twine_address: String,
        l1_token: String,
        l2_token: String,
        amount: u64,
        data: Vec<u8>,
    },
    SplTokenDeposit {
        receiver_twine_address: String,
        l1_token: String,
        l2_token: String,
        amount: u64,
        data: Vec<u8>,
    },
    NativeTokenForcedWithdrawal {
        from_twine_address: String,
        to_l1_pubkey: String,
        l1_token: String,
        l2_token: String,
        amount: u64,
        signature: Vec<u8>,
    },
    SplTokenForcedWithdrawal {
        from_twine_address: String,
        to_l1_pubkey: String,
        l1_token: String,
        l2_token: String,
        amount: u64,
        signature: Vec<u8>,
    },
    ExecuteL2NativeWithdrawal {
        public_values: Vec<u8>,
        execution_proof: Vec<u8>,
    },
    ExecuteL2SplWithdrawal {
        public_values: Vec<u8>,
        execution_proof: Vec<u8>,
    },
    ProcessNativeRefund {
        public_values: Vec<u8>,
        execution_proof: Vec<u8>,
    },
    ProcessSplRefund {
        public_values: Vec<u8>,
        execution_proof: Vec<u8>,
    },
    ProcessNativeForcedWithdrawal {
        public_values: Vec<u8>,
        execution_proof: Vec<u8>,
    },
    ProcessSplForcedWithdrawal {
        public_values: Vec<u8>,
        execution_proof: Vec<u8>,
    },
    RemoveTokenMapping {
        l1_token: String,
        l2_token: String,
    },
}
#[derive(BorshSerialize, BorshDeserialize)]
struct UpdateTokenMappingPayload {
    l1_token: String,
    l2_token: String,
    l1_decimals: u8,
    l2_decimals: u8,
}

#[derive(BorshSerialize, BorshDeserialize)]
struct RemoveTokenMappingPayload {
    l1_token: String,
    l2_token: String,
}
#[derive(BorshSerialize, BorshDeserialize)]
struct SetGatewayRoleChainAdminPayload {
    new_admin: Pubkey,
}
#[derive(BorshSerialize, BorshDeserialize)]
struct AddRoleInGatewayPayload {
    address: Pubkey,
    role: RoleType,
}

#[derive(BorshSerialize, BorshDeserialize)]
struct RemoveRoleInGatewayPayload {
    address: Pubkey,
    role: RoleType,
}

#[derive(BorshSerialize, BorshDeserialize)]
struct NativeTokenDepositPayload {
    receiver_twine_address: String,
    l1_token: String,
    l2_token: String,
    amount: u64,
    data: Vec<u8>,
}

#[derive(BorshSerialize, BorshDeserialize)]
struct SplTokenDepositPayload {
    receiver_twine_address: String,
    l1_token: String,
    l2_token: String,
    amount: u64,
    data: Vec<u8>,
}

#[derive(BorshSerialize, BorshDeserialize)]
struct NativeTokenForcedWithdrawalPayload {
    from_twine_address: String,
    to_l1_pubkey: String,
    l1_token: String,
    l2_token: String,
    amount: u64,
    signature: Vec<u8>,
}

#[derive(BorshSerialize, BorshDeserialize)]
struct SplTokenForcedWithdrawalPayload {
    from_twine_address: String,
    to_l1_pubkey: String,
    l1_token: String,
    l2_token: String,
    amount: u64,
    signature: Vec<u8>,
}

#[derive(BorshSerialize, BorshDeserialize)]
struct ExecuteL2NativeWithdrawalPayload {
    public_values: Vec<u8>,
    execution_proof: Vec<u8>,
}

#[derive(BorshSerialize, BorshDeserialize)]
struct ExecuteL2SplWithdrawalPayload {
    public_values: Vec<u8>,
    execution_proof: Vec<u8>,
}

#[derive(BorshSerialize, BorshDeserialize)]
struct ProcessNativeRefund {
    public_values: Vec<u8>,
    execution_proof: Vec<u8>,
}

#[derive(BorshSerialize, BorshDeserialize)]
struct ProcessSplRefund {
    public_values: Vec<u8>,
    execution_proof: Vec<u8>,
}
#[derive(BorshSerialize, BorshDeserialize)]
struct ProcessNativeForcedWithdrawal {
    public_values: Vec<u8>,
    execution_proof: Vec<u8>,
}

#[derive(BorshSerialize, BorshDeserialize)]
struct ProcessSplForcedWithdrawal {
    public_values: Vec<u8>,
    execution_proof: Vec<u8>,
}


impl GatewayInstruction {
    pub fn unpack_instruction(input: &[u8]) -> Result<Self, ProgramError> {
        let (&discriminator, rest) = input
            .split_first()
            .ok_or(ProgramError::InvalidInstructionData)?;
        match discriminator {
            0 => Ok(Self::InitializeTokensGatewayRoleManager),
            1 => Ok(Self::InitializeTokensGateway),
            2 => {
                let payload = UpdateTokenMappingPayload::try_from_slice(rest)
                    .map_err(|_| ProgramError::InvalidInstructionData)?;
                Ok(Self::UpdateTokenMapping {
                    l1_token: payload.l1_token,
                    l2_token: payload.l2_token,
                    l1_decimals: payload.l1_decimals,
                    l2_decimals: payload.l2_decimals,
                })
            }
            3 => {
                let payload = SetGatewayRoleChainAdminPayload::try_from_slice(rest)
                    .map_err(|_| ProgramError::InvalidInstructionData)?;
                Ok(Self::SetGatewayRoleChainAdmin {
                    new_admin: payload.new_admin,
                })
            }

            4 => {
                let payload = AddRoleInGatewayPayload::try_from_slice(rest)
                    .map_err(|_| ProgramError::InvalidInstructionData)?;
                Ok(Self::AddRoleInGateway {
                    address: payload.address,
                    role: payload.role,
                })
            }
            5 => {
                let payload = RemoveRoleInGatewayPayload::try_from_slice(rest)
                    .map_err(|_| ProgramError::InvalidInstructionData)?;
                Ok(Self::RemoveRoleInGateway {
                    address: payload.address,
                    role: payload.role,
                })
            }
            6 => {
                let payload = NativeTokenDepositPayload::try_from_slice(rest)
                    .map_err(|_| ProgramError::InvalidInstructionData)?;
                Ok(Self::NativeTokenDeposit {
                    receiver_twine_address: payload.receiver_twine_address,
                    l1_token: payload.l1_token,
                    l2_token: payload.l2_token,
                    amount: payload.amount,
                    data: payload.data,
                })
            }

            7 => {
                let payload = SplTokenDepositPayload::try_from_slice(rest)
                    .map_err(|_| ProgramError::InvalidInstructionData)?;
                Ok(Self::SplTokenDeposit {
                    receiver_twine_address: payload.receiver_twine_address,
                    l1_token: payload.l1_token,
                    l2_token: payload.l2_token,
                    amount: payload.amount,
                    data: payload.data,
                })
            }

            8 => {
                let payload = NativeTokenForcedWithdrawalPayload::try_from_slice(rest)
                    .map_err(|_| ProgramError::InvalidInstructionData)?;
                Ok(Self::NativeTokenForcedWithdrawal {
                    from_twine_address: payload.from_twine_address,
                    to_l1_pubkey: payload.to_l1_pubkey,
                    l1_token: payload.l1_token,
                    l2_token: payload.l2_token,
                    amount: payload.amount,
                    signature: payload.signature,
                })
            }
            9 => {
                let payload = SplTokenForcedWithdrawalPayload::try_from_slice(rest)
                    .map_err(|_| ProgramError::InvalidInstructionData)?;
                Ok(Self::SplTokenForcedWithdrawal {
                    from_twine_address: payload.from_twine_address,
                    to_l1_pubkey: payload.to_l1_pubkey,
                    l1_token: payload.l1_token,
                    l2_token: payload.l2_token,
                    amount: payload.amount,
                    signature: payload.signature,
                })
            }
            10 => {
                let payload = ExecuteL2NativeWithdrawalPayload::try_from_slice(rest)
                    .map_err(|_| ProgramError::InvalidInstructionData)?;
                Ok(Self::ExecuteL2NativeWithdrawal {
                    public_values: payload.public_values,
                    execution_proof: payload.execution_proof,
                })
            }
            11 => {
                let payload = ExecuteL2SplWithdrawalPayload::try_from_slice(rest)
                    .map_err(|_| ProgramError::InvalidInstructionData)?;
                Ok(Self::ExecuteL2SplWithdrawal {
                    public_values: payload.public_values,
                    execution_proof: payload.execution_proof,
                })
            }
            12 => {
                let payload = ProcessNativeRefund::try_from_slice(rest)
                    .map_err(|_| ProgramError::InvalidInstructionData)?;
                Ok(Self::ProcessNativeRefund {
                    public_values: payload.public_values,
                    execution_proof: payload.execution_proof,
                })
            }
            13 => {
                let payload = ProcessSplRefund::try_from_slice(rest)
                    .map_err(|_| ProgramError::InvalidInstructionData)?;
                Ok(Self::ProcessSplRefund {
                    public_values: payload.public_values,
                    execution_proof: payload.execution_proof,
                })
            }
            14 => {
                let payload = ProcessNativeForcedWithdrawal::try_from_slice(rest)
                    .map_err(|_| ProgramError::InvalidInstructionData)?;
                Ok(Self::ProcessNativeForcedWithdrawal {
                    public_values: payload.public_values,
                    execution_proof: payload.execution_proof,
                })
            }
            15 => {
                let payload = ProcessSplForcedWithdrawal::try_from_slice(rest)
                    .map_err(|_| ProgramError::InvalidInstructionData)?;
                Ok(Self::ProcessSplForcedWithdrawal {
                    public_values: payload.public_values,
                    execution_proof: payload.execution_proof,
                })
            }
            16 => {
                let payload = RemoveTokenMappingPayload::try_from_slice(rest)
                    .map_err(|_| ProgramError::InvalidInstructionData)?;
                Ok(Self::RemoveTokenMapping {
                    l1_token: payload.l1_token,
                    l2_token: payload.l2_token,
                })
            }
            _ => Err(ProgramError::InvalidInstructionData),
        }
    }
}
