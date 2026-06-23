# Contributing to openprose-lint

This is the deterministic linter, LSP, and WASM build for [OpenProse](https://github.com/openprose/prose). It checks the static, spec-driven parts of the language that should stay deterministic even when execution is delegated to an LLM.

## Prerequisites

- **Rust 1.96+** (`rust-version = "1.96"` in `Cargo.toml`).
- **Bun 1.3+** for pinned repo-local development tooling, including `true-up`.
- **Submodules.** The OpenProse spec is vendored as a git submodule at `reference/openprose-prose`, and `build.rs` reads it at compile time to generate `spec_vocab.rs`. Without it, the build fails.

```bash
git submodule update --init --recursive
```

## Build

```bash
cargo build
```

This produces two binaries under `${CARGO_TARGET_DIR:-target}/debug/`:

- `openprose-lint` (the default; the CLI)
- `openprose-lsp` (the language server)

`publish = false` — this crate is not on crates.io, so there is no `cargo install` from a registry. Build from source.

## Running the linter

Run subcommands through `cargo run --` (or invoke the built binary directly):

```bash
# Lint a current OpenProse program (.md) or a directory of programs
cargo run -- lint path/to/program.md
cargo run -- lint --profile strict path/to/program.md

# Preflight briefing for VM agents (structured analysis)
cargo run -- briefing path/to/program.md

# Lint legacy imperative .prose files
cargo run -- lint-legacy path/to/file.prose

# Spec gap discovery across a corpus
cargo run -- discover path/to/programs/

# Runtime capability requirements
cargo run -- capabilities path/to/program.md

# Validate a deterministic adapter manifest
cargo run -- adapter validate specs/adapters/pi-v1-md.json
```

See `README.md` for the full command list, profiles (`compat` default, `strict`), exit codes, and the lint-rule catalog. When adding examples, keep the public surface to `lint` for current Markdown programs and `lint-legacy` for archived imperative programs; do not reintroduce private generation-suffixed aliases.

## CI is local — run it before you push

**This repo has no GitHub Actions.** `scripts/ci.sh` *is* the CI: a single local gate that runs the full check chain and exits nonzero on the first failure. Run it by hand before pushing:

```bash
bash scripts/ci.sh
```

It runs, in order:

1. `cargo fmt --check`
2. `bun install --frozen-lockfile`
3. `bun run true-up:gate`
4. `cargo clippy --all-targets --all-features -- -D warnings`
5. `cargo test`
6. `cargo build`
7. `cargo run -- specs`
8. `cargo run -- conformance`
9. `cargo run -- lint --profile compat reference/openprose-prose/skills/open-prose/examples`

### Activate the pre-push hook

The versioned hook at `.githooks/pre-push` delegates to `scripts/ci.sh`, so the gate runs automatically on every push. Activate it once per clone:

```bash
git config core.hooksPath .githooks
```

Or use the helper, which does the same thing:

```bash
bash scripts/install-hooks.sh
```

Escape hatches (for genuine emergencies only): `OPENPROSE_SKIP=1` bypasses the whole gate, and `OPENPROSE_SKIP_PUSH_HOOK=1` skips it on push.

## Tests and conformance

```bash
# Unit and integration tests
cargo test

# Conformance suite (vendored manifest + cases under specs/conformance/)
cargo run -- conformance
```

The conformance suite is vendored at `specs/conformance/` (`manifest.json` plus the cases in `specs/conformance/cases/`). Both are part of the CI gate.

## Code style

- `cargo fmt` must leave nothing to reformat (`cargo fmt --check` is in the gate).
- `cargo clippy --all-targets --all-features -- -D warnings` must pass — warnings are errors.

Run both locally before pushing; the gate will reject anything that does not pass.

## Spec source architecture

`openprose/prose` is the sole source of truth for the language. It is pinned as the `reference/openprose-prose` submodule (pin recorded in `specs/openprose.json`):

- VM spec: `reference/openprose-prose/skills/open-prose/prose.md`
- Forme (wiring): `reference/openprose-prose/skills/open-prose/forme.md`
- Deps: `reference/openprose-prose/skills/open-prose/deps.md`

To update to a newer spec, bump the submodule:

```bash
cd reference/openprose-prose && git fetch origin && git checkout origin/main && cd ../..
```

Build-time vocabulary extraction reads the spec during `cargo build` and regenerates `spec_vocab.rs`, so a submodule bump can change lint behavior — re-run `bash scripts/ci.sh` afterward.

## Decisions and agent notes

- `AGENTS.md` records repo-specific guidance for AI agents working here, including the doctrine in `docs/doctrine.md`. Read it before making claims about what conformance does and does not prove.
- This repo also tracks agent-to-agent context in mycelium git-notes. Check notes on files and commits before acting, and leave notes after meaningful work.

## License

MIT. By contributing, you agree your contributions are licensed under the same terms (see `LICENSE`).
