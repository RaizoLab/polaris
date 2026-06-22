#![no_std]
use soroban_sdk::{contract, contractimpl, contracttype, token, Address, Env, Map, Vec, symbol_short};

/// Blend-compatible request struct (see blend-contracts `pool::Request`).
#[contracttype]
#[derive(Clone)]
pub struct Request {
    pub request_type: u32,
    pub address: Address,
    pub amount: i128,
}

/// Blend-compatible positions return type (simplified).
#[contracttype]
#[derive(Clone)]
pub struct Positions {
    pub supply: Map<u32, i128>,
}

#[contracttype]
#[derive(Clone)]
enum DataKey {
    Supply(Address, Address),
    SupplyLedger(Address, Address),
    AssetIndex(Address),
    NextIndex,
}

pub const REQUEST_SUPPLY: u32 = 0;
pub const REQUEST_WITHDRAW: u32 = 1;

/// Mock interest: +0.1% per 100 ledgers elapsed (~rough stand-in for Blend yield).
const INTEREST_BPS_PER_100_LEDGERS: i128 = 10;

#[contract]
pub struct MockBlendPool;

#[contractimpl]
impl MockBlendPool {
    pub fn submit(
        env: Env,
        from: Address,
        spender: Address,
        to: Address,
        requests: Vec<Request>,
    ) -> Positions {
        spender.require_auth();
        if from != spender {
            from.require_auth();
        }

        let pool = env.current_contract_address();
        let mut positions = Positions {
            supply: Map::new(&env),
        };

        for i in 0..requests.len() {
            let request = requests.get(i).unwrap();
            match request.request_type {
                REQUEST_SUPPLY => {
                    Self::do_supply(&env, &from, &spender, &pool, &request.address, request.amount);
                }
                REQUEST_WITHDRAW => {
                    Self::do_withdraw(&env, &from, &to, &pool, &request.address, request.amount);
                }
                _ => panic!("unsupported request type"),
            }
        }

        let index = Self::asset_index(&env, &requests.get(0).unwrap().address);
        let balance = Self::accrued_supply(&env, &from, &requests.get(0).unwrap().address);
        positions.supply.set(index, balance);

        env.events()
            .publish((symbol_short!("submit"), from, to), requests.len());

        positions
    }

    pub fn supply_balance(env: Env, user: Address, asset: Address) -> i128 {
        Self::accrued_supply(&env, &user, &asset)
    }

    fn do_supply(
        env: &Env,
        from: &Address,
        spender: &Address,
        pool: &Address,
        asset: &Address,
        amount: i128,
    ) {
        if amount <= 0 {
            panic!("amount must be positive");
        }
        token::Client::new(env, asset).transfer(spender, pool, &amount);

        let key = DataKey::Supply(from.clone(), asset.clone());
        let ledger_key = DataKey::SupplyLedger(from.clone(), asset.clone());
        let current = env.storage().persistent().get(&key).unwrap_or(0);
        let accrued = Self::accrued_supply(env, from, asset);
        env.storage().persistent().set(&key, &(accrued + amount));
        if current == 0 {
            env.storage()
                .persistent()
                .set(&ledger_key, &env.ledger().sequence());
        }
    }

    fn do_withdraw(
        env: &Env,
        from: &Address,
        to: &Address,
        pool: &Address,
        asset: &Address,
        amount: i128,
    ) {
        if amount <= 0 {
            panic!("amount must be positive");
        }
        let available = Self::accrued_supply(env, from, asset);
        if available < amount {
            panic!("insufficient supply balance");
        }

        token::Client::new(env, asset).transfer(pool, to, &amount);

        let key = DataKey::Supply(from.clone(), asset.clone());
        let ledger_key = DataKey::SupplyLedger(from.clone(), asset.clone());
        env.storage()
            .persistent()
            .set(&key, &(available - amount));
        env.storage()
            .persistent()
            .set(&ledger_key, &env.ledger().sequence());
    }

    fn accrued_supply(env: &Env, user: &Address, asset: &Address) -> i128 {
        let key = DataKey::Supply(user.clone(), asset.clone());
        let ledger_key = DataKey::SupplyLedger(user.clone(), asset.clone());
        let principal: i128 = env.storage().persistent().get(&key).unwrap_or(0);
        if principal == 0 {
            return 0;
        }

        let start_ledger: u32 = env
            .storage()
            .persistent()
            .get(&ledger_key)
            .unwrap_or(env.ledger().sequence());
        let elapsed = env.ledger().sequence().saturating_sub(start_ledger);
        let periods = (elapsed / 100) as i128;
        let interest = principal
            .saturating_mul(INTEREST_BPS_PER_100_LEDGERS)
            .saturating_mul(periods)
            / 10_000;
        principal.saturating_add(interest)
    }

    fn asset_index(env: &Env, asset: &Address) -> u32 {
        let key = DataKey::AssetIndex(asset.clone());
        if let Some(index) = env.storage().instance().get(&key) {
            return index;
        }
        let next: u32 = env.storage().instance().get(&DataKey::NextIndex).unwrap_or(0);
        env.storage().instance().set(&key, &next);
        env.storage().instance().set(&DataKey::NextIndex, &(next + 1));
        next
    }
}

mod test;
