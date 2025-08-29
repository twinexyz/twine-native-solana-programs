use solana_program::program_error::ProgramError;
use thiserror::Error;

#[derive(Error, Debug, Copy, Clone)]
pub enum ProgramCustomError {
    #[error("Invalid Index")]
    InvalidIndex,
    #[error("Nonce not found")]
    NonceNotFound,
    #[error("State root mismatch")]
    InvalidStateRootSequence,
    #[error("Invalid Instruction")]
    InvalidInstructionData,
    #[error("Error in verification")]
    TwineVerificationError,
    #[error("Receipt root mismatch")]
    InvalidReceiptRoot,
    #[error("Invalid Transaction Type")]
    InvalidTransactionType,
    #[error("Account not initialized yet")]
    UninitializedAccount,
    #[error("Previous Batch Data is empty")]
    EmptyPreviousBatch,
    #[error("Failed to serialize the state")]
    SerializeFailed,
    #[error("Failed to decode public value")]
    PublicValueDecodeFailed,
    #[error("Input data exceeds max length")]
    InvalidDataLength,
    #[error("Invalid Token Address Format")]
    InvalidTokenAddressFormat,
    #[error("Finalize in Serial batch order")]
    InvalidBatchSequence,
    #[error("Insufficient Funds for Transfer")]
    InsufficientFundsForTransfer,
    #[error("Invalid Receiver Address Format")]
    InvalidReceiverAddressFormat,
    #[error("Token Decimal Mapping Not Found")]
    TokenMappingNotFound,
    #[error("Invalid L2 Token Address Format")]
    InvalidL2TokenAddressFormat,
    #[error("Batch needs to be finalized first")]
    BatchNotFinalized,
    #[error("Deposit Rolling Hash is mismatched")]
    DepositRollingHashMismatch,
    #[error("Copied Nonce must have required gap")]
    InvalidNonceGap,
    #[error("Withdraw Rolling Hash is mismatched")]
    WithdrawRollingHashMismatch,
    #[error("Previous Batch needs to be finalized")]
    PreviousBatchNotFinalized,
    #[error("Failed to remove the particular role")]
    RemoveFailed,
    #[error("LayerZero Rolling Hash is mismatched")]
    LayerZeroRollingHashMismatch,
    #[error("Commitment of Empty Batch not allowed")]
    EmptyBatchCommitment,
    #[error("Finalization should be done in sequence")]
    InvalidBatchFinalizationSequence,
    #[error("Last commited batch hash did not match")]
    LastCommitedBatchHashMismatch,
    #[error("Cannot execute less message than before")]
    MessageExecutedCountError,
    #[error("PDA derived does not equal PDA passed in")]
    InvalidPDA,
    #[error("Last finalized batch hash did not match")]
    LastFinalizedBatchHashMismatch,
    #[error("Blocks must be comitted in sequential order")]
    InvalidStartNonce,
    #[error("Messages must be copied in sequential order")]
    InvalidBlockCommitmentSequence,
    #[error("Calculated and Provided Batch Hash did not match")]
    BatchHashMismatch,
    #[error("The provided account did not sign the transaction.")]
    InvalidSigner,
    #[error("Unauthorized: Caller does not have the required role")]
    Unauthorized,
    #[error("All batch data needs to be filled before finalization")]
    BatchNotFilled,
    #[error("Provided startBlock/endBlock and the block data mismatch")]
    InvalidBlockData,
    #[error("Overflow occurred while updating total deposits or withdrawals.")]
    OverflowError,
    #[error("The withdrawal with the provided nonce has already been executed.")]
    WithdrawalAlreadyExecuted,
    #[error("The transaction count is greater than transactions present in the queue")]
    GreaterCount,
    #[error("Failed to serialize event.")]
    FailedToSerializeEvent,
}


impl From<ProgramCustomError> for ProgramError {
    fn from(e: ProgramCustomError) -> Self {
        ProgramError::Custom(e as u32)
    }
}
