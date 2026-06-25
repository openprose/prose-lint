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
5. `cargo run --bin openprose-lint -- specs verify --spec openprose` verifies
   repo identity, root ownership, and artifact blobs from the pinned submodule
   commit. If the pinned spec ships a spec identity manifest,
   `paths.version_manifest` points to it and the verifier checks the manifest
   hashes, skill metadata, and package provenance. Without a manifest, the
   verifier synthesizes a source-identity check from the registry-declared
   load-bearing paths.
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
4. Run `cargo run --bin openprose-lint -- specs verify --spec openprose`. If the
   pinned spec ships `skills/open-prose/spec-version.json`, set
   `paths.version_manifest` in `specs/openprose.json`; otherwise the command
   uses the registry-declared source-identity fallback.
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
`forme.md`. Registry mode can also verify a pinned source without an upstream
manifest by synthesizing the artifact set from `specs/openprose.json`; that
fallback proves the checked-out source and declared artifact blobs, but it does
not claim package provenance or skill metadata that the upstream manifest has not
declared.

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
