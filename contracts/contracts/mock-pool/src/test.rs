#![cfg(test)]

use super::*;
use soroban_sdk::{testutils::Address as _, token::TokenClient, Address, Env};

fn setup_pool(env: &Env) -> (Address, Address, TokenClient<'_>) {
    let admin = Address::generate(env);
    let depositor = Address::generate(env);

    let sac = env.register_stellar_asset_contract_v2(admin.clone());
    let token_admin = soroban_sdk::token::StellarAssetClient::new(env, &sac.address());
    token_admin.mint(&depositor, &10_000);

    let pool_id = env.register(MockPool, ());
    let pool_client = MockPoolClient::new(env, &pool_id);
    pool_client.initialize(&admin, &sac.address());

    (pool_id, depositor, TokenClient::new(env, &sac.address()))
}

#[test]
fn deposit_increases_total_liquidity() {
    let env = Env::default();
    env.mock_all_auths();

    let (pool_id, depositor, token) = setup_pool(&env);
    let pool_client = MockPoolClient::new(&env, &pool_id);

    token.transfer(&depositor, &pool_id, &1_000);
    pool_client.deposit(&1_000);

    assert_eq!(pool_client.total_liquidity(), 1_000);
    assert_eq!(token.balance(&pool_id), 1_000);
}
