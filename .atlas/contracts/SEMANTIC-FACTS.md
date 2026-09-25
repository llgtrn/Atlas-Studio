---
id: atlas.contract.semantic-facts
type: contract
status: active
canonical: true
---
# Semantic Facts Contract

## Purpose

R4 converts admitted engineering evidence into one typed semantic language. Source extraction, census, normalization, reconciliation, the engineering graph, ATLAS and later compiler stages MUST NOT invent separate truth vocabularies.

The current Rust **SemanticFact** record is a bootstrap envelope. It may carry facts while R4 is materialized, but it MUST NOT become the permanent universal model as an untyped subject/predicate/object database.

## Four independent taxonomies

Atlas keeps four dimensions separate:

~~~text
FactKind
EvidenceKind
EpistemicStatus
Disposition
~~~

- **FactKind** answers what semantic record exists.
- **EvidenceKind** answers what evidence supports, contradicts or produced it.
- **EpistemicStatus** answers what Atlas knows about the statement at a pinned scope/revision.
- **Disposition** answers how an artifact or obligation was operationally accounted for.

No value in one taxonomy may be silently substituted for another.

## Canonical EpistemicStatus

Canonical serialized values:

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

| Status | Meaning |
| --- | --- |
| **OBSERVED** | Directly extracted or measured from admitted evidence at a pinned revision. |
| **DECLARED** | Authored declaration, contract, configuration or design statement; not automatically verified implementation. |
| **DERIVED** | Deterministic consequence of admitted facts under a named rule/pass. |
| **INFERRED** | Non-deductive conclusion supported by evidence but not proven by extraction/derivation rules. |
| **HYPOTHESIS** | Candidate explanation or proposed fact awaiting corroboration. |
| **CONFLICT** | Incompatible claims/facts remain unresolved for one semantic obligation. |
| **UNKNOWN** | The obligation exists but available evidence is insufficient. |
| **UNSUPPORTED** | The current extractor/runtime cannot evaluate the obligation. |
| **IGNORED** | Explicitly excluded by policy for this scope, with the policy decision evidenced. |

UNSUPPORTED and IGNORED here describe the state of a semantic obligation. They are not aliases for **ArtifactDisposition**.

A status transition MUST retain prior evidence lineage. Normalization may not promote or demote status.

## FactKind

The bootstrap runtime enum **SemanticFactKind** is the current carrier for FactKind. R4 deep semantics MUST converge toward explicit typed records rather than indefinitely adding free-form predicates.

Minimum function-level family:

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

Domain-specific kinds may extend this family without replacing the universal spine.

## Common semantic header

Every typed semantic record MUST be attributable through a logical header equivalent to:

~~~text
SemanticRecordHeader
├─ record_id
├─ fact_kind
├─ epistemic_status
├─ subject_identity
├─ scope_identity
├─ repository_identity
├─ revision
├─ extractor_identity
├─ evidence_refs[]
└─ provenance
~~~

Physical formats may intern or compress fields, but their meaning must survive.

## Function representation

A canonical function is an identity with typed facets, not one generic triple.

### FunctionIdentity

Must account for stable function/method identity, repository and revision, language, containing scope, declaration/definition span, symbol identity and generated/macro origin when applicable.

### FunctionSignature

Must account for parameters/order/types, return type, generics, ABI/calling convention, visibility/export surface, async/coroutine form and language-specific safety/effect qualifiers when relevant.

### SymbolFact

Represents declaration/reference/definition identity and name resolution. Overloads, namespaces and shadowing MUST be scope-aware. Name equality alone is not identity equivalence.

### TypeFact

Represents canonical type identity and the semantic role linking a subject to that type. Language syntax may remain evidence while normalized type identity remains scope/revision aware.

### CallFact

Must distinguish:

~~~text
STATIC_RESOLVED
DYNAMIC_RESOLVED_SET
DYNAMIC_PARTIAL
UNRESOLVED
~~~

