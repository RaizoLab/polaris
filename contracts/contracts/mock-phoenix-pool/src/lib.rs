#![no_std]
use soroban_sdk::{contract, contractimpl, contracttype, token, Address, Env, symbol_short};

#[contracttype]
#[derive(Clone)]
pub struct SimulateSwapResponse {
    pub ask_amount: i128,
    pub spread_amount: i128,
}

#[contracttype]
#[derive(Clone)]
enum DataKey {
    TokenA,
    TokenB,
    FeeBps,
}

#[contract]
pub struct MockPhoenixPool;

#[contractimpl]
impl MockPhoenixPool {
    pub fn initialize(env: Env, admin: Address, token_a: Address, token_b: Address, fee_bps: u32) {
        admin.require_auth();
        env.storage().instance().set(&DataKey::TokenA, &token_a);
        env.storage().instance().set(&DataKey::TokenB, &token_b);
        env.storage().instance().set(&DataKey::FeeBps, &fee_bps);
    }

    /// Phoenix-compatible stable swap with slippage protection via `ask_asset_min_amount`.
    pub fn swap(
        env: Env,
        sender: Address,
        offer_asset: Address,
        offer_amount: i128,
        ask_asset_min_amount: Option<i128>,
        max_spread_bps: Option<u32>,
    ) -> i128 {
        sender.require_auth();
        if offer_amount <= 0 {
            panic!("offer amount must be positive");
        }

        let (token_a, token_b, fee_bps) = Self::config(&env);
        let ask_asset = if offer_asset == token_a {
            token_b.clone()
        } else if offer_asset == token_b {
            token_a.clone()
        } else {
            panic!("unknown offer asset");
        };

        let ask_amount = Self::quote(offer_amount, fee_bps);
        if let Some(min) = ask_asset_min_amount {
            if ask_amount < min {
                panic!("slippage exceeded");
            }
        }
        if let Some(max_spread) = max_spread_bps {
            let spread = offer_amount - ask_amount;
            let spread_bps = spread.saturating_mul(10_000) / offer_amount.max(1);
            if spread_bps as u32 > max_spread {
                panic!("spread exceeds max");
            }
        }

        let pool = env.current_contract_address();
        token::Client::new(&env, &offer_asset).transfer(&sender, &pool, &offer_amount);
        token::Client::new(&env, &ask_asset).transfer(&pool, &sender, &ask_amount);

        env.events().publish(
            (symbol_short!("phx_swap"), sender, offer_asset, ask_asset),
            ask_amount,
        );

        ask_amount
    }

    pub fn simulate_swap(env: Env, _offer_asset: Address, sell_amount: i128) -> SimulateSwapResponse {
        let (_, _, fee_bps) = Self::config(&env);
        let ask_amount = Self::quote(sell_amount, fee_bps);
        SimulateSwapResponse {
            ask_amount,
            spread_amount: sell_amount - ask_amount,
        }
    }

    fn quote(offer_amount: i128, fee_bps: u32) -> i128 {
        offer_amount.saturating_mul(10_000 - fee_bps as i128) / 10_000
    }

    fn config(env: &Env) -> (Address, Address, u32) {
        let token_a: Address = env.storage().instance().get(&DataKey::TokenA).expect("not initialized");
        let token_b: Address = env.storage().instance().get(&DataKey::TokenB).expect("not initialized");
        let fee_bps: u32 = env.storage().instance().get(&DataKey::FeeBps).unwrap_or(30);
        (token_a, token_b, fee_bps)
    }
}

mod test;
