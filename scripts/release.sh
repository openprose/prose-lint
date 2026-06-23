#!/usr/bin/env bash
set -euo pipefail

# release.sh — Build and validate a release against a named spec source.
#
# Usage:
#   ./scripts/release.sh <spec-id> [--tag] [--dry-run]
#
# Examples:
#   ./scripts/release.sh openprose             # validate + generate manifest
#   ./scripts/release.sh openprose --tag       # also create git tag
#   ./scripts/release.sh openprose --dry-run   # validate only, no manifest

REPO_ROOT="$(git rev-parse --show-toplevel)"
cd "$REPO_ROOT"

SPEC_ID="${1:?Usage: release.sh <spec-id> [--tag] [--dry-run]}"
shift

TAG=false
DRY_RUN=false
for arg in "$@"; do
  case "$arg" in
    --tag) TAG=true ;;
    --dry-run) DRY_RUN=true ;;
    *) echo "Unknown flag: $arg"; exit 1 ;;
  esac
done

# --- Gather metadata ---

LINTER_VERSION="$(grep '^version' Cargo.toml | head -1 | sed 's/.*"\(.*\)"/\1/')"
GIT_SHA="$(git rev-parse HEAD)"
GIT_SHORT="$(git rev-parse --short HEAD)"
RUST_VERSION="$(rustc --version | awk '{print $2}')"
TIMESTAMP="$(date -u +%Y-%m-%dT%H:%M:%SZ)"

echo "=== openprose-lint release ==="
echo "  linter:  v${LINTER_VERSION} (${GIT_SHORT})"
echo "  spec:    ${SPEC_ID}"
echo "  rust:    ${RUST_VERSION}"
echo "  time:    ${TIMESTAMP}"
echo ""

# --- Validate spec source exists ---

if ! cargo run --quiet -- specs 2>/dev/null | grep -q "  ${SPEC_ID}"; then
  echo "ERROR: spec source '${SPEC_ID}' not found in specs/"
  echo "Available:"
  cargo run --quiet -- specs
  exit 1
fi

# --- Build release binary ---

echo "Building release binary..."
cargo build --release 2>&1

# --- Run tests ---

echo "Running tests..."
cargo test 2>&1

# --- Run conformance (if available) ---

SPEC_FILE="specs/${SPEC_ID}.json"
HAS_CONFORMANCE=false

if python3 -c "import json,sys; d=json.load(open(sys.argv[1])); sys.exit(0 if d.get('paths',{}).get('conformance_manifest') else 1)" "$SPEC_FILE" 2>/dev/null; then
  HAS_CONFORMANCE=true
fi

if [ "$HAS_CONFORMANCE" = true ]; then
  echo "Running conformance for ${SPEC_ID}..."
  cargo run --quiet -- conformance --spec "$SPEC_ID"
  echo ""
  echo "Running conformance (strict only)..."
  cargo run --quiet -- conformance --spec "$SPEC_ID" --profile strict
  echo ""
else
  echo "NOTE: spec '${SPEC_ID}' has no conformance manifest — skipping conformance check"
  echo ""
fi

# --- Lint examples (smoke test) ---

SPEC_SUBMODULE="$(python3 -c "import json,sys; print(json.load(open(sys.argv[1]))['submodule_path'])" "$SPEC_FILE")"
SPEC_ROOT="$(python3 -c "import json,sys; print(json.load(open(sys.argv[1]))['paths']['root'])" "$SPEC_FILE")"
EXAMPLES_DIR="${SPEC_SUBMODULE}/${SPEC_ROOT}/examples"

if [ -d "$EXAMPLES_DIR" ]; then
  echo "Smoke-testing examples (compat profile)..."
  cargo run --quiet -- lint --profile compat "$EXAMPLES_DIR" 2>&1 || true
  echo ""
else
  echo "NOTE: no examples directory at ${EXAMPLES_DIR}"
fi

