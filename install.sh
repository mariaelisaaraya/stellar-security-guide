#!/usr/bin/env bash
# stellar-security-guide — installer
#
# Usage:
#   Hosted:  curl -fsSL https://your-domain/install.sh | bash
#   Local:   ./install.sh
#   Update:  ./install.sh --update
#   Remove:  ./install.sh --uninstall
#   Prefix:  ./install.sh --prefix=/tmp/sandbox

set -euo pipefail

SKILL_NAME="soroban-common-mistakes"
REPO_URL="https://github.com/mariaelisaaraya/stellar-security-guide"
BUNDLE_URL="${BUNDLE_URL:-${REPO_URL}/archive/refs/heads/main.tar.gz}"

# --- Colors ---
RED=$'\033[0;31m'
GREEN=$'\033[0;32m'
YELLOW=$'\033[1;33m'
CYAN=$'\033[0;36m'
PURPLE=$'\033[1;35m'
BOLD=$'\033[1m'
DIM=$'\033[2m'
RESET=$'\033[0m'

log()  { printf "\n  %s▸%s %s\n" "$GREEN"  "$RESET" "$1"; }
warn() { printf "  %s!%s %s\n"  "$YELLOW" "$RESET" "$1"; }
fail() { printf "\n  %s✗%s %s\n\n" "$RED" "$RESET" "$1" >&2; exit 1; }
ok()   { printf "  %s✓%s %s\n"  "$GREEN"  "$RESET" "$1"; }
has_cmd() { command -v "$1" >/dev/null 2>&1; }

# --- Flags ---
UPDATE_MODE=false
UNINSTALL_MODE=false
PREFIX="$HOME"

for arg in "$@"; do
  case "$arg" in
    --update)    UPDATE_MODE=true ;;
    --uninstall) UNINSTALL_MODE=true ;;
    --prefix=*)  PREFIX="${arg#--prefix=}" ;;
  esac
done

CLAUDE_SKILLS="$PREFIX/.claude/skills"
INSTALL_DIR="$CLAUDE_SKILLS/$SKILL_NAME"
CONFIG_DIR="$PREFIX/.stellar-security"
MANIFEST="$CONFIG_DIR/manifest.json"

# --- Uninstall ---
if [ "$UNINSTALL_MODE" = true ]; then
  printf "\n  %s%sUninstalling %s...%s\n\n" "$CYAN" "$BOLD" "$SKILL_NAME" "$RESET"
  [ -d "$INSTALL_DIR" ] && rm -rf "$INSTALL_DIR" && ok "Removed $INSTALL_DIR"
  [ -d "$CONFIG_DIR"  ] && rm -rf "$CONFIG_DIR"  && ok "Removed config"
  printf "\n  %sTo reinstall: ./install.sh%s\n\n" "$DIM" "$RESET"
  exit 0
fi

# --- Banner ---
printf "\n"
printf "  %s███████╗███████╗ ██████╗%s\n"  "$PURPLE" "$RESET"
printf "  %s██╔════╝██╔════╝██╔════╝%s\n"  "$PURPLE" "$RESET"
printf "  %s███████╗███████╗██║  ███╗%s\n" "$PURPLE" "$RESET"
printf "  %s╚════██║╚════██║██║   ██║%s\n" "$PURPLE" "$RESET"
printf "  %s███████║███████║╚██████╔╝%s\n" "$PURPLE" "$RESET"
printf "  %s╚══════╝╚══════╝ ╚═════╝%s\n"  "$PURPLE" "$RESET"
printf "\n"
printf "   %sstellar-security-guide%s\n" "$BOLD" "$RESET"
printf "   %sSoroban contract security for LATAM builders%s\n\n" "$DIM" "$RESET"
[ "$UPDATE_MODE" = true ] && printf "   %sUpdating...%s\n\n" "$DIM" "$RESET"

# --- Prerequisites ---
log "Checking prerequisites..."
has_cmd curl || has_cmd wget || fail "curl or wget is required"
if has_cmd claude; then ok "Claude Code found"
else warn "Claude Code not found  →  npm i -g @anthropic-ai/claude-code"
fi

# --- Acquire source ---
# If the script is running from inside the repo itself, use the local files.
# Otherwise download from GitHub.
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]:-$0}")" 2>/dev/null && pwd || pwd)"
LOCAL_SKILL="$SCRIPT_DIR/skills/$SKILL_NAME"

