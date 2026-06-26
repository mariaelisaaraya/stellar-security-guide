// =============================================================================
//  fixed-vault — the corrected version of vulnerable-vault
//
//  Every fix is tagged with the check number it addresses from the
//  soroban-common-mistakes skill. Compare side-by-side with
//  examples/vulnerable-vault/src/lib.rs to see what changed and why.
// =============================================================================

#![cfg_attr(not(test), no_std)]
use soroban_sdk::{
    contract, contractimpl, contracttype, panic_with_error, symbol_short, token, Address, Env,
};

// TTL constants: extend Persistent balance entries proactively (~30 days).
const BALANCE_THRESHOLD: u32 = 100;
const BALANCE_EXTEND_TO: u32 = 518_400;
// FIX #5b: Instance storage (Admin, Token, FeeBps) also needs TTL extension.
// Without this, a dormant vault archives its own config, blocking all withdrawals
// until a separate restore_footprint transaction is submitted.
const INSTANCE_THRESHOLD: u32 = 100;
const INSTANCE_EXTEND_TO: u32 = 518_400;

// FIX #17: typed error enum instead of bare panic! / expect()
// Integrators and fuzzers can distinguish every failure mode.
#[soroban_sdk::contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
#[repr(u32)]
pub enum Error {
    AlreadyInitialized  = 1,
    NotInitialized      = 2,
    InvalidAmount       = 3,
    InvalidFee          = 4,
    InsufficientBalance = 5,
    Overflow            = 6,
}

// FIX #6 (implicit): all storage keys live in a typed enum.
// No raw symbol_short!("fee") keys that can silently collide.
#[contracttype]
#[derive(Clone)]
pub enum DataKey {
    Admin,
    Token,
    FeeBps,
    Balance(Address), // FIX #4: per-user key → goes in Persistent storage
}

#[contract]
pub struct VaultContract;

// Helper: load admin or raise NotInitialized — avoids repeating the pattern.
// FIX #18: no bare unwrap() on critical storage reads.
fn get_admin(env: &Env) -> Address {
    env.storage()
        .instance()
        .get(&DataKey::Admin)
        .unwrap_or_else(|| panic_with_error!(env, Error::NotInitialized))
}

fn get_token(env: &Env) -> Address {
    env.storage()
        .instance()
        .get(&DataKey::Token)
        .unwrap_or_else(|| panic_with_error!(env, Error::NotInitialized))
}

// FIX #5b: called at the top of every public entry point so the instance
// (Admin, Token, FeeBps) never archives while the vault is in active use.
fn bump_instance(env: &Env) {
    env.storage()
        .instance()
        .extend_ttl(INSTANCE_THRESHOLD, INSTANCE_EXTEND_TO);
}

#[contractimpl]
impl VaultContract {
    // FIX #2: reinitialization guard — can only run once.
    pub fn initialize(env: Env, admin: Address, token: Address) {
        if env.storage().instance().has(&DataKey::Admin) {
            panic_with_error!(&env, Error::AlreadyInitialized);
        }
        bump_instance(&env); // FIX #5b
        env.storage().instance().set(&DataKey::Admin, &admin);
        env.storage().instance().set(&DataKey::Token, &token);
        env.storage().instance().set(&DataKey::FeeBps, &0_i128);
    }

    pub fn deposit(env: Env, from: Address, amount: i128) {
        from.require_auth();
        bump_instance(&env); // FIX #5b

        // FIX #10: reject zero or negative amounts.
        if amount <= 0 {
            panic_with_error!(&env, Error::InvalidAmount);
        }

        // FIX #4: per-user balance lives in Persistent, not Instance.
        // Instance is loaded in full on every call and has a single TTL —
        // growing per-user data there inflates every invocation.
        let current: i128 = env
            .storage()
            .persistent()
            .get(&DataKey::Balance(from.clone()))
            .unwrap_or(0);

        // FIX #8 + #11: checked arithmetic + accumulate (not overwrite).
        let new_balance = current
            .checked_add(amount)
            .unwrap_or_else(|| panic_with_error!(&env, Error::Overflow));

        env.storage()
            .persistent()
            .set(&DataKey::Balance(from.clone()), &new_balance);

        // FIX #5: proactively extend TTL so the entry never archives.
        env.storage().persistent().extend_ttl(
            &DataKey::Balance(from.clone()),
            BALANCE_THRESHOLD,
            BALANCE_EXTEND_TO,
        );

        // FIX #19: emit event so off-chain indexers can track deposits.
        env.events()
            .publish((symbol_short!("deposit"), from), amount);

        // FIX #23: no depositor Vec, no loop — each user's balance is an
        // independent Persistent key; iteration is unnecessary.
    }

