# Spec-Linter Integration Contract

This repo tracks `openprose/prose` explicitly, not loosely.

The integration contract is:

1. `openprose/prose` owns the language docs and example corpus.
2. This repo pins an exact prose commit through the `reference/openprose-prose` submodule.
3. `spec-support.json` declares which spec registry entry is the local default.
4. `specs/openprose.json` maps the current OpenProse layout:
   - v1 VM: `skills/open-prose/prose.md`
   - Forme: `skills/open-prose/forme.md`
   - deps: `skills/open-prose/deps.md`
   - legacy v0 compiler: `skills/open-prose/v0/compiler.md`
5. `bun run true-up:gate` is the first repository-consistency gate after formatting.
6. `cargo test` is the first Rust behavioral gate.
7. `cargo run --bin openprose-lint -- lint --profile compat reference/openprose-prose/skills/open-prose/examples` is the smoke test for the current declarative example corpus. The public command surface intentionally exposes `lint` for current Markdown programs and `lint-legacy` for archived imperative programs; private generation-suffixed aliases are not valid commands.
8. `cargo run --bin openprose-lint -- conformance` only works when the selected spec publishes a conformance manifest.

## Profiles

- `strict`: release-gating behavior for the current normative spec
- `compat`: migration behavior for historical syntax and corpus drift

The current CLI default remains `compat` to preserve the existing smoke-test workflow while strict conformance is being established.

## Release choreography

1. Land spec changes in `openprose/prose`.
2. Tag or otherwise identify the spec commit to pin.
3. Bump the submodule in this repo to that commit.
4. Update `specs/openprose.json` and `spec-support.json` if paths or defaults changed.
5. Run local and CI tests.
6. Run conformance if the pinned spec includes a manifest.
7. Release the linter only if the relevant gates are green.

## Drift policy

Drift is allowed to exist only in documented form:

- strict conformance failures block release when a conformance manifest exists
- compat drift may exist temporarily, but must remain explicit in diagnostics or manifests
- examples never override conformance expectations
