#![no_std]
use soroban_sdk::{contract, contractimpl, contracttype, token, Address, Env, symbol_short};

#[contracttype]
#[derive(Clone)]
pub enum DataKey {
    Token,
    TotalLiquidity,
}

#[contract]
pub struct MockPool;

#[contractimpl]
impl MockPool {
    /// Initialize the mock pool with the token it accepts.
    pub fn initialize(env: Env, admin: Address, token: Address) {
        admin.require_auth();
        if env.storage().instance().has(&DataKey::Token) {
            panic!("already initialized");
        }
        env.storage().instance().set(&DataKey::Token, &token);
        env.storage().instance().set(&DataKey::TotalLiquidity, &0_i128);
    }

    /// Record a liquidity deposit. Tokens must already be transferred to this contract.
    pub fn deposit(env: Env, amount: i128) {
        if amount <= 0 {
            panic!("amount must be positive");
        }

        let token_addr: Address = env
            .storage()
            .instance()
            .get(&DataKey::Token)
            .expect("not initialized");
        let pool_addr = env.current_contract_address();

        let token_client = token::Client::new(&env, &token_addr);
        let total: i128 = env
            .storage()
            .instance()
            .get(&DataKey::TotalLiquidity)
            .unwrap_or(0);
        let new_total = total
            .checked_add(amount)
            .expect("total liquidity overflow");

        if token_client.balance(&pool_addr) < new_total {
            panic!("insufficient tokens received");
        }

        env.storage()
            .instance()
            .set(&DataKey::TotalLiquidity, &new_total);

        env.events()
            .publish((symbol_short!("pool_dep"), pool_addr), amount);
    }

    /// Withdraw liquidity to `to`. Tokens are transferred from this pool.
    pub fn withdraw(env: Env, to: Address, amount: i128) {
        if amount <= 0 {
            panic!("amount must be positive");
        }

        let token_addr: Address = env
            .storage()
            .instance()
            .get(&DataKey::Token)
            .expect("not initialized");
        let pool_addr = env.current_contract_address();

        let total: i128 = env
            .storage()
            .instance()
            .get(&DataKey::TotalLiquidity)
            .unwrap_or(0);
        if total < amount {
            panic!("insufficient pool liquidity");
        }

        let token_client = token::Client::new(&env, &token_addr);
        token_client.transfer(&pool_addr, &to, &amount);

        env.storage()
            .instance()
            .set(&DataKey::TotalLiquidity, &(total - amount));

        env.events()
            .publish((symbol_short!("pool_wd"), pool_addr), amount);
    }

    pub fn total_liquidity(env: Env) -> i128 {
        env.storage()
            .instance()
            .get(&DataKey::TotalLiquidity)
            .unwrap_or(0)
    }

    pub fn token(env: Env) -> Address {
        env.storage()
            .instance()
            .get(&DataKey::Token)
            .expect("not initialized")
    }
}

mod test;
