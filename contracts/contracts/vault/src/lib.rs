#![no_std]
use soroban_sdk::{
    contract, contractclient, contractimpl, contracttype, symbol_short, token, Address, Env,
};

#[contracttype]
#[derive(Clone)]
pub enum DataKey {
    Balance(Address),
    Agent,
    Token,
    Pool,
    PoolDeposited,
}

#[contractclient(name = "MockPoolClient")]
pub trait MockPoolInterface {
    fn deposit(env: Env, amount: i128);
    fn total_liquidity(env: Env) -> i128;
}

#[contract]
pub struct Vault;

#[contractimpl]
impl Vault {
    /// Initialize the vault with admin-configured agent, token, and mock pool addresses.
    pub fn initialize(env: Env, admin: Address, agent: Address, token: Address, pool: Address) {
        admin.require_auth();
        if env.storage().instance().has(&DataKey::Agent) {
            panic!("already initialized");
        }

        env.storage().instance().set(&DataKey::Agent, &agent);
        env.storage().instance().set(&DataKey::Token, &token);
        env.storage().instance().set(&DataKey::Pool, &pool);
        env.storage().instance().set(&DataKey::PoolDeposited, &0_i128);
    }

    /// Deposit tokens into the vault on behalf of the authenticated user.
    pub fn deposit(env: Env, user: Address, amount: i128) {
        user.require_auth();
        if amount <= 0 {
            panic!("amount must be positive");
        }

        let token_addr = Self::token(env.clone());
        let vault_addr = env.current_contract_address();
        let token_client = token::Client::new(&env, &token_addr);
        token_client.transfer(&user, &vault_addr, &amount);

        let key = DataKey::Balance(user.clone());
        let balance: i128 = env.storage().persistent().get(&key).unwrap_or(0);
        let new_balance = balance
            .checked_add(amount)
            .expect("balance overflow");
        env.storage().persistent().set(&key, &new_balance);

        env.events()
            .publish((symbol_short!("deposit"), user), amount);
    }

    /// Withdraw tokens from the vault for the authenticated user.
    pub fn withdraw(env: Env, user: Address, amount: i128) {
        user.require_auth();
        if amount <= 0 {
            panic!("amount must be positive");
        }

        let key = DataKey::Balance(user.clone());
        let balance: i128 = env.storage().persistent().get(&key).unwrap_or(0);
        if balance < amount {
            panic!("insufficient balance");
        }

        let token_addr = Self::token(env.clone());
        let vault_addr = env.current_contract_address();
        let token_client = token::Client::new(&env, &token_addr);
        if token_client.balance(&vault_addr) < amount {
            panic!("insufficient vault liquidity");
        }

        env.storage()
            .persistent()
            .set(&key, &(balance - amount));
        token_client.transfer(&vault_addr, &user, &amount);

        env.events()
            .publish((symbol_short!("withdraw"), user), amount);
    }

    /// Return the persistent balance recorded for a user.
    pub fn balance(env: Env, user: Address) -> i128 {
        let key = DataKey::Balance(user);
        env.storage().persistent().get(&key).unwrap_or(0)
    }

    /// Allow the delegated agent to deploy vault liquidity into the mock pool.
    pub fn agent_deposit_to_pool(env: Env, amount: i128) {
        if amount <= 0 {
            panic!("amount must be positive");
        }

        let agent: Address = env
            .storage()
            .instance()
            .get(&DataKey::Agent)
            .expect("not initialized");
        agent.require_auth();

        let pool: Address = env
            .storage()
            .instance()
            .get(&DataKey::Pool)
            .expect("not initialized");
        let token_addr = Self::token(env.clone());
        let vault_addr = env.current_contract_address();

        let token_client = token::Client::new(&env, &token_addr);
        if token_client.balance(&vault_addr) < amount {
            panic!("insufficient vault liquidity");
        }

        token_client.transfer(&vault_addr, &pool, &amount);

        let pool_client = MockPoolClient::new(&env, &pool);
        pool_client.deposit(&amount);

        let pool_deposited: i128 = env
            .storage()
            .instance()
            .get(&DataKey::PoolDeposited)
            .unwrap_or(0);
        env.storage()
            .instance()
            .set(&DataKey::PoolDeposited, &(pool_deposited + amount));

        env.events()
            .publish((symbol_short!("agent_dep"), agent), amount);
    }

    pub fn pool_deposited(env: Env) -> i128 {
        env.storage()
            .instance()
            .get(&DataKey::PoolDeposited)
            .unwrap_or(0)
    }

    pub fn get_agent(env: Env) -> Address {
        env.storage()
            .instance()
            .get(&DataKey::Agent)
            .expect("not initialized")
    }

    pub fn token(env: Env) -> Address {
        env.storage()
            .instance()
            .get(&DataKey::Token)
            .expect("not initialized")
    }

    pub fn pool(env: Env) -> Address {
        env.storage()
            .instance()
            .get(&DataKey::Pool)
            .expect("not initialized")
    }
}

mod test;
