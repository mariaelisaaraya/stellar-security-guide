# Examples

Practice contracts for the `soroban-common-mistakes` review skill.

## vulnerable-vault

A Soroban vault contract with **12 deliberate security issues** spanning all
five check categories. Use it to see the skill in action before applying it to
your own code.

**Do not deploy this contract.** It will lose funds.

### How to use

1. Install the skill from the repo root:
   ```bash
   ./install.sh
   ```

2. Open `vulnerable-vault/src/lib.rs` in Claude Code and ask:
   ```
   review this contract for security issues
   ```
   or
   ```
   run a pre-deployment checklist on this file
   ```

3. Claude will work through all 23 patterns and report findings in a structured
   table. Compare its output against the bug comments in the source file.

### Issues embedded in vulnerable-vault

| Category | Check # | What's broken |
|----------|---------|---------------|
| Authorization | #1 | `withdraw` and `set_fee` have no `require_auth` |
| Authorization | #2 | `initialize` has no reinitialization guard |
| Storage | #4 | Depositor list (unbounded) stored in Instance |
| Storage | #5 | No `extend_ttl` on any data |
| Math | #8 | `amount - fee` without `checked_sub` |
| Math | #9 | `amount / 10000 * fee` divides before multiplying |
| Math | #10 | No input validation on `amount` or `fee_bps` |
| Math | #11 | `deposit` overwrites balance instead of accumulating |
| Math | #12 | `withdraw` never decrements the stored balance |
| External | —  | Token client called without any allowlist check |
| Quality | #17 | `get_balance` uses `.expect()` instead of typed error |
| Quality | #18 | `withdraw` uses `.unwrap()` on Token storage read |
| Quality | #19 | No events emitted on deposit or withdrawal |
| Quality | #22 | `soroban-sdk` version is floating, not pinned |
| Quality | #23 | Unbounded loop over depositors Vec in `deposit` |

### Contributing examples

Add a new subdirectory with its own `Cargo.toml` and `src/lib.rs`. A good
example targets a real contract pattern (escrow, lending, NFT, oracle) and
demonstrates at least 3–4 issues from different categories.
