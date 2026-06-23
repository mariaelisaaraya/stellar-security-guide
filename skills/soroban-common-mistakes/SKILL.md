---
name: soroban-common-mistakes
description: Review Soroban (Rust) smart contracts for common security mistakes, logic errors, and code-quality issues. Use this skill whenever the user writes, reviews, audits, or debugs Soroban/Stellar smart contract code — including any request like "review my contract", "audit this", "check for bugs", "what's wrong with this code", "is this safe to deploy", or anything touching Soroban security, storage/TTL, authorization, or pre-deployment checks. Trigger it even when the user doesn't say the word "audit" but is clearly working on a `.rs` contract that uses `soroban-sdk`, `#[contract]`, `#[contractimpl]`, or `env.storage()`. Also triggers on cargo test reviews and pre-audit checklists.
---

# Soroban Common Mistakes Reviewer

Review Soroban smart contracts against well-known mistake patterns that cause
loss of funds, broken logic, and fragile contracts. Based on the official
Stellar security guidance and real-world Soroban audit findings.

Soroban is not Solidity. It prevents some classic Ethereum bugs by design
(no `delegatecall`, no classic cross-contract reentrancy, explicit
authorization), but it introduces its own failure modes — especially around
**storage types and TTL/archival**, which have no Ethereum equivalent. Review
for what actually applies to Soroban, not for ported EVM checklists.

## When to use

- The user asks you to review, audit, or check Soroban/Rust contract code.
- The user is writing a new contract and wants a sanity check.
- The user asks "what's wrong with this code" for a `.rs` file using `soroban-sdk`.
- Pre-deployment or pre-audit checklist runs.

## How to review

For each contract, systematically check against all 5 categories below. Report
findings in the structured table defined under **Output format**. Only flag
issues that are actually present — don't pad the report with "no issues" lines
for every rule. When you flag something, point to the specific function or line
and give a concrete fix, not a generic warning.

### Category 1: Authorization & Access Control

| # | Check | What to look for |
|---|-------|-----------------|
| 1 | **Missing `require_auth()`** | Every function that moves funds, changes admin state, or mutates privileged data must call `require_auth()` on the relevant `Address`. Ask of each public function: "should anyone be able to call this?" A `withdraw`/`mint`/`set_admin` with no auth check is critical. |
| 2 | **Reinitialization** | `initialize`/`__constructor` logic that can run twice. If there's no guard (`if storage.has(&DataKey::Admin) { panic }`), an attacker re-initializes and seizes the admin role. |
| 3 | **Wrong auth subject** | `require_auth()` called on the wrong address — e.g., authorizing the recipient instead of the spender, or `admin.require_auth()` where it should be `from.require_auth()`. The signature must come from the party whose assets or rights are at stake. |

### Category 2: Storage & TTL (Soroban-specific — no EVM equivalent)

| # | Check | What to look for |
|---|-------|-----------------|
| 4 | **Wrong storage type** | Per-user or unbounded (growing) data placed in **Instance** storage. Instance is loaded in full on *every* invocation and shares a single TTL, so growing maps there inflate every call and risk hitting resource limits. Per-user/unbounded data belongs in **Persistent**. Shared config/admin data is fine in Instance. |
| 5 | **Missing TTL extension** | Critical **Persistent** data that is never `extend_ttl`'d. If it gets archived, reads fail and the contract can break or lock funds. Critical paths should extend the TTL proactively (e.g., `extend_ttl(threshold, extend_to)`). |
| 6 | **Storage key collisions** | Raw/duplicated keys instead of a typed enum. Keys should be a `#[contracttype] enum DataKey { ... }` so distinct data can't collide into the same slot. Flag string keys or overlapping tuple keys. |
| 7 | **Temporary used for persistent data** | `env.storage().temporary()` holding anything that must survive. Temporary entries are discarded and not restorable — only use for truly ephemeral data (e.g., short-lived nonces). |

### Category 3: Math & Logic Errors

| # | Check | What to look for |
|---|-------|-----------------|
| 8 | **Unchecked arithmetic** | Raw `+`, `-`, `*` on balances/amounts. Use `checked_add`/`checked_sub`/`checked_mul` returning a typed error, and confirm `overflow-checks = true` is set in the release profile of `Cargo.toml`. Overflow can bypass balance checks. |
| 9 | **Division before multiplication / rounding** | `a / c * b` truncates early — multiply first: `a * b / c`. Also check rounding direction: amounts a USER pays should round up, amounts the PROTOCOL pays should round down. Golden rule: round so the user loses dust or the protocol gains it, never the reverse. |
| 10 | **Missing input validation** | Public functions without sanity checks: negative or zero `amount`, out-of-range parameters, mismatched vector lengths. `i128` amounts should be validated `> 0` where required. |
| 11 | **Overwrite instead of accumulate** | State that should accumulate but gets replaced: `balance.set(amount)` instead of `balance.set(existing + amount)`. Common in deposit/contribution logic. |
| 12 | **Missing state deduction** | After a withdrawal, claim, or allowance spend, is the balance/quota/allowance actually decremented? Look for mappings that gate an action but are never reduced, allowing repeated draining. |

