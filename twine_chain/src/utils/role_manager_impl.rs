use solana_program::pubkey::Pubkey;
use crate::core::state::{RoleType, TwineChainRoleManager};

impl TwineChainRoleManager {
    pub fn has_role(&self, address: &Pubkey, role: RoleType) -> bool {
        self.roles
            .iter()
            .any(|(r, r_type)| r == address && *r_type == role)
    }

    pub fn remove_role(&mut self, address: &Pubkey, role: RoleType) -> bool {
        if let Some(index) = self
            .roles
            .iter()
            .position(|(r, r_type)| r == address && *r_type == role)
        {
            self.roles.swap_remove(index);
            true
        } else {
            false
        }
    }
}
