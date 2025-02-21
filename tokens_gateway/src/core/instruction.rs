use borsh::{BorshDeserialize, BorshSerialize};

#[derive(BorshSerialize, BorshDeserialize, Debug)]
pub enum GatewayInstruction {
    Initialize, // For vault or mapping accounts initialization

}
pub fn unpack_instruction(input: &[u8]) -> Result<GatewayInstruction, borsh::maybestd::io::Error> {
    GatewayInstruction::try_from_slice(input)
}