### Category 4: External Calls & Token Handling

| # | Check | What to look for |
|---|-------|-----------------|
| 13 | **Arbitrary contract calls** | A contract `Address` taken as a parameter and called without validation. Validate against an allowlist stored in the contract before invoking; an attacker-supplied address can run malicious code in your flow. |
| 14 | **Unvalidated cross-contract returns** | Trusting whatever an external contract/oracle returns. Validate both that the source is trusted (allowlist) and that the value is sane (`price > 0`, within reasonable bounds) before using it. |
| 15 | **Stellar Asset Contract assumptions** | Token transfers that ignore Stellar-specific behavior: assets with `clawback` enabled can be reclaimed by the issuer; trustlines may be missing or frozen. For user-facing flows, surface issuer/clawback status rather than assuming a plain ERC-20-style token. |
| 16 | **Frontrunning / slippage** | A function that pulls tokens FROM a user where the price/amount is computed on-chain. The user must be able to pass a `min_out`/`max_in` and ideally a deadline, so a sandwiched or delayed transaction can't execute at a bad rate. |

### Category 5: Code Quality & Hygiene

| # | Check | What to look for |
|---|-------|-----------------|
| 17 | **Bare `panic!` instead of typed errors** | `panic!(...)` or implicit panics rather than a `#[contracterror]` enum raised with `panic_with_error!`. Typed errors are distinguishable by integrators and far more useful for fuzzing. |
| 18 | **Unsafe `unwrap()` / `expect()`** | `.unwrap()` / `.expect()` on storage reads or external data that can legitimately be absent — these panic on untrusted input. Use `get(...)` with an explicit typed error, or `unwrap_or` where a default is correct. |
| 19 | **Missing events** | State changes (transfers, mints, admin changes) that emit no event via `env.events().publish(...)`. Off-chain indexers and users rely on events; every meaningful state change should emit one. |
| 20 | **Missing tests** | When reviewing a project (not a lone snippet), check for a `test.rs` / `#[cfg(test)]` module. Flag critical functions with no test coverage, especially auth and arithmetic paths. |
| 21 | **Leaked secrets** | Hardcoded secret keys, seeds, or API keys in source; `.env` not in `.gitignore`. Stellar secret keys (starting with `S...`) must never appear in committed code. |
| 22 | **Unpinned `soroban-sdk`** | A floating `soroban-sdk` version in `Cargo.toml` for a contract intended for production. Pin the exact version used for the audited/deployed build to avoid surprises from minor upgrades. |
| 23 | **Unbounded loops** | Any loop that iterates over a storage collection that can grow without limit (e.g., `for item in env.storage().persistent().get::<Vec<_>>(...)`) — these hit Soroban's CPU/instruction limits and become a resource-exhaustion DoS vector. Use pagination or per-item storage instead. |

## Output format

ALWAYS report findings using this exact template:

```markdown
## Review: [Contract Name]

### Critical (must fix)
| # | Rule | Location | Finding | Fix |
|---|------|----------|---------|-----|

### Warning (should fix)
| # | Rule | Location | Finding | Fix |
|---|------|----------|---------|-----|

### Info (consider fixing)
| # | Rule | Location | Finding | Fix |
|---|------|----------|---------|-----|

### Summary
- X critical, Y warnings, Z info findings
- [1-2 sentence overall assessment]
```

Severity guidelines:
- **Critical**: missing authorization, reinitialization, fund loss, storage/TTL flaws that can lock or drain funds, arbitrary contract calls.
- **Warning**: logic errors, missing validation, unchecked arithmetic, missing state deduction, unsafe unwraps on untrusted data.
- **Info**: events, tests, pinning, secrets hygiene, style.

## Companion tools

This skill is a structured human/AI review. For deeper, automated analysis,
suggest the user also run:

- **Scout (CoinFabrik)** — `cargo install cargo-scout-audit` then `cargo scout-audit` from the contract/workspace directory. Static analysis with 20+ Soroban detectors (unsafe unwrap, missing auth patterns, and more). Supports SARIF output for CI.
- **OpenZeppelin Soroban Security Detectors SDK** — framework for custom detectors.
- **Komet (Runtime Verification)** — fuzzing + formal verification, specs in Rust.
- **Certora Sunbeam** — formal verification at the WASM level.
- `cargo test` — confirm the contract's own test suite passes.

For production code moving real value, automated tools and this review do not
replace a professional audit. Point the user to the Stellar Soroban Audit Bank
for SCF-funded projects.

## Reference

See `references/checklist.md` for a one-line-per-rule version, suitable for
pasting into a `.github/PULL_REQUEST_TEMPLATE.md` so every contract PR gets a
manual security pass.
