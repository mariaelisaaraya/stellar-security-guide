# Guía de Seguridad Stellar & Soroban (edición LATAM 🌎)

> Una guía de seguridad práctica para **builders que desarrollan contratos inteligentes en Soroban**
> y para **operadores que corren nodos** en la red Stellar. Escrita para la
> comunidad LATAM — DeFiWise, Be-Energy, Lendara, y cualquier proyecto que ponga
> dinero real sobre Stellar.

**Licencia:** MIT · **Contribuciones:** bienvenidas. Mandá tu PR.

---

## ⚠️ Antes de empezar: dos cosas distintas

Esta guía cubre **dos capas de seguridad que se suelen confundir**:

| Capa | Para quién | Qué protege |
|------|------------|-------------|
| **Parte A — Contratos Soroban** | Desarrolladores | Tu contrato de bugs de lógica que drenan fondos (auth, overflow, storage, TTL) |
| **Parte B — Nodos Stellar** | Operadores de infra | Tu servidor/validador/RPC de compromiso (firewall, claves, puertos) |

Si estás escribiendo contratos, tu prioridad es la **Parte A**. Si corrés un validador
o un RPC para tu dApp, sumá la **Parte B**.

> 📌 **No reinventamos la rueda.** Esta guía se construye sobre material oficial de
> la Stellar Development Foundation (SDF). Cuando algo está mejor explicado ahí,
> linkeamos en vez de copiar:
> - [`stellar/stellar-dev-skill`](https://github.com/stellar/stellar-dev-skill) — skill oficial con sección de seguridad Soroban.
> - [sorobansecurity.com](https://sorobansecurity.com) — base de conocimiento comunitaria (reportes de auditoría + base de vulnerabilidades).
> - [Stellar Developers Docs](https://developers.stellar.org).

---

## 1. Introducción

### Por qué importa la seguridad en Stellar

En Stellar, "código" y "dinero" son la misma cosa. Un contrato Soroban con un
chequeo de autorización faltante no es un bug cosmético: es una billetera abierta.
Y un validador o RPC mal expuesto no solo te afecta a vos — afecta a todos los que
dependen de tu infraestructura (oráculos, pools, frontends).

### Modelo de amenaza (las dos capas)

**A nivel de contrato**, las amenazas suelen ser *lógicas*: funciones privilegiadas
sin auth, reinicialización, overflow aritmético, datos críticos archivados por TTL
vencido, confianza ciega en contratos externos u oráculos.

**A nivel de nodo**, las amenazas son *operacionales*: endpoints RPC/admin expuestos
a internet, SSH sin hardening, secretos de validador filtrados (`NODE_SEED`),
ataques de denegación de servicio.

> **Modelo de amenaza concreto para contratos.** Siempre asumí que el atacante puede:
> - Pasar cualquier valor como argumento de función.
> - Controlar el orden y timing de las transacciones.
> - Controlar cualquier cuenta que *no* requiera su firma explícita.
> - Desplegar contratos que imitan perfectamente tu interfaz.

---

# PARTE A — Seguridad de Contratos Inteligentes Soroban

> Esta es la sección de mayor impacto. La mayoría de las pérdidas en contratos
> inteligentes vienen de errores de lógica, no de infraestructura.

## A.1 Qué te da Soroban gratis (y qué no)

Soroban previene por diseño algunas clases de vulnerabilidades típicas de Ethereum:

- **Sin `delegatecall`** → los ataques basados en proxy que ejecutan bytecode arbitrario no existen.
- **Sin reentrancy cross-contract clásica** → el modelo de ejecución es sincrónico. (Nota: la auto-reentrancy es posible, aunque raramente explotable.)
- **Autorización explícita** → nada se autoriza implícitamente; tenés que llamar `require_auth()` a propósito.

Lo que **no** te da gratis: validación de inputs, manejo de overflow,
gestión del TTL de storage, ni validar con quién estás hablando. Eso es 100% tu
responsabilidad.

## A.2 Clases de vulnerabilidades (con código)

### 1) Autorización faltante

El bug más común y más caro. Cualquier persona puede llamar funciones privilegiadas.

```rust
// ❌ MAL: nadie verifica quién está llamando
pub fn withdraw(env: Env, to: Address, amount: i128) {
    transfer_tokens(&env, &to, amount);
}

// ✅ BIEN: requiere la firma del admin
pub fn withdraw(env: Env, to: Address, amount: i128) {
    let admin: Address = env.storage().instance().get(&DataKey::Admin).unwrap();
    admin.require_auth();
    transfer_tokens(&env, &to, amount);
}
```

### 2) Ataques de reinicialización

Si `initialize` puede llamarse dos veces, un atacante se convierte en el admin.

```rust
// ✅ BIEN: solo puede inicializarse una vez
pub fn initialize(env: Env, admin: Address) {
    if env.storage().instance().has(&DataKey::Admin) {
        panic_with_error!(&env, Error::AlreadyInitialized);
    }
    env.storage().instance().set(&DataKey::Admin, &admin);
}
```

### 3) Llamadas a contratos arbitrarios

Nunca confíes en cualquier `Address` que llegue como parámetro.

```rust
// ✅ BIEN: valida contra una lista permitida conocida
pub fn swap(env: Env, token: Address, amount: i128) {
    let allowed: Vec<Address> = env.storage().instance()
        .get(&DataKey::AllowedTokens).unwrap();
    if !allowed.contains(&token) {
        panic_with_error!(&env, Error::TokenNotAllowed);
    }
    // ... continuar
}
```

### 4) Overflow / underflow aritmético

Siempre usá aritmética verificada. El overflow puede saltarse chequeos de saldo.

```rust
// ✅ BIEN
let new_balance = balance.checked_add(amount)
    .ok_or(Error::Overflow)?;
```

> 💡 En `Cargo.toml`, habilitá `overflow-checks = true` en el perfil release.

### 5) Colisiones de claves de storage

Datos distintos compartiendo la misma clave = corrupción.

```rust
// ✅ BIEN: enum tipado para las claves
#[contracttype]
#[derive(Clone)]
pub enum DataKey {
    Admin,
    Balance(Address),
    Config,
    Allowance(Address, Address),
}
```

### 6) Condiciones de carrera / orden de estado y frontrunning

Realizá chequeos y cambios de estado de forma atómica, sin dejar brecha entre
"validar" y "actuar". Para swaps y cualquier operación donde el precio o monto se
calcula en cadena al momento de ejecución, siempre dejá que el usuario pase una
protección contra slippage:

```rust
// ✅ BIEN: el usuario controla el slippage aceptable
pub fn swap(env: Env, user: Address, amount_in: i128, min_out: i128) {
    user.require_auth();

    let balance = get_balance(&env, &user);
    if balance < amount_in {
        panic_with_error!(&env, Error::InsufficientBalance);
    }

    let amount_out = calculate_output(amount_in);
    if amount_out < min_out {
        panic_with_error!(&env, Error::SlippageExceeded);
    }

    // Actualizá todo el estado junto — sin brecha entre chequeos y efectos
    set_balance(&env, &user, balance - amount_in);
    transfer_output(&env, &user, amount_out);
}
```

Sin `min_out` (e idealmente un deadline), un ataque sandwich puede ejecutar
la transacción a una tasa arbitrariamente mala.

### 7) Vulnerabilidades de TTL / archivado

Datos críticos archivados por TTL vencido pueden romper el contrato. Extendé
el TTL de forma proactiva en operaciones críticas.

```rust
env.storage().instance().extend_ttl(100, 518400);          // ~30 días
env.storage().persistent().extend_ttl(&DataKey::CriticalData, 100, 518400);
```

### 8) Validación de retornos cross-contract

No confíes en lo que devuelve un contrato externo (especialmente oráculos). Validá
que el oráculo sea de confianza **y** que el valor tenga sentido.

```rust
if !trusted_oracles.contains(&oracle) {
    panic_with_error!(&env, Error::UntrustedOracle);
}
let price: i128 = oracle_client.get_price(&asset);
if price <= 0 || price > MAX_REASONABLE_PRICE {
    panic_with_error!(&env, Error::InvalidPrice);
}
```

## A.3 Storage: elegí el tipo correcto

| Tipo | Usalo para | Costo / Vida útil |
|------|-----------|-------------------|
| **Instance** | Datos compartidos / admin / configuración | **Se carga entero en cada invocación** y comparte **un único TTL** |
| **Persistent** | Datos por usuario o en crecimiento (saldos, allowances) | TTL por clave; recuperable si se archiva |
| **Temporary** | Datos verdaderamente efímeros | Se descarta; **nunca** lo uses para algo que deba persistir |

> 🔑 **Regla de oro:** datos *ilimitados* (en crecimiento) o por usuario → **Persistent**,
> nunca Instance. Si ponés un mapa en crecimiento en Instance storage, hacés
> *cada* invocación del contrato más cara y arriesgás llegar a los límites de recursos.

## A.4 Manejo de errores: errores tipados, no `panic!` a secas

Definí errores con `#[contracterror]` y lanzalos con `panic_with_error!`.
Obtenés errores estructurados y distinguibles, mucho más útiles para fuzzing y
para quien integre tu contrato.

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

## A.5 Seguridad "clásica" de Stellar (no Soroban)

Aunque trabajes en Soroban, tu dApp toca la capa clásica de Stellar:

- **Trustlines maliciosas:** verificá el emisor antes de crear una trustline. Siempre mostrá el código de asset completo + emisor en la UI. Usá listas de assets conocidos (`stellar.toml`).
- **Clawback:** algunos assets permiten que el emisor los recupere. Chequeá `auth_clawback_enabled` en la cuenta del emisor y avisale al usuario o rechazá el asset:

```typescript
const issuerAccount = await server.loadAccount(asset.issuer);
const clawbackEnabled = issuerAccount.flags.auth_clawback_enabled;

if (clawbackEnabled) {
  // Avisá claramente al usuario o rechazá el asset
}
```

- **Account merge:** una cuenta mergeada puede recrearse con una configuración distinta. No guardes estado de cuenta a largo plazo para operaciones críticas.

## A.6 Checklist del contrato (pre-deploy)

- [ ] Cada función privilegiada hace cumplir el `require_auth()` correspondiente.
- [ ] La inicialización solo puede ocurrir una vez.
- [ ] Las llamadas a contratos externos se validan contra una lista permitida.
- [ ] Toda la aritmética usa operaciones verificadas (`checked_add`, etc.).
- [ ] Las claves de storage son tipadas (enum) y sin colisiones.
- [ ] Los TTL de datos críticos se extienden de forma proactiva.
- [ ] Validación de inputs en todas las funciones públicas.
- [ ] Se emiten eventos para cada cambio de estado auditable.
- [ ] Errores tipados via `#[contracterror]`.
- [ ] `overflow-checks = true` en release.
- [ ] Protección contra slippage (`min_out` / `max_in`) en funciones donde el precio o monto se calcula en cadena.
- [ ] Sin loops ilimitados sobre colecciones de storage.

**Preguntas de revisión:** ¿Puede alguien llamar funciones de admin sin auth? ¿Puede
reinicializarse? ¿Las llamadas externas están validadas? ¿La aritmética es segura?
¿Pueden colisionar las claves? ¿Sobreviven los datos críticos al archivado? ¿Se
validan los retornos cross-contract?

## A.7 Seguridad del cliente y la dApp

El contrato puede ser perfectamente seguro mientras que el frontend que lo maneja no lo es.
Estos chequeos aplican a cualquier app web o móvil que interactúe con Stellar.

- [ ] **Network passphrase validada** antes de firmar cualquier transacción (testnet ≠ pubnet).
- [ ] **Simulación de transacción antes de enviar** — llamá `simulateTransaction` y mostrá cualquier falla al usuario antes de que firme.
- [ ] **Todos los detalles de la operación mostrados claramente** — montos, emisor del asset, destino, fees. El usuario tiene que saber qué está aprobando.
- [ ] **Confirmación requerida** para transacciones de alto valor.
- [ ] **Todos los estados de error manejados** — timeouts de red, fees insuficientes, fallos de transacción.
- [ ] **Sin validación solo en el cliente** — el contrato impone las reglas reales; la UI valida solo por UX.
- [ ] **Direcciones de contratos verificadas** contra un registro conocido y fijo antes de llamarlos.
- [ ] **Estado de trustline y clawback chequeado** antes de iniciar transferencias (ver A.5).

## A.8 Herramientas de seguridad

**Análisis estático**
- **Scout (CoinFabrik)** — 23 detectores. `cargo install cargo-scout-audit` → `cargo scout-audit`. Output: HTML/MD/JSON/PDF/SARIF (CI/CD). Extensión VSCode disponible. Detectores clave: `overflow-check`, `unprotected-update-current-contract-wasm`, `set-contract-storage`, `unrestricted-transfer-from`, `divide-before-multiply`, `dos-unbounded-operation`, `unsafe-unwrap`. → https://github.com/CoinFabrik/scout-soroban
- **OpenZeppelin Security Detectors SDK** — framework para detectores personalizados. Pre-construidos: `auth_missing`, `unchecked_ft_transfer`, extensión de TTL incorrecta, panics de contrato, uso inseguro de temporary storage. Arquitectura: `sdk` (núcleo) + `detectors` (pre-construidos) + `soroban-scanner` (CLI). → https://github.com/OpenZeppelin/soroban-security-detectors-sdk

**Verificación formal**
- **Certora Sunbeam** — verificación formal a nivel WASM. → https://docs.certora.com/en/latest/docs/sunbeam/index.html
- **Komet (Runtime Verification)** — fuzzing + testing + verificación formal, specs en Rust. → https://github.com/runtimeverification/komet

**Monitoreo post-deploy**
- **OpenZeppelin Monitor (Stellar alpha)** — self-hosted via Docker, observabilidad con Prometheus + Grafana.

## A.9 Auditorías y bug bounty

- **Soroban Audit Bank (SDF):** US$3M+ desplegados en 43+ auditorías. Para proyectos financiados por SCF. Co-pago del 5% (reembolsable si se remedian los issues Críticos/Altos/Medios en 20 días hábiles). Auditorías de seguimiento activadas en $10M y $100M TVL. Preparación con el framework **STRIDE** + Audit Readiness Checklist. → https://stellar.org/grants-and-funding/soroban-audit-bank
- **Immunefi — Stellar Core:** hasta US$250K. Scope: `stellar-core`, `rs-soroban-sdk`, `rs-soroban-env`, `soroban-tools` (CLI + RPC), `js-soroban-client`, `rs-stellar-xdr`, fork `wasmi`. Requiere PoC; solo forks locales (no mainnet/testnet). Pago en XLM. → https://immunefi.com/bug-bounty/stellar/
- **Immunefi — OpenZeppelin Stellar:** hasta US$25K por bug (cap total del programa US$250K). Scope: librería OpenZeppelin Stellar Contracts. → https://immunefi.com/bug-bounty/openzeppelin-stellar/
- **HackerOne — apps web del SDF:** scope son las aplicaciones web, servidores de producción y dominios del SDF. Ventana de remediación de 90 días antes de divulgación pública. → https://stellar.org/grants-and-funding/bug-bounty
- **Firmas partners:** OtterSec, Veridise, Runtime Verification, CoinFabrik, Coinspect, Certora, Halborn, Zellic, Code4rena.

> El repo `stellar-dev-skill` en sí **no** está en scope del bug bounty del SDF.

---

# PARTE B — Hardening de Nodos Stellar

> Solo necesario si operás un validador o un RPC. Si solo desarrollás
> contratos, podés saltarte esta parte.

## B.1 Hardening general de Linux (para nodos Stellar)

Antes de instalar `stellar-core`, el servidor tiene que estar endurecido. Un validador
comprometido a nivel del sistema operativo no se arregla con buena config de Stellar:
ya perdiste. Esta sección apunta a **Ubuntu Server 22.04/24.04 LTS** o Debian — el
setup más común para nodos Stellar — pero los conceptos aplican a cualquier distro.

> 🎯 **Principio guía:** un nodo Stellar es una máquina de *propósito único*.
> Todo lo que no sea necesario para correr `stellar-core` (o `stellar-rpc`) es
> superficie de ataque. Cuanto menos instalado y corriendo, mejor.

### B.1.1 Primer arranque: actualizá todo

Lo primero, siempre, antes de tocar nada más:

```bash
sudo apt update && sudo apt full-upgrade -y
sudo apt autoremove --purge -y
sudo reboot   # si se actualizó el kernel
```

Un sistema sin parches es la forma más barata de entrar para un atacante. No empieces
a configurar sobre una base desactualizada.

### B.1.2 Usuario dedicado, nunca root

Nunca operes el nodo como `root` y nunca corras `stellar-core` con privilegios.
Creá un usuario administrativo para vos y, por separado, un usuario de servicio
sin shell para el proceso Stellar.

```bash
# Usuario administrativo (con sudo) para vos
sudo adduser stellaradmin
sudo usermod -aG sudo stellaradmin

# Usuario de servicio para correr stellar-core: sin login, sin shell
sudo useradd --system --no-create-home --shell /usr/sbin/nologin stellar
```

La idea: vos entrás como `stellaradmin` y usás `sudo` cuando hace falta; el binario
Stellar corre bajo el usuario `stellar`, que no puede hacer login ni abrir una shell
aunque alguien logre ejecutar código como él.

### B.1.3 Hardening de SSH

SSH es la puerta principal del servidor y el blanco número uno de los bots. Estos
cambios detienen la gran mayoría de los ataques automatizados.

**1. Generá una clave en tu máquina local** (no en el servidor) y copiala:

```bash
# En TU computadora, no en el servidor
ssh-keygen -t ed25519 -C "stellar-node-key"
ssh-copy-id stellaradmin@IP_DE_TU_SERVIDOR
```

**2. Creá `/etc/ssh/sshd_config.d/99-hardening.conf`** con esta configuración mínima
(un archivo drop-in es más limpio que editar el config principal):

```text
# Sin login de root
PermitRootLogin no

# Solo claves, nunca contraseñas
PasswordAuthentication no
PubkeyAuthentication yes
KbdInteractiveAuthentication no
ChallengeResponseAuthentication no

# Restringí quién puede entrar
AllowUsers stellaradmin

# Endurece la sesión
MaxAuthTries 3
LoginGraceTime 30
X11Forwarding no
AllowAgentForwarding no
ClientAliveInterval 300
ClientAliveCountMax 2
```

**3. Aplicá los cambios** (sin cerrar tu sesión actual, por las dudas):

```bash
sudo sshd -t                      # validá la sintaxis ANTES de reiniciar
sudo systemctl restart ssh
```

> ⚠️ Probá una **segunda** conexión SSH en una nueva terminal antes de cerrar la que
> tenés abierta. Si algo salió mal, todavía tenés cómo entrar a arreglarlo.

> 💡 **¿Cambiar el puerto SSH? Leé esto primero.** En Ubuntu 22.10+ SSH usa
> *socket activation* de systemd: configurar `Port` en `sshd_config` se ignora
> silenciosamente a menos que también deshabilites la unidad socket:
> ```bash
> sudo systemctl disable --now ssh.socket
> sudo systemctl enable --now ssh.service
> ```
> Un puerto no estándar solo reduce el ruido de bots — no es seguridad real.
> Mantener el puerto 22 con auth solo por claves + rate limiting está perfectamente bien.

### B.1.4 Firewall default-deny

El firewall es lo que separa tus puertos internos del internet crudo.
Acá está la base; los puertos específicos de Stellar (11625/11626) están detallados
en **B.2**.

```bash
sudo apt install ufw -y

# Política por defecto: bloquear todo entrante, permitir todo saliente
sudo ufw default deny incoming
sudo ufw default allow outgoing

# SSH con rate-limit (detiene brute force). Ajustá el puerto si lo cambiaste.
sudo ufw limit ssh

# Puerto de consenso Stellar (requerido para participar en la red)
sudo ufw allow 11625/tcp

sudo ufw enable
sudo ufw status verbose
```

Regla de oro: **solo abrí lo que un servicio verdaderamente necesita**. El `HTTP_PORT`
(11626) está deliberadamente ausente aquí — se queda en `localhost`.

### B.1.5 fail2ban contra fuerza bruta

`fail2ban` lee los logs y automáticamente banea IPs que fallan el login repetidamente.
Complementa el rate-limit de SSH.

```bash
sudo apt install fail2ban -y
```

Creá `/etc/fail2ban/jail.local`:

```text
[sshd]
enabled  = true
# En instalaciones mínimas de Ubuntu 24.04, sshd loguea al journal de systemd y
# /var/log/auth.log puede no existir — 'backend = systemd' maneja eso.
backend  = systemd
maxretry = 3
bantime  = 1h
findtime = 10m
```

```bash
sudo systemctl enable --now fail2ban
sudo fail2ban-client status sshd
```

### B.1.6 Actualizaciones de seguridad automáticas

No querés que los parches dependan de tu memoria. Dejá que las actualizaciones de
seguridad se instalen solas.

```bash
sudo apt install unattended-upgrades -y
sudo dpkg-reconfigure --priority=low unattended-upgrades
```

> 💡 Para un validador, considerá programar los reinicios de kernel en una ventana
> de bajo tráfico y notificar a tu quórum si vas a estar offline un rato.

### B.1.7 Sincronización de tiempo (crítico para validadores)

Esto es **específicamente importante en Stellar**: el Stellar Consensus Protocol es
sensible al tiempo. Un reloj que se desvía puede sacar a tu validador del consenso
o degradar su reputación en la red. Asegurate de que NTP esté activo:

```bash
sudo timedatectl set-ntp true
timedatectl status        # verificá "System clock synchronized: yes"
```

Si querés algo más robusto que `systemd-timesyncd`, instalá `chrony`:

```bash
sudo apt install chrony -y
sudo systemctl enable --now chrony
chronyc tracking
```

### B.1.8 Reducí la superficie de ataque

Un nodo Stellar no necesita ser un servidor web, un servidor de mail ni un servidor
de impresión. Listá qué está escuchando en la red y apagá lo que no uses:

```bash
# Mirá qué procesos escuchan en qué puertos
sudo ss -tulpn

# Ejemplo: si no los usás, no deberían estar corriendo
sudo systemctl disable --now avahi-daemon cups 2>/dev/null || true
```

Cada servicio que apagás es una vulnerabilidad menos de qué preocuparte.

### B.1.9 Hardening del kernel (sysctl)

Algunos parámetros del kernel reducen el riesgo de spoofing, ataques de red y
exposición de información. Creá `/etc/sysctl.d/99-stellar-hardening.conf`:

```text
# Ignorar pings broadcast y proteger contra spoofing
net.ipv4.icmp_echo_ignore_broadcasts = 1
net.ipv4.conf.all.rp_filter = 1
net.ipv4.conf.default.rp_filter = 1

# No aceptar redirects ni source routing (vectores MITM)
net.ipv4.conf.all.accept_redirects = 0
net.ipv4.conf.all.send_redirects = 0
net.ipv4.conf.all.accept_source_route = 0

# Loguear paquetes con direcciones imposibles (martians)
net.ipv4.conf.all.log_martians = 1

# Protección contra SYN flood
net.ipv4.tcp_syncookies = 1

# Restringir acceso a logs y punteros del kernel
kernel.dmesg_restrict = 1
kernel.kptr_restrict = 2
```

```bash
sudo sysctl --system   # aplica sin reiniciar
```

### B.1.10 Auditoría e integridad

Dos herramientas para *detectar* cuándo algo cambió o se rompió:

```bash
# Lynis: auditoría de seguridad del sistema (corré periódicamente)
sudo apt install lynis -y
sudo lynis audit system

# auditd: registra eventos de seguridad (accesos, cambios de config)
sudo apt install auditd -y
sudo systemctl enable --now auditd

# AIDE: detecta modificaciones en archivos críticos
sudo apt install aide -y
sudo aideinit
```

Revisá el reporte de Lynis y subí tu "hardening index" parche a parche.
Es una buena métrica para mostrar en el repo.

### B.1.11 Permisos y backup del `NODE_SEED`

El secreto del validador (`NODE_SEED`) es la pieza más sensible del nodo.
Donde sea que lo guardes (idealmente en un archivo de config separado, no en el principal):

```bash
# El archivo de config que contiene el seed: legible solo por su dueño
sudo chmod 600 /etc/stellar/stellar-core.cfg
sudo chown stellar:stellar /etc/stellar/stellar-core.cfg
```

Hacé backup del seed **encriptado y offline** (ej: en un gestor de contraseñas o
un volumen encriptado). Nunca en un repo, nunca en texto plano, nunca en un backup
sin encriptar. Si se filtra, alguien puede suplantar la identidad de tu nodo en la red.

### B.1.12 Checklist de hardening Linux

- [ ] Sistema completamente actualizado y reiniciado si hubo nuevo kernel.
- [ ] Usuario admin con sudo + usuario de servicio `stellar` sin shell.
- [ ] SSH solo con claves, root deshabilitado, `AllowUsers` restringido.
- [ ] Firewall `ufw` con default-deny y rate-limit en SSH.
- [ ] `fail2ban` activo en SSH (con `backend = systemd`).
- [ ] `unattended-upgrades` configurado.
- [ ] NTP sincronizado (`timedatectl` o `chrony`).
- [ ] Servicios innecesarios apagados (`ss -tulpn` limpio).
- [ ] Hardening `sysctl` aplicado.
- [ ] Lynis corrido + auditd/AIDE activos.
- [ ] `NODE_SEED` con permisos `600` y backup encriptado offline.

## B.2 Puertos y firewall

| Puerto | Servicio | Exposición |
|--------|---------|------------|
| **11625** TCP | `PEER_PORT` (consenso) | Entrante desde **0.0.0.0/0** + saliente. **Requerido** para participar en la red. |
| **11626** TCP | `HTTP_PORT` (admin, **sin auth**) | **Nunca** a internet. Escucha en `localhost` por defecto. Si lo compartís en una red interna, ponerlo **detrás de un reverse proxy con autenticación**. |
| 11726 | Horizon HTTP | Seguro de exponer — diseñado para internet. |
| 5432 | PostgreSQL (si aplica) | **Solo red interna.** Acceso de escritura a esta DB = corromper tu vista de la red. |

```bash
# Puerto peer: abierto a la red (requerido para consenso)
sudo ufw allow from 0.0.0.0/0 to any port 11625 proto tcp

# SSH: con rate-limit
sudo ufw limit ssh

# HTTP_PORT NO se abre al público. Se queda en localhost.
# Si necesitás exponerlo internamente, hacelo via reverse proxy con auth.

sudo ufw default deny incoming
sudo ufw default allow outgoing
sudo ufw enable
```

> ⚠️ El `HTTP_PORT` (11626) expone comandos administrativos (`/info`,
> `/metrics`, `/ll?level=...`, programar upgrades, cambiar niveles de log) **sin
> ninguna autenticación**. Exponerlo a internet = entregar el control del nodo.

## B.3 Seguridad del validador

- **`NODE_SEED`:** esta es la identidad criptográfica de tu nodo. **Nunca** la compartas ni la dejes en texto plano en repos. Generala con `stellar-core gen-seed`.
- **`NODE_IS_VALIDATOR=true`** solo si realmente validás; definí slices de quórum seguros y conservadores.
- **Roles separados:** no uses el mismo nodo para validar *y* servir un RPC público. El validador debe tener la menor superficie de ataque posible.
- **Aislamiento:** corré `stellar-core` y `stellar-rpc` en contenedores separados (Docker/Podman) para limitar el radio de explosión.

## B.4 RPC y backends de dApp

- Exponé solo lo necesario del **Stellar RPC** (es el reemplazo preferido sobre Horizon, que queda como legado).
- **Secrets** (API keys, seeds de service accounts) en variables de entorno o un gestor como **HashiCorp Vault** — nunca hardcodeados.
- Si exponés endpoints públicos: **rate limiting** + **WAF**.

## B.5 Monitoreo

- **Métricas:** el endpoint `11626/metrics` expone métricas Prometheus. Sumá Grafana con dashboards de la comunidad. La imagen oficial `stellar/stellar-core-prometheus-exporter` los scrapea (por defecto apunta a `http://127.0.0.1:11626`).
- **Logs:** configurados a un archivo o leídos via `journalctl -u stellar-core` (dependiendo de cómo lo corras). El nivel de log se ajusta en vivo a través del endpoint HTTP, no con un flag `--logs`.
- **Contratos:** **OpenZeppelin Monitor** para vigilar tus contratos deployados.

---

# PARTE C — Automatización y checklist pre-producción

Artefactos listos para usar en este repo (ver las carpetas `ansible/` y `docker/`
y `.github/workflows/`):

- [x] **Ansible playbook** para provisionar un nodo Stellar endurecido → `ansible/harden-stellar-node.yml`
- [x] **Docker Compose**: nodo RPC aislado detrás de un reverse proxy → `docker/docker-compose.yml`
- [x] **GitHub Action** que corre `cargo scout-audit` en cada PR de contratos → `.github/workflows/scout-audit.yml`
- [ ] **Script de verificación** (Lynis + verificar que 11626 no esté expuesto) — PRs bienvenidos.

### Checklist rápido antes de producción

**Contratos**
- [ ] Auditoría externa o, como mínimo, Scout + peer review.
- [ ] Todos los ítems de los checklists A.6 (contrato) y A.7 (cliente) cumplidos.
- [ ] Plan de upgrade y rollback definido (si el contrato es mutable).

**Nodos**
- [ ] 11626 confirmado **no** accesible desde internet (`nmap` desde afuera).
- [ ] `NODE_SEED` fuera de cualquier repo o backup en texto plano.
- [ ] Firewall default-deny + SSH con rate-limit.
- [ ] Monitoreo y alertas funcionando.

---

## Recursos y contribuciones

**Stellar oficial**
- Docs: https://developers.stellar.org
- Dev skill (con seguridad): https://github.com/stellar/stellar-dev-skill
- Guía de admin / validadores: https://developers.stellar.org/docs/validators
- Audit Bank: https://stellar.org/grants-and-funding/soroban-audit-bank

**Comunidad de seguridad**
- Portal Soroban Security: https://sorobansecurity.com
- Scout (CoinFabrik): https://github.com/CoinFabrik/scout-soroban

**Lectura adicional (Linux)**
- Para hardening más profundo del SO, las guías oficiales de Ubuntu y Debian son la referencia autorizada: [Ubuntu Security](https://ubuntu.com/security) · [Debian Security](https://www.debian.org/security/).

---

### Contribuir

¿Corrés un nodo o desplegaste contratos en Stellar desde LATAM? Tu experiencia
cuenta. Abrí un issue o mandá un PR. La idea es que esta guía sea **viva** y
escrita por personas que ponen proyectos reales en producción.

> Primera issue sugerida: *"Adaptación Stellar — Convocatoria a builders de LATAM"*

**Licencia:** MIT
