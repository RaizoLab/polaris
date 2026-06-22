#![cfg(test)]

use super::*;
use mock_phoenix_pool::{MockPhoenixPool, MockPhoenixPoolClient};
use soroban_sdk::{
    testutils::Address as _,
    token::{StellarAssetClient, TokenClient},
    Address, Env,
};

#[test]
fn agent_swap_with_slippage_protection() {
    let env = Env::default();
    env.mock_all_auths_allowing_non_root_auth();

    let admin = Address::generate(&env);
    let vault = Address::generate(&env);
    let issuer_a = Address::generate(&env);
    let issuer_b = Address::generate(&env);

    let usdc = env.register_stellar_asset_contract_v2(issuer_a.clone());
    let usdt = env.register_stellar_asset_contract_v2(issuer_b.clone());
    StellarAssetClient::new(&env, &usdc.address()).mint(&vault, &2_000);

    let pool_id = env.register(MockPhoenixPool, ());
    StellarAssetClient::new(&env, &usdt.address()).mint(&pool_id, &2_000);
    MockPhoenixPoolClient::new(&env, &pool_id).initialize(
        &admin,
        &usdc.address(),
        &usdt.address(),
        &30,
    );

    let adapter_id = env.register(PhoenixAdapter, ());
    let adapter = PhoenixAdapterClient::new(&env, &adapter_id);
    adapter.initialize(&admin, &vault);

    TokenClient::new(&env, &usdc.address()).transfer(&vault, &adapter_id, &1_000);
    let quote = adapter.quote(&pool_id, &usdc.address(), &1_000);
    let out = adapter.swap(
        &pool_id,
        &usdc.address(),
        &usdt.address(),
        &1_000,
        &990,
        &50,
    );

    assert_eq!(out, quote.ask_amount);
    assert_eq!(TokenClient::new(&env, &usdt.address()).balance(&vault), 997);
}

#[test]
#[should_panic(expected = "slippage exceeded")]
fn adapter_rejects_swap_below_min_output() {
    let env = Env::default();
    env.mock_all_auths_allowing_non_root_auth();

    let admin = Address::generate(&env);
    let vault = Address::generate(&env);
    let issuer_a = Address::generate(&env);
    let issuer_b = Address::generate(&env);

    let usdc = env.register_stellar_asset_contract_v2(issuer_a.clone());
    let usdt = env.register_stellar_asset_contract_v2(issuer_b.clone());
    StellarAssetClient::new(&env, &usdc.address()).mint(&vault, &1_000);

    let pool_id = env.register(MockPhoenixPool, ());
    StellarAssetClient::new(&env, &usdt.address()).mint(&pool_id, &1_000);
    MockPhoenixPoolClient::new(&env, &pool_id).initialize(
        &admin,
        &usdc.address(),
        &usdt.address(),
        &30,
    );

    let adapter_id = env.register(PhoenixAdapter, ());
    let adapter = PhoenixAdapterClient::new(&env, &adapter_id);
    adapter.initialize(&admin, &vault);

    TokenClient::new(&env, &usdc.address()).transfer(&vault, &adapter_id, &500);
    adapter.swap(
        &pool_id,
        &usdc.address(),
        &usdt.address(),
        &500,
        &499,
        &50,
    );
}
