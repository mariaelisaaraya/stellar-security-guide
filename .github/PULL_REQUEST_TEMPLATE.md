<!--
  This template auto-loads on every pull request. For contract changes, tick the
  relevant boxes. Full guide: ../GUIDE.md  ·  Detailed checklist: ../skills/soroban-common-mistakes/references/checklist.md
-->

## What does this PR do?

<!-- Brief description of the change. -->

## Type of change

- [ ] Smart contract (Soroban / Rust)
- [ ] Node / infrastructure
- [ ] Docs / guide
- [ ] Other

---

## Soroban security checklist

> Only required if this PR touches contract code. Tick what applies; note any N/A.

**Authorization**
- [ ] Every fund-moving / privileged function calls `require_auth()` on the right address
- [ ] `initialize` / `__constructor` can only run once
- [ ] Auth is required from the party whose assets/rights are at stake

**Storage & TTL**
- [ ] Per-user / unbounded data in Persistent (not Instance); typed `DataKey` enum
- [ ] Critical Persistent data is proactively `extend_ttl`'d
- [ ] Nothing that must persist lives in Temporary storage

**Math & Logic**
- [ ] Checked arithmetic + `overflow-checks = true`; multiply before divide; rounding favors the protocol
- [ ] Inputs validated; totals accumulate (not overwrite); balances/allowances decremented after use

**External Calls & Tokens**
- [ ] External contract addresses validated against an allowlist; cross-contract returns range-checked
- [ ] Stellar Asset (clawback/trustline) behavior considered; slippage protection where price is on-chain

**Code Quality**
- [ ] Typed errors (`#[contracterror]` + `panic_with_error!`), no unsafe `unwrap()`
- [ ] Events emitted on state changes; tests cover auth + arithmetic paths
- [ ] No leaked secrets; `soroban-sdk` pinned; no unbounded loops

**Tooling**
- [ ] `cargo scout-audit` run locally and/or CI passing
