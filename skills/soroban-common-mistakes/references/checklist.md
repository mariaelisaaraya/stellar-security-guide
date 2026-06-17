# Soroban Common Mistakes — Quick Reference Checklist

Copy this into your PR review template (`.github/PULL_REQUEST_TEMPLATE.md`) or
pre-deploy checklist, so every contract PR gets a manual security pass alongside
automated tools like Scout.

## Authorization & Access Control

* **require_auth present**: Every fund-moving or privileged function calls `require_auth()` on the address whose assets/rights are at stake
* **No reinitialization**: `initialize`/`__constructor` can only run once (guarded by a stored flag, or relying on Soroban's one-time `__constructor` semantics)
* **Correct auth subject**: Auth is required from the spender/owner being charged — not the recipient or an arbitrary downstream address

## Storage & TTL

* **Right storage type**: Per-user or unbounded data lives in Persistent; only shared config/admin data lives in Instance
* **TTL extended**: Critical Persistent entries are proactively `extend_ttl`'d so they don't archive out from under the contract
* **Typed keys**: Storage uses a `#[contracttype] enum DataKey` — no string or overlapping keys that can collide
* **Temporary is ephemeral-only**: Nothing that must survive is stored in Temporary storage

## Math & Logic

* **Checked arithmetic**: Balances/amounts use `checked_add` / `checked_sub` / `checked_mul`; `overflow-checks = true` set in the release profile
* **Multiply before divide**: No expression divides before multiplying (`a * b / c`, not `a / c * b`)
* **Rounding direction**: Protocol payouts round down; user payments round up — rounding always favors the protocol
* **Input validation**: All public function params validated (amounts `> 0`, sane ranges, matching vector lengths)
* **Accumulate, don't overwrite**: Running totals use `+=`-style logic rather than replacement
* **State deduction**: Allowances, quotas, and balances are decremented after use

## External Calls & Tokens

* **Allowlisted contract calls**: External `Address` params are validated against an allowlist before being invoked
* **Validated cross-contract returns**: Oracle/contract return values are range-checked and the source is trusted before use
* **Stellar Asset behavior handled**: Clawback and trustline status considered for token flows (not assumed to behave like a plain ERC-20)
* **Slippage protection**: Users can specify `min_out` / `max_in` (and a deadline) when price or amount is computed on-chain

## Code Quality

* **Typed errors**: Uses `#[contracterror]` + `panic_with_error!`, not bare `panic!`
* **No unsafe unwraps**: No `.unwrap()` / `.expect()` on absent-able or untrusted data; uses typed errors or `unwrap_or`
* **Events emitted**: Every meaningful state change publishes an event via `env.events().publish(...)`; key fields are topics
* **Unit tests exist**: Meaningful `#[cfg(test)]` coverage for public functions, especially auth and arithmetic paths
* **No leaked secrets**: No `S...` secret keys or API keys in source; `.env` in `.gitignore`
* **Pinned soroban-sdk**: Exact `soroban-sdk` version pinned for production builds
* **No unbounded loops**: No loops over storage collections that grow without limit (resource-limit DoS risk)
