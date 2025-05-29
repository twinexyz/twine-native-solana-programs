use super::state::{RoleType,FinalizeInputWithdrawal};
use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::program_error::ProgramError;
use solana_program::pubkey::{Pubkey};

#[derive(BorshSerialize, BorshDeserialize, Debug)]
pub enum GatewayInstruction {
    InitializeTokensGateway,
    InitializeTokensGatewayRoleManager,
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
    NativeTokenDepoist {
        receiver_twine_address: String,
        l1_token: String,
        l2_token: String,
        amount: u64,
    },
    SplTokenDepoist {
        receiver_twine_address: String,
        l1_token: String,
        l2_token: String,
        amount: u64,
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
    FinalzeNativeWithdrawal{
        withdrawal_inputs: FinalizeInputWithdrawal,
    },
    FinalizeSplWithdrawal{
        withdrawal_inputs: FinalizeInputWithdrawal,
    },
}
#[derive(BorshDeserialize)]
struct UpdateTokenMappingPayload {
    l1_token: String,
    l2_token: String,
    l1_decimals: u8,
    l2_decimals: u8,
}
#[derive(BorshDeserialize)]
struct SetGatewayRoleChainAdminPayload {
    new_admin: Pubkey,
}
#[derive(BorshDeserialize)]
struct AddRoleInGatewayPayload {
    address: Pubkey,
    role: RoleType,
}

#[derive(BorshDeserialize)]
struct RemoveRoleInGatewayPayload {
    address: Pubkey,
    role: RoleType,
}

#[derive(BorshDeserialize)]
struct NativeTokenDepoistPayload {
    receiver_twine_address: String,
    l1_token: String,
    l2_token: String,
    amount: u64,
}

#[derive(BorshDeserialize)]
struct SplTokenDepoistPayload {
    receiver_twine_address: String,
    l1_token: String,
    l2_token: String,
    amount: u64,
}

#[derive(BorshDeserialize)]
struct NativeTokenForcedWithdrawalPayload {
    from_twine_address: String,
    to_l1_pubkey: String,
    l1_token: String,
    l2_token: String,
    amount: u64,
    signature: Vec<u8>,
}

#[derive(BorshDeserialize)]
struct SplTokenForcedWithdrawalPayload {
    from_twine_address: String,
    to_l1_pubkey: String,
    l1_token: String,
    l2_token: String,
    amount: u64,
    signature: Vec<u8>,
}


#[derive(BorshDeserialize)]
struct FinalizeNativeWithdrawalPayload{
    withdrawal_inputs: FinalizeInputWithdrawal,
}

#[derive(BorshDeserialize)]
struct FinalizeSplWithdrawalPayload{
    withdrawal_inputs: FinalizeInputWithdrawal,
}

impl GatewayInstruction {
    pub fn unpack_instruction(input: &[u8]) -> Result<Self, ProgramError> {
        let (&discriminator, rest) = input
            .split_first()
            .ok_or(ProgramError::InvalidInstructionData)?;
        match discriminator {
            0 => Ok(Self::InitializeTokensGateway),
            1 => Ok(Self::InitializeTokensGatewayRoleManager),
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
                let payload = NativeTokenDepoistPayload::try_from_slice(rest)
                    .map_err(|_| ProgramError::InvalidInstructionData)?;
                Ok(Self::NativeTokenDepoist {
                    receiver_twine_address: payload.receiver_twine_address,
                    l1_token: payload.l1_token,
                    l2_token: payload.l2_token,
                    amount: payload.amount,
                })
            }

            7 => {
                let payload = SplTokenDepoistPayload::try_from_slice(rest)
                    .map_err(|_| ProgramError::InvalidInstructionData)?;
                Ok(Self::SplTokenDepoist {
                    receiver_twine_address: payload.receiver_twine_address,
                    l1_token: payload.l1_token,
                    l2_token: payload.l2_token,
                    amount: payload.amount,
                })
            }

            8 => {
                let payload = SplTokenDepoistPayload::try_from_slice(rest)
                    .map_err(|_| ProgramError::InvalidInstructionData)?;
                Ok(Self::SplTokenDepoist {
                    receiver_twine_address: payload.receiver_twine_address,
                    l1_token: payload.l1_token,
                    l2_token: payload.l2_token,
                    amount: payload.amount,
                })
            }

            9 => {
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
            10 => {
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

            11 => {
                let payload = FinalizeNativeWithdrawalPayload::try_from_slice(rest)
                    .map_err(|_| ProgramError::InvalidInstructionData)?;
                Ok(Self::FinalzeNativeWithdrawal {
                    withdrawal_inputs:payload.withdrawal_inputs,
                })
            }
            12 => {
                let payload = FinalizeSplWithdrawalPayload::try_from_slice(rest)
                    .map_err(|_| ProgramError::InvalidInstructionData)?;
                Ok(Self::FinalizeSplWithdrawal {
                    withdrawal_inputs:payload.withdrawal_inputs,
                })
            }
            _ => Err(ProgramError::InvalidInstructionData),
        }
    }
}
