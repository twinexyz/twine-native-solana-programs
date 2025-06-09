use solana_program::program_error::ProgramError;


pub fn is_valid_ethereum_address(address: &str) -> Result<bool, ProgramError> {
    // Check if the address starts with "0x" and has a total length of 42 characters (including "0x")
    if !address.starts_with("0x") || address.len() != 42 {
        return Ok(false);
    }

    // Check if the remaining characters are valid hexadecimal
    let is_valid = address[2..].chars().all(|c| c.is_ascii_hexdigit());

    Ok(is_valid)
}
