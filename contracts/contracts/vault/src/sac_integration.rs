#![cfg(test)]

use crate::sac::{balance, decimals, transfer};
use crate::{Vault, VaultClient};
use auth::Auth;
use mock_pool::MockPool;
use soroban_sdk::{
    testutils::Address as _,
    token::{StellarAssetClient, TokenClient},
    Address, Env,
};

const USDC_DECIMALS: u32 = 7;
const XLM_DECIMALS: u32 = 7;

fn mint_sac(env: &Env, issuer: &Address, holder: &Address, amount: i128) -> Address {
    let sac = env.register_stellar_asset_contract_v2(issuer.clone());
    let admin = StellarAssetClient::new(env, &sac.address());
    admin.mint(holder, &amount);
    sac.address()
}

fn setup_vault_for_token<'a>(
    env: &'a Env,
    admin: &Address,
    agent: &Address,
    token: &Address,
) -> (Address, VaultClient<'a>) {
    let auth_id = env.register(Auth, ());
    let auth = auth::AuthClient::new(env, &auth_id);
    auth.initialize(admin, agent);

    let pool_id = env.register(MockPool, ());
    let pool = mock_pool::MockPoolClient::new(env, &pool_id);
    pool.initialize(admin, token);
    auth.add_protocol(admin, &pool_id);

    let vault_id = env.register(Vault, ());
    let vault = VaultClient::new(env, &vault_id);
    vault.initialize(admin, &auth_id, &pool_id);
    (vault_id, vault)
}

#[test]
fn sac_transfer_through_vault_deposit_and_withdraw() {
    let env = Env::default();
    env.mock_all_auths();

    let issuer = Address::generate(&env);
    let user = Address::generate(&env);
    let admin = Address::generate(&env);
    let agent = Address::generate(&env);

    let token = mint_sac(&env, &issuer, &user, 1_000_000_000);
    let (vault_id, vault) = setup_vault_for_token(&env, &admin, &agent, &token);

    vault.deposit(&user, &token, &250_000_000);
    assert_eq!(balance(&env, &token, &vault_id), 250_000_000);
    assert_eq!(vault.balance(&user, &token), 250_000_000);

    vault.withdraw(&user, &token, &100_000_000);
    assert_eq!(balance(&env, &token, &user), 850_000_000);
    assert_eq!(vault.balance(&user, &token), 150_000_000);
}

#[test]
fn usdc_and_xlm_sac_tokens_are_supported() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let agent = Address::generate(&env);
    let user = Address::generate(&env);

    let usdc_issuer = Address::generate(&env);
    let xlm_issuer = Address::generate(&env);

    let usdc = mint_sac(&env, &usdc_issuer, &user, 5_000_000_000);
    let xlm = mint_sac(&env, &xlm_issuer, &user, 10_000_000_000);

    let (_, vault) = setup_vault_for_token(&env, &admin, &agent, &usdc);
    let usdc_client = TokenClient::new(&env, &usdc);
    let xlm_client = TokenClient::new(&env, &xlm);

    vault.deposit(&user, &usdc, &1_000_000_000);
    vault.deposit(&user, &xlm, &2_000_000_000);

    assert_eq!(vault.balance(&user, &usdc), 1_000_000_000);
    assert_eq!(vault.balance(&user, &xlm), 2_000_000_000);
    assert_eq!(usdc_client.decimals(), USDC_DECIMALS);
    assert_eq!(xlm_client.decimals(), XLM_DECIMALS);
}

#[test]
fn sac_wrap_and_unwrap_classic_asset_flow() {
    let env = Env::default();
    env.mock_all_auths();

    let issuer = Address::generate(&env);
    let distributor = Address::generate(&env);
    let holder = Address::generate(&env);
    let admin = Address::generate(&env);
    let agent = Address::generate(&env);

    // Wrapping: SAC deployment represents a classic asset on Soroban.
    let sac = env.register_stellar_asset_contract_v2(issuer.clone());
    let token = sac.address();
    let sac_admin = StellarAssetClient::new(&env, &token);
    sac_admin.mint(&distributor, &1_000_000_000);

    assert_eq!(decimals(&env, &token), 7);
    assert_eq!(balance(&env, &token, &distributor), 1_000_000_000);

    // Move wrapped tokens into Soroban circulation (vault deposit).
    transfer(&env, &token, &distributor, &holder, 400_000_000);
    let (vault_id, vault) = setup_vault_for_token(&env, &admin, &agent, &token);
    vault.deposit(&holder, &token, &300_000_000);

    assert_eq!(balance(&env, &token, &vault_id), 300_000_000);
    assert_eq!(balance(&env, &token, &holder), 100_000_000);

    // Unwrapping: withdraw SAC back to the holder's account (classic-side custody).
    vault.withdraw(&holder, &token, &200_000_000);
    assert_eq!(balance(&env, &token, &holder), 300_000_000);
    assert_eq!(vault.balance(&holder, &token), 100_000_000);

    // Remaining classic + Soroban balances still reconcile to total supply.
    let total_in_accounts =
        balance(&env, &token, &distributor) + balance(&env, &token, &holder) + balance(&env, &token, &vault_id);
    assert_eq!(total_in_accounts, 1_000_000_000);
}
