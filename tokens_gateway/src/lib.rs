#![allow(warnings)]
pub mod core;
pub mod finalize_withdrawal;
pub mod initialize;
pub mod native;
pub mod roles;
pub mod setters;
pub mod spl;
pub mod utils;
pub use solana_program;
solana_program::declare_id!("C49GbSeRLKTqnF7y5XaxGH47WkX7hCNecYPcG3oMeMFx");

