# Chronica documentation tooling

This directory owns documentation mechanics, not Chronica semantics.

## Commands

```bash
pnpm docs:check
pnpm docs:new -- <type> <docs/...md>
pnpm arch:contracts
pnpm arch:contracts:test
```

`pnpm docs:check` is the canonical documentation-governance command. It runs `tools/docs/documentation-governance.test.mjs` first, then validates repository layout/reference integrity with `tools/docs/check-layout.mjs`.

The governance test covers tooling metadata itself—taxonomy uniqueness, canonical-owner placement, fail-closed generator routing—and guards a small set of composition-level semantic boundaries such as `ExecutionAdmission != Canonicalization`, candidate-vs-selected binding and compile-vs-disclose. It does not score prose shape.

The layout checker validates exact canonical-owner registration, one annotated INDEX entry per maintained Markdown document, case collisions, stale/moved references, broken local Markdown links, and forbidden duplicate documentation universes.

Architecture contract tooling validates embedded `chronica-contract` data only where canonical owner documents intentionally declare it. Contract tooling does not require a minimum number of equations, invariants or performance contracts, and it does not validate prose by fixed wording/shape.

The checker deliberately does **not** score prose by character count, heading count, Mermaid presence, equation count, contract count, or a universal section skeleton.

## Tooling registry

`architecture-registry.mjs` is the single tooling metadata source for:

- approved docs top-level entries;
- router/governance paths;
- forbidden retired documentation roots;
- approved architecture responsibility directories;
- exact canonical architecture-owner paths;
- generated document kinds and their legal target shapes.

It is consumed by the layout checker, document generator and architecture-contract tooling so those tools cannot silently drift into different taxonomies.

This registry is **placement/governance metadata only**. It does not own architecture meaning; the documents it lists remain the semantic authorities.

## Closed-world architecture rule

`docs/architecture/` is fail-closed:

```text
README.md
+ exact paths in canonicalArchitectureOwners
= the complete maintained architecture document set
```

A file does not become architecture authority merely because it is placed under `docs/architecture/foundation/` or another approved directory. A genuinely new semantic owner requires architectural review, explicit registration in `canonicalArchitectureOwners`, and exactly one annotated entry in `docs/INDEX.md`.

`docs:new architecture-owner ...` may scaffold a candidate owner, but `docs:check` intentionally remains red until that candidate is explicitly registered after review.

## Document classes

- `architecture-owner` — one canonical semantic responsibility.
- `blueprint` — composition of existing owners.
- `decision` — an architecture decision and its consequences/supersession.
- `guide` — repeatable procedure; operational runbooks are guides and remain under `docs/guides/`.
- `reference` — stable facts, mappings or provenance.

Frontend and Ops documents may use domain-appropriate shapes while remaining subordinate to canonical architecture owners.

## Source-of-truth rule

A semantic responsibility has one canonical owner document. Do not maintain flat copies, compatibility prose, generated mirrors or archive directories as parallel sources of meaning. When a path changes, update references; Git history is the archive.

Templates are creation aids. They are not schema-level authority over human writing, and they do not prove runtime implementation.
