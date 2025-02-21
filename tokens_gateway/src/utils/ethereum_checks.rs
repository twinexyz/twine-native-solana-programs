use solana_program::program_error::ProgramError;
use sha3::{Digest, Keccak256};

/// Validates Ethereum address format and checksum (EIP-55)
pub fn is_valid_ethereum_address(address: &str) -> Result<bool, ProgramError> {

    if !address.starts_with("0x") || address.len() != 42 {
        return Ok(false);
    }
    
    // Validate hexadecimal characters
    let hex_part = &address[2..];
    if !hex_part.chars().all(|c| c.is_ascii_hexdigit()) {
        return Ok(false);
    }

    // EIP-55 checksum validation
    let address_hash = Keccak256::digest(hex_part.to_lowercase().as_bytes());
    let address_bytes = hex::decode(hex_part).map_err(|_| ProgramError::InvalidArgument)?;

    for (i, char) in hex_part.char_indices() {
        let byte = address_hash[i / 2];
        let nibble = if i % 2 == 0 { byte >> 4 } else { byte & 0xf };
        
        if nibble >= 8 && char.is_ascii_lowercase() {
            return Ok(false);
        }
        if nibble < 8 && char.is_ascii_uppercase() {
            return Ok(false);
        }
    }

    Ok(true)
}
