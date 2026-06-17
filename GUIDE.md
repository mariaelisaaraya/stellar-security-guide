# Stellar & Soroban Security Guide (LATAM edition 🌎)

> A practical security guide for **builders developing Soroban smart contracts**
> and for **operators running nodes** on the Stellar network. Written for the
> LATAM community — DeFiWise, Be-Energy, Lendara, and any project putting real
> money on Stellar.

**License:** MIT · **Contributions:** welcome! Send your PR.

---

## ⚠️ Before you start: two different things

This guide covers **two security layers that often get mixed up**:

| Layer | Who it's for | What it protects |
|-------|--------------|------------------|
| **Part A — Soroban Contracts** | Developers | Your contract from logic bugs that drain funds (auth, overflow, storage, TTL) |
| **Part B — Stellar Nodes** | Infra operators | Your server/validator/RPC from compromise (firewall, keys, ports) |

If you're writing contracts, your priority is **Part A**. If you run a validator
or an RPC for your dApp, add **Part B**.

> 📌 **We don't reinvent the wheel.** This guide builds on official material from
> the Stellar Development Foundation (SDF). When something is better explained
> there, we link instead of copying:
> - [`stellar/stellar-dev-skill`](https://github.com/stellar/stellar-dev-skill) — official skill with a Soroban security section.
> - [sorobansecurity.com](https://sorobansecurity.com) — community knowledge base (audit reports + vulnerability database).
> - [Stellar Developers Docs](https://developers.stellar.org).

---

## 1. Introduction

### Why security matters on Stellar

On Stellar, "code" and "money" are the same thing. A Soroban contract with a
missing authorization check is not a cosmetic bug: it's an open wallet. And a
poorly exposed validator or RPC doesn't only affect you — it affects everyone
depending on your infrastructure (oracles, pools, frontends).

### Threat model (the two layers)

**At the contract level**, threats are usually *logical*: privileged functions
without auth, reinitialization, arithmetic overflow, critical data archived due
to expired TTL, blind trust in external contracts or oracles.

**At the node level**, threats are *operational*: RPC/admin endpoints exposed to
the internet, unhardened SSH, leaked validator secrets (`NODE_SEED`),
denial-of-service attacks.

---

# PART A — Soroban Smart Contract Security

> This is the highest-impact section. Most losses in smart contracts come from
> logic errors, not from infrastructure.

## A.1 What Soroban gives you for free (and what it doesn't)

Soroban prevents by design some vulnerability classes typical of Ethereum:

- **No `delegatecall`** → proxy-based attacks that execute arbitrary bytecode don't exist.
- **No classical cross-contract reentrancy** → the execution model is synchronous. (Note: self-reentrancy is possible, though rarely exploitable.)
- **Explicit authorization** → nothing is authorized implicitly; you must call `require_auth()` on purpose.

What it does **not** give you for free: input validation, overflow handling,
storage TTL management, or validating who you're talking to. Those are 100% your
responsibility.

## A.2 Vulnerability classes (with code)

### 1) Missing authorization

The most common and most expensive bug. Anyone can call privileged functions.

```rust
// ❌ BAD: nobody verifies who is calling
pub fn withdraw(env: Env, to: Address, amount: i128) {
    transfer_tokens(&env, &to, amount);
}

// ✅ GOOD: requires the admin's signature
pub fn withdraw(env: Env, to: Address, amount: i128) {
    let admin: Address = env.storage().instance().get(&DataKey::Admin).unwrap();
    admin.require_auth();
    transfer_tokens(&env, &to, amount);
}
```

### 2) Reinitialization attacks

If `initialize` can be called twice, an attacker becomes the admin.

```rust
// ✅ GOOD: can only be initialized once
pub fn initialize(env: Env, admin: Address) {
    if env.storage().instance().has(&DataKey::Admin) {
        panic_with_error!(&env, Error::AlreadyInitialized);
    }
    env.storage().instance().set(&DataKey::Admin, &admin);
}
```

### 3) Arbitrary contract calls

Never trust any `Address` passed in as a parameter.

```rust
// ✅ GOOD: validate against a known allowlist
pub fn swap(env: Env, token: Address, amount: i128) {
    let allowed: Vec<Address> = env.storage().instance()
        .get(&DataKey::AllowedTokens).unwrap();
    if !allowed.contains(&token) {
        panic_with_error!(&env, Error::TokenNotAllowed);
    }
    // ... continue
}
```

### 4) Integer overflow / underflow

Always use checked arithmetic. Overflow can bypass balance checks.

```rust
// ✅ GOOD
let new_balance = balance.checked_add(amount)
    .ok_or(Error::Overflow)?;
```

> 💡 In `Cargo.toml`, enable `overflow-checks = true` in the release profile.

### 5) Storage key collisions

Different data sharing the same key = corruption.

```rust
// ✅ GOOD: typed enum for keys
#[contracttype]
#[derive(Clone)]
pub enum DataKey {
    Admin,
    Balance(Address),
    Config,
    Allowance(Address, Address),
}
```

### 6) Race conditions / state ordering

Perform checks and state changes atomically, leaving no gap between "validate"
and "act" (e.g., check slippage before transferring).

### 7) TTL / archival vulnerabilities

Critical data archived due to an expired TTL can break the contract. Extend the
TTL proactively in critical operations.

```rust
env.storage().instance().extend_ttl(100, 518400);          // ~30 days
env.storage().persistent().extend_ttl(&DataKey::CriticalData, 100, 518400);
```

### 8) Cross-contract return validation

Don't trust what an external contract returns (especially oracles). Validate
that the oracle is trusted **and** that the value makes sense.

```rust
if !trusted_oracles.contains(&oracle) {
    panic_with_error!(&env, Error::UntrustedOracle);
}
let price: i128 = oracle_client.get_price(&asset);
if price <= 0 || price > MAX_REASONABLE_PRICE {
    panic_with_error!(&env, Error::InvalidPrice);
}
```

## A.3 Storage: choose the right type

| Type | Use for | Cost / Lifetime |
|------|---------|-----------------|
| **Instance** | Shared / admin / config data | **Loaded entirely on every invocation** and shares **a single TTL** |
| **Persistent** | Per-user or growing data (balances, allowances) | TTL per key; restorable if archived |
| **Temporary** | Truly ephemeral data | Discarded; **never** use for anything that must persist |

> 🔑 **Golden rule:** *unbounded* (growing) or per-user data → **Persistent**,
> never Instance. If you put a growing map in Instance storage, you make *every*
> contract invocation more expensive and risk hitting resource limits.

## A.4 Error handling: typed errors, not bare `panic!`

Define errors with `#[contracterror]` and raise them with `panic_with_error!`.
You get structured, distinguishable errors that are far more useful for fuzzing
and for anyone integrating your contract.

```rust
#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
#[repr(u32)]
pub enum Error {
    AlreadyInitialized = 1,
    NotAuthorized = 2,
    Overflow = 3,
    InvalidAmount = 4,
    // ...
}
```

## A.5 "Classic" Stellar security (non-Soroban)

Even if you work on Soroban, your dApp touches the classic Stellar layer:

- **Malicious trustlines:** verify the issuer before creating a trustline. Always display the full asset code + issuer in the UI. Use known asset lists (`stellar.toml`).
- **Clawback:** some assets allow the issuer to seize them. Check `auth_clawback_enabled` on the issuer's account and warn the user.
- **Account merge:** a merged account can be recreated with a different configuration. Don't cache account state long-term for critical operations.

## A.6 Contract checklist (pre-deploy)

- [ ] Every privileged function enforces the appropriate `require_auth()`.
- [ ] Initialization can only happen once.
- [ ] External contract calls validated against an allowlist.
- [ ] All arithmetic uses checked operations (`checked_add`, etc.).
- [ ] Storage keys are typed (enum) and collision-free.
- [ ] Critical data TTLs extended proactively.
- [ ] Input validation on all public functions.
- [ ] Events emitted for every auditable state change.
- [ ] Typed errors via `#[contracterror]`.
- [ ] `overflow-checks = true` in release.

**Review questions:** Can anyone call admin functions without auth? Can it be
reinitialized? Are external calls validated? Is the arithmetic safe? Can keys
collide? Does critical data survive archival? Are cross-contract return values
validated?

## A.7 Security tooling

**Static analysis**
- **Scout (CoinFabrik)** — 23 detectors. `cargo install cargo-scout-audit` → `cargo scout-audit`. Output: HTML/MD/JSON/PDF/SARIF (CI/CD). VSCode extension available. → https://github.com/CoinFabrik/scout-soroban
- **OpenZeppelin Security Detectors SDK** — framework for custom detectors (missing auth, unchecked transfers, improper TTL, panics). → https://github.com/OpenZeppelin/soroban-security-detectors-sdk

**Formal verification**
- **Certora Sunbeam** — formal verification at the WASM level. → https://docs.certora.com/en/latest/docs/sunbeam/index.html
- **Komet (Runtime Verification)** — fuzzing + testing + formal verification, specs in Rust. → https://github.com/runtimeverification/komet

**Post-deploy monitoring**
- **OpenZeppelin Monitor (Stellar alpha)** — self-hosted via Docker, observability with Prometheus + Grafana.

## A.8 Audits and bug bounty

- **Soroban Audit Bank (SDF):** US$3M+ deployed across 43+ audits. For SCF-funded projects. 5% co-payment (refundable). Preparation with the **STRIDE** framework + Audit Readiness Checklist. → https://stellar.org/grants-and-funding/soroban-audit-bank
- **Immunefi — Stellar Core:** up to US$250K (stellar-core, SDKs, CLI/RPC). PoC required, local forks only.
- **Immunefi — OpenZeppelin Stellar:** up to US$25K.
- **Partner firms:** OtterSec, Veridise, Runtime Verification, CoinFabrik, Coinspect, Certora, Halborn, Zellic, Code4rena.

> The `stellar-dev-skill` repo itself is **not** in scope for SDF's bug bounty.

---

# PART B — Stellar Node Hardening

> Only needed if you operate a validator or an RPC. If you only develop
> contracts, you can skip this part.

## B.1 General Linux hardening (for Stellar nodes)

Before installing `stellar-core`, the server must be hardened. A validator
compromised at the operating-system level can't be fixed with good Stellar
config: you've already lost. This section targets **Ubuntu Server 22.04/24.04
LTS** or Debian — the most common setup for Stellar nodes — but the concepts
apply to any distro.

> 🎯 **Guiding principle:** a Stellar node is a *single-purpose* machine.
> Anything not needed to run `stellar-core` (or `stellar-rpc`) is attack
> surface. The less installed and running, the better.

### B.1.1 First boot: update everything

First thing, always, before touching anything else:

```bash
sudo apt update && sudo apt full-upgrade -y
sudo apt autoremove --purge -y
sudo reboot   # if the kernel was updated
```

An unpatched system is the cheapest way in for an attacker. Don't start
configuring on top of an outdated base.

### B.1.2 Dedicated user, never root

Never operate the node as `root` and never run `stellar-core` with privileges.
Create an administrative user for yourself and, separately, a shell-less service
user for the Stellar process.

```bash
# Administrative user (with sudo) for you
sudo adduser stellaradmin
sudo usermod -aG sudo stellaradmin

# Service user to run stellar-core: no login, no shell
sudo useradd --system --no-create-home --shell /usr/sbin/nologin stellar
```

The idea: you log in as `stellaradmin` and use `sudo` when needed; the Stellar
binary runs under the `stellar` user, which cannot log in or open a shell even
if someone manages to execute code as it.

### B.1.3 Harden SSH

SSH is the server's front door and the number-one target for bots. These changes
stop the vast majority of automated attacks.

**1. Generate a key on your local machine** (not on the server) and copy it:

```bash
# On YOUR computer, not the server
ssh-keygen -t ed25519 -C "stellar-node-key"
ssh-copy-id stellaradmin@YOUR_SERVER_IP
```

**2. Create `/etc/ssh/sshd_config.d/99-hardening.conf`** with this minimal
configuration (a drop-in file is cleaner than editing the main config):

```text
# No root login
PermitRootLogin no

# Keys only, never passwords
PasswordAuthentication no
PubkeyAuthentication yes
KbdInteractiveAuthentication no
ChallengeResponseAuthentication no

# Restrict who can log in
AllowUsers stellaradmin

# Harden the session
MaxAuthTries 3
LoginGraceTime 30
X11Forwarding no
AllowAgentForwarding no
ClientAliveInterval 300
ClientAliveCountMax 2
```

**3. Apply the changes** (without closing your current session, just in case):

```bash
sudo sshd -t                      # validate syntax BEFORE restarting
sudo systemctl restart ssh
```

> ⚠️ Test a **second** SSH connection in a new terminal before closing the one
> you have open. If you got something wrong, you still have a way in to fix it.

> 💡 **Changing the SSH port? Read this first.** On Ubuntu 22.10+ SSH uses
> systemd *socket activation*: setting `Port` in `sshd_config` is silently
> ignored unless you also disable the socket unit:
> ```bash
> sudo systemctl disable --now ssh.socket
> sudo systemctl enable --now ssh.service
> ```
> A non-standard port only reduces bot noise anyway — it is not real security.
> Keeping port 22 with key-only auth + rate limiting is perfectly fine.

### B.1.4 Default-deny firewall

The firewall is what separates your internal ports from the raw internet.
Here's the base; the Stellar-specific ports (11625/11626) are detailed in
**B.2**.

```bash
sudo apt install ufw -y

# Default policy: block everything inbound, allow everything outbound
sudo ufw default deny incoming
sudo ufw default allow outgoing

# SSH with rate-limit (stops brute force). Adjust the port if you changed it.
sudo ufw limit ssh

# Stellar consensus port (required to participate in the network)
sudo ufw allow 11625/tcp

sudo ufw enable
sudo ufw status verbose
```

Golden rule: **only open what a service truly needs**. The `HTTP_PORT` (11626)
is deliberately absent here — it stays on `localhost`.

### B.1.5 fail2ban against brute force

`fail2ban` reads logs and automatically bans IPs that repeatedly fail to log
in. It complements the SSH rate-limit.

```bash
sudo apt install fail2ban -y
```

Create `/etc/fail2ban/jail.local`:

```text
[sshd]
enabled  = true
# On Ubuntu 24.04 minimal installs sshd logs to the systemd journal and
# /var/log/auth.log may not exist — 'backend = systemd' handles that.
backend  = systemd
maxretry = 3
bantime  = 1h
findtime = 10m
```

```bash
sudo systemctl enable --now fail2ban
sudo fail2ban-client status sshd
```

### B.1.6 Automatic security updates

You don't want patching to depend on your memory. Let security updates install
themselves.

```bash
sudo apt install unattended-upgrades -y
sudo dpkg-reconfigure --priority=low unattended-upgrades
```

> 💡 For a validator, consider scheduling kernel reboots in a low-traffic
> window and notifying your quorum if you'll be offline for a while.

### B.1.7 Time synchronization (critical for validators)

This is **specifically important on Stellar**: the Stellar Consensus Protocol is
time-sensitive. A drifting clock can knock your validator out of consensus or
degrade its reputation on the network. Make sure NTP is active:

```bash
sudo timedatectl set-ntp true
timedatectl status        # verify "System clock synchronized: yes"
```

If you want something more robust than `systemd-timesyncd`, install `chrony`:

```bash
sudo apt install chrony -y
sudo systemctl enable --now chrony
chronyc tracking
```

### B.1.8 Reduce the attack surface

A Stellar node doesn't need to be a web server, a mail server, or a print
server. List what's listening on the network and turn off what you don't use:

```bash
# See which processes are listening on which ports
sudo ss -tulpn

# Example: if you don't use these, they shouldn't be running
sudo systemctl disable --now avahi-daemon cups 2>/dev/null || true
```

Every service you turn off is one less vulnerability to worry about.

### B.1.9 Kernel hardening (sysctl)

Some kernel parameters reduce the risk of spoofing, network attacks, and
information exposure. Create `/etc/sysctl.d/99-stellar-hardening.conf`:

```text
# Ignore broadcast pings and protect against spoofing
net.ipv4.icmp_echo_ignore_broadcasts = 1
net.ipv4.conf.all.rp_filter = 1
net.ipv4.conf.default.rp_filter = 1

# Don't accept redirects or source routing (MITM vectors)
net.ipv4.conf.all.accept_redirects = 0
net.ipv4.conf.all.send_redirects = 0
net.ipv4.conf.all.accept_source_route = 0

# Log packets with impossible addresses (martians)
net.ipv4.conf.all.log_martians = 1

# SYN flood protection
net.ipv4.tcp_syncookies = 1

# Restrict access to kernel logs and pointers
kernel.dmesg_restrict = 1
kernel.kptr_restrict = 2
```

```bash
sudo sysctl --system   # applies without rebooting
```

### B.1.10 Auditing and integrity

Two tools to *detect* when something changed or broke:

```bash
# Lynis: system security audit (run periodically)
sudo apt install lynis -y
sudo lynis audit system

# auditd: records security events (access, config changes)
sudo apt install auditd -y
sudo systemctl enable --now auditd

# AIDE: detects modifications to critical files
sudo apt install aide -y
sudo aideinit
```

Go through the Lynis report and raise your "hardening index" patch by patch.
It's a good metric to show in the repo.

### B.1.11 Permissions and backup of the `NODE_SEED`

The validator secret (`NODE_SEED`) is the most sensitive piece of the node.
Wherever you store it (ideally in a separate config file, not the main one):

```bash
# The config file containing the seed: readable by its owner only
sudo chmod 600 /etc/stellar/stellar-core.cfg
sudo chown stellar:stellar /etc/stellar/stellar-core.cfg
```

Back up the seed **encrypted and offline** (e.g., in a password manager or an
encrypted volume). Never in a repo, never in plain text, never in an
unencrypted backup. If it leaks, someone can impersonate your node's identity
on the network.

### B.1.12 Linux hardening checklist

- [ ] System fully updated and rebooted if there was a new kernel.
- [ ] Admin user with sudo + shell-less `stellar` service user.
- [ ] SSH keys-only, root disabled, `AllowUsers` restricted.
- [ ] `ufw` firewall with default-deny and SSH rate-limit.
- [ ] `fail2ban` active on SSH (with `backend = systemd`).
- [ ] `unattended-upgrades` configured.
- [ ] NTP synchronized (`timedatectl` or `chrony`).
- [ ] Unnecessary services off (`ss -tulpn` is clean).
- [ ] Hardening `sysctl` applied.
- [ ] Lynis run + auditd/AIDE active.
- [ ] `NODE_SEED` with `600` permissions and encrypted offline backup.

## B.2 Ports and firewall

| Port | Service | Exposure |
|------|---------|----------|
| **11625** TCP | `PEER_PORT` (consensus) | Inbound from **0.0.0.0/0** + outbound. **Required** to participate in the network. |
| **11626** TCP | `HTTP_PORT` (admin, **no auth**) | **Never** to the internet. Listens on `localhost` by default. If you share it on an internal network, put it **behind a reverse proxy with authentication**. |
| 11726 | Horizon HTTP | Safe to expose — designed for the internet. |
| 5432 | PostgreSQL (if applicable) | **Internal network only.** Write access to this DB = corrupting your view of the network. |

```bash
# Peer port: open to the network (required for consensus)
sudo ufw allow from 0.0.0.0/0 to any port 11625 proto tcp

# SSH: rate-limited
sudo ufw limit ssh

# HTTP_PORT is NOT opened to the public. It stays on localhost.
# If you need to expose it internally, do it via a reverse proxy with auth.

sudo ufw default deny incoming
sudo ufw default allow outgoing
sudo ufw enable
```

> ⚠️ The `HTTP_PORT` (11626) exposes administrative commands (`/info`,
> `/metrics`, `/ll?level=...`, scheduling upgrades, changing log levels) **with
> no authentication**. Exposing it to the internet = handing over control of
> the node.

## B.3 Validator security

- **`NODE_SEED`:** this is your node's cryptographic identity. **Never** share it or leave it in plain text in repos. Generate it with `stellar-core gen-seed`.
- **`NODE_IS_VALIDATOR=true`** only if you actually validate; define safe, conservative **quorum slices**.
- **Separate roles:** don't use the same node to validate *and* serve a public RPC. The validator should have the smallest possible attack surface.
- **Isolation:** run `stellar-core` and `stellar-rpc` in separate containers (Docker/Podman) to limit the blast radius.

## B.4 RPC and dApp backends

- Expose only what's necessary from the **Stellar RPC** (it's the preferred replacement over Horizon, which remains as legacy).
- **Secrets** (API keys, service-account seeds) in environment variables or a manager like **HashiCorp Vault** — never hardcoded.
- If you expose public endpoints: **rate limiting** + **WAF**.

## B.5 Monitoring

- **Metrics:** the `11626/metrics` endpoint exposes Prometheus metrics. Add Grafana with community dashboards. The official `stellar/stellar-core-prometheus-exporter` image scrapes them (defaults to `http://127.0.0.1:11626`).
- **Logs:** configured to a file or read via `journalctl -u stellar-core` (depending on how you run it). The log level is adjusted live through the HTTP endpoint, not via a `--logs` flag.
- **Contracts:** **OpenZeppelin Monitor** to watch your deployed contracts.

---

# PART C — Automation and pre-production checklist

Ready-to-use artifacts in this repo (see the `ansible/` and `docker/` folders
and `.github/workflows/`):

- [x] **Ansible playbook** to provision a hardened Stellar node → `ansible/harden-stellar-node.yml`
- [x] **Docker Compose** example: isolated RPC node behind a reverse proxy → `docker/docker-compose.yml`
- [x] **GitHub Action** running `cargo scout-audit` on every contract PR → `.github/workflows/scout-audit.yml`
- [ ] **Check script** (Lynis + verifying 11626 is not exposed) — PRs welcome.

### Quick checklist before production

**Contracts**
- [ ] External audit or, at minimum, Scout + peer review.
- [ ] All items in checklist A.6 satisfied.
- [ ] Upgrade and rollback plan defined (if the contract is mutable).

**Nodes**
- [ ] 11626 confirmed **not** reachable from the internet (`nmap` from outside).
- [ ] `NODE_SEED` out of any repo or plaintext backup.
- [ ] Default-deny firewall + rate-limited SSH.
- [ ] Monitoring and alerts working.

---

## Resources and contributions

**Official Stellar**
- Docs: https://developers.stellar.org
- Development skill (with security): https://github.com/stellar/stellar-dev-skill
- Admin guide / validators: https://developers.stellar.org/docs/validators
- Audit Bank: https://stellar.org/grants-and-funding/soroban-audit-bank

**Security community**
- Soroban Security Portal: https://sorobansecurity.com
- Scout (CoinFabrik): https://github.com/CoinFabrik/scout-soroban

**Further reading (Linux)**
- To go deeper than this guide: [How-To-Secure-A-Linux-Server](https://github.com/imthenachoman/How-To-Secure-A-Linux-Server) (general reference, not Stellar-specific).

---

### Contributing

Do you run a node or have you deployed contracts on Stellar from LATAM? Your
experience counts. Open an issue or send a PR. The idea is for this guide to be
**living** and written by people who put real projects into production.

> Suggested first issue: *"Stellar adaptation — Call for LATAM builders"*

**License:** MIT
