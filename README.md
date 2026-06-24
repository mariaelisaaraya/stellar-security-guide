# Stellar Security Guide 🦈🛡️🦈

[![Rust](https://img.shields.io/badge/Rust-Soroban-CE422B?style=flat-square&logo=rust)](https://www.rust-lang.org)
[![Ansible](https://img.shields.io/badge/Ansible-hardening-EE0000?style=flat-square&logo=ansible)](https://www.ansible.com)
[![Docker](https://img.shields.io/badge/Docker-RPC%20node-2496ED?style=flat-square&logo=docker)](https://www.docker.com)
[![GitHub Actions](https://img.shields.io/badge/GitHub%20Actions-CI-2088FF?style=flat-square&logo=githubactions)](https://github.com/features/actions)
[![Claude Code](https://img.shields.io/badge/Claude%20Code-skill-D97706?style=flat-square)](https://anthropic.com)

🌎 [Versión en español](./GUIDE.es.md)

> A practical, open security toolkit for everyone building on **Stellar** and
> **Soroban** from the smart contracts that hold the money to the nodes that
> run the network. Written for the LATAM builder community, usable by anyone.

This is not just a document. It's a **guide + working automation + an AI review
skill**, so security isn't only something you read — part of it runs for you.

---

## Why this exists

On Stellar, code *is* money. A missing authorization check is an open wallet,
and an exposed node is a problem for everyone who depends on it. Good security
material for Soroban is still scarce (especially in Spanish) so this repo
gathers it in one place, cross-checked against official Stellar Development
Foundation sources, and keeps it practical.

---

## What's inside

| Path | What it is |
|------|-----------|
| 📖 **[`GUIDE.md`](./GUIDE.md)** | The full security guide. Part A: Soroban contract security. Part B: node hardening. Part C: automation & checklists. **Start here.** |
| 🔎 **[`skills/soroban-common-mistakes/`](./skills/soroban-common-mistakes/)** | A Claude Code skill that reviews Soroban contracts against 23 common mistake patterns. Installable in Claude Code or claude.ai. |
| 🧪 **[`examples/`](./examples/)** | Practice contracts: `vulnerable-vault` (15 deliberate bugs across all 5 skill categories) and `fixed-vault` (every bug corrected and tagged). Use them to try the skill before pointing it at your own code. |
| 🤖 **[`ansible/`](./ansible/)** | One-command server hardening. Provisions a fresh Ubuntu/Debian box into a hardened Stellar node (users, SSH, firewall, fail2ban, NTP, sysctl, auditing). |
| 🐳 **[`docker/`](./docker/)** | An isolated Stellar RPC node behind a reverse proxy, with the admin endpoint kept off the internet. `docker compose up` and you're running. |
| ⚙️ **[`.github/workflows/`](./.github/workflows/)** | A GitHub Action that runs [Scout](https://github.com/CoinFabrik/scout-soroban) on every contract PR and reports findings to the Security tab. |
| 📦 **[`install.sh`](./install.sh)** | One-command installer for the `soroban-common-mistakes` skill into Claude Code. |

```
stellar-security-guide/
├── README.md                         ← you are here (overview + index)
├── GUIDE.md                          ← the full security guide
├── install.sh                        ← one-command skill installer
├── ansible/
│   ├── harden-stellar-node.yml       ← hardening playbook
│   └── inventory.ini                 ← your server list (template)
├── docker/
│   ├── docker-compose.yml            ← isolated RPC node + reverse proxy
│   ├── Caddyfile                     ← HTTPS reverse proxy config
│   ├── prometheus.yml                ← Prometheus scrape config (monitoring)
│   └── .env.example                  ← copy to .env and fill in
├── examples/
│   ├── vulnerable-vault/             ← broken contract: 15 bugs to find
│   └── fixed-vault/                  ← same contract with all bugs resolved
├── .github/
│   ├── workflows/
│   │   └── scout-audit.yml           ← Scout CI on every PR
│   └── PULL_REQUEST_TEMPLATE.md      ← contract review checklist on every PR
└── skills/
    └── soroban-common-mistakes/      ← AI contract-review skill
        ├── SKILL.md
        ├── references/checklist.md
        ├── README.md
        └── LICENSE
```

---

## Quick start

**I'm writing a Soroban contract.**
Install the review skill with one command:
```bash
curl -fsSL https://raw.githubusercontent.com/mariaelisaaraya/stellar-security-guide/main/install.sh | bash
```
Then read [Part A of the guide](./GUIDE.md#part-a--soroban-smart-contract-security)
and run `cargo scout-audit` on your contract. Drop
[`.github/workflows/scout-audit.yml`](./.github/workflows/scout-audit.yml) into
your contracts repo so every PR is scanned automatically.

**I'm running a validator or RPC node.**
Read [Part B of the guide](./GUIDE.md#part-b--stellar-node-hardening), harden the
box with the [Ansible playbook](./ansible/), and deploy the RPC with the
[Docker setup](./docker/). Start on **testnet** before pubnet.

**I want to try the skill before using it on my own code.**
Open [`examples/vulnerable-vault/src/lib.rs`](./examples/vulnerable-vault/src/lib.rs)
in Claude Code and ask *"review this contract for security issues"*. The skill
will find the 15 deliberate bugs. Then compare with `examples/fixed-vault/` to
see every fix explained.

**I just want the checklist.**
Grab [`skills/soroban-common-mistakes/references/checklist.md`](./skills/soroban-common-mistakes/references/checklist.md)
— it's also wired into the PR template so it shows up on every pull request.

---

## Sources & trust

Every technical claim here was cross-checked against primary sources:
the official [Stellar Developers docs](https://developers.stellar.org), the
[`stellar/stellar-dev-skill`](https://github.com/stellar/stellar-dev-skill)
security material, and [CoinFabrik's Scout](https://github.com/CoinFabrik/scout-soroban)
documentation. Where this guide and an official source disagree, the official
source wins — open an issue and we'll fix it.

> ⚠️ This is community material, not a substitute for a professional audit. For
> contracts moving real value, see the
> [Soroban Audit Bank](https://stellar.org/grants-and-funding/soroban-audit-bank).

---

## Contributing

This guide is meant to be **living** — written by people who put real projects
into production on Stellar. You're warmly invited to contribute:

- Found something outdated or wrong? Open an issue.
- Run a node or shipped a contract? Add your hard-won lessons via a PR.
- New tooling worth listing? Send it our way.

All contributions are MIT-licensed. Be kind, cite sources, and keep it
practical.

---

## License

[MIT](./LICENSE) — use it, fork it, improve it.
