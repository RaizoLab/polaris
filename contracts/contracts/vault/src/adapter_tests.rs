#![cfg(test)]

use super::*;
use auth::Auth;
use blend_adapter::BlendAdapter;
use mock_blend_pool::MockBlendPool;
use mock_phoenix_pool::MockPhoenixPool;
use mock_pool::MockPool;
use phoenix_adapter::PhoenixAdapter;
use soroban_sdk::{
    testutils::Address as _,
    token::{StellarAssetClient, TokenClient},
    Address, Env,
};

struct AdapterTestContext<'a> {
    env: Env,
    vault_id: Address,
    usdc: Address,
    usdt: Address,
    blend_adapter_id: Address,
    phoenix_adapter_id: Address,
    phoenix_pool: Address,
    vault: VaultClient<'a>,
    blend_adapter: blend_adapter::BlendAdapterClient<'a>,
}

impl AdapterTestContext<'_> {
    fn setup() -> Self {
        let env = Env::default();
        env.mock_all_auths_allowing_non_root_auth();

        let admin = Address::generate(&env);
        let agent = Address::generate(&env);
        let user = Address::generate(&env);

        let usdc_sac = env.register_stellar_asset_contract_v2(Address::generate(&env));
        let usdt_sac = env.register_stellar_asset_contract_v2(Address::generate(&env));
        let usdc = usdc_sac.address();
        let usdt = usdt_sac.address();
        StellarAssetClient::new(&env, &usdc).mint(&user, &20_000);

        let auth_id = env.register(Auth, ());
        let auth = auth::AuthClient::new(&env, &auth_id);
        auth.initialize(&admin, &agent);

        let mock_pool = env.register(MockPool, ());
        mock_pool::MockPoolClient::new(&env, &mock_pool).initialize(&admin, &usdc);

        let vault_id = env.register(Vault, ());
        let vault = VaultClient::new(&env, &vault_id);
        vault.initialize(&admin, &auth_id, &mock_pool);

        let blend_pool = env.register(MockBlendPool, ());
        let blend_adapter_id = env.register(BlendAdapter, ());
        let blend_adapter = blend_adapter::BlendAdapterClient::new(&env, &blend_adapter_id);
        blend_adapter.initialize(&admin, &vault_id, &blend_pool);
        auth.add_protocol(&admin, &blend_adapter_id);

        let phoenix_pool = env.register(MockPhoenixPool, ());
        StellarAssetClient::new(&env, &usdt).mint(&phoenix_pool, &10_000);
        mock_phoenix_pool::MockPhoenixPoolClient::new(&env, &phoenix_pool).initialize(
            &admin,
            &usdc,
            &usdt,
            &30,
        );

        let phoenix_adapter_id = env.register(PhoenixAdapter, ());
        let phoenix_adapter = phoenix_adapter::PhoenixAdapterClient::new(&env, &phoenix_adapter_id);
        phoenix_adapter.initialize(&admin, &vault_id);
        auth.add_protocol(&admin, &phoenix_adapter_id);
        auth.add_protocol(&admin, &phoenix_pool);

        vault.deposit(&user, &usdc, &10_000);

        Self {
            env,
            vault_id,
            usdc,
            usdt,
            blend_adapter_id,
            phoenix_adapter_id,
            phoenix_pool,
            vault,
            blend_adapter,
        }
    }
}

#[test]
fn vault_supplies_usdc_to_blend_and_earns_interest() {
    let ctx = AdapterTestContext::setup();

    ctx.vault
        .agent_supply_to_blend(&ctx.blend_adapter_id, &ctx.usdc, &5_000);

    assert_eq!(ctx.vault.blend_supplied(&ctx.blend_adapter_id), 5_000);
    assert_eq!(ctx.blend_adapter.supplied(&ctx.usdc), 5_000);
    assert_eq!(ctx.blend_adapter.blend_supply_balance(&ctx.usdc), 5_000);
}

#[test]
fn agent_swaps_on_phoenix_with_slippage_protection() {
    let ctx = AdapterTestContext::setup();

    let ask = ctx.vault.agent_swap_phoenix(
        &ctx.phoenix_adapter_id,
        &ctx.phoenix_pool,
        &ctx.usdc,
        &ctx.usdt,
        &2_000,
        &1_980,
        &50,
    );

    assert_eq!(ask, 1_994);
    assert_eq!(
        TokenClient::new(&ctx.env, &ctx.usdt).balance(&ctx.vault_id),
        1_994
    );
}
