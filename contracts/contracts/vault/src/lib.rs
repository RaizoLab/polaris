#![no_std]
use soroban_sdk::{
    contract, contractclient, contractimpl, contracttype, symbol_short, token, Address, Env,
};

#[contracttype]
#[derive(Clone)]
pub enum DataKey {
    Balance(Address, Address),
    Auth,
    PoolDeposited(Address),
}

#[contractclient(name = "AuthClient")]
pub trait AuthInterface {
    fn assert_is_agent(env: Env, caller: Address);
    fn is_protocol_whitelisted(env: Env, protocol: Address) -> bool;
    fn agent(env: Env) -> Address;
}

#[contractclient(name = "MockPoolClient")]
pub trait MockPoolInterface {
    fn deposit(env: Env, amount: i128);
    fn withdraw(env: Env, to: Address, amount: i128);
    fn total_liquidity(env: Env) -> i128;
    fn token(env: Env) -> Address;
}

#[contract]
pub struct Vault;

#[contractimpl]
impl Vault {
    /// Initialize the vault with the auth contract and a default liquidity pool.
    pub fn initialize(env: Env, admin: Address, auth: Address, pool: Address) {
        admin.require_auth();
        if env.storage().instance().has(&DataKey::Auth) {
            panic!("already initialized");
        }

        env.storage().instance().set(&DataKey::Auth, &auth);
        env.storage()
            .instance()
            .set(&DataKey::PoolDeposited(pool.clone()), &0_i128);
    }

    /// Deposit a Stellar Asset Contract token into the vault.
    pub fn deposit(env: Env, user: Address, token: Address, amount: i128) {
        user.require_auth();
        if amount <= 0 {
            panic!("amount must be positive");
        }

        Self::transfer_in(&env, &user, &token, amount);

        let key = DataKey::Balance(user.clone(), token.clone());
        let balance: i128 = env.storage().persistent().get(&key).unwrap_or(0);
        let new_balance = balance
            .checked_add(amount)
            .expect("balance overflow");
        env.storage().persistent().set(&key, &new_balance);

        env.events()
            .publish((symbol_short!("deposit"), user, token), amount);
    }

    /// Withdraw a Stellar Asset Contract token from the vault.
    pub fn withdraw(env: Env, user: Address, token: Address, amount: i128) {
        user.require_auth();
        if amount <= 0 {
            panic!("amount must be positive");
        }

        let key = DataKey::Balance(user.clone(), token.clone());
        let balance: i128 = env.storage().persistent().get(&key).unwrap_or(0);
        if balance < amount {
            panic!("insufficient balance");
        }

        let vault_addr = env.current_contract_address();
        let token_client = token::Client::new(&env, &token);
        if token_client.balance(&vault_addr) < amount {
            panic!("insufficient vault liquidity");
        }

        env.storage()
            .persistent()
            .set(&key, &(balance - amount));
        token_client.transfer(&vault_addr, &user, &amount);

        env.events()
            .publish((symbol_short!("withdraw"), user, token), amount);
    }

    pub fn balance(env: Env, user: Address, token: Address) -> i128 {
        let key = DataKey::Balance(user, token);
        env.storage().persistent().get(&key).unwrap_or(0)
    }

    /// Agent deploys vault liquidity into a whitelisted mock pool.
    pub fn agent_deposit_to_pool(env: Env, pool: Address, amount: i128) {
        if amount <= 0 {
            panic!("amount must be positive");
        }

        let auth = Self::auth_client(&env);
        let agent = auth.agent();
        agent.require_auth();
        auth.assert_is_agent(&agent);
        if !auth.is_protocol_whitelisted(&pool) {
            panic!("protocol not whitelisted");
        }

        let token_addr = MockPoolClient::new(&env, &pool).token();
        let vault_addr = env.current_contract_address();
        let token_client = token::Client::new(&env, &token_addr);

        if token_client.balance(&vault_addr) < amount {
            panic!("insufficient vault liquidity");
        }

        token_client.transfer(&vault_addr, &pool, &amount);
        MockPoolClient::new(&env, &pool).deposit(&amount);

        let key = DataKey::PoolDeposited(pool.clone());
        let pool_deposited: i128 = env.storage().instance().get(&key).unwrap_or(0);
        env.storage()
            .instance()
            .set(&key, &(pool_deposited + amount));

        env.events()
            .publish((symbol_short!("agent_dep"), agent, pool), amount);
    }

    /// Agent rebalances liquidity between two whitelisted protocols without user interaction.
    pub fn rebalance(env: Env, from_pool: Address, to_pool: Address, amount: i128) {
        if amount <= 0 {
            panic!("amount must be positive");
        }
        if from_pool == to_pool {
            panic!("pools must differ");
        }

        let auth = Self::auth_client(&env);
        let agent = auth.agent();
        agent.require_auth();
        auth.assert_is_agent(&agent);

        if !auth.is_protocol_whitelisted(&from_pool)
            || !auth.is_protocol_whitelisted(&to_pool)
        {
            panic!("protocol not whitelisted");
        }

        let from = MockPoolClient::new(&env, &from_pool);
        let to = MockPoolClient::new(&env, &to_pool);
        let token_addr = from.token();
        if to.token() != token_addr {
            panic!("pool token mismatch");
        }

        let vault_addr = env.current_contract_address();
        from.withdraw(&vault_addr, &amount);
        token::Client::new(&env, &token_addr).transfer(&vault_addr, &to_pool, &amount);
        to.deposit(&amount);

        let from_key = DataKey::PoolDeposited(from_pool.clone());
        let to_key = DataKey::PoolDeposited(to_pool.clone());
        let from_deposited: i128 = env.storage().instance().get(&from_key).unwrap_or(0);
        if from_deposited < amount {
            panic!("insufficient tracked pool deposits");
        }
        env.storage()
            .instance()
            .set(&from_key, &(from_deposited - amount));

        let to_deposited: i128 = env.storage().instance().get(&to_key).unwrap_or(0);
        env.storage()
            .instance()
            .set(&to_key, &(to_deposited + amount));

        env.events().publish(
            (symbol_short!("rebalance"), agent, from_pool, to_pool),
            amount,
        );
    }

    pub fn pool_deposited(env: Env, pool: Address) -> i128 {
        let key = DataKey::PoolDeposited(pool);
        env.storage().instance().get(&key).unwrap_or(0)
    }

    pub fn auth(env: Env) -> Address {
        env.storage()
            .instance()
            .get(&DataKey::Auth)
            .expect("not initialized")
    }

    fn auth_client(env: &Env) -> AuthClient<'_> {
        let auth_addr: Address = env
            .storage()
            .instance()
            .get(&DataKey::Auth)
            .expect("not initialized");
        AuthClient::new(env, &auth_addr)
    }

    fn transfer_in(env: &Env, from: &Address, token_addr: &Address, amount: i128) {
        let vault_addr = env.current_contract_address();
        token::Client::new(env, token_addr).transfer(from, &vault_addr, &amount);
    }
}

mod sac;
#[cfg(test)]
mod sac_integration;
mod test;
