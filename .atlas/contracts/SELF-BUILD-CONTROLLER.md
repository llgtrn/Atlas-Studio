---
id: atlas.contract.self-build-controller
type: contract
status: active
canonical: true
---
# Self-Build Controller Contract

## Purpose

The Self-Build Controller is the canonical planner for Atlas improving Atlas. It turns evidence-backed capability gaps into bounded engineering work without granting an external model authority over canonical state.

The controller does not write production code, select its own authority, mutate main, issue CensusCertificates, or seal Atlas. It emits a typed SelfBuildWorkOrder that the normal research, synthesis, verification, selection and admission pipeline executes.

## Canonical loop

~~~text
current Atlas revision
→ self-census + dependency census
→ closure / conflicts / metrics / verification evidence
→ capability-gap graph
→ roadmap + Genome + policy comparison
→ SelfBuildWorkOrder
→ research / donor census
→ candidate mechanisms
→ synthesis CandidateChangeSet
→ census generated implementation
→ verify / benchmark / prove
→ SelectedDesign
→ AdmissionTransaction
→ admitted Atlas revision
→ recensus
→ recompute capability gaps
↺
~~~

## Inputs

The controller may consume only pinned, attributable inputs, including:

- current repository revision and logical Atlas root when available;
- canonical roadmap/blueprint coordinates;
- Genome and security policy;
- CapabilityGap records and CensusCertificate state;
- donor/dependency discoveries and Technology Genomes;
- verification failures and unresolved obligations;
- semantic metrics, benchmark observations and performance objectives;
- blueprint-revision decisions;
- explicit human intent or policy goals.

Model preference alone is not a capability gap.

## SelfBuildWorkOrder

The v1 machine envelope is ../schemas/self-build-work-order.schema.json.

A work order binds at least:

- exact parent repository revision;
- target capability/gap;
- why the work is needed and evidence references;
- allowed native scope and affected owners;
- constraint-envelope reference;
- selection-authority mode;
- permitted provider roles/tools;
- mandatory semantic/security/dependency/license/verification gates;
- recensus scope;
- completion and stop conditions;
- rollback/escalation expectations.

A work order is planning authority, not implementation truth.

## Prioritization

Prioritization must be policy-bounded and explainable. Valid drivers include prerequisite blocking, correctness risk, semantic closure, security, verification coverage, dependency independence, measured performance objectives, extinction readiness and explicitly selected roadmap priority.

A provider may propose a priority. The controller records the actual policy/evidence that authorized it.

## Autonomy modes

HUMAN_REQUIRED, POLICY_AUTO and HYBRID have the same meaning as SelectedDesign authority.

POLICY_AUTO is permitted only for a bounded work order whose mutation class, allowed scope, verification requirements and admission policy are already authorized. Identity/schema/wire/Genome/security-policy changes should default to stronger authority unless an explicit policy says otherwise.

## Failure and liveness

The controller must fail closed when prerequisites are missing. It must not manufacture work merely to remain busy, recursively weaken a gate that blocks it, or loop indefinitely on the same failed candidate without changing evidence or strategy.

A blocked work order remains durable with the exact blocker and may be superseded by a new work order after new evidence, blueprint revision or human steering.

## R4 through R8 role

R4 supplies semantic perception. R5 supplies incremental recomputation. R6 supplies closure and gap evidence. R7 makes the controller actionable through research, synthesis, validation, policy selection and admission. R8 makes the resulting knowledge and selected implementation durable enough for source-independent continuation.

## Final invariant

Atlas may decide what bounded engineering problem to attempt next, but no model or controller may skip the same semantic and admission gates imposed on any other generated candidate.
