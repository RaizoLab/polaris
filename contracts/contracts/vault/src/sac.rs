//! Helpers for Stellar Asset Contract (SAC) interoperability.
#![allow(dead_code)]

use soroban_sdk::{token, Address, Env};

/// Transfer SAC tokens from `from` to `to` using the SEP-41 interface.
pub fn transfer(env: &Env, token: &Address, from: &Address, to: &Address, amount: i128) {
    token::Client::new(env, token).transfer(from, to, &amount);
}

/// Read the SAC balance for `holder`.
pub fn balance(env: &Env, token: &Address, holder: &Address) -> i128 {
    token::Client::new(env, token).balance(holder)
}

/// Read SAC metadata decimals.
pub fn decimals(env: &Env, token: &Address) -> u32 {
    token::Client::new(env, token).decimals()
}
