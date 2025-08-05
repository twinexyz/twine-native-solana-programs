use solana_program::program_error::ProgramError;
use thiserror::Error;

#[derive(Error, Debug, Copy, Clone)]
pub enum ProgramCustomError {
    #[error("Invalid Token")]                        // error 0
    InvalidToken,
    #[error("Invalid Amount")]                       // error 1
    InvalidAmount,
    #[error("Invalid Amount")]                       // error 2
    InvalidBatchNumber,                             
    #[error("Invalid Length")]                       // error 3
    InvalidIndex,
    #[error("Invalid Account")]                      // error 4
    InvalidAccount,
    #[error("Invalid Receiver")]                     // error 5
    InvalidReceiver,
    #[error("Invalid L1 Token")]                     // error 6
    InvalidL1Token,
    #[error("Invalid PDA derived")]                  // error 7
    InvalidPDA,
    #[error("Invalid Instruction")]                  // error 8
    InvalidInstructionData,
    #[error("Invalid Token Account")]                // error 9
    InvalidTokenAccount,
    #[error("Invalid address provided")]             // error 10
    InvalidAddress,
    #[error("Receiver account not found")]           // error 11
    ReceiverAccountNotFound,
    #[error("Invalid Token address format")]         // error 12
    InvalidTokenAddress,
    #[error("Nonce not found in withdrawals")]       // error 13
    NonceNotFound,
    #[error("Failed to decode public value")]        // error 14
    PublicValueDecodeFailed,
    #[error("Failed to serialize the state")]        // error 15
    SerializeFailed,
    #[error("Withdraw is already executed")]         // error 16
    WithdrawalAlreadyExecuted,
    #[error("Invalid L2 token address format")]      // error 17
    InvalidL2Token,
    #[error("Token mint not found in the vault.")]   // error 18
    TokenNotFound,
    #[error("Failed to remove the particular role")] // error 19
    RemoveFailed,
    #[error("Batch needs to be committed before finalization")] // error 20
    BatchNotCommitted,
    #[error("Overflow occurred while updating total deposits.")] // error 21
    Overflow,
    #[error("The deposit queue has reached its maximum capacity.")] // error 22
    QueueOverflow,
    #[error("Invalid Argument provided for signature verification.")] // error 23
    InvalidArgument,
    #[error("Unauthorized: Caller does not have the required role.")] // error 24
    Unauthorized,
    #[error("Token decimal mapping not found for the specified token")] // error 25
    TokenMappingNotFound,
    #[error("Insufficient funds in user's token account for transfer.")] // error 26
    InsufficientFundsForTransfer,
    #[error("Batch needs to be finalized before finalizing withdrawal")] // error 27
    BatchNotFinalized,
    #[error("Insufficient funds in the vault for the requested withdrawal.")] // error 28
    InsufficientFunds,
    #[error("The provided public key does not match the expected public key.")] // error 29
    PublicKeyMismatch,
}

impl From<ProgramCustomError> for ProgramError {
    fn from(e: ProgramCustomError) -> Self {
        ProgramError::Custom(e as u32)
    }
}
