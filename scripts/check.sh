#!/usr/bin/env bash
#
# check.sh — one-shot local CI mirror.
#
# Runs everything CI runs except the OS matrix, OS deps, and the bundled
# Tauri release build (use `pnpm tauri build --bundles none` separately
# if you need that). Exits non-zero on the first failure.
#
# Usage:
#   ./scripts/check.sh           # full
#   ./scripts/check.sh --fast    # skip tauri/clippy (just frontend lint+test+build)
#
# Requires: ~/.spire_env (pnpm + cargo on PATH). Source it before calling.

set -euo pipefail

FAST=0
for arg in "$@"; do
  case "$arg" in
    --fast) FAST=1 ;;
    -h|--help)
      sed -n '2,12p' "$0"
      exit 0
      ;;
    *) echo "unknown arg: $arg" >&2; exit 2 ;;
  esac
done

# Resolve repo root even when called from elsewhere.
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
cd "$ROOT"

# Pull pnpm + cargo onto PATH (idempotent if already there).
if [ -f "$HOME/.spire_env" ]; then
  # shellcheck disable=SC1091
  source "$HOME/.spire_env"
fi

step() { printf "\n\033[1;36m▶ %s\033[0m\n" "$*"; }
fail() { printf "\033[1;31m✗ %s\033[0m\n" "$*"; exit 1; }

command -v pnpm >/dev/null  || fail "pnpm not on PATH (did you source ~/.spire_env?)"
command -v cargo >/dev/null || fail "cargo not on PATH (did you source ~/.spire_env?)"

step "pnpm install"
pnpm install --frozen-lockfile

step "cargo fmt --check"
( cd src-tauri && cargo fmt --all -- --check )

step "cargo clippy"
( cd src-tauri && cargo clippy --all-targets -- -D warnings )

step "cargo test"
( cd src-tauri && cargo test --all )

if [ "$FAST" -eq 0 ]; then
  step "gen-types --check"
  ./scripts/gen-types.sh --check
fi

step "pnpm typecheck"
pnpm typecheck

step "pnpm test"
pnpm test

step "pnpm build"
pnpm build

if [ "$FAST" -eq 0 ]; then
  step "pnpm tauri build (bundles none)"
  pnpm tauri build --bundles none
fi

printf "\n\033[1;32m✓ check.sh clean\033[0m\n"