A call record contains caller, callsite, candidate/resolved callee identities, dispatch kind and argument/result bindings when known. Dynamic calls are explicit; failure to resolve never erases the call.

A callsite is identified by its caller and its anchor token: the callee's name (method identifier or last path segment), or the argument list's opening parenthesis when the callee is not a path. The anchor belongs to exactly one call expression, so no two calls (in particular the calls of a chain, which share their first token) can share an identity; an expression's start position is not a callsite identity (G74).

### ControlFlowFact

Represents blocks, terminators, successors, exceptional/unwind edges and entry/exit relationships. CFG identity is function- and revision-scoped.

### DataFlowFact

Represents value/definition/use flow, including unresolved aliasing where applicable. Data flow must not be fabricated from lexical proximity.

### StateAccessFact

Represents READ, WRITE, TRANSITION, CREATE or DELETE against a typed state identity, with operation site, alias/resolution status and transaction/authority boundary when known.

### EffectFact

Represents observable effects including filesystem, network, process, FFI, persistence, event emission, authorization checks, allocation and panic/failure behavior.

Reads of ambient inputs are effects of their own kind, not I/O: `ENVIRONMENT_READ` covers process environment variables, arguments and the working and well-known directories, and `CLOCK_READ` covers reads of a clock. The same code reads different values in different processes or at different times, and neither is honestly `EXTERNAL_IO` (G91, ADR 0036).

### OwnershipFact

Represents ownership/borrow/move/copy/allocation/free/escape behavior where the source language/runtime exposes it.

### ConcurrencyFact

Represents task/thread/process boundaries, locks, atomics, channels, scheduling relationships and known ordering facts.

### PersistenceFact

Represents durable read/write/commit/rollback/checkpoint/recovery relationships and durable resource identity.

### EvidenceLink

Links a semantic record to evidence with a typed relation such as SUPPORTS, CONTRADICTS, DERIVES_FROM, OBSERVED_AT or DECLARED_BY.

Evidence links are first-class. Provenance must not be reconstructed later from filenames or UI state.

## EvidenceKind

R4 uses a distinct evidence vocabulary. Minimum kinds:

~~~text
SOURCE_TEXT
PARSER_OUTPUT
COMPILER_METADATA
BUILD_METADATA
TEST_OBSERVATION
RUNTIME_TRACE
BINARY_INSPECTION
DECLARED_ADL
POLICY_RECORD
RESEARCH_REFERENCE
DERIVATION_RECORD
~~~

Research references may support INFERRED or HYPOTHESIS claims, but do not become OBSERVED implementation evidence without corroboration against the admitted target revision.

## Disposition

Artifact and obligation disposition remains operational. The existing **ArtifactDisposition** belongs to this dimension.

A disposition may cause an emitted fact to receive UNKNOWN, UNSUPPORTED or IGNORED, but the disposition enum and epistemic enum remain distinct identities.

## Negative facts

Verified absence is not UNKNOWN. A claim such as “function F performs no network send” is valid only when the relevant obligation set was exhaustively checked and closure evidence identifies the pass and scope. Failure to find an effect without closure remains UNKNOWN.

## Conflict preservation

Incompatible observations or claims remain independently addressable with their evidence. Atlas may additionally emit a CONFLICT record referencing them, but normalization may not overwrite a side or choose a winner.

## Bootstrap compatibility rule

Until typed records are materialized in core/, **SemanticFact { subject, predicate, object }** may transport facts between current R4 stages only if:

1. kind and status are typed;
2. provenance is preserved;
3. the record can be losslessly mapped to its target typed FactKind;
4. no new semantic dimension is modeled only as an undocumented free-form predicate;
5. graph/ATLAS projections remain derived and never become competing truth.

## R4 migration invariant

~~~text
bootstrap SemanticFact envelope
        ↓
typed function/symbol/type/call/control/data/state/effect records
        ↓
normalized typed records
        ↓
reconciled semantic world
        ↓
derived engineering graph / ATLAS
~~~

The reverse direction is forbidden. Atlas must not collapse typed semantics into a permanent generic triple store.
