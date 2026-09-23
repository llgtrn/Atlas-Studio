---
id: atlas.contract.adl-to-atlas
type: contract
status: active
canonical: true
---
# ADL to ATLAS Contract

## Purpose

This contract defines how authored Atlas Development Language source and AI-assisted candidate implementation enter the same canonical semantic world as census-derived source knowledge.

ADL is an authoring surface. External provider output is candidate material. `*.atlas` is the dense canonical semantic artifact. They are not the same representation.

Human/AI collaboration is governed by `HUMAN-AI-ADL-AUTHORING.md`. Full creation/seal ordering is governed by `ATLAS-CREATION-PIPELINE.md`.

## Convergence rule

There is one semantic convergence point:

~~~text
EXISTING IMPLEMENTATION        AUTHORED ADL           AI-GENERATED CANDIDATE
pinned source @ revision       ADL @ revision         CandidateChangeSet source
        ↓                           ↓                         ↓
Inventory/SourceFrontend       parse/elaborate        untrusted Inventory/Frontend
        ↓                           ↓                         ↓
SemanticExtractor             typed DECLARED         SemanticExtractor
        ↓                           │                         ↓
typed OBSERVED                 │                  typed OBSERVED generated-source facts
        └──────────────┬────────────┴─────────────────────────┘
                       ↓
                     Census
                       ↓
                   Normalize
                       ↓
                   Reconcile
                       ↓
             validation / selection
                       ↓
                 SelectedDesign
                       ↓
              SEALED logical Atlas
                       ↓
          deterministic mechanical compaction
                       ↓
                    *.atlas
~~~

The synthesis provider's claim about what its code does remains separate from the semantic observations produced by censusing that code.

Direct ADL → EngineeringGraph, direct provider-output → canonical graph, direct provider-output → SelectedDesign and direct ADL/provider-output → opaque binary publication are forbidden.

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

## Candidate implementation reconciliation

When external AI generates implementation code during Atlas creation:

1. its intended semantics are candidate/declared claims;
2. its generated files enter the untrusted corpus path;
3. Atlas inventories and semantically censuses them;
4. observed generated implementation is compared with the provider's declared intent;
5. discrepancies become explicit obligations/conflicts;
6. security/dependency/license/test/benchmark/proof gates run;
7. only validated candidates may participate in selection.

A provider may not self-certify that generated code matches its proposal.

The machine candidate envelope is `../schemas/candidate-change-set.schema.json`.

Provider interaction lineage uses `../schemas/provider-receipt.schema.json`.

Typed fast-decision output uses `../schemas/decision-proposal.schema.json`.

## ATLAS publication

A SEALED logical Atlas produced from ADL MUST preserve all semantic information required to reproduce the selected design meaning, including:

- typed declarations and derived records;
- constraints/invariants;
- graph/binding relationships;
- state/effect/resource semantics;
- evidence/provenance;
- unresolved or rejected alternatives where Genome policy requires them;
- compiler/language/Genome version pins.

ATLAS may semantically compact repeated structure through exact interning, DAG sharing, factoring, graph/column packing and content addressing. Compaction may not delete meaning.

AI/research/decision/synthesis providers MUST NOT participate in the canonical post-seal compaction run. Post-seal compaction is mechanical and governed by `ATLAS-SEMANTIC-COMPACTION.md`.

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

## Implementation status: constraint/invariant evaluation

`constraint` and `invariant` blocks share one representation (`ConstraintDecl`/`ConstraintCheck`) and one evaluation path (`core::language::adl::evaluate_constraints`), materialized as `AdlCompileReport.constraint_results`.

