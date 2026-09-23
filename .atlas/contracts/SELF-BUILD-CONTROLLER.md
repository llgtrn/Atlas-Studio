---
id: atlas.contract.self-build-controller
type: contract
status: active
canonical: true
---
# Self-Build Controller Contract

## Purpose

The Self-Build Controller is the canonical planner for Atlas improving Atlas. It turns evidence-backed capability gaps into bounded engineering work without granting an external model authority over canonical state.

Recursive generations, stable-validator separation, scenario expansion, convergence and extinction probes are normative in `RECURSIVE-SELF-CENSUS.md`. The controller MUST plan inside that contract rather than treating self-build as a one-shot backlog.

The controller does not write production code, select its own authority, mutate main, issue CensusCertificates, or seal Atlas. It emits a typed SelfBuildWorkOrder that the normal research, synthesis, verification, selection and admission pipeline executes.

## Canonical loop

~~~text
stable G_n
→ self-census + dependency census + scenario frontier
→ closure / conflicts / verification evidence
→ capability-gap + extinction-gap graph
→ bounded SelfBuildWorkOrder
→ research / donor census / alternatives
→ untrusted CandidateChangeSet C_n+1
→ census candidate
→ validate with G_n where applicable
  + independent evidence for new capability
  + candidate self-census only as supplementary evidence
→ SelectedDesign
→ AdmissionTransaction
→ promote G_n+1
→ mandatory stronger recensus
→ expand bounded scenario frontier
→ recompute gaps + convergence
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

## Generation and convergence requirements

Every automated/hybrid cycle MUST:

- keep the admitted generation separate from the candidate;
- census generated implementation as untrusted;
- require independent evidence for material new claims;
- recensus scopes invalidated by stronger observation capability;
- include evidence-linked scenario regression/falsification;
- refuse one-pass fixed-point claims;
- refuse extinction until the isolated donor-disappearance probe succeeds.

Planning gap labels from `RECURSIVE-SELF-CENSUS.md` are not new EpistemicStatus values.

POLICY_AUTO may propose convergence only after the policy/Genome convergence window; unattended convergence requires at least two clean promoted generations.

## Failure and liveness

The controller must fail closed when prerequisites are missing. It must not manufacture work merely to remain busy, recursively weaken a gate that blocks it, or loop indefinitely on the same failed candidate without changing evidence or strategy.

A blocked work order remains durable with the exact blocker and may be superseded by a new work order after new evidence, blueprint revision or human steering.

## R4 through R8 role

R4 supplies semantic perception. R5 supplies incremental recomputation. R6 supplies closure and gap evidence. R7 makes the controller actionable through research, synthesis, validation, policy selection and admission. R8 makes the resulting knowledge and selected implementation durable enough for source-independent continuation.

## Final invariant

Atlas may decide what bounded engineering problem to attempt next, but no model or controller may skip the same semantic and admission gates imposed on any other generated candidate.
