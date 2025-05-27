use crate::core::error::ProgramCustomError;
use crate::core::state::{SplTokensVaultData, TokenDepositData};
use solana_program::program_error::ProgramError;
use solana_program::pubkey::Pubkey;

impl SplTokensVaultData {
    pub(crate) fn update_deposit(
        &mut self,
        minted_token_id: Pubkey,
        amount: u64,
    ) -> Result<(), ProgramError> {
        if let Some(entry) = self
            .total_deposited_amount
            .iter_mut()
            .find(|e| e.token_id == minted_token_id)
        {
            entry.amount = entry
                .amount
                .checked_add(amount)
                .ok_or(ProgramError::from(ProgramCustomError::Overflow))?;
        } else {
            self.total_deposited_amount.push(TokenDepositData {
                token_id: minted_token_id,
                amount,
            });
        }
        Ok(())
    }
    /// Updates the withdrawal amount for a specific token
    pub(crate) fn update_withdraw(
        &mut self,
        minted_token_id: Pubkey,
        amount: u64,
    ) -> Result<(), ProgramError> {
        if let Some(entry) = self
            .total_deposited_amount
            .iter_mut()
            .find(|e| e.token_id == minted_token_id)
        {
            entry.amount = entry
                .amount
                .checked_sub(amount)
                .ok_or(ProgramError::from(ProgramCustomError::InsufficientFunds))?;
        } else {
            return Err(ProgramCustomError::TokenNotFound.into());
        }
        Ok(())
    }

    /// Gets the deposit amount for a specific token
    pub fn get_deposit(&self, token_id: Pubkey) -> Result<u64, ProgramError> {
        self.total_deposited_amount
            .iter()
            .find(|e| e.token_id == token_id)
            .map(|e| e.amount)
            .ok_or(ProgramError::from(ProgramCustomError::TokenNotFound))
    }
}
