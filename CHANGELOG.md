# Changelog

All notable changes to `openprose-lint` are documented here.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

Note: `publish = false` in `Cargo.toml` — this crate is not released to crates.io.
"Releases" here are git tags and source distribution under the OpenProse org.

## [Unreleased]

Pre-public readiness pass ahead of open-sourcing under the OpenProse org.

### Changed

- Made `openprose/prose` the sole spec source of truth. The reference spec is
  vendored as the `reference/openprose-prose` git submodule, pinned by commit in
  `specs/openprose.json`. (A previously referenced second fork has been removed.)
- Replaced GitHub Actions with a local CI gate. `scripts/ci.sh` runs the full
  check chain: `cargo fmt --check`, `bun install --frozen-lockfile`,
  `bun run true-up:gate`, `cargo clippy --all-targets --all-features
  -D warnings`, `cargo test`, `cargo build`, then `cargo run -- specs`,
  `cargo run -- conformance`, and `cargo run -- lint --profile compat` over
  the spec's bundled examples. There is no `.github/workflows`.
- Made the current declarative Markdown linter the public `lint` command.
  Legacy imperative `.prose` linting remains available as `lint-legacy`, and
  private generation-suffixed lint aliases were removed rather than preserved
  as compatibility commands.
- Pinned the toolchain to Rust 1.96.0 in the pre-push hook; `rust-version` in
  `Cargo.toml` is `1.96`.
- Treated `docs/specs/` as accepted design contracts only. Removed the per-spec
  `Status:` labels and added a `spec_hygiene` step to `scripts/ci.sh` that fails
  if a lifecycle/status label reappears there. A spec lives in `docs/specs/` once
  its model ships and is gated; proposals stay on a branch until merged.

### Added

- Vendored conformance suite at `specs/conformance/` (a `manifest.json` plus
  six `.prose` cases) so `cargo run -- conformance` runs offline against fixed,
  in-repo expectations.
- `specs verify`, a deterministic spec identity verifier for package bundles and
  git-pinned checkouts. It checks an `openprose.spec-identity` manifest against
  SHA-256 artifact hashes, complete package.json provenance for declared
  packages, trusted repo identity, SKILL metadata, checkout root ownership, and
  artifact blobs from an external expected git commit. It also enforces the
  required load-bearing artifact surface for the declared `runtime_contract`,
  rejects unknown future runtime contracts, and rejects symlinked artifact paths.
- Git pre-push hook (`.githooks/pre-push`) that delegates to `scripts/ci.sh`.
  Contributors activate it with `git config core.hooksPath .githooks`.
- `LICENSE` (MIT, "Copyright (c) 2025-2026 OpenProse"), matching the
  `license = "MIT"` field already declared in `Cargo.toml`.
- `CONTRIBUTING.md`.

### Removed

- The `demo/` browser editor (a CodeMirror + WASM/LSP playground) and its
  `@antithesishq/bombadil` UI-test scaffolding. Prose is now written mostly by
  agents rather than by hand in a browser, so the human-facing editor was
  dropped. The linter library still compiles to WebAssembly (`--lib`).

### Fixed

- Renamed the current Markdown linter internals away from the old generation
  label (`src/current_lint.rs`) so source, tests, docs, and true-up edges share
  the public mental model.
- `cargo build --target wasm32-unknown-unknown` now succeeds. The native-only
  `openprose-lint` and `openprose-lsp` binaries are compiled out under wasm32
  (only the embeddable `cdylib` library is built).
- Resolved all Rust 1.96 clippy lints; the clippy gate is clean under
  `-D warnings`.
- Made the host-mediated adapter dogfood tests hermetic (per-test staging dir +
  serialization) to remove an intermittent parallel-test failure.

## [0.2.0]

Linter, LSP, and WASM build for the OpenProse language (markdown-based,
multi-agent orchestration). Ships two binaries — `openprose-lint`
(default-run) and `openprose-lsp` — plus a `cdylib` for WASM.

### Added

- `lint`: lints current-spec `.md` programs (single files or program
  directories) with heading classification, contract parsing (`## Contract`
  and bare labels), service resolution, and `use:` import awareness. Rule
  families: structural and frontmatter errors (`MDE0xx`) and vocabulary,
  contract, component, and cross-validation warnings (`MDW0xx`).
- `briefing`: emits a versioned, structured preflight block (`openprose-lint
  briefing v1`) for a Prose-Complete VM agent — pre-parsed contract, service
  resolution, feature flags, and a diagnostic summary.
- `conformance`: runs a spec manifest of cases against fixed per-profile
  expected diagnostics and reports mismatches.
- `capabilities`: infers a program's runtime capability requirements and
  implied substrate dependencies, with optional `--runtime-manifest` to check a
  self-declared runtime against those requirements.
- `adapter validate`: validates a deterministic adapter manifest that pins
  exact OpenProse files, channels, and phase attachments.
- `adapter dogfood`: renders an adapter's exact prompts and files, stages a temp
  program copy, runs a live adapter-initialized agent, and validates the root
  VM's final JSON proof payloads — checking published outputs on disk and that
  an `--expect-binding` output is present and non-empty.
- `lint-legacy`: lints legacy v0 `.prose` files; vocabulary is extracted from the spec
  at build time (`build.rs` → `spec_vocab.rs`).
- `discover`: reports undocumented vocabulary across a corpus (spec-gap
  discovery).
- `specs`: lists available spec sources.
- `openprose-lsp` binary: an LSP server (diagnostics, hover) for OpenProse
  programs.
- `compat` (default) and `strict` lint profiles, selectable with `--profile`.

### Exit codes

- `0`: no lint errors.
- `1`: one or more lint errors (warnings do not fail the run).
- `2`: CLI usage or filesystem error.
