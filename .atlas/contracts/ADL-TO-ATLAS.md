---
id: atlas.contract.adl-to-atlas
type: contract
status: active
canonical: true
---
# ADL to ATLAS Contract

## Purpose

This contract defines how authored Atlas Development Language source enters the same canonical semantic world as census-derived source knowledge.

ADL is an authoring surface. `*.atlas` is the dense canonical semantic artifact. They are not the same representation.

## Convergence rule

There is one semantic convergence point:

```text
EXISTING IMPLEMENTATION                     AUTHORED ADL
pinned source @ revision                    ADL source @ revision
        ↓                                           ↓
inventory / SourceFrontend                  parse / elaborate
        ↓                                           ↓
SemanticExtractor                           typed declared records
        ↓                                           │
typed observed records                      │
        └──────────────────┬────────────────┘
                           ↓
                         Census
                           ↓
                       Normalize
                           ↓
                       Reconcile
                           ↓
                    selected design
                           ↓
                       *.atlas
```

Direct ADL → EngineeringGraph and direct ADL → opaque binary publication are forbidden.

## Semantic mapping requirements

Every ADL construct that survives parsing MUST lower to typed Atlas semantic records with, where applicable:

- stable record identity;
- subject/scope identity;
- repository/design identity;
- source revision;
- semantic kind;
- typed payload;
- epistemic status;
- provenance/source span;
- compiler/extractor identity;
- evidence or declaration references;
- unresolved obligations/diagnostics.

A construct that cannot yet be represented losslessly MUST remain unsupported/unknown or block publication. It may not be flattened into an undocumented string merely to make compilation continue.

## Status rules

Raw ADL declarations enter as `DECLARED`.

Named deterministic compiler passes may emit `DERIVED` records with lineage to their ADL inputs and rule identity.

ADL compilation alone cannot create `OBSERVED` implementation evidence.

When authored ADL and observed implementation facts coexist, both survive into normalization/reconciliation. Agreement does not erase either epistemic path; disagreement becomes an explicit reconciliation obligation.

## Identity and references

ADL names are source-level handles, not automatically canonical global identities.

Resolution MUST account for enough repository/design, revision, module/scope, declaration and type/function identity to prevent accidental equivalence.

Name equality alone is never sufficient to merge records.

## Typed normalization

ADL-derived records participate in the same typed normalization rules as census-derived records.

Normalization may canonicalize representation where the rule is explicit. It may not:

- erase source spans or declaration lineage;
- turn source spelling into compiler-resolved identity without proof;
- choose a winner between conflicting declared/observed records;
- collapse distinct scopes because names match.

## Compatibility projections

Human-readable graph views, JSON reports and bootstrap `SemanticFact` triples may be derived from the typed semantic world.

They MUST NOT become the only carrier of ADL meaning.

The authoritative direction is:

```text
typed ADL semantics
→ normalized typed semantics
→ reconciled semantic world
→ graph / report / compatibility projection
```

Never the reverse.

## ATLAS publication

A SEALED logical Atlas produced from ADL MUST preserve all semantic information required to reproduce the selected design meaning, including:

- typed declarations and derived records;
- constraints/invariants;
- graph/binding relationships;
- state/effect/resource semantics;
- evidence/provenance;
- unresolved or rejected alternatives where Genome policy requires them;
- compiler/language/Genome version pins.

ATLAS may semantically compress repeated structure through interning, DAG sharing and content addressing. Compression may not delete meaning.

## Source retention

ATLAS is not required to embed ADL text when the authenticated source is retained externally and policy allows THIN mode.

FAT mode MAY embed compressed ADL/source blobs as evidence.

Embedded source is evidence/reconstruction material, not a replacement for canonical typed semantics.

## Materialization

`*.atlasx/` is a deterministic expanded executable representation selected from one logical Atlas root.

ADL does not directly own generated Rust, TypeScript, C, WASM or machine code. Those are downstream materializations/backends with lineage to the semantic world.

## Round-trip rule

A text regeneration from ATLAS is a projection unless exact source preservation is explicitly requested.

Semantic equivalence is required; byte-for-byte reproduction of original ADL formatting/comments is not required unless source blobs were embedded and that mode explicitly promises it.

## Versioning

An ADL language-version change or Atlas semantic-schema change that alters meaning MUST produce a new identity/version and explicit compatibility decision. Existing sealed artifacts are never silently reinterpreted under newer semantics.
