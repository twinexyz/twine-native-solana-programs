
use crate::core::state::{TokenDecimalMapping, TokenDecimalMappings};
use num_bigint::BigUint;
use num_traits::ops::checked::{CheckedMul,CheckedDiv};
use solana_program::program_error::ProgramError;
use crate::core::error::ProgramCustomError;

impl TokenDecimalMappings {
    pub(crate) fn update_mapping(
        &mut self,
        l1_token: String,
        l2_token: String,
        l1_decimals: u8,
        l2_decimals: u8,
    ) -> Result<(), ProgramError> {
        if let Some(mapping) = self.mappings.iter_mut().find(|m| m.l1_token == l1_token) {
            mapping.l2_token = l2_token;
            mapping.l1_decimals = l1_decimals;
            mapping.l2_decimals = l2_decimals;
        } else {
            self.mappings.push(TokenDecimalMapping {
                l1_token,
                l2_token,
                l1_decimals,
                l2_decimals,
            });
        }
        Ok(())
    }

    pub fn get_mapping(&self, l1_token: &str) -> Option<&TokenDecimalMapping> {
        self.mappings.iter().find(|m| m.l1_token == l1_token)
    }

    pub fn convert_l1_to_l2(
        amount: u64,
        l1_decimals: u8,
        l2_decimals: u8,
    ) -> Result<String, ProgramError> {
        let amount = BigUint::from(amount);
        Self::convert_amount(amount, l1_decimals, l2_decimals)
    }

    pub fn convert_l2_to_l1(
        amount: &str,
        l2_decimals: u8,
        l1_decimals: u8,
    ) -> Result<String, ProgramError> {
        let amount = Self::parse_amount_to_biguint(amount)?;
        Self::convert_amount(amount, l2_decimals, l1_decimals)
    }

    fn convert_amount(amount: BigUint, from_decimals: u8, to_decimals: u8) -> Result<String, ProgramError> {
        let result = if from_decimals < to_decimals {
            amount
                .checked_mul(&BigUint::from(10u32).pow((to_decimals - from_decimals) as u32))
                .ok_or(ProgramError::Custom(ProgramCustomError::Overflow as u32))?
        } else {
            amount
                .checked_div(&BigUint::from(10u32).pow((from_decimals - to_decimals) as u32))
                .ok_or(ProgramError::Custom(ProgramCustomError::Overflow as u32))?
        };
    
        Ok(result.to_string())
    }

    pub fn parse_amount_to_u64(amount: &str) -> Result<u64, ProgramError> {
        amount
            .parse::<u64>()
            .map_err(|_| ProgramCustomError::InvalidAmount.into())
    }

    fn parse_amount_to_biguint(amount: &str) -> Result<BigUint, ProgramError> {
        amount
            .parse::<BigUint>()
            .map_err(|_| ProgramCustomError::InvalidAmount.into())
    }
}