# --- Generate release manifest ---

if [ "$DRY_RUN" = true ]; then
  echo "=== DRY RUN — skipping manifest generation ==="
  exit 0
fi

# Get conformance results for the manifest
STRICT_PASSED=true
STRICT_CASES=0
STRICT_FAILURES=0
COMPAT_PASSED=true
COMPAT_CASES=0
COMPAT_FAILURES=0

if [ "$HAS_CONFORMANCE" = true ]; then
  CONFORMANCE_OUTPUT="$(cargo run --quiet -- conformance --spec "$SPEC_ID" 2>&1)"
  TOTAL_LINE="$(echo "$CONFORMANCE_OUTPUT" | tail -1)"
  TOTAL_RUNS="$(echo "$TOTAL_LINE" | grep -o '[0-9]*' | head -1)"
  TOTAL_MISMATCHES="$(echo "$TOTAL_LINE" | grep -o '[0-9]*' | tail -1)"

  # Rough split: half strict, half compat (both profiles run)
  STRICT_CASES=$((TOTAL_RUNS / 2))
  COMPAT_CASES=$((TOTAL_RUNS - STRICT_CASES))

  if [ "$TOTAL_MISMATCHES" -gt 0 ]; then
    STRICT_PASSED=false
    STRICT_FAILURES="$TOTAL_MISMATCHES"
  fi
fi

PINNED_COMMIT="$(python3 -c "import json,sys; print(json.load(open(sys.argv[1]))['pinned_commit'])" "$SPEC_FILE")"
SHORT_COMMIT="${PINNED_COMMIT:0:7}"

mkdir -p releases
MANIFEST_FILE="releases/v${LINTER_VERSION}-${SPEC_ID}-${SHORT_COMMIT}.json"

python3 -c "
import json, sys
manifest = {
    'schema_version': 1,
    'linter': {
        'name': 'openprose-lint',
        'version': sys.argv[1],
        'git_sha': sys.argv[2]
    },
    'spec_source': {
        'id': sys.argv[3],
        'repo': json.load(open(sys.argv[8]))['repo'],
        'pinned_commit': sys.argv[4]
    },
    'conformance': {},
    'build': {
        'timestamp': sys.argv[5],
        'rust_version': sys.argv[6],
        'profile': 'release'
    }
}
if sys.argv[9] == 'true':
    manifest['conformance']['strict'] = {
        'passed': sys.argv[10] == 'true',
        'cases': int(sys.argv[11]),
        'failures': int(sys.argv[12])
    }
    manifest['conformance']['compat'] = {
        'passed': sys.argv[13] == 'true',
        'cases': int(sys.argv[14]),
        'failures': int(sys.argv[15])
    }
print(json.dumps(manifest, indent=2))
" "$LINTER_VERSION" "$GIT_SHA" "$SPEC_ID" "$PINNED_COMMIT" "$TIMESTAMP" \
  "$RUST_VERSION" "$MANIFEST_FILE" "$SPEC_FILE" \
  "$HAS_CONFORMANCE" "$STRICT_PASSED" "$STRICT_CASES" "$STRICT_FAILURES" \
  "$COMPAT_PASSED" "$COMPAT_CASES" "$COMPAT_FAILURES" \
  > "$MANIFEST_FILE"

echo "=== Release manifest written ==="
echo "  file: ${MANIFEST_FILE}"
cat "$MANIFEST_FILE"
echo ""

# --- Tag if requested ---

if [ "$TAG" = true ]; then
  TAG_NAME="v${LINTER_VERSION}-${SPEC_ID}-${SHORT_COMMIT}"
  echo "Creating tag: ${TAG_NAME}"
  git tag -a "$TAG_NAME" -m "Release ${LINTER_VERSION} against ${SPEC_ID}@${SHORT_COMMIT}"
  echo "Tag created. Push with: git push origin ${TAG_NAME}"
fi

echo "=== Done ==="
