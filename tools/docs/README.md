# Atlas-Studio documentation-tooling source — noncanonical

This directory contains the **current local source checkout's documentation-tooling implementation**. It is not the canonical documentation, architecture, or nomenclature authority for Ynventa. The sole canonical concept registry is `Chronica/.ynventa/declared/nomenclature/concepts.jsonl` (21 concepts, DECISION 0115); architecture decisions and evidence are governed by current `.ynventa` facts. Source tooling may validate local fixtures but cannot establish a second, independent `docs/architecture/` authority.

## Commands

```bash
pnpm docs:check
pnpm docs:new -- <type> <docs/...md>
pnpm arch:contracts
pnpm arch:contracts:test
```

`pnpm docs:check` is **this source checkout's local documentation/fixture governance command**, not the global eight-repository architecture or nomenclature verification. It runs `tools/docs/documentation-governance.test.mjs` first, then validates repository layout/reference integrity with `tools/docs/check-layout.mjs`.

The governance test covers tooling metadata itself—taxonomy uniqueness, canonical-owner placement, fail-closed generator routing—and guards a small set of composition-level semantic boundaries such as `ExecutionAdmission != Canonicalization`, candidate-vs-selected binding and compile-vs-disclose. It does not score prose shape.

The layout checker validates exact canonical-owner registration, one annotated INDEX entry per maintained Markdown document, case collisions, stale/moved references, broken local Markdown links, and forbidden duplicate documentation universes.

Architecture contract tooling validates embedded `chronica-contract` data only where canonical owner documents intentionally declare it. Contract tooling does not require a minimum number of equations, invariants or performance contracts, and it does not validate prose by fixed wording/shape.

The checker deliberately does **not** score prose by character count, heading count, Mermaid presence, equation count, contract count, or a universal section skeleton.

## Tooling registry

`architecture-registry.mjs` is a local metadata source for the existing generator/checker implementation; the file must not invent a separate global ontology or supersede `.ynventa` facts:

- approved docs top-level entries;
- router/governance paths;
- forbidden retired documentation roots;
- approved architecture responsibility directories;
- exact canonical architecture-owner paths;
- generated document kinds and their legal target shapes.

It is consumed by the layout checker, document generator and architecture-contract tooling so those tools cannot silently drift into different taxonomies.

This registry is **local source-tool placement metadata only**. Neither this metadata nor the documents it lists are Chronica's canonical semantic authority; those are governed by the current `.ynventa` facts/decision and the registered nomenclature. Exact frozen source paths remain evidence, not live architecture owners.

## Local documentation-corpus checking — not a second canonical architecture

The checker described below treats its **source-corpus `docs/architecture/` input** as fail-closed if that input exists. This is not permission to make or preserve a competing global documentation tree; the canonical source layout is the universal biological grammar:

```text
README.md
+ exact paths in canonicalArchitectureOwners
= the complete maintained architecture document set
```

A document in this source-tool corpus does not become architecture authority through directory placement. The global fact/decision owner remains in the admitted `.ynventa` knowledge and 21-concept registry. A genuinely new semantic owner requires architectural review, explicit registration in `canonicalArchitectureOwners`, and exactly one annotated entry in `docs/INDEX.md`.

`docs:new architecture-owner ...` may scaffold a candidate owner, but `docs:check` intentionally remains red until that candidate is explicitly registered after review.

## Document classes

- `architecture-owner` — one canonical semantic responsibility.
- `blueprint` — composition of existing owners.
- `decision` — an architecture decision and its consequences/supersession.
- `guide` — repeatable procedure; operational runbooks are guides and remain under `docs/guides/`.
- `reference` — stable facts, mappings or provenance.

Frontend/operations source documents here are development or migration evidence only. Product UI/renderers and design-system implementation converge exclusively into Chronica `phenotype/`, with many Phenora expressions.

## Source-of-truth rule

A semantic responsibility has one canonical **registered owner and admitted fact/decision provenance**, not a competing source Markdown document. Do not maintain flat copies, compatibility prose, generated mirrors or archive directories as parallel sources of meaning. When a path changes, update references; Git history is the archive.

Templates are creation aids. They are not schema-level authority over human writing, and they do not prove runtime implementation.
