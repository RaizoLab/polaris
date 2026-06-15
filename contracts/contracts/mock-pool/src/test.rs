#![cfg(test)]

use super::*;
use soroban_sdk::{
    testutils::Address as _,
    token::{StellarAssetClient, TokenClient},
    Address, Env,
};

#[test]
fn withdraw_returns_liquidity_to_recipient() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let recipient = Address::generate(&env);

    let sac = env.register_stellar_asset_contract_v2(admin.clone());
    let token_admin = StellarAssetClient::new(&env, &sac.address());
    token_admin.mint(&admin, &2_000);

    let pool_id = env.register(MockPool, ());
    let pool = MockPoolClient::new(&env, &pool_id);
    pool.initialize(&admin, &sac.address());

    let token = TokenClient::new(&env, &sac.address());
    token.transfer(&admin, &pool_id, &1_000);
    pool.deposit(&1_000);

    pool.withdraw(&recipient, &400);

    assert_eq!(pool.total_liquidity(), 600);
    assert_eq!(token.balance(&recipient), 400);
    assert_eq!(token.balance(&pool_id), 600);
}
