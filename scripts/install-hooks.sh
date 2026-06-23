#!/usr/bin/env bash
set -euo pipefail

repo_root="$(git rev-parse --show-toplevel)"
cd "$repo_root"

chmod +x .githooks/pre-push scripts/install-hooks.sh
git config core.hooksPath .githooks

echo "Installed versioned git hooks for $(basename "$repo_root")"
echo "hooksPath=$(git config --get core.hooksPath)"
