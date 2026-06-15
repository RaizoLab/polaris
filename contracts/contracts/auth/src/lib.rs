#![no_std]
use soroban_sdk::{contract, contractimpl, contracttype, symbol_short, Address, Env};

#[contracttype]
#[derive(Clone)]
pub enum DataKey {
    Admin,
    Agent,
    UserAuthorized(Address),
    Protocol(Address),
}

#[contract]
pub struct Auth;

#[contractimpl]
impl Auth {
    /// Configure the vault agent and admin.
    pub fn initialize(env: Env, admin: Address, agent: Address) {
        admin.require_auth();
        if env.storage().instance().has(&DataKey::Agent) {
            panic!("already initialized");
        }
        env.storage().instance().set(&DataKey::Admin, &admin);
        env.storage().instance().set(&DataKey::Agent, &agent);
    }

    /// User opts in to non-custodial delegation of the vault agent.
    pub fn authorize(env: Env, user: Address) {
        user.require_auth();
        env.storage()
            .persistent()
            .set(&DataKey::UserAuthorized(user.clone()), &true);
        env.events()
            .publish((symbol_short!("authorize"), user), ());
    }

    /// User revokes agent delegation in a single transaction.
    pub fn revoke(env: Env, user: Address) {
        user.require_auth();
        env.storage()
            .persistent()
            .remove(&DataKey::UserAuthorized(user.clone()));
        env.events()
            .publish((symbol_short!("revoke"), user), ());
    }

    pub fn is_authorized(env: Env, user: Address) -> bool {
        env.storage()
            .persistent()
            .get(&DataKey::UserAuthorized(user))
            .unwrap_or(false)
    }

    /// Admin whitelists a protocol the agent may rebalance across.
    pub fn add_protocol(env: Env, admin: Address, protocol: Address) {
        admin.require_auth();
        Self::require_admin(&env, &admin);
        env.storage()
            .persistent()
            .set(&DataKey::Protocol(protocol.clone()), &true);
        env.events()
            .publish((symbol_short!("whitelist"), protocol), ());
    }

    pub fn remove_protocol(env: Env, admin: Address, protocol: Address) {
        admin.require_auth();
        Self::require_admin(&env, &admin);
        env.storage()
            .persistent()
            .remove(&DataKey::Protocol(protocol.clone()));
        env.events()
            .publish((symbol_short!("rm_proto"), protocol), ());
    }

    pub fn is_protocol_whitelisted(env: Env, protocol: Address) -> bool {
        env.storage()
            .persistent()
            .get(&DataKey::Protocol(protocol))
            .unwrap_or(false)
    }

    /// Verify `caller` matches the configured agent (no nested auth).
    pub fn assert_is_agent(env: Env, caller: Address) {
        let agent: Address = env
            .storage()
            .instance()
            .get(&DataKey::Agent)
            .expect("not initialized");
        if caller != agent {
            panic!("caller is not the delegated agent");
        }
    }

    pub fn agent(env: Env) -> Address {
        env.storage()
            .instance()
            .get(&DataKey::Agent)
            .expect("not initialized")
    }

    pub fn admin(env: Env) -> Address {
        env.storage()
            .instance()
            .get(&DataKey::Admin)
            .expect("not initialized")
    }

    fn require_admin(env: &Env, admin: &Address) {
        let stored: Address = env
            .storage()
            .instance()
            .get(&DataKey::Admin)
            .expect("not initialized");
        if admin != &stored {
            panic!("not admin");
        }
    }
}

mod test;
