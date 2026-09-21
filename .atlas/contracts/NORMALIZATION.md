---
id: atlas.contract.normalization
type: contract
status: active
canonical: true
---
# Normalization Contract

Atlas has one normalized semantic path:

~~~text
raw admitted observations
→ deterministic normalization
→ reconciliation
→ universal graph projection
~~~

No extractor, UI, compiler stage, or ATLAS serializer may maintain a parallel normalized truth universe.

## Purpose

Normalization removes representational variance. It does not decide truth.

Normalization MAY:

- trim and canonicalize stable textual forms;
- canonicalize predicate vocabulary;
- normalize path and symbol spelling according to language rules;
- assign deterministic semantic proposition IDs;
- group equivalent propositions from independent extractors;
- surface conflict candidates.

Normalization MUST NOT:

- upgrade epistemic status;
- discard provenance;
- use last-write-wins;
- perform fuzzy semantic merging without an explicit later reconciliation rule;
- convert UNKNOWN or UNSUPPORTED into verified absence.

## Proposition identity

Bootstrap normalized proposition identity is derived from:

~~~text
revision scope
+ FactKind
+ canonical subject
+ canonical predicate
+ canonical object
~~~

The raw extractor-local fact ID is not part of canonical proposition identity.

Two independent extractors that emit the same canonical proposition for the same revision therefore converge on the same normalized ID.

## Equivalence

Exact canonical proposition equality creates an equivalence class.

Normalization retains all supporting records or provenance. Deduplication MUST NOT erase independent evidence.

Semantic equivalence that requires language reasoning, alias analysis, type resolution, or behavioral inference is not an exact normalization operation and belongs to reconciliation.

## Conflict candidates

A conflict slot is identified by:

~~~text
revision scope
+ FactKind
+ canonical subject
+ canonical predicate
~~~

If admitted positive claims occupy the same conflict slot with different canonical objects, normalization records a conflict candidate.

UNKNOWN, UNSUPPORTED, and IGNORED are accounting states and do not by themselves create a positive-value conflict.

Normalization reports conflict candidates; reconciliation decides whether they are genuine conflicts and how they are represented canonically.

## Ordering

Normalized output is deterministically ordered by FactKind, subject, predicate, object, normalized ID, and provenance.

## Losslessness

Until evidence aggregation is materialized, normalization retains one row per input fact.

Equivalent rows may share the same normalized semantic ID. This is intentional: identity converges while provenance remains lossless.

## Graph boundary

Only normalized and then reconciled semantics are eligible to become canonical graph materialization.

Current bootstrap graph projection may consume normalized facts before full reconciliation only while R4 completion matrix marks that path as bootstrap.
