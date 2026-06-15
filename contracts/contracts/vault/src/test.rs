#![cfg(test)]

use super::*;
use auth::Auth;
use mock_pool::MockPool;
use soroban_sdk::{
    testutils::Address as _,
    token::{StellarAssetClient, TokenClient},
    Address, Env,
};

struct TestContext<'a> {
    env: Env,
    admin: Address,
    agent: Address,
    user: Address,
    vault_id: Address,
    pool_a: Address,
    pool_b: Address,
    token: TokenClient<'a>,
    vault: VaultClient<'a>,
    auth: auth::AuthClient<'a>,
    pool_a_client: mock_pool::MockPoolClient<'a>,
    pool_b_client: mock_pool::MockPoolClient<'a>,
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

        let auth_id = env.register(Auth, ());
        let auth = auth::AuthClient::new(&env, &auth_id);
        auth.initialize(&admin, &agent);

        let pool_a = env.register(MockPool, ());
        let pool_a_client = mock_pool::MockPoolClient::new(&env, &pool_a);
        pool_a_client.initialize(&admin, &sac.address());
        auth.add_protocol(&admin, &pool_a);

        let pool_b = env.register(MockPool, ());
        let pool_b_client = mock_pool::MockPoolClient::new(&env, &pool_b);
        pool_b_client.initialize(&admin, &sac.address());
        auth.add_protocol(&admin, &pool_b);

        let vault_id = env.register(Vault, ());
        let vault = VaultClient::new(&env, &vault_id);
        vault.initialize(&admin, &auth_id, &pool_a);

        Self {
            env,
            admin,
            agent,
            user,
            vault_id,
            pool_a,
            pool_b,
            token,
            vault,
            auth,
            pool_a_client,
            pool_b_client,
        }
    }
}

#[test]
fn user_revokes_agent_delegation_in_single_transaction() {
    let ctx = TestContext::setup();

    ctx.auth.authorize(&ctx.user);
    assert!(ctx.auth.is_authorized(&ctx.user));

    ctx.auth.revoke(&ctx.user);
    assert!(!ctx.auth.is_authorized(&ctx.user));
}

#[test]
fn deposit_updates_persistent_balance() {
    let ctx = TestContext::setup();

    ctx.vault
        .deposit(&ctx.user, &ctx.token.address, &1_000);

    assert_eq!(ctx.vault.balance(&ctx.user, &ctx.token.address), 1_000);
    assert_eq!(ctx.token.balance(&ctx.vault_id), 1_000);
    assert_eq!(ctx.token.balance(&ctx.user), 9_000);
}

#[test]
fn withdraw_updates_persistent_balance() {
    let ctx = TestContext::setup();

    ctx.vault
        .deposit(&ctx.user, &ctx.token.address, &1_000);
    ctx.vault
        .withdraw(&ctx.user, &ctx.token.address, &400);

    assert_eq!(ctx.vault.balance(&ctx.user, &ctx.token.address), 600);
    assert_eq!(ctx.token.balance(&ctx.vault_id), 600);
    assert_eq!(ctx.token.balance(&ctx.user), 9_400);
}

#[test]
#[should_panic(expected = "insufficient balance")]
fn withdraw_rejects_overdraft() {
    let ctx = TestContext::setup();

    ctx.vault
        .deposit(&ctx.user, &ctx.token.address, &500);
    ctx.vault
        .withdraw(&ctx.user, &ctx.token.address, &501);
}

#[test]
fn agent_deposits_vault_liquidity_into_whitelisted_pool() {
    let ctx = TestContext::setup();

    ctx.vault
        .deposit(&ctx.user, &ctx.token.address, &2_000);
    ctx.vault.agent_deposit_to_pool(&ctx.pool_a, &1_500);

    assert_eq!(ctx.vault.pool_deposited(&ctx.pool_a), 1_500);
    assert_eq!(ctx.pool_a_client.total_liquidity(), 1_500);
    assert_eq!(ctx.token.balance(&ctx.vault_id), 500);
    assert_eq!(ctx.token.balance(&ctx.pool_a), 1_500);
    assert_eq!(ctx.vault.balance(&ctx.user, &ctx.token.address), 2_000);
}

#[test]
fn agent_rebalances_between_whitelisted_pools_without_user() {
    let ctx = TestContext::setup();

    ctx.vault
        .deposit(&ctx.user, &ctx.token.address, &3_000);
    ctx.vault.agent_deposit_to_pool(&ctx.pool_a, &2_000);
    ctx.vault
        .rebalance(&ctx.pool_a, &ctx.pool_b, &800);

    assert_eq!(ctx.vault.pool_deposited(&ctx.pool_a), 1_200);
    assert_eq!(ctx.vault.pool_deposited(&ctx.pool_b), 800);
    assert_eq!(ctx.pool_a_client.total_liquidity(), 1_200);
    assert_eq!(ctx.pool_b_client.total_liquidity(), 800);
    assert_eq!(ctx.token.balance(&ctx.vault_id), 1_000);
}

#[test]
#[should_panic(expected = "protocol not whitelisted")]
fn agent_cannot_deposit_to_unlisted_pool() {
    let ctx = TestContext::setup();
    let rogue_pool = Address::generate(&ctx.env);

    ctx.vault
        .deposit(&ctx.user, &ctx.token.address, &1_000);
    ctx.vault.agent_deposit_to_pool(&rogue_pool, &100);
}

#[test]
#[should_panic(expected = "insufficient vault liquidity")]
fn withdraw_fails_when_liquidity_is_in_pool() {
    let ctx = TestContext::setup();

    ctx.vault
        .deposit(&ctx.user, &ctx.token.address, &1_000);
    ctx.vault.agent_deposit_to_pool(&ctx.pool_a, &1_000);
    ctx.vault
        .withdraw(&ctx.user, &ctx.token.address, &1);
}
