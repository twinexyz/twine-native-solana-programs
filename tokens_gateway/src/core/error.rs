use solana_program::program_error::ProgramError;
use thiserror::Error;

#[derive(Error, Debug, Copy, Clone)]
pub enum ProgramCustomError {
    #[error("Invalid Token")]                    // error 0
    InvalidToken,
    #[error("Invalid Amount")]                   // error 1
    InvalidAmount,
    #[error("Invalid Amount")]  
    InvalidBatchNumber,
    #[error("Invalid Length")]                   // error 2
    InvalidIndex,
    #[error("Invalid Account")]                  // error 3
    InvalidAccount,
    #[error("Invalid Receiver")]                 // error 4
    InvalidReceiver,
    #[error("Invalid L1 Token")]                 // error 5
    InvalidL1Token,
    #[error("Invalid PDA derived")]              // error 6
    InvalidPDA,
    #[error("Invalid Instruction")]              // error 7
    InvalidInstructionData,
    #[error("Invalid Token Account")]            // error 8
    InvalidTokenAccount,
    #[error("Invalid address provided")]         // error 9
    InvalidAddress,
    #[error("Receiver account not found")]       // error 10
    ReceiverAccountNotFound,
    #[error("Invalid Token address format")]     // error 11
    InvalidTokenAddress,
    #[error("Nonce not found in withdrawals")]   // error 12
    NonceNotFound,
    #[error("Failed to decode public value")]    // error 13
    PublicValueDecodeFailed,
    #[error("Failed to serialize the state")]    // error 14
    SerializeFailed,
    #[error("Withdraw is already executed")]     // error 15
    WithdrawalAlreadyExecuted,
    #[error("Invalid L2 token address format")]  // error 16
    InvalidL2Token,
    #[error("Token mint not found in the vault.")] // error 17
    TokenNotFound,
    #[error("Failed to remove the particular role")] // error 18
    RemoveFailed,
    #[error("Batch needs to be committed before finalization")] // error 19
    BatchNotCommitted,
    #[error("Overflow occurred while updating total deposits.")] // error 20
    Overflow,
    #[error("The deposit queue has reached its maximum capacity.")] // error 21
    QueueOverflow,
    #[error("Invalid Argument provided for signature verification.")] // error 22
    InvalidArgument,
    #[error("Unauthorized: Caller does not have the required role.")] // error 23
    Unauthorized,
    #[error("Token decimal mapping not found for the specified token")] // error 24
    TokenMappingNotFound,
    #[error("Insufficient funds in user's token account for transfer.")] // error 25
    InsufficientFundsForTransfer,
    #[error("Batch needs to be finalized before finalizing withdrawal")] // error 26
    BatchNotFinalized,
    #[error("Insufficient funds in the vault for the requested withdrawal.")] // error 27
    InsufficientFunds,
    #[error("The provided public key does not match the expected public key.")] // error 28
    PublicKeyMismatch,
}

impl From<ProgramCustomError> for ProgramError {
    fn from(e: ProgramCustomError) -> Self {
        ProgramError::Custom(e as u32)
    }
}
