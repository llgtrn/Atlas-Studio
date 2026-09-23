---
id: atlas.contract.verification-metrics-performance
type: contract
status: active
canonical: true
---
# Verification, Metrics and Performance Semantics Contract

## Purpose

Atlas treats verification, measurement, proof obligations and performance objectives as first-class construction semantics.

They are NOT post-build utilities attached after a canonical `*.atlas` already exists.

A final canonical `*.atlas` may be published only after the exact selected candidate has satisfied the seal policy's required verification obligations and the resulting evidence has been bound into the logical seal.

## Canonical construction model

~~~text
semantic implementation candidate
+ obligations/invariants
+ MetricContract
+ Workload
+ VerificationWorld / EnvironmentGraph
+ VerificationPlan
+ FailureScenario
+ performance objectives
+ CostModel
        ↓
CandidateAtlas
        ↓
analytic prediction / constraint solving / pruning
        ↓
materialize selected verification worlds
        ↓
VERIFY
  semantic / type / static / security / dependency / license
  unit / property / fuzz / differential / integration / system
  failure / recovery / compatibility
        ↓
BENCH + METRICS
  empirical Observation under pinned Workload + Environment
        ↓
PROVE / OBLIGATION EVALUATION
  bind evidence to each required obligation
        ↓
failed / unknown required obligation?
   yes → structured failure → repair / redesign / regenerate ↺
   no
        ↓
SealEligibleAtlas
        ↓
authorized selection + final exact-candidate gate
        ↓
SEALED logical Atlas
        ↓
deterministic compaction
        ↓
canonical *.atlas
~~~

Mathematical models reduce search cost. Evidence decides admission.

There is no valid canonical flow of:

~~~text
write final *.atlas
→ later verify whether it should have existed
~~~

## Construction-state vocabulary

The following states are semantically distinct:

~~~text
CandidateAtlas
SealEligibleAtlas
SEALED logical Atlas
physical canonical *.atlas
~~~

**CandidateAtlas** is a logical candidate semantic world under construction. It may be incomplete, failing, experimentally materialized, benchmarked, repaired or rejected. It is not a canonical published `*.atlas`.

**SealEligibleAtlas** is a candidate for which every obligation required by the active seal policy has an acceptable evidence state for the exact selected candidate/revision/profile. Eligibility is still not publication.

**SEALED logical Atlas** is the immutable logical engineering meaning authorized for publication. The seal binds the selected design, relevant constraints, exact obligation set, evidence/attestation roots, policy identity and all permitted unresolved states.

The physical canonical `*.atlas` is only the deterministic compacted representation of that SEALED logical Atlas.

Debug/migration tooling MAY serialize unsealed candidate state, but such bytes MUST be explicitly noncanonical/unsealed and MUST NOT be advertised as a canonical `*.atlas`.

## MetricContract

A metric has stable semantic identity independent of one telemetry backend. It identifies the semantic subject, measurement boundaries, unit, aggregation/dimensions and objective when applicable.

Prometheus/OpenTelemetry names are materializations, not canonical metric identity.

## Objective versus Observation

An objective is a requirement. An Observation is a measured result from a particular workload/environment/run. They must never collapse into one field.

Performance claims are scoped by workload and environment.

## VerificationWorld

VerificationWorld / EnvironmentGraph describes required services, topology, resources, fixtures, capability boundaries and fault conditions. Docker Compose, containers, VMs, Firecracker, Kubernetes, local processes and CI systems are possible execution backends, not the semantic source of truth.

## Evidence and obligations

A test is one evidence producer. An obligation is what must hold. Evidence may come from static analysis, tests, property/fuzz campaigns, differential checks, proof/model checking, failure scenarios, benchmarks and runtime observation.

A pass never means universal correctness beyond the declared scope and environment.

Construction MUST evaluate the exact obligations required by the active seal policy. A required obligation may not disappear merely because no convenient test exists.

A provider claim such as "this should be correct" is not verification evidence.

A benchmark number is not correctness evidence unless the obligation itself is a measured performance/resource property.

A formal proof artifact is evidence for the exact property/model/assumptions it covers; it does not prove unrelated implementation properties.

## VERIFY / BENCH / PROVE are construction operations

Atlas defines three logical operation classes independent of any particular CLI spelling.

### VERIFY

VERIFY orchestrates the evidence-producing checks required for a candidate, which may include:

- semantic/graph/obligation consistency;
- compiler/type/static analysis;
- security, capability and policy analysis;
- dependency/license/provenance checks;
- unit/property/fuzz/differential tests;
- integration/system scenarios;
- VerificationWorld materialization;
- compatibility checks;
- failure/recovery scenarios;
- Atlas recensus and semantic-delta checks.

