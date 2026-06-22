#![no_std]
use soroban_sdk::{
    contract, contractclient, contractimpl, contracttype, symbol_short, token, Address, Env,
};

#[contracttype]
#[derive(Clone)]
pub struct SimulateSwapResponse {
    pub ask_amount: i128,
    pub spread_amount: i128,
}

#[contracttype]
#[derive(Clone)]
enum DataKey {
    Vault,
}

#[contractclient(name = "PhoenixPoolClient")]
pub trait PhoenixPoolInterface {
    fn swap(
        env: Env,
        sender: Address,
        offer_asset: Address,
        offer_amount: i128,
        ask_asset_min_amount: Option<i128>,
        max_spread_bps: Option<u32>,
    ) -> i128;
    fn simulate_swap(env: Env, offer_asset: Address, sell_amount: i128) -> SimulateSwapResponse;
}

#[contract]
pub struct PhoenixAdapter;

#[contractimpl]
impl PhoenixAdapter {
    pub fn initialize(env: Env, admin: Address, vault: Address) {
        admin.require_auth();
        if env.storage().instance().has(&DataKey::Vault) {
            panic!("already initialized");
        }
        env.storage().instance().set(&DataKey::Vault, &vault);
    }

    /// Swap stablecoins through a Phoenix liquidity pool with slippage protection.
    pub fn swap(
        env: Env,
        pool: Address,
        offer_asset: Address,
        ask_asset: Address,
        offer_amount: i128,
        min_ask_amount: i128,
        max_spread_bps: u32,
    ) -> i128 {
        Self::require_vault(&env);
        if offer_amount <= 0 || min_ask_amount <= 0 {
            panic!("amounts must be positive");
        }

        let vault = Self::vault(env.clone());
        let adapter = env.current_contract_address();
        let token = token::Client::new(&env, &offer_asset);

        if token.balance(&adapter) < offer_amount {
            panic!("insufficient adapter balance");
        }

        let ask_amount = PhoenixPoolClient::new(&env, &pool).swap(
            &adapter,
            &offer_asset,
            &offer_amount,
            &Some(min_ask_amount),
            &Some(max_spread_bps),
        );

        token::Client::new(&env, &ask_asset).transfer(&adapter, &vault, &ask_amount);

        env.events().publish(
            (symbol_short!("phx_swap"), vault, offer_asset, ask_asset),
            ask_amount,
        );

        ask_amount
    }

    pub fn quote(
        env: Env,
        pool: Address,
        offer_asset: Address,
        offer_amount: i128,
    ) -> SimulateSwapResponse {
        PhoenixPoolClient::new(&env, &pool).simulate_swap(&offer_asset, &offer_amount)
    }

    pub fn vault(env: Env) -> Address {
        env.storage()
            .instance()
            .get(&DataKey::Vault)
            .expect("not initialized")
    }

    fn require_vault(env: &Env) {
        let vault: Address = env
            .storage()
            .instance()
            .get(&DataKey::Vault)
            .expect("not initialized");
        vault.require_auth();
    }
}

mod test;
