---
id: atlas.contract.semantic-facts
type: contract
status: active
canonical: true
---
# Semantic Facts Contract

R4 has one semantic language. Atlas MUST NOT invent separate ontologies for Genome, Census, Graph, ATLAS storage, or generated AtlasX products.

## Orthogonal axes

Every semantic record separates four concerns:

| Axis | Meaning | Examples |
| --- | --- | --- |
| FactKind | what the record says | FUNCTION_IDENTITY, CALL, STATE_ACCESS, EFFECT |
| EvidenceKind | what supports the record | SOURCE_CODE, TEST, RUNTIME_TRACE, COMPILER_METADATA |
| EpistemicStatus | how the claim is known | OBSERVED, DECLARED, DERIVED, INFERRED |
| Disposition | how an artifact or obligation was handled | PARSED, UNSUPPORTED, UNKNOWN, IGNORED_BY_EXPLICIT_POLICY |

These axes MUST NOT be collapsed into one enum or string field.

## Canonical epistemic status

The allowed status set is:

~~~text
OBSERVED
DECLARED
DERIVED
INFERRED
HYPOTHESIS
CONFLICT
UNKNOWN
UNSUPPORTED
IGNORED
~~~

Semantics:

- OBSERVED: directly obtained from admitted source, compiler/build metadata, test execution, or runtime observation.
- DECLARED: explicitly stated by repository-owned declarative input such as ADL or another admitted declaration.
- DERIVED: deterministically computed from admitted facts.
- INFERRED: evidence-backed but not deductively guaranteed.
- HYPOTHESIS: candidate explanation or design requiring validation.
- CONFLICT: admitted claims cannot currently be reconciled.
- UNKNOWN: an obligation exists but Atlas does not know the answer.
- UNSUPPORTED: the obligation is known but no admitted extractor can currently satisfy it.
- IGNORED: explicitly excluded from deeper processing by policy while remaining accounted for.

No status may be silently promoted. In particular, model output is never OBSERVED merely because a model produced it.

## Typed semantic records

The durable R4 ontology includes, at minimum:

~~~text
FunctionIdentity
FunctionSignature
SymbolFact
TypeFact
CallFact
ControlFlowFact
DataFlowFact
StateAccessFact
EffectFact
OwnershipFact
ConcurrencyFact
PersistenceFact
EvidenceLink
~~~

Additional typed records may be added only when they extend this ontology without creating a parallel truth system.

A Function in the canonical semantic world is not one generic triple. It is an identity linked to a set of typed records, each independently evidenced and revision-scoped.

## Bootstrap SemanticFact envelope

The existing Rust SemanticFact structure is a compatibility envelope for R4 bootstrap source/accounting and declared ADL facts.

It MAY transport simple normalized propositions during refoundation.

It MUST NOT become the permanent representation for all function semantics, CFG, dataflow, ownership, concurrency, persistence, or effects.

Typed records are required before a semantic dimension can be considered R4-complete.

## Identity

A semantic identity is deterministic for pinned input.

Identity MUST distinguish revisions and semantic propositions while allowing independent extractors to converge on the same proposition.

Raw extractor-local IDs are provenance; they are not canonical semantic identity.

## Evidence and provenance

Every semantic record MUST retain:

- source artifact identity;
- source revision when available;
- extractor identity;
- content hash when available;
- source span or equivalent location when applicable.

Evidence links are many-to-many. Multiple independent evidence records may support one semantic fact.

## Conflict

Conflicting observations are retained. Atlas MUST NOT choose one by last-write-wins, extractor priority, or model preference during normalization.

Conflict resolution belongs to reconciliation and must produce evidence for the resolution.

## Projection rule

Canonical semantic records may project into the universal engineering graph.

The graph is a derived projection. Graph materialization MUST NOT mutate the source semantic record or become a competing truth store.
