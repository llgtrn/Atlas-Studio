# Atlas-Studio documentation-tooling source — noncanonical

This directory contains the **current local source checkout's documentation-tooling implementation**. It is not the canonical documentation, architecture, or nomenclature authority for Ynventa. The sole canonical concept registry is `Chronica/.ynventa/declared/nomenclature/concepts.jsonl` (21 concepts, DECISION 0115); architecture decisions and evidence are governed by current `.ynventa` facts. Source tooling may validate local fixtures but cannot establish a second, independent `docs/architecture/` authority.

## Commands

The root `package.json` currently exposes only `atlas:contract`, `atlas:systemize`, `atlas:docs:audit` and `atlas:code:analyze`; the four `pnpm docs:*`/`arch:contracts*` aliases formerly listed here are absent. Do not present them as runnable clean-checkout commands. Existing source entrypoints include `node tools/docs/check-layout.mjs`, `node --test tools/docs/documentation-governance.test.mjs` and `node tools/docs/new-doc.mjs <kind> <target>`; the generator writes a retired local corpus and is described below for source comparison, not prescribed for new Chronica documentation.

The historical `docs:check` alias was intended to run `tools/docs/documentation-governance.test.mjs` and `tools/docs/check-layout.mjs`. At this revision the alias is missing, the test cannot start without the undeclared `yaml` dependency, and the layout checker requires an absent `docs/architecture/` corpus. These are source-tool compatibility defects; do not recreate a second documentation universe or weaken a gate to claim convergence.

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

A source-tool corpus document never becomes architectural authority through directory placement. The existing checker/generator metadata still demands `canonicalArchitectureOwners` and an annotated `docs/INDEX.md` entry for that retired corpus; this is an unconverted source behavior, not the approved way to allocate a new semantic owner. Current ownership changes require an explicit authorized owner decision in admitted `.ynventa` knowledge, then derived documentation and implementation mapping.

The existing `new-doc.mjs architecture-owner ...` source program scaffolds into that retired corpus and asks users to register a local Markdown owner. Do not use it for new canonical work until an authorized implementation change reconciles it with the admitted `.ynventa` workflow. Retain its templates and tests as source evidence; Markdown registration alone never admits a decision.

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
