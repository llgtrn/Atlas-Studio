---
id: atlas.contract.normalization
type: contract
status: active
canonical: true
---
# Normalization Contract

## Purpose

Normalization converts raw typed observations into deterministic canonical semantic representation. It standardizes representation; it does not grant truth, resolve disputes, or erase provenance.

~~~text
Raw typed observations
→ canonicalize
→ resolve stable identities where proven
→ form exact equivalence classes
→ deduplicate only exact semantic duplicates
→ preserve conflict candidates
→ normalized facts
→ reconciliation
~~~

## Non-authority rule

Normalization MUST NOT:

- promote DECLARED, INFERRED or HYPOTHESIS to OBSERVED;
- turn UNKNOWN into verified absence;
- turn UNSUPPORTED into UNKNOWN or vice versa;
- select a winner between conflicting observations;
- infer identity from name similarity alone;
- discard evidence because another record looks equivalent;
- write truth outside the normalized semantic path.

## Canonicalization

Canonicalization is semantic-family specific.

Safe examples include deterministic predicate/tag casing, path separator normalization after repository identity is known, canonical language/ABI identifiers, enum serialization and stable ordering where order is semantically irrelevant.

Potentially unsafe transforms such as identifier case folding, Unicode rewriting, symlink resolution, type-alias collapse or macro expansion require language/profile-specific rules and evidence.

## Identity

Identity resolution precedes semantic dedup.

A normalized identity is scoped by enough information to avoid accidental aliasing: repository, revision, namespace/module/type/function scope, declaration identity, callsite/block identity or content fingerprint as applicable.

Name equality is not identity equality. Overloads, shadowed/generated symbols and cross-repository references stay distinct unless a binding/equivalence record proves sameness.

## Record identity

~~~text
RawRecordId
  = extractor-specific observation identity

NormalizedRecordId
  = deterministic identity of canonical semantic content
    within its required repository/revision/scope
~~~

Normalized IDs MUST NOT depend on input enumeration order.

Changing evidence multiplicity alone need not change semantic identity; changing semantic content, scope or required revision identity does.

## Equivalence

Records enter one equivalence class only when a deterministic rule proves they express the same semantic claim in the same required scope/revision.

Allowed proof sources include identical stable identities plus identical typed payload, compiler-provided canonical identity, explicit alias/binding semantics and language/profile canonicalization rules.

Forbidden shortcuts include same display name, unqualified text, fuzzy similarity, model judgment without corroboration or same source span under different revisions.

## Deduplication

Exact semantic duplicates may merge only when:

1. FactKind matches;
2. canonical subject/scope/revision identities match;
3. typed payload is equivalent;
4. EpistemicStatus is compatible without promotion;
5. all evidence/provenance links are retained or unioned;
6. source extractor identities remain attributable.

Different epistemic statuses are not automatically duplicates. A DECLARED claim and an independently OBSERVED fact may agree semantically while both epistemic paths remain inspectable.

The current R4 bootstrap normalization is intentionally lossless and may perform zero deduplication until these rules are implemented.

## Conflict handling

Normalization may detect a conflict candidate but does not resolve it.

Incompatible types, state semantics or extractor call-target sets remain present with their evidence. Reconciliation owns emission/closure of CONFLICT obligations.

## Provenance

Every normalized record retains lineage to all raw inputs that contributed to it.

Atlas must be able to answer which artifact/revision, extractor/version, source span/metadata record, normalization rule and raw record IDs produced a normalized record.

Provenance is semantic data, not debug logging.

## Epistemic preservation

EpistemicStatus is part of the normalized semantic record. Normalization may canonicalize serialized spelling but may not change meaning. A later status transition is a new derivation/reconciliation event with evidence.

## Disposition preservation

Artifact/obligation disposition remains separately inspectable even when it caused an emitted UNKNOWN, UNSUPPORTED or IGNORED status.

Normalization never replaces ArtifactDisposition with EpistemicStatus.

## Deterministic ordering

Recommended physical sort key:

~~~text
FactKind
→ repository/revision
→ scope identity
→ subject identity
→ relation/typed payload identity
→ epistemic status
→ normalized record id
~~~

Ordering is for deterministic representation, not semantic truth.

## Closure invariants

~~~text
input semantic information
= normalized semantic information
+ explicitly represented exact-dedup equivalence
~~~

No semantic claim may disappear without an equivalence/accounting reason.

A normalization run reports input count, output count, exact duplicates merged, equivalence classes, conflict candidates, unknown/unsupported/ignored counts, provenance completeness and deterministic output hash when available.

## Bootstrap compatibility

The current implementation that trims subject/object text and canonicalizes predicates is an N0 bootstrap pass. It is valid because it is lossless and status-preserving.

R4 must not label N0 as full semantic normalization until stable identity, equivalence, exact dedup and conflict-preservation behavior above are implemented and tested.
