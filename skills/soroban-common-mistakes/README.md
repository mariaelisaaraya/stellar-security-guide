# soroban-common-mistakes

An [Agent Skill](https://agentskills.io) for Claude Code (and any compatible AI
coding agent) that reviews **Soroban (Rust) smart contracts** against common
security, logic, and code-quality mistakes.

Adapted for Soroban from the structure of
[solidity-common-mistakes-skill](https://github.com/AAYUSH-GUPTA-coder/solidity-common-mistakes-skill),
with Soroban-specific rules sourced from the official
[Stellar dev skill](https://github.com/stellar/stellar-dev-skill) security
guidance. EVM-only checks (reentrancy/CEI, `tx.origin`, `transfer()` gas
stipend, SafeERC20, floating pragma) were dropped because they don't apply to
Soroban; Soroban-only checks (storage type, TTL/archival, reinitialization,
cross-contract validation) were added.

## What it does

When you ask Claude to review, audit, or debug a Soroban contract, this skill
activates and systematically checks for:

| Category | Checks |
|----------|--------|
| **Authorization** | Missing `require_auth()`, reinitialization, wrong auth subject |
| **Storage & TTL** | Wrong storage type, missing TTL extension, key collisions, misuse of Temporary |
| **Math & Logic** | Unchecked arithmetic, division-before-multiplication, input validation, overwrite-vs-accumulate, missing state deduction |
| **External Calls & Tokens** | Arbitrary contract calls, unvalidated cross-contract returns, Stellar Asset/clawback handling, slippage |
| **Code Quality** | Bare `panic!`, unsafe `unwrap()`, missing events/tests, leaked secrets, unpinned `soroban-sdk` |

Output is a severity-ranked findings table (Critical → Warning → Info) with
specific locations and fix recommendations.

## Install

### Claude Code

```bash
git clone <this-repo-url> ~/soroban-common-mistakes-skill
ln -s ~/soroban-common-mistakes-skill ~/.claude/skills/soroban-common-mistakes
```

Verify:

```bash
ls ~/.claude/skills/soroban-common-mistakes/SKILL.md
```

### Claude.ai (browser)

1. Download this folder as a ZIP (or use the provided `.skill` file).
2. Open [claude.ai](https://claude.ai).
3. Go to **Customize → Skills → "+" → Create skill** and upload it.
4. Toggle the skill **ON**.
5. Start a new chat, paste your Soroban code, and ask *"review this contract"*.

## Run

Navigate to a Soroban project and start Claude Code, then invoke naturally:

```
review this contract for common mistakes
```
```
audit contracts/marketplace/src/lib.rs
```
```
is this safe to deploy?
```
```
run a pre-deployment checklist on my contracts
```

## PR Review Checklist

`references/checklist.md` is a copy-paste-ready checklist for your
`.github/PULL_REQUEST_TEMPLATE.md`.

## Companion Tools

This skill suggests (but doesn't require) these for deeper analysis:

- [Scout](https://github.com/CoinFabrik/scout-soroban) — static analysis (`cargo scout-audit`)
- [OpenZeppelin Soroban Security Detectors](https://github.com/OpenZeppelin/soroban-security-detectors-sdk)
- [Komet](https://github.com/runtimeverification/komet) — fuzzing + formal verification
- [Certora Sunbeam](https://docs.certora.com/en/latest/docs/sunbeam/index.html) — formal verification

## Credits

- Structure inspired by [solidity-common-mistakes](https://github.com/AAYUSH-GUPTA-coder/solidity-common-mistakes-skill) by Aayush Gupta.
- Soroban security patterns from the official [Stellar Development Foundation](https://github.com/stellar/stellar-dev-skill) material.

## License

MIT