- A constraint/invariant body that does not match a recognized check syntax (including an empty body) is diagnosed with `ATLAS-E052` at parse time. The declaration is still recorded (for provenance — Atlas observed that a constraint/invariant named X was declared), but it carries zero checks.
- A declaration with zero checks NEVER reports `passed: true`. Per `ARCHITECTURAL-INTEGRITY.md`'s rule that an invariant Atlas cannot evaluate is UNKNOWN/INCOMPLETE, not PASS, `evaluate_constraints` emits a defense-in-depth `ATLAS-E053` diagnostic and reports `passed: false` for any declaration with no evaluable checks, regardless of how it reached that state.
- `invariant` declarations are evaluated by the exact same pass as `constraint` declarations. They are not a documentation-only or declared-but-unchecked category.

This closes a real gap: earlier revisions silently returned `passed: true` for unrecognized constraint syntax (an unchecked constraint reporting success), and separately never evaluated `invariant` blocks at all (parsed and recorded, but absent from `constraint_results` and therefore invisible to `atlas-cli check`'s readiness gate).

## Implementation status: constraint_results gates coding_admission

A declared materialization's `MATERIALIZATION_LANGUAGE_NOT_OBSERVED` delta (the observed path exists, but none of its files are the declared `source_language`) now also becomes a failing `ConstraintResult` (`ATLAS-E054`), the same way `MISSING_MATERIALIZATION` already did (`ATLAS-E040`) — previously it was computed but only ever recorded as an inert `Diagnostic`-kind census fact, never gating anything. When both deltas would fire for the same materialization (the whole declared path is missing), only one failing result is produced, not two.

`runtime::systemize`'s `coding_admission` gate — which `runtime::prepare_work` builds its blocker list from, the gate AI-assisted work is checked against — previously inspected only `adl.diagnostics` (parse/link diagnostics: `ATLAS-E0xx` from unrecognized syntax, unknown relation/binding targets). It never inspected `adl.constraint_results`, the field `evaluate_constraints` and the materialization-delta conversion above actually report failures through. A declared constraint, invariant, or materialization could fail while `coding_admission.allowed` stayed `true`. It now raises `ADL_CONSTRAINT_VIOLATED` whenever any `constraint_results` entry has `passed: false`, via the pure predicate `adl_constraint_violation_blocks` (`runtime/src/lib.rs`).

This was not a hypothetical: this repository's own real `.atlas/declared/system.adl` declares `materialize WebUI { path = "apps/ui" }`, and `apps/ui/` does not exist in this repository. Before this fix, `atlas-cli check` correctly reported `ADL_CHECK_NOT_READY` for this, while `atlas-cli systemize`'s `coding_admission.allowed` simultaneously reported `true` for the exact same repository state — two canonical readiness gates disagreeing about the same real, present condition. Both now agree: `coding_admission.allowed` is `false` with blocker `ADL_CONSTRAINT_VIOLATED`, tracking the real, still-open `WebUI` materialization gap (a genuine pre-existing incompleteness in this repository's own self-declaration, not introduced by this fix — TARGET work, out of scope for this change, which is about the evaluation machinery being correct and consistent, not about completing every declared materialization).

## Implementation status: declared-edge identity cannot be tricked into colliding

`parse_relation` extracts a relation's `from`/`relation`/`to` from raw ADL source text via simple `split_once("->")`, not through a restrictive lexer — any of the three can contain a literal `:`. `compile_adl` previously joined them unescaped (`format!("{}:{}:{}", from, relation, to)`) to compute the declared edge's `stable_id`, so two genuinely different relations (e.g. `A ->r:B-> C` and `A ->r-> B:C`, both joining to `"A:r:B:C"`) could compute the identical edge id. Fixed: each field is now escaped (`core::identity::escape_identity_field`) before joining, the same fix applied to `core::census::dependency`'s `identity_key()` (`DEPENDENCY-CENSUS.md#implementation-status`) and `core::graph::engineering_graph`'s `Diagnostic` node id (`UNIVERSAL-GRAPH-CONTRACT.md#implementation-status`) — all three are the same collision class, closed the same way, in fields this codebase cannot prove are free of the separator because they come from a permissive parser rather than a real lexer. Found via falsification-first testing: the regression test was run against the unfixed code and confirmed to fail with a real id collision before the fix was written.
