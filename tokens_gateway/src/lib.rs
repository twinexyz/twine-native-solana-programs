#![allow(warnings)]
pub mod spl;
pub mod core;
pub mod roles;
pub mod utils;
pub mod process_payout;
pub mod native;
pub mod setters;
pub mod initialize;
pub use solana_program;
pub mod finalize_withdrawal;
pub mod execute_l2_withdrawal;
solana_program::declare_id!("6MXuzcudnF3A5vA4VZGRrVvH3z2T6d9xcH67DCtWqVwt");

