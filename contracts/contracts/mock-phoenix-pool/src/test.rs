#![cfg(test)]

use super::*;
use soroban_sdk::{
    testutils::Address as _,
    token::{StellarAssetClient, TokenClient},
    Address, Env,
};

#[test]
fn swap_applies_fee_and_slippage_floor() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let trader = Address::generate(&env);
    let issuer_a = Address::generate(&env);
    let issuer_b = Address::generate(&env);

    let usdc = env.register_stellar_asset_contract_v2(issuer_a.clone());
    let usdt = env.register_stellar_asset_contract_v2(issuer_b.clone());
    StellarAssetClient::new(&env, &usdc.address()).mint(&trader, &5_000);
    let pool_id = env.register(MockPhoenixPool, ());
    StellarAssetClient::new(&env, &usdt.address()).mint(&pool_id, &5_000);

    let pool = MockPhoenixPoolClient::new(&env, &pool_id);
    pool.initialize(&admin, &usdc.address(), &usdt.address(), &30);

    let sim = pool.simulate_swap(&usdc.address(), &1_000);
    assert_eq!(sim.ask_amount, 997);

    let out = pool.swap(&trader, &usdc.address(), &1_000, &Some(990), &Some(50));
    assert_eq!(out, 997);
    assert_eq!(TokenClient::new(&env, &usdt.address()).balance(&trader), 997);
}

#[test]
#[should_panic(expected = "slippage exceeded")]
fn swap_rejects_insufficient_output() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let trader = Address::generate(&env);
    let issuer_a = Address::generate(&env);
    let issuer_b = Address::generate(&env);

    let usdc = env.register_stellar_asset_contract_v2(issuer_a.clone());
    let usdt = env.register_stellar_asset_contract_v2(issuer_b.clone());
    StellarAssetClient::new(&env, &usdc.address()).mint(&trader, &1_000);
    let pool_id = env.register(MockPhoenixPool, ());
    StellarAssetClient::new(&env, &usdt.address()).mint(&pool_id, &1_000);

    let pool = MockPhoenixPoolClient::new(&env, &pool_id);
    pool.initialize(&admin, &usdc.address(), &usdt.address(), &30);

    pool.swap(&trader, &usdc.address(), &1_000, &Some(998), &None);
}