    // FIX #1: the owner of the balance must authorize their own withdrawal.
    // FIX #12: balance is decremented before the token transfer.
    pub fn withdraw(env: Env, to: Address, amount: i128) {
        to.require_auth();
        bump_instance(&env); // FIX #5b

        // FIX #10
        if amount <= 0 {
            panic_with_error!(&env, Error::InvalidAmount);
        }

        // FIX #18: typed helper, no bare unwrap.
        let token_id = get_token(&env);

        let balance: i128 = env
            .storage()
            .persistent()
            .get(&DataKey::Balance(to.clone()))
            .unwrap_or(0);

        // FIX #12: enforce solvency, then decrement.
        if balance < amount {
            panic_with_error!(&env, Error::InsufficientBalance);
        }
        let new_balance = balance
            .checked_sub(amount)
            .unwrap_or_else(|| panic_with_error!(&env, Error::Overflow));

        env.storage()
            .persistent()
            .set(&DataKey::Balance(to.clone()), &new_balance);

        env.storage().persistent().extend_ttl(
            &DataKey::Balance(to.clone()),
            BALANCE_THRESHOLD,
            BALANCE_EXTEND_TO,
        );

        let client = token::Client::new(&env, &token_id);
        client.transfer(&env.current_contract_address(), &to, &amount);

        // FIX #19
        env.events()
            .publish((symbol_short!("withdraw"), to), amount);
    }

    // FIX #1: only admin can change the fee.
    // FIX #10: fee must be in range 0..=10_000 (0 %..=100 %).
    pub fn set_fee(env: Env, fee_bps: i128) {
        get_admin(&env).require_auth();
        bump_instance(&env); // FIX #5b

        if !(0..=10_000).contains(&fee_bps) {
            panic_with_error!(&env, Error::InvalidFee);
        }

        env.storage().instance().set(&DataKey::FeeBps, &fee_bps);
        // FIX #19: emit event so off-chain monitors detect admin fee changes.
        env.events().publish((symbol_short!("set_fee"),), fee_bps);
    }

    // FIX #9: multiply before dividing to avoid early truncation.
    // amount * fee_bps / 10_000  ≠  amount / 10_000 * fee_bps
    pub fn calculate_fee(env: Env, amount: i128) -> i128 {
        bump_instance(&env); // FIX #5b
        let fee_bps: i128 = env
            .storage()
            .instance()
            .get(&DataKey::FeeBps)
            .unwrap_or(0);

        amount
            .checked_mul(fee_bps)
            .map(|v| v / 10_000)
            .unwrap_or_else(|| panic_with_error!(&env, Error::Overflow))
    }

    // FIX #17: returns 0 for unknown users instead of panicking with an
    // opaque expect() message that integrators cannot catch.
    pub fn get_balance(env: Env, user: Address) -> i128 {
        bump_instance(&env); // FIX #5b
        env.storage()
            .persistent()
            .get(&DataKey::Balance(user))
            .unwrap_or(0)
    }
}

// FIX #20: test suite covering the four critical behavioral paths.
#[cfg(test)]
mod test {
    use super::*;
    use soroban_sdk::{testutils::Address as _, Env};

    /// Register a vault, mock all auths, initialize with a stub token, and
    /// return (contract_id, admin, token_stub) so callers can build a client.
    fn create_vault(env: &Env) -> (Address, Address, Address) {
        env.mock_all_auths();
        let admin = Address::generate(env);
        let token = Address::generate(env);
        let vault_id = env.register_contract(None, VaultContract);
        VaultContractClient::new(env, &vault_id).initialize(&admin, &token);
        (vault_id, admin, token)
    }

    /// A second call to initialize must be rejected with AlreadyInitialized.
    #[test]
    fn reinitialization_rejected() {
        let env = Env::default();
        let (vault_id, admin, token) = create_vault(&env);
        let client = VaultContractClient::new(&env, &vault_id);
        assert!(client.try_initialize(&admin, &token).is_err());
    }

    /// Two deposits from the same user must accumulate, not overwrite.
    #[test]
    fn double_deposit_accumulates() {
        let env = Env::default();
        let (vault_id, _, _) = create_vault(&env);
        let client = VaultContractClient::new(&env, &vault_id);
        let user = Address::generate(&env);
        client.deposit(&user, &100_i128);
        client.deposit(&user, &50_i128);
        assert_eq!(client.get_balance(&user), 150_i128);
    }

    /// Withdrawing more than the stored balance must fail before any token
    /// transfer, and the stored balance must remain intact.
    #[test]
    fn double_withdrawal_blocked() {
        let env = Env::default();
        let (vault_id, _, _) = create_vault(&env);
        let client = VaultContractClient::new(&env, &vault_id);
        let user = Address::generate(&env);
        client.deposit(&user, &100_i128);
        assert!(client.try_withdraw(&user, &101_i128).is_err());
        assert_eq!(client.get_balance(&user), 100_i128);
    }

    /// withdraw must enforce require_auth; a call without authorization must
    /// be rejected before any state is read or mutated.
    #[test]
    fn unauthorized_withdraw_rejected() {
        let env = Env::default();
        // initialize() has no require_auth, so no mock needed for setup.
        let admin = Address::generate(&env);
        let token = Address::generate(&env);
        let vault_id = env.register_contract(None, VaultContract);
        let client = VaultContractClient::new(&env, &vault_id);
        client.initialize(&admin, &token);
        // No mock_all_auths → require_auth() in withdraw will fail.
        let attacker = Address::generate(&env);
        assert!(client.try_withdraw(&attacker, &1_i128).is_err());
    }
}
