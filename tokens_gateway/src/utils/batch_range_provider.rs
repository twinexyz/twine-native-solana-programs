use twine_chain::utils::constants::MESSAGE_NONCE_GAP;
use solana_program::program_error::ProgramError;

pub fn batch_range_provider(message_nonce: u64) -> Result<(u64, u64), ProgramError> { 
    if message_nonce == 0 { return Err(ProgramError::InvalidArgument); }
    
    let nonce_index = (message_nonce - 1) / MESSAGE_NONCE_GAP + 1;
    
    // Calculate the range 
    let start_nonce = (nonce_index - 1) * MESSAGE_NONCE_GAP + 1;
    let end_nonce = nonce_index * MESSAGE_NONCE_GAP;
    
    Ok((start_nonce, end_nonce))
}
