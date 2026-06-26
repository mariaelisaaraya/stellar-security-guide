// test-trigger
// =============================================================================
//  vulnerable-vault — an intentionally broken Soroban contract
//
//  DO NOT DEPLOY THIS CONTRACT. It exists solely to practice using the
//  soroban-common-mistakes review skill. It contains at least 12 deliberate
//  security issues across the five check categories.
//
//  HOW TO USE:
//  1. Install the skill:  ./install.sh  (from the repo root)
//  2. Open this file in Claude Code and ask:
//       "review this contract for security issues"
//       "run a pre-deployment checklist on this"
//  3. Claude will walk through all 23 patterns and flag the bugs below.
//
//  After reviewing, compare with examples/fixed-vault/src/lib.rs to see
//  the corrected version.
// =============================================================================

#![no_std]
use soroban_sdk::{
    contract, contractimpl, contracttype, symbol_short, token, Address, Env, Vec,
};

#[contracttype]
#[derive(Clone)]
pub enum DataKey {
    Admin,
    Token,
    Balance(Address),
    Depositors, // grows unboundedly — wrong place for this (see deposit())
}

#[contract]
pub struct VaultContract;

#[contractimpl]
impl VaultContract {
    // BUG #2 — No reinitialization guard. Anyone can call this again and
    // overwrite the admin, seizing control of the vault.
    pub fn initialize(env: Env, admin: Address, token: Address) {
        env.storage().instance().set(&DataKey::Admin, &admin);
        env.storage().instance().set(&DataKey::Token, &token);
    }

    pub fn deposit(env: Env, from: Address, amount: i128) {
        from.require_auth();

        // BUG #10 — No input validation. amount can be 0 or negative.

        // BUG #4 — Depositors list grows without limit in Instance storage.
        // Instance is loaded in full on every invocation; this inflates every
        // call and risks hitting resource limits. Should use Persistent storage
        // with a per-user key instead.
        let mut depositors: Vec<Address> = env
            .storage()
            .instance()
            .get(&DataKey::Depositors)
            .unwrap_or(Vec::new(&env));

        // BUG #11 — Overwrites balance instead of accumulating it.
        // A second deposit from the same user replaces their balance with
        // only the new amount.
        env.storage()
            .instance()
            .set(&DataKey::Balance(from.clone()), &amount);

        // BUG #8 — Unchecked arithmetic computing a fee.
        let fee: i128 = env
            .storage()
            .instance()
            .get(&symbol_short!("fee"))
            .unwrap_or(0_i128);
        let _net = amount - fee; // can underflow if fee > amount

        // BUG #9 — Division before multiplication truncates early.
        // Should be: amount * fee / 10000
        let _fee_amount = amount / 10000 * fee;

        // BUG #23 — Unbounded loop over a Vec that grows with every deposit.
        // If depositors grows large enough this hits the instruction limit.
        for _depositor in depositors.iter() {
            // imagine some per-depositor reward distribution here
        }

        depositors.push_back(from);
        env.storage()
            .instance()
            .set(&DataKey::Depositors, &depositors);

        // BUG #5  — No extend_ttl. Critical data will eventually archive,
        //           breaking balance reads for users who haven't transacted
        //           in a while.
        // BUG #19 — No event emitted for the deposit.
    }

    // BUG #1 — No require_auth. Anyone can withdraw any amount to any address.
    // BUG #12 — Balance is never decremented after the transfer, so the same
    //           funds can be withdrawn repeatedly.
    pub fn withdraw(env: Env, to: Address, amount: i128) {
        // BUG #18 — Unsafe unwrap. Panics with an opaque error if Token was
        //           never set (e.g., initialize was never called).
        let token_id: Address = env.storage().instance().get(&DataKey::Token).unwrap();

        let client = token::Client::new(&env, &token_id);
        client.transfer(&env.current_contract_address(), &to, &amount);
        // BUG #19 — No event emitted for the withdrawal.
    }

    // BUG #1 — No require_auth. Anyone can change the vault fee.
    // BUG #10 — fee_bps is not validated; it can be set to a negative value
    //           or above 10 000 (100 %), draining depositors.
    pub fn set_fee(env: Env, fee_bps: i128) {
        env.storage()
            .instance()
            .set(&symbol_short!("fee"), &fee_bps);
    }

    pub fn get_balance(env: Env, user: Address) -> i128 {
        // BUG #17 — Bare expect() (typed as a panic) instead of a
        //           #[contracterror] enum raised with panic_with_error!.
        //           Integrators cannot distinguish this failure from others.
        env.storage()
            .instance()
            .get(&DataKey::Balance(user))
            .expect("user has no balance")
    }
}
