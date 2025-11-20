use solana_program::program_error::ProgramError;
use thiserror::Error;

#[derive(Error, Debug, Copy, Clone)]
pub enum ProgramCustomError {
    #[error("Invalid Token")]
    InvalidToken,
    #[error("Invalid Amount")]
    InvalidAmount,
    #[error("Invalid Length")]
    InvalidIndex,
    #[error("Invalid Account")]
    InvalidAccount,
    #[error("Invalid Receiver")]
    InvalidReceiver,
    #[error("Invalid L1 Token")]
    InvalidL1Token,
    #[error("Invalid BatchNumber")]
    InvalidBatchNumber,
    #[error("Invalid PDA derived")]
    InvalidPDA,
    #[error("Invalid Instruction")]
    InvalidInstructionData,
    #[error("Invalid Transaction ")]
    InvalidTransaction,
    #[error("Invalid Token Account")]
    InvalidTokenAccount,
    #[error("Invalid address provided")]
    InvalidAddress,
    #[error("Receiver account not found")]
    ReceiverAccountNotFound,
    #[error("Withdraw is already executed")]
    WithdrawalAlreadyExecuted,
    #[error("Invalid Token address format")]
    InvalidTokenAddress,
    #[error("Failed to serialize the state")]
    SerializeFailed,
    #[error("Nonce not found in withdrawals")]
    NonceNotFound,
    #[error("Failed to decode public value")]
    PublicValueDecodeFailed,
    #[error("Invalid L2 token address format")]
    InvalidL2Token,
    #[error("Token mint not found in the vault.")]
    TokenNotFound,
    #[error("Failed to remove the particular role")]
    RemoveFailed,
    #[error("Transaction needs to be executed on L2")]
    L2ExecutionPending,
    #[error("Batch needs to be committed before finalization")]
    BatchNotCommitted,
    #[error("Overflow occurred while updating total deposits.")]
    Overflow,
    #[error("The deposit queue has reached its maximum capacity.")]
    QueueOverflow,
    #[error("Invalid Argument provided for signature verification.")]
    InvalidArgument,
    #[error("Unauthorized: Caller does not have the required role.")]
    Unauthorized,
    #[error("Token decimal mapping not found for the specified token")]
    TokenMappingNotFound,
    #[error("Insufficient funds in user's token account for transfer.")]
    InsufficientFundsForTransfer,
    #[error("Batch needs to be finalized before finalizing withdrawal")]
    BatchNotFinalized,
    #[error("Insufficient funds in the vault for the requested withdrawal.")]
    InsufficientFunds,
    #[error("The provided public key does not match the expected public key.")]
    PublicKeyMismatch,
    #[error("Failed to serialize event.")]
    FailedToSerializeEvent,
}


impl From<ProgramCustomError> for ProgramError {
    fn from(e: ProgramCustomError) -> Self {
        ProgramError::Custom(e as u32)
    }
}
