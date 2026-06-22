#![cfg(test)]

use super::*;
use soroban_sdk::{
    testutils::{Address as _, Ledger},
    token::{StellarAssetClient, TokenClient},
    vec, Address, Env,
};

#[test]
fn supply_and_withdraw_with_interest_accrual() {
    let env = Env::default();
    env.mock_all_auths();

    let user = Address::generate(&env);
    let issuer = Address::generate(&env);
    let sac = env.register_stellar_asset_contract_v2(issuer.clone());
    let token_admin = StellarAssetClient::new(&env, &sac.address());
    token_admin.mint(&user, &10_000);

    let pool_id = env.register(MockBlendPool, ());
    let pool = MockBlendPoolClient::new(&env, &pool_id);
    let token = TokenClient::new(&env, &sac.address());

    let requests = vec![
        &env,
        Request {
            request_type: REQUEST_SUPPLY,
            address: sac.address(),
            amount: 5_000,
        },
    ];
    pool.submit(&user, &user, &user, &requests);

    assert_eq!(pool.supply_balance(&user, &sac.address()), 5_000);

    env.ledger().set_sequence_number(env.ledger().sequence() + 200);
    assert!(pool.supply_balance(&user, &sac.address()) > 5_000);

    let withdraw = vec![
        &env,
        Request {
            request_type: REQUEST_WITHDRAW,
            address: sac.address(),
            amount: 2_000,
        },
    ];
    pool.submit(&user, &user, &user, &withdraw);
    assert!(token.balance(&user) >= 2_000);
}
