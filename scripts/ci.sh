#!/usr/bin/env bash
set -euo pipefail

# ci.sh — openprose-lint internal CI / gate.
#
# This repo has NO GitHub Actions. This script IS the CI: a single trusted,
# DHH-style local gate that runs the full check chain and exits nonzero on the
# first failure. It is invoked automatically by the pre-push hook
# (.githooks/pre-push) and can be run by hand at any time:
#
#   bash scripts/ci.sh
#
# Set OPENPROSE_SKIP=1 to bypass the entire gate (escape hatch for emergencies).

if [[ "${OPENPROSE_SKIP:-}" == "1" ]]; then
  echo "[ci] OPENPROSE_SKIP=1, skipping all checks"
  exit 0
fi

repo_root="$(git rev-parse --show-toplevel)"
cd "$repo_root"

fail() {
  echo "[ci] FAIL: $*" >&2
  exit 1
}

step() {
  echo
  echo "[ci] ==> $*"
  "$@" || fail "$*"
}

# Spec hygiene: docs/specs/ holds ACCEPTED design contracts only — there is no
# in-tree draft/lifecycle state. An unenforced "Status:" label drifts from reality
# (two specs once sat at "Status: Draft" while their models had shipped and were
# gated), so it is banned outright. Proposals live on a branch until merged;
# merging IS acceptance. See docs/README.md "Specs Are Accepted Contracts".
spec_hygiene() {
  local hits
  hits="$(grep -rniE '^[[:space:]]*(\*\*)?(status|lifecycle)(\*\*)?[[:space:]]*:' docs/specs 2>/dev/null || true)"
  if [[ -n "$hits" ]]; then
    echo "[ci] spec hygiene FAIL: docs/specs/ must not declare a lifecycle status label." >&2
    echo "$hits" >&2
    echo "[ci] Delete the line — docs/specs/ holds accepted contracts only (see docs/README.md)." >&2
    return 1
  fi
}

step cargo fmt --check
step bun install --frozen-lockfile
step bun run true-up:gate
step spec_hygiene
step cargo clippy --all-targets --all-features -- -D warnings
step cargo test
step cargo build
step cargo run -- specs
step cargo run -- conformance
# Public command-surface smoke: current Markdown programs must lint through `lint`.
step cargo run -- lint --profile compat reference/openprose-prose/skills/open-prose/examples

echo
echo "[ci] all checks passed"
