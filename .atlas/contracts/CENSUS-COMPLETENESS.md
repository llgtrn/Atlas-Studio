---
id: atlas.contract.census-completeness
type: contract
status: active
canonical: true
---
# Census Completeness Contract

Atlas census is proof-producing accounting, not best-effort repository summarization.

For a pinned corpus, repository revision, admitted dependency-resolution contexts and Genome, every admitted artifact, every discovered executable scope and every active direct/transitive dependency edge MUST be accounted for. Atlas may report UNKNOWN or UNSUPPORTED, but it may not silently omit.

Repository boundaries do not bound census completeness. The normative transitive dependency rules are defined by `DEPENDENCY-CENSUS.md`.

## Closure pipeline

```text
root corpus inventory
  ↓
dependency resolution + transitive closure
  ↓
expanded corpus inventory / artifact classification
  ↓
syntax / build / dependency passes
  ↓
type / symbol / control / data / state / effect passes
  ↓
binding / temporal / evidence passes
  ↓
cross-scope reconciliation
  ↓
adversarial gap queries
  ↓
fixed point
  ↓
CensusCertificate
  ↓
SEALED *.atlas eligibility
```

## Dependency closure

For every admitted build/runtime context Atlas resolves the dependency graph to transitive closure. Every active direct/transitive edge is typed and every resolved dependency instance is either source-backed and admitted to inventory/census, or terminates at an explicit binary/toolchain/system/service/external boundary.

A manifest or lockfile is evidence for closure; it is not closure by itself. Build-time, generated, proc-macro, native/FFI, plugin and dynamic dependencies must be accounted or explicitly unresolved.

Atlas Studio self-census follows exactly the same rule. Atlas cannot claim CLOSED/SEALED self-census while its own active dependency graph contains silent external nodes.

## Inventory closure

Every tracked/admitted file, generated input, build definition, schema, configuration, migration, test fixture, binary descriptor or unsupported artifact receives an accounting record with identity, hash, class and disposition.

UNKNOWN is explicit. Silent skip is forbidden.

## Function and semantic accounting

Every discovered function/method is represented in Atlas. Adaptive census controls depth, not existence.

The canonical R4 function representation is defined by `SEMANTIC-FACTS.md`. A function is an identity plus typed semantic facets (signature, symbol/type, call, control/data flow, state/effect, ownership/concurrency/persistence and evidence). The bootstrap `SemanticFact { subject, predicate, object }` envelope is not the final function ontology.

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

## Generational recensus closure

A census fixed point is scoped to the Atlas observation capability that produced it.

When a promoted generation can observe, represent, resolve or verify a previously invisible class, affected prior closure MUST be recensused in a new evidence/certificate lineage. This includes Atlas itself, invalidated dependencies, affected donor/provider scopes, prior negative-evidence claims and affected ABSORBED/EXTINCTION_READY conclusions.

Old CLOSED/SEALED certificates remain historical evidence for their exact pinned inputs; they are not silently reinterpreted under a stronger generation.

Normative generation/convergence semantics: `RECURSIVE-SELF-CENSUS.md`.

## Scenario closure

Where policy requires runtime/failure-world evidence, semantic closure binds a finite admitted scenario frontier. Required architecture-falsification, dependency-disappearance, security, concurrency, persistence/recovery or failure scenarios may not be silently skipped because static graph closure succeeded.

Scenario growth MUST be bounded and linked to concrete obligations. "No random finding" is not verified absence.

## Seal formula

A logical Atlas may be sealed only when required closure classes satisfy policy:

```text
Inventory
∧ Dependency Closure
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

The normative contract is `CENSUS-CERTIFICATE.md` and the machine schema is `../schemas/census-certificate.schema.json`.

Every sealed root records at least corpus identity, revision set, Genome hash, inventory totals, dependency node/edge/context totals and closure state, accounted totals, semantic coverage, unresolved/unsupported artifacts, dynamic edges, binding gaps, conflicts, fixed-point iteration count, independent-pass agreement and Atlas root hash.

States:

```text
DRAFT → CENSUSED → RECONCILED → CLOSED → SEALED
```

Only policy-eligible SEALED Atlas roots may materialize production AtlasX.