VERIFY emits structured evidence and diagnostics. It does not select a design and does not seal an artifact.

### BENCH

BENCH executes measurement plans under pinned Workload + Environment assumptions and emits empirical Observations.

BENCH MAY measure latency, throughput, CPU, memory, IO, bandwidth, startup, recovery, energy, monetary cost or other typed metrics.

A predicted value from CostModel is never relabeled as a benchmark observation.

### PROVE

PROVE means **obligation evaluation**, not a blanket claim that all software correctness has been mathematically proven.

PROVE takes:

~~~text
required obligation
+ exact candidate/revision
+ assumptions/profile
+ admissible evidence set
→ satisfied / violated / unresolved according to the obligation's policy
~~~

Evidence may include formal proof/model checking where available, but may also be a policy-defined combination of static analysis, tests, failure campaigns, differential evidence and measurements.

The term "prove" MUST always remain scoped to the named obligation and its assumptions.

### CLI projection

A future or current CLI MAY expose surfaces such as:

~~~text
atlas build
atlas verify
atlas bench
atlas prove
atlas inspect
atlas seal
~~~

These command spellings are UX/API projections, not the canonical semantic model.

Conceptually, a high-level `atlas build <input>` may orchestrate construction internally as:

~~~text
construct
→ predict / optimize
→ verify
→ bench where required
→ prove/evaluate obligations
→ repair loop
→ select
→ seal
→ compact
→ publish *.atlas
~~~

Calling `atlas verify`, `atlas bench` or `atlas prove` manually is useful for development/debugging, but creation of a final `*.atlas` invokes the same logical engines as part of construction whenever required by policy.

## Performance model

CostModel is distinct from empirical evidence. Atlas may represent complexity, queueing, critical paths, contention, memory/IO/network cost, reliability and other mathematical models.

PredictedPerformance is distinct from TheoreticalCost and EmpiricalObservation.

Calibration binds a model version to observation sets, error/parameter adjustments, environment family and validity domain.

## Construction optimization

Atlas may use analytic models, constraint solving, LP/MILP/SMT, numerical methods, Bayesian/evolutionary search or other replaceable solvers to prune candidate architecture space. Solver output is decision evidence, not observed truth.

Final performance admission must use the empirical evidence required by policy.

## Failure semantics

FailureScenario is first-class and may describe node/process loss, storage failure/corruption, network partition/loss/latency, dependency outage, restart/failover, resource exhaustion or clock perturbation.

Steady-state and failure-state performance are separate regimes.

## Artifact identity, EvidenceBundle and mutable observations

Stable semantic definitions, objectives, MetricContracts and verification plans may be part of logical Atlas identity.

Run-specific observations MUST remain attributable, content-addressed evidence rather than silently rewriting semantic definitions.

A seal SHOULD bind an evidence/attestation root sufficient to establish the exact evidence set used for admission. Large raw traces or benchmark samples may remain in content-addressed EvidenceBundles or authenticated external evidence storage according to policy; the sealed Atlas retains the identities/hashes required to verify the admission claim.

New observations produced after sealing do not retroactively mutate the old seal. They may:

- create additional attestations against the same immutable artifact;
- trigger a new candidate/revision;
- invalidate a deployment policy externally;
- become input to a later calibration/model revision.

## Seal gate

A candidate MUST NOT become SEALED merely because one test suite passed.

The seal gate evaluates the exact required obligation set for the selected artifact/profile.

Depending on policy, required classes may include:

- semantic consistency;
- security/authority;
- dependency/provenance/license;
- correctness and compatibility;
- failure/recovery;
- performance/resource objectives;
- reproducibility;
- CensusCertificate/closure;
- selected-design and admission lineage.

Required VIOLATED, CONFLICT, UNKNOWN or UNSUPPORTED obligations block the seal unless the active policy explicitly permits that exact unresolved state for that exact non-critical obligation.

No synthesis, research, decision or verification provider may weaken the seal policy to make its own candidate pass.

## CI and execution backends

Traditional CI is one executor/materializer of Atlas verification semantics.

Docker Compose, GitHub Actions, Kubernetes, VMs, Firecracker, local processes and remote workers may execute a VerificationPlan. None is the source of truth for what must be verified.

## Final invariant

Atlas uses mathematics to reduce search cost, measurement to characterize reality, and scoped evidence to decide whether a candidate is eligible to become a sealed canonical `*.atlas`.

~~~text
candidate
→ verify / bench / prove obligations
→ evidence
→ repair until policy passes
→ seal
→ compact
→ *.atlas
~~~

Never the reverse.
