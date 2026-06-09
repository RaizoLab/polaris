#![cfg(test)]

use super::*;
use mock_pool::MockPool;
use soroban_sdk::{
    testutils::Address as _,
    token::{StellarAssetClient, TokenClient},
    Address, Env,
};

struct TestContext<'a> {
    agent: Address,
    user: Address,
    vault_id: Address,
    pool_id: Address,
    token: TokenClient<'a>,
    vault: VaultClient<'a>,
    pool: mock_pool::MockPoolClient<'a>,
}

impl TestContext<'_> {
    fn setup() -> Self {
        let env = Env::default();
        env.mock_all_auths();

        let admin = Address::generate(&env);
        let agent = Address::generate(&env);
        let user = Address::generate(&env);

        let sac = env.register_stellar_asset_contract_v2(admin.clone());
        let token_admin = StellarAssetClient::new(&env, &sac.address());
        token_admin.mint(&user, &10_000);

        let token = TokenClient::new(&env, &sac.address());

        let pool_id = env.register(MockPool, ());
        let pool = mock_pool::MockPoolClient::new(&env, &pool_id);
        pool.initialize(&admin, &sac.address());

        let vault_id = env.register(Vault, ());
        let vault = VaultClient::new(&env, &vault_id);
        vault.initialize(&admin, &agent, &sac.address(), &pool_id);

        Self {
            agent,
            user,
            vault_id,
            pool_id,
            token,
            vault,
            pool,
        }
    }
}

#[test]
fn deposit_updates_persistent_balance() {
    let ctx = TestContext::setup();

    ctx.vault.deposit(&ctx.user, &1_000);

    assert_eq!(ctx.vault.balance(&ctx.user), 1_000);
    assert_eq!(ctx.token.balance(&ctx.vault_id), 1_000);
    assert_eq!(ctx.token.balance(&ctx.user), 9_000);
}

#[test]
fn withdraw_updates_persistent_balance() {
    let ctx = TestContext::setup();

    ctx.vault.deposit(&ctx.user, &1_000);
    ctx.vault.withdraw(&ctx.user, &400);

    assert_eq!(ctx.vault.balance(&ctx.user), 600);
    assert_eq!(ctx.token.balance(&ctx.vault_id), 600);
    assert_eq!(ctx.token.balance(&ctx.user), 9_400);
}

#[test]
#[should_panic(expected = "insufficient balance")]
fn withdraw_rejects_overdraft() {
    let ctx = TestContext::setup();

    ctx.vault.deposit(&ctx.user, &500);
    ctx.vault.withdraw(&ctx.user, &501);
}

#[test]
fn agent_deposits_vault_liquidity_into_mock_pool() {
    let ctx = TestContext::setup();

    ctx.vault.deposit(&ctx.user, &2_000);
    ctx.vault.agent_deposit_to_pool(&1_500);

    assert_eq!(ctx.vault.pool_deposited(), 1_500);
    assert_eq!(ctx.pool.total_liquidity(), 1_500);
    assert_eq!(ctx.token.balance(&ctx.vault_id), 500);
    assert_eq!(ctx.token.balance(&ctx.pool_id), 1_500);
    assert_eq!(ctx.vault.balance(&ctx.user), 2_000);
    assert_eq!(ctx.vault.get_agent(), ctx.agent);
}

#[test]
#[should_panic(expected = "insufficient vault liquidity")]
fn agent_cannot_deposit_more_than_vault_holds() {
    let ctx = TestContext::setup();

    ctx.vault.deposit(&ctx.user, &500);
    ctx.vault.agent_deposit_to_pool(&501);
}

#[test]
#[should_panic(expected = "insufficient vault liquidity")]
fn withdraw_fails_when_liquidity_is_in_pool() {
    let ctx = TestContext::setup();

    ctx.vault.deposit(&ctx.user, &1_000);
    ctx.vault.agent_deposit_to_pool(&1_000);
    ctx.vault.withdraw(&ctx.user, &1);
}
