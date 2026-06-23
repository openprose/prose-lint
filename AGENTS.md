# Working in openprose-lint

Agent guidance for this repo, and the **canonical source of truth** for agent
instructions. `CLAUDE.md` is a symlink to this file, so every tool reads the same
guidance. The human-facing overview is in `README.md`; underground agent-to-agent
notes live in the mycelium git-notes.

Follow this file and the most specific `AGENTS.md` in the subtree you are editing.
Do not commit local tool metadata or generated workspace state.

## What this is

`openprose-lint` is a deterministic Rust linter + LSP + WASM build for the
**OpenProse** language. The language spec is the OpenProse repo, pinned here as a
submodule. Always init submodules before building:

```bash
git submodule update --init --recursive
```

## Source of truth

- **Spec:** `openprose/prose`, vendored at `reference/openprose-prose` (the sole
  spec source — see `specs/openprose.json`). Don't add other spec sources/forks.
- **Conformance:** vendored in-repo at `specs/conformance/` (manifest + cases).
  Run it with `cargo run -- conformance`.

## Doctrine

Read `docs/doctrine.md` before making claims about what this repo does or does not
prove.

Short version: **the coding agent is the compiler; this repo is a witness, not a
definition.** Conformance here is a track-1 mechanistic claim (integration
contract). Native OpenProse prose-completeness is a track-2 semantic claim, out of
scope for this suite by construction. Do not let a passing conformance run be
mistaken for an answer to a semantic question.

## CI is local, not GitHub Actions

This repo gates on a **local** test suite, not CI servers. The single entrypoint
is `scripts/ci.sh` — it runs formatting, the repo-local true-up gate, a
spec-hygiene check, clippy (`-D warnings`), tests, build, conformance, and lint
smoke checks, and exits non-zero on any failure.

```bash
bash scripts/ci.sh                       # the full gate — run before pushing
git config core.hooksPath .githooks      # activate the pre-push hook (runs ci.sh)
```

Toolchain is pinned to **Rust 1.96** (`rust-version` in `Cargo.toml`). Code must
pass `cargo fmt --check` and `cargo clippy --all-targets --all-features -- -D warnings`.

## Conventions

- **Tests:** `cargo test` (unit tests in `src/`, integration in `tests/`).
- **Drift/leak gating:** this repo is set up for [true-up](https://www.npmjs.com/package/true-up);
  `.true-up.json` is tracked, the `.true-up/` graph cache is gitignored. `scripts/ci.sh`
  runs `bun install --frozen-lockfile` and `bun run true-up:gate` before Rust checks.
- **Decisions:** record non-obvious decisions as mycelium git-notes (`mycelium.sh`).
- **Docs audiences:** `README.md` = external users; `AGENTS.md` (this file) = internal
  dev agents; `SKILL.md` (if present) = external agent users. Keep each to its audience.
- **Command surface:** document `lint` for current Markdown programs and `lint-legacy`
  for archived imperative programs. Do not restore private generation-suffixed lint aliases.
- **Specs:** `docs/specs/` holds accepted design contracts only — no `Status:`/`Draft`
  lifecycle labels (a `spec_hygiene` step in `scripts/ci.sh` fails if one reappears).
  Proposals live on a branch until merged; merging is acceptance. See `docs/README.md`.
