#![no_std]
use soroban_sdk::{
    contract, contractclient, contractimpl, contracttype, symbol_short, token, Address, Env, Vec,
};

/// Blend-compatible request (matches blend-contracts `pool::Request`).
#[contracttype]
#[derive(Clone)]
pub struct Request {
    pub request_type: u32,
    pub address: Address,
    pub amount: i128,
}

pub const REQUEST_SUPPLY: u32 = 0;
pub const REQUEST_WITHDRAW: u32 = 1;

#[contracttype]
#[derive(Clone)]
enum DataKey {
    Vault,
    Pool,
    Supplied(Address),
}

/// Blend-compatible positions return (simplified).
#[contracttype]
#[derive(Clone)]
pub struct Positions {
    pub supply: soroban_sdk::Map<u32, i128>,
}

#[contractclient(name = "BlendPoolClient")]
pub trait BlendPoolInterface {
    fn submit(
        env: Env,
        from: Address,
        spender: Address,
        to: Address,
        requests: Vec<Request>,
    ) -> Positions;
    fn supply_balance(env: Env, user: Address, asset: Address) -> i128;
}

#[contract]
pub struct BlendAdapter;

#[contractimpl]
impl BlendAdapter {
    pub fn initialize(env: Env, admin: Address, vault: Address, pool: Address) {
        admin.require_auth();
        if env.storage().instance().has(&DataKey::Vault) {
            panic!("already initialized");
        }
        env.storage().instance().set(&DataKey::Vault, &vault);
        env.storage().instance().set(&DataKey::Pool, &pool);
    }

    /// Supply an asset to Blend on behalf of the vault position.
    pub fn supply(env: Env, asset: Address, amount: i128) {
        Self::require_vault(&env);
        if amount <= 0 {
            panic!("amount must be positive");
        }

        let vault = Self::vault(env.clone());
        let pool = Self::pool(env.clone());
        let adapter = env.current_contract_address();

        let token = token::Client::new(&env, &asset);
        if token.balance(&adapter) < amount {
            panic!("insufficient adapter balance");
        }

        let requests = Vec::from_array(
            &env,
            [Request {
                request_type: REQUEST_SUPPLY,
                address: asset.clone(),
                amount,
            }],
        );

        BlendPoolClient::new(&env, &pool).submit(&vault, &adapter, &vault, &requests);

        let key = DataKey::Supplied(asset.clone());
        let supplied: i128 = env.storage().instance().get(&key).unwrap_or(0);
        env.storage()
            .instance()
            .set(&key, &(supplied + amount));

        env.events()
            .publish((symbol_short!("blend_sup"), vault, asset), amount);
    }

    /// Withdraw supplied assets (including accrued interest) from Blend to the vault.
    pub fn withdraw(env: Env, asset: Address, amount: i128) {
        Self::require_vault(&env);
        if amount <= 0 {
            panic!("amount must be positive");
        }

        let vault = Self::vault(env.clone());
        let pool = Self::pool(env.clone());
        let adapter = env.current_contract_address();

        let requests = Vec::from_array(
            &env,
            [Request {
                request_type: REQUEST_WITHDRAW,
                address: asset.clone(),
                amount,
            }],
        );

        BlendPoolClient::new(&env, &pool).submit(&vault, &vault, &adapter, &requests);
        token::Client::new(&env, &asset).transfer(&adapter, &vault, &amount);

        let key = DataKey::Supplied(asset.clone());
        let supplied: i128 = env.storage().instance().get(&key).unwrap_or(0);
        if supplied < amount {
            panic!("insufficient tracked supply");
        }
        env.storage()
            .instance()
            .set(&key, &(supplied - amount));

        env.events()
            .publish((symbol_short!("blend_wd"), vault, asset), amount);
    }

    pub fn supplied(env: Env, asset: Address) -> i128 {
        let key = DataKey::Supplied(asset.clone());
        env.storage().instance().get(&key).unwrap_or(0)
    }

    pub fn blend_supply_balance(env: Env, asset: Address) -> i128 {
        let vault = Self::vault(env.clone());
        let pool = Self::pool(env.clone());
        BlendPoolClient::new(&env, &pool).supply_balance(&vault, &asset)
    }

    pub fn vault(env: Env) -> Address {
        env.storage()
            .instance()
            .get(&DataKey::Vault)
            .expect("not initialized")
    }

    pub fn pool(env: Env) -> Address {
        env.storage()
            .instance()
            .get(&DataKey::Pool)
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
