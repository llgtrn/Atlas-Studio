---
id: atlas.contract.census-completeness
type: contract
status: active
canonical: true
---
# Census Completeness Contract

Atlas census is proof-producing accounting, not best-effort repository summarization.

For a pinned corpus, repository revision and Genome, every admitted artifact and every discovered executable scope MUST be accounted for. Atlas may report UNKNOWN or UNSUPPORTED, but it may not silently omit.

## Closure pipeline

```text
corpus inventory
  ↓
artifact classification
  ↓
syntax / build / dependency passes
  ↓
semantic extractors → raw typed observations
  ↓
census obligation accounting
  ↓
deterministic normalization
  ↓
binding / temporal / evidence passes
  ↓
multi-engine + cross-scope reconciliation
  ↓
adversarial gap queries
  ↓
fixed point
  ↓
CensusCertificate
  ↓
SEALED *.atlas eligibility
```

## Inventory closure

Every tracked/admitted file, generated input, build definition, schema, configuration, migration, test fixture, binary descriptor or unsupported artifact receives an accounting record with identity, hash, class and disposition.

UNKNOWN is explicit. Silent skip is forbidden.

## Function and semantic accounting

Every discovered function/method is represented in Atlas. Adaptive census controls depth, not existence.

The canonical typed record vocabulary is defined by SEMANTIC-FACTS.md. Extractor obligations are defined by SEMANTIC-EXTRACTION.md. Generic SemanticFact triples are bootstrap transport only and do not satisfy typed function-semantic closure.

Each function MUST account for, when applicable:

- identity, signature, types, generics and visibility;
- containing scope and revision;
- callers/callees and unresolved dynamic call targets;
- control-flow graph;
- reads/writes and dataflow;
- state transitions;
- effects and external interactions;
- ownership/borrow/move/copy/allocation behavior;
- concurrency, locks, atomics and transaction boundaries;
- persistence/recovery behavior;
- interfaces/capabilities/bindings;
- constraints/invariants and failure paths;
- tests, runtime traces, source spans and other evidence;
- epistemic status and unresolved unknowns.

Important blocks and expressions are lowered to semantic atoms such as CALL, LOAD, STORE, READ_STATE, WRITE_STATE, EMIT_EVENT, AUTH_CHECK, LOCK, ATOMIC, ALLOC, FREE, PERSIST, NETWORK_SEND, FFI_CALL, BRANCH, PANIC and RETURN.

## Multi-engine reconciliation

Atlas SHOULD use independent extractors where available. Parser, semantic index, compiler metadata, build graph, tests and runtime traces may disagree. Disagreement produces CONFLICT and deeper census; it is never silently collapsed.

## Bidirectional closure

Low-level facts must aggregate upward. High-level claims must decompose downward.

Example:

```text
Function F WRITE State X
  → Module MUST expose mutation of X

Repository claims Capability Y
  → MUST resolve interface/binding/implementation/effect/evidence path
```

## Negative evidence

"None" requires proof that the relevant obligation set was exhaustively checked. Failure to find something is UNKNOWN, not verified absence.

## Seal formula

A logical Atlas may be sealed only when required closure classes satisfy policy:

```text
Inventory
∧ Parse/Accounting
∧ Semantic Obligations
∧ Binding
∧ Evidence
∧ Temporal
∧ Cross-Scope Reconciliation
∧ Fixed Point
```

Critical scopes may require UNKNOWN = 0.

## CensusCertificate

The normative certificate contract is CENSUS-CERTIFICATE.md and its machine schema is contracts/schema/census-certificate.v1.schema.json.

Every sealed root records at least corpus identity, revision set, Genome hash, inventory totals, accounted totals, semantic coverage, unresolved/unsupported artifacts, dynamic edges, binding gaps, conflicts, fixed-point iteration count, independent-pass agreement and Atlas root hash.

States:

```text
DRAFT → CENSUSED → RECONCILED → CLOSED → SEALED
```

Only policy-eligible SEALED Atlas roots may materialize production AtlasX.
