# Spec-Linter Integration Contract

This repo tracks `openprose/prose` explicitly, not loosely.

The integration contract is:

1. `openprose/prose` owns the language docs and example corpus.
2. This repo pins an exact prose commit through the `reference/openprose-prose` submodule.
3. `spec-support.json` declares which spec registry entry is the local default.
4. `specs/openprose.json` maps the current OpenProse layout:
   - Prose VM: `skills/open-prose/prose.md`
   - Forme: `skills/open-prose/forme.md`
   - deps: `skills/open-prose/deps.md`
   - legacy v0 compiler: `skills/open-prose/v0/compiler.md`
5. If a spec identity manifest exists in the pinned spec, `paths.version_manifest`
   points to it and `cargo run --bin openprose-lint -- specs verify --spec openprose`
   verifies the manifest hashes, repo identity, root ownership, skill metadata,
   and each artifact blob from the pinned submodule commit. The current pinned
   `openprose` entry intentionally has no `paths.version_manifest`, so this
   shortcut fails closed until the upstream spec ships the manifest.
6. `bun run true-up:gate` is the first repository-consistency gate after formatting.
7. `cargo test` is the first Rust behavioral gate.
8. `cargo run --bin openprose-lint -- lint --profile compat reference/openprose-prose/skills/open-prose/examples` is the smoke test for the current declarative example corpus. The public command surface intentionally exposes `lint` for current Markdown programs and `lint-legacy` for archived imperative programs; private generation-suffixed aliases are not valid commands.
9. `cargo run --bin openprose-lint -- conformance` only works when the selected spec publishes a conformance manifest.

## Profiles

- `strict`: release-gating behavior for the current normative spec
- `compat`: migration behavior for historical syntax and corpus drift

The current CLI default remains `compat` to preserve the existing smoke-test workflow while strict conformance is being established.

## Release choreography

1. Land spec changes in `openprose/prose`.
2. Tag or otherwise identify the spec commit to pin.
3. Bump the submodule in this repo to that commit.
4. If the pinned spec ships `skills/open-prose/spec-version.json`, set
   `paths.version_manifest` in `specs/openprose.json` and run
   `cargo run --bin openprose-lint -- specs verify --spec openprose`. Until then,
   keep `paths.version_manifest` unset so `--spec openprose` fails closed instead
   of implying the pinned checkout has been identity-manifested.
5. For package bundles, run `specs verify` in direct manifest mode with every
   declared package's `package.json`; package versions are provenance labels,
   while file hashes and the source identity are the contract.
6. Update `specs/openprose.json` and `spec-support.json` if paths or defaults changed.
7. Run local and CI tests.
8. Run conformance if the pinned spec includes a manifest.
9. Release the linter only if the relevant gates are green.

## Spec identity manifests

The optional spec identity manifest has schema `openprose.spec-identity` and is
verified by `openprose-lint specs verify`. It records the source repo, skill
version, `runtime_contract`, optional package versions, and SHA-256 digests for
load-bearing files such as `SKILL.md`, `contract-markdown.md`, `prose.md`, and
`forme.md`.

A manifest committed inside `openprose/prose` should not need to contain its own
git commit hash; that would be self-referential. The linter instead compares the
checkout HEAD to the external pin supplied by `specs/openprose.json` or
`--expect-commit`. Package bundles may include `source.commit` because the bundle
is generated after the source commit exists.

Direct checks must also supply a trusted repo identity through `--expect-repo`;
registry checks get it from `specs/openprose.json`. When a git repo is supplied,
the artifact root must live inside the checked git tree and each manifest digest
is compared to the blob at the pinned commit, not just to live filesystem bytes.
Package checks are complete, not best-effort: if the manifest declares a
package, verification requires a matching `package.json`. `SKILL.md`
frontmatter is parsed so the manifest's skill version and `runtime_contract`
cannot drift from the hashed skill document.

The verifier also checks the required artifact surface for the declared
`runtime_contract`. Contract 2 manifests must include ProseScript and
Responsibility Runtime artifacts in addition to the base Contract Markdown,
Forme, and Prose VM artifacts. Reactor docs are hashed and checked when a
manifest declares them, but historical Reactor package commits did not all ship
`reactor.md`.

Runtime contracts fail closed: a manifest with an unknown future
`runtime_contract` is invalid until prose-lint explicitly models that contract's
required artifact surface. Direct and package-bundle checks also reject
symlinked artifact paths, including symlinked ancestor directories, so a bundle
cannot satisfy a root-scoped manifest by pointing at files outside the declared
root.

## Drift policy

Drift is allowed to exist only in documented form:

- strict conformance failures block release when a conformance manifest exists
- compat drift may exist temporarily, but must remain explicit in diagnostics or manifests
- examples never override conformance expectations
