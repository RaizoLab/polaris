#![cfg(test)]

use super::*;
use mock_blend_pool::MockBlendPool;
use soroban_sdk::{
    testutils::Address as _,
    token::{StellarAssetClient, TokenClient},
    Address, Env,
};

#[test]
fn vault_supplies_usdc_to_blend_via_adapter() {
    let env = Env::default();
    env.mock_all_auths_allowing_non_root_auth();

    let admin = Address::generate(&env);
    let vault = Address::generate(&env);
    let issuer = Address::generate(&env);

    let sac = env.register_stellar_asset_contract_v2(issuer.clone());
    let token_admin = StellarAssetClient::new(&env, &sac.address());
    token_admin.mint(&vault, &10_000);

    let pool_id = env.register(MockBlendPool, ());
    let adapter_id = env.register(BlendAdapter, ());
    let adapter = BlendAdapterClient::new(&env, &adapter_id);
    adapter.initialize(&admin, &vault, &pool_id);

    let token = TokenClient::new(&env, &sac.address());
    token.transfer(&vault, &adapter_id, &4_000);
    adapter.supply(&sac.address(), &4_000);

    assert_eq!(adapter.supplied(&sac.address()), 4_000);
    assert_eq!(adapter.blend_supply_balance(&sac.address()), 4_000);
    assert_eq!(token.balance(&pool_id), 4_000);
}

#[test]
fn vault_withdraws_blend_supply() {
    let env = Env::default();
    env.mock_all_auths_allowing_non_root_auth();

    let admin = Address::generate(&env);
    let vault = Address::generate(&env);
    let issuer = Address::generate(&env);

    let sac = env.register_stellar_asset_contract_v2(issuer.clone());
    let token_admin = StellarAssetClient::new(&env, &sac.address());
    token_admin.mint(&vault, &10_000);

    let pool_id = env.register(MockBlendPool, ());
    let adapter_id = env.register(BlendAdapter, ());
    let adapter = BlendAdapterClient::new(&env, &adapter_id);
    adapter.initialize(&admin, &vault, &pool_id);

    let token = TokenClient::new(&env, &sac.address());
    token.transfer(&vault, &adapter_id, &3_000);
    adapter.supply(&sac.address(), &3_000);
    adapter.withdraw(&sac.address(), &1_000);

    assert_eq!(adapter.supplied(&sac.address()), 2_000);
    assert!(token.balance(&vault) >= 1_000);
}