TMP_DIR=$(mktemp -d)
trap 'rm -rf "$TMP_DIR"' EXIT

if [ -d "$LOCAL_SKILL" ] && [ -f "$LOCAL_SKILL/SKILL.md" ]; then
  log "Using local skill from $LOCAL_SKILL"
  SKILL_SRC="$LOCAL_SKILL"
else
  log "Downloading from GitHub..."
  ARCHIVE="$TMP_DIR/repo.tar.gz"
  if has_cmd curl; then
    curl -fsSL "$BUNDLE_URL" -o "$ARCHIVE" || fail "Download failed"
  else
    wget -q "$BUNDLE_URL" -O "$ARCHIVE" || fail "Download failed"
  fi
  tar -xzf "$ARCHIVE" -C "$TMP_DIR"
  SKILL_SRC=$(find "$TMP_DIR" -type d -name "$SKILL_NAME" | head -1)
  [ -n "$SKILL_SRC" ] && [ -f "$SKILL_SRC/SKILL.md" ] \
    || fail "Skill not found inside archive"
fi
ok "Source ready"

# --- Install ---
log "Installing to $CLAUDE_SKILLS/..."
mkdir -p "$CLAUDE_SKILLS" "$CONFIG_DIR"

if [ "$UPDATE_MODE" = true ] && [ -d "$INSTALL_DIR" ]; then
  rm -rf "$INSTALL_DIR"
fi

cp -Rf "$SKILL_SRC" "$INSTALL_DIR"
ok "Installed $SKILL_NAME"

# --- Install counter (best-effort, silent) ---
if has_cmd curl; then
  curl -sf "https://abacus.jasoncameron.dev/hit/mariaelisaaraya.stellar-security-guide/install" >/dev/null 2>&1 || true
elif has_cmd wget; then
  wget -q "https://abacus.jasoncameron.dev/hit/mariaelisaaraya.stellar-security-guide/install" -O /dev/null 2>/dev/null || true
fi

# --- Manifest ---
cat > "$MANIFEST" <<MANIFEST
{"installedBy":"stellar-security-guide","installedAt":"$(date -u +%Y-%m-%dT%H:%M:%SZ)","prefix":"$PREFIX","skillPath":"$INSTALL_DIR"}
MANIFEST
ok "Wrote manifest to $MANIFEST"

# --- Welcome card ---
printf "\n"
printf "  %s━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━%s\n" "$DIM" "$RESET"
printf "\n"
printf "  %s%s✓ Installed: soroban-common-mistakes%s\n" "$BOLD" "$GREEN" "$RESET"
printf "\n"
printf "  %sWhat it does:%s\n" "$BOLD" "$RESET"
printf "  Reviews Soroban contracts against 22 security patterns\n"
printf "  across 5 categories: auth, storage/TTL, math,\n"
printf "  external calls, and code quality.\n"
printf "\n"
printf "  %sTry it now:%s\n" "$BOLD" "$RESET"
printf "  %s→%s  Open a Soroban project in Claude Code\n" "$GREEN" "$RESET"
printf "  %s→%s  %s\"review this contract for security issues\"%s\n" "$GREEN" "$RESET" "$CYAN" "$RESET"
printf "  %s→%s  %s\"is this safe to deploy?\"%s\n"                 "$GREEN" "$RESET" "$CYAN" "$RESET"
printf "  %s→%s  %s\"run a pre-deployment checklist\"%s\n"          "$GREEN" "$RESET" "$CYAN" "$RESET"
printf "\n"
printf "  %sCompanion tools:%s\n" "$BOLD" "$RESET"
printf "  %s→%s  cargo install cargo-scout-audit   (static analysis)\n" "$DIM" "$RESET"
printf "  %s→%s  Copy %s.github/PULL_REQUEST_TEMPLATE.md%s to your contracts repo\n" "$DIM" "$RESET" "$CYAN" "$RESET"
printf "  %s→%s  See GUIDE.md for node hardening and full checklists\n" "$DIM" "$RESET"
printf "\n"
printf "  %s━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━%s\n" "$DIM" "$RESET"
printf "\n"

if [ "$UPDATE_MODE" = true ]; then
  printf "  %s%sUpdate complete!%s\n\n" "$GREEN" "$BOLD" "$RESET"
else
  printf "  %s%sSetup complete!%s\n\n" "$GREEN" "$BOLD" "$RESET"
fi

printf "  %sUninstall later:%s ./install.sh --uninstall\n\n" "$DIM" "$RESET"
