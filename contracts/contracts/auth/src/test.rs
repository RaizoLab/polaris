#![cfg(test)]

use super::*;
use soroban_sdk::{testutils::Address as _, Address, Env};

#[test]
fn user_authorize_and_revoke_in_single_transaction() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let agent = Address::generate(&env);
    let user = Address::generate(&env);

    let auth_id = env.register(Auth, ());
    let auth = AuthClient::new(&env, &auth_id);
    auth.initialize(&admin, &agent);

    assert!(!auth.is_authorized(&user));
    auth.authorize(&user);
    assert!(auth.is_authorized(&user));

    auth.revoke(&user);
    assert!(!auth.is_authorized(&user));
}

#[test]
fn assert_is_agent_succeeds_for_delegated_agent() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let agent = Address::generate(&env);

    let auth_id = env.register(Auth, ());
    let auth = AuthClient::new(&env, &auth_id);
    auth.initialize(&admin, &agent);

    auth.assert_is_agent(&agent);
}

#[test]
#[should_panic(expected = "caller is not the delegated agent")]
fn assert_is_agent_rejects_non_agent() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let agent = Address::generate(&env);
    let impostor = Address::generate(&env);

    let auth_id = env.register(Auth, ());
    let auth = AuthClient::new(&env, &auth_id);
    auth.initialize(&admin, &agent);

    auth.assert_is_agent(&impostor);
}

#[test]
fn admin_whitelists_protocols() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let agent = Address::generate(&env);
    let pool = Address::generate(&env);

    let auth_id = env.register(Auth, ());
    let auth = AuthClient::new(&env, &auth_id);
    auth.initialize(&admin, &agent);

    assert!(!auth.is_protocol_whitelisted(&pool));
    auth.add_protocol(&admin, &pool);
    assert!(auth.is_protocol_whitelisted(&pool));

    auth.remove_protocol(&admin, &pool);
    assert!(!auth.is_protocol_whitelisted(&pool));
}
