use solana_program::program_error::ProgramError;
use thiserror::Error;

#[derive(Error, Debug, Copy, Clone)]
pub enum ProgramCustomError {
    #[error("PDA derived does not equal PDA passed in")]
    InvalidPDA,
    #[error("Failed to serialize the state")]
    SerializeFailed, 
    #[error("Failed to call endpoint function")]
    EndpointFunctionFailed,
}

impl From<ProgramCustomError> for ProgramError {
    fn from(e: ProgramCustomError) -> Self {
        ProgramError::Custom(e as u32)
    }
}
