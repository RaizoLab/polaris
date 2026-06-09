#![no_std]
use soroban_sdk::{contract, contractimpl};

#[contract]
pub struct Vault;

#[contractimpl]
impl Vault {
    pub fn version() -> u32 {
        1
    }
}

mod test;
