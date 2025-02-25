use borsh::{BorshDeserialize, BorshSerialize};

#[derive(BorshSerialize, BorshDeserialize, Debug)]
pub enum TwineChainInstruction {
    Initialize, 

}
pub fn unpack_instruction(input: &[u8]) -> Result<TwineChainInstruction, borsh::maybestd::io::Error> {
    TwineChainInstruction::try_from_slice(input)
}
