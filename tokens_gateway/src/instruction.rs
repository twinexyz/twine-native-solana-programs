use borsh::{BorshDeserialize, BorshSerialize};

#[derive(BorshSerialize, BorshDeserialize, Debug)]
pub enum GatewayInstruction {
    Initialize, // For vault or mapping accounts initialization
    // DepositSOL { 
    //     twine_receiver: String, 
    //     l1_token: String, 
    //     l2_token: String, 
    //     amount: u64 
    // },

}
pub fn unpack_instruction(input: &[u8]) -> Result<GatewayInstruction, borsh::maybestd::io::Error> {
    GatewayInstruction::try_from_slice(input)
}