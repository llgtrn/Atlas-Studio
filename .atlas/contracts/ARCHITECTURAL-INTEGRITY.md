---
id: atlas.contract.architectural-integrity
type: contract
status: active
canonical: true
---
# Architectural Integrity and Collapse Prevention Contract

## Purpose

Atlas MUST preserve more than compilability and local functional behavior. It MUST preserve the load-bearing semantic architecture that makes the selected system coherent under ownership, authority, state, temporal, failure, persistence, concurrency, security and interface constraints.

This contract defines the canonical architecture-integrity layer between observed semantics, selected design, construction admission, blueprint evolution, logical seal and AtlasX materialization.

An implementation may compile, pass unit tests and still be architecturally invalid.

RFC 2119 MUST/MUST NOT/SHOULD language is normative.

## Core definition

Architecture is not a folder layout, naming convention or diagram.

For Atlas, architecture is the typed set of semantic topology and invariants that constrain how admitted system parts may own state, depend on each other, exercise authority, communicate, fail, recover and evolve.

Architectural integrity exists when the exact observed candidate satisfies the exact selected ArchitecturalIntegrityEnvelope under the active Genome and profile.

Architectural collapse means any candidate state in which one or more required hard architectural invariants are violated, or required impact/equivalence evidence is incomplete such that Atlas cannot establish preservation.

A hard architectural violation is admission-blocking even when:

- the code compiles;
- local tests pass;
- benchmarks improve;
- a provider says the change is equivalent;
- the changed files appear small;
- the implementation preserves the public API.

## Three independent correctness levels

Atlas MUST keep these levels distinct:

~~~text
L1 Syntactic integrity
   parses / type-checks / compiles

L2 Local semantic integrity
   the changed component performs its declared behavior

L3 Architectural integrity
   the complete admitted system still obeys selected load-bearing topology
   and hard invariants
~~~

L1 and L2 do not imply L3.

Verification systems MUST NOT use a green local test result as a substitute for an architectural-integrity result.

## Architectural authority

Humans, AI providers, donor evidence and research may propose architecture.

None of them become canonical merely by proposing it.

Canonical architectural authority is the combination of:

- active Constitution and Genome requirements;
- selected contracts and blueprints;
- SelectedDesign identity;
- the pinned ArchitecturalIntegrityEnvelope;
- observed candidate semantics;
- evidence-backed BlueprintRevisionDecision lineage where architecture intentionally changes.

A provider may propose an invariant or a replacement. It may not self-certify compliance, equivalence or canonical admission.

## Load-bearing classification

Every architecture-relevant element that participates in a selected design SHOULD be classifiable as one of:

- LOAD_BEARING — removing, bypassing or semantically changing it can invalidate a hard architectural invariant;
- STRUCTURAL — materially shapes composition or behavior but may be replaced when its contract is preserved;
- REPLACEABLE — implementation mechanism may change without changing selected architecture when declared equivalence obligations pass;
- DECORATIVE — no selected load-bearing semantic obligation depends on it.

Classification is about architectural criticality, not census depth.

A REPLACEABLE or DECORATIVE element is never permission for silent omission from inventory, provenance, security or dependency accounting.

A LOAD_BEARING element may be replaced only when either:

1. Atlas proves the replacement preserves every affected hard invariant and required external behavior; or
2. a BlueprintRevisionDecision explicitly selects a new architecture and supersedes the affected invariant set.

## Required invariant classes

The ArchitecturalIntegrityEnvelope may contain any typed invariant supported by the active schema. The baseline canonical classes are:

- OWNERSHIP_BOUNDARY — who owns mutable state/resources and who may transfer ownership;
- DEPENDENCY_DIRECTION — permitted and forbidden dependency directions;
- AUTHORITY_BOUNDARY — which component may exercise which capability or privilege;
- STATE_SOURCE_OF_TRUTH — authoritative state owner and forbidden shadow ownership;
- INTERFACE_PROTOCOL — required interface/protocol boundary and allowed bypasses;
- LIFECYCLE_ORDERING — required initialization, transition and shutdown order;
- TEMPORAL_ORDERING — required happens-before / sequencing relationships;
- FAILURE_CONTAINMENT — where failures may propagate and where they must be contained;
- RECOVERY_INVARIANT — required recovery, replay, rollback or idempotency behavior;
- PERSISTENCE_BOUNDARY — durability ownership and persistence ordering;
- CONCURRENCY_BOUNDARY — synchronization, isolation and concurrency constraints;
- SECURITY_TRUST_BOUNDARY — trust zones, taint/sanitization and forbidden crossings;
- EXTERNAL_BOUNDARY — explicit system/service/toolchain boundary that may not become ambient authority;
- RESOURCE_SAFETY_BOUNDARY — hard resource constraints whose violation invalidates safe operation.

Additional classes require explicit schema/version evolution rather than free-form reinterpretation.

## Falsifiability requirement

No hard architectural decision is complete unless Atlas can state what observation would falsify it.

Every HARD invariant MUST therefore carry:

- stable invariant identity;
- invariant class;
- subject/owner references;
- normalized rule or evaluator reference;
- at least one falsification condition;
- evidence/provenance references;
- violation action;
- supersession lineage when replaced.

Examples:

~~~text
Invariant:
Runtime MUST NOT construct identity directly.

Falsifier:
Observed Runtime -> user-table write or Runtime -> identity-construction operation
without the selected IdentityKernel boundary.

Result:
HARD_VIOLATION -> candidate rejected.
~~~

~~~text
Invariant:
Producer availability MUST NOT depend synchronously on Consumer availability.

Falsifier:
Observed synchronous Producer -> Consumer request on the selected critical path.

Result:
HARD_VIOLATION unless an explicitly selected blueprint revision supersedes
the invariant.
~~~

If Atlas cannot evaluate a required hard invariant, the result is UNKNOWN/INCOMPLETE, not PASS.

## Machine contracts

The first machine-readable profile is:

- ../schemas/architectural-integrity-envelope.schema.json
- ../schemas/architectural-integrity-report.schema.json

The envelope declares selected architectural obligations.

The report records evaluation of one exact candidate/revision/materialization against one exact envelope.

Schema validity alone is not proof of correctness. Report fields that summarize counts/verdicts MUST be derived from observed typed evidence and checked for internal consistency by the eventual implementation.

## Observed architecture versus declared architecture

Atlas MUST distinguish:

~~~text
Declared / selected architecture
!=
Observed implementation architecture
~~~

Declared architecture comes from selected contracts/design/envelope.

Observed architecture is derived from census, semantic extraction, dependency closure, runtime/test evidence and admitted external-boundary facts.

The integrity verifier compares the two.

A declaration that says an edge is forbidden does not prove the edge is absent.

A provider-written architecture summary is not observed evidence.

## Candidate admission gate

Architecture validation is transaction-level, not merely operation-local.

For a CandidateChangeSet or ACP transaction Atlas MUST conceptually:

~~~text
pin base semantic state
→ apply proposal to an isolated candidate state
→ census / derive exact observed semantic delta
→ compute architectural impact closure
→ evaluate every affected required invariant
→ evaluate load-bearing replacement equivalence where applicable
→ emit ArchitecturalIntegrityReport
→ admit only if policy says the report is eligible
~~~

A HARD violation rejects the candidate.

A required UNKNOWN keeps the candidate non-seal-eligible unless the active Genome explicitly permits that unknown class.

Tests, benchmarks or provider confidence cannot override a HARD violation.

## Impact closure and continuous revalidation

Atlas MUST NOT blindly revalidate only the changed file.

Every admitted semantic change is mapped to affected semantic identities and then through architecture-relevant relationships, including where applicable:

- ownership edges;
- call/dependency edges;
- state ownership;
- capability/authority bindings;
- interface/protocol bindings;
- temporal/lifecycle relationships;
- failure/recovery domains;
- persistence/concurrency relationships;
- selected external boundaries.

The transitive set of potentially affected architecture invariants is the architectural impact closure.

R5-class incremental infrastructure MAY avoid a whole-world rerun only when it can prove that unaffected invariants remain unaffected.

Cache reuse MUST NOT bypass impact closure.

## Equivalence for replacements

A replacement of a LOAD_BEARING or STRUCTURAL element MUST NOT be accepted because the new implementation has similar code shape or the same public API.

Required equivalence is obligation-specific.

Depending on the affected envelope it may require proof/evidence for:

- same ownership boundary;
- same authority boundary;
- same externally observable protocol;
- same state-transition semantics;
- same failure containment;
- same recovery behavior;
- same persistence ordering;
- same concurrency guarantees;
- same permitted dependency directions.

Where equivalence cannot be established, Atlas must either reject the replacement or select an explicit blueprint revision.

## Blueprint evolution

Intentional architecture change is legal only through BLUEPRINT-EVOLUTION.md.

Before a replacement architecture is SELECTED, a candidate that contradicts the current hard envelope remains a violation.

A selected BlueprintRevisionDecision MUST identify the invariant identities it adds, removes, changes or supersedes, and MUST define recensus/revalidation impact.

No implementation agent may silently weaken an invariant to make a candidate pass.

## Census and conflict handling

Architecture integrity depends on observed facts and therefore inherits census epistemic discipline.

Independent extractors may disagree about an architecture-relevant fact.

Disagreement is CONFLICT, not permission to choose the convenient interpretation.

A HARD invariant whose required observation is conflicted remains unresolved until policy-approved reconciliation closes the conflict.

## Extinction interaction

Physical donor-source extinction MUST preserve architectural meaning.

A donor scope that contributes a LOAD_BEARING mechanism is not extinction-eligible until:

- the Atlas-native replacement exists;
- all affected hard invariants pass against the replacement;
- required equivalence evidence is durable;
- donor runtime/build/test source dependency is zero for the absorbed scope;
- source deletion occurs;
- post-delete recensus and architectural-integrity revalidation pass.

Deleting the donor source before architectural replacement proof is architectural data loss, not extinction.

## Seal gate

A selected candidate is architecturally seal-eligible only when the active policy can establish:

~~~text
pinned_architectural_integrity_envelope
AND architectural_impact_closure_closed
AND all_required_hard_invariants_evaluated
AND hard_violation_count == 0
AND required_unknown_count == 0
AND required_load_bearing_equivalence_closed
AND intentional_architecture_changes_have_selected_blueprint_revision
AND architectural_integrity_report_verdict == ELIGIBLE
~~~

Genome may permit bounded non-hard unknowns, but MUST NOT silently reinterpret them as PASS.

The logical Atlas seal MUST bind the envelope identity/hash and the exact eligible report/evidence roots used for admission.

## AtlasX materialization

AtlasX materialization MUST preserve the selected architecture.

Binding resolution, profile selection, partial materialization and deterministic expansion MUST NOT introduce an edge, authority path, state owner, failure path or lifecycle relation forbidden by the pinned envelope.

Materialization-critical invariants MUST be revalidated against the exact materialized closure.

If materialization would violate a hard invariant, materialization fails closed.

## Architectural drift diagnostics

The eventual implementation SHOULD distinguish at least:

- ARCHITECTURE_HARD_VIOLATION;
- ARCHITECTURE_REQUIRED_UNKNOWN;
- ARCHITECTURE_CONFLICT;
- ARCHITECTURE_IMPACT_CLOSURE_OPEN;
- ARCHITECTURE_EQUIVALENCE_UNPROVEN;
- ARCHITECTURE_UNSELECTED_REVISION;
- ARCHITECTURE_MATERIALIZATION_VIOLATION.

Diagnostics are durable evidence. They are never hidden only in logs.

## Anti-shortcuts

Atlas MUST NOT:

- equate compile success with architecture preservation;
- equate unit/integration test success with architecture preservation;
- trust a provider's self-reported equivalence;
- use folder/module names as the sole architecture model;
- delete or bypass a load-bearing boundary because a replacement is shorter/faster;
- weaken an invariant because a candidate otherwise looks attractive;
- materialize a profile that silently rewrites architecture;
- reuse stale integrity evidence after an affected semantic change;
- declare extinction before post-delete architectural revalidation.

## Implementation status

This document and its schemas are CONTRACT now.

The complete production implementation is TARGET and is sequenced across R4-R8:

- R4 supplies typed observed semantic dimensions needed to see architecture;
- R5 supplies dependency-aware incremental impact closure;
- R6 supplies reconciliation/closure evidence;
- R7 supplies candidate admission, construction verification and explicit architecture revision;
- R8 binds architectural-integrity roots into durable Atlas and revalidates deterministic AtlasX materialization.

The contract MUST NOT be described as production-real until those capabilities exist and are falsification-tested.

**G138 (ADR 0056)** implements the envelope record and the report evaluator (construction milestones M5/M6) over the declarations Atlas can falsify with its census. It is partial:

- **Implemented.** `core::integrity` derives the envelope, and HARD invariants currently cover two classes:
  - `DEPENDENCY_DIRECTION` from census-quantified layering and Cargo-observable `depends_on`;
  - `AUTHORITY_BOUNDARY` from `forbid effect` and effect envelopes.
- **Pinning.** The repository pins the envelope at `.atlas/declared/integrity-envelope.json`.
- **Report.** `atlas-systemizer integrity report` evaluates the pin against the decided census. A pinned invariant the candidate no longer declares is `ARCHITECTURE_UNSELECTED_REVISION`. Counts and verdict are re-derived by `check_report`.
- **Still TARGET.**
  - The other invariant classes.
  - Incremental impact closure in the report (it states `FULL_RECOMPUTE`).
  - Load-bearing replacement equivalence.
  - Seal binding.

## Final invariant

Atlas does not require one human or one model to hold the entire system in its head.

It requires every load-bearing architectural assumption to be explicit, falsifiable, evidence-linked and mechanically admission-blocking when violated.
