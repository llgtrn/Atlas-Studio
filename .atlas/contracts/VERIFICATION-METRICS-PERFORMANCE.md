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

## One canonical meaning per term

Several names in this contract are common English words that already have a different, narrower, already-implemented meaning elsewhere in this repository. An implementer grepping the Rust codebase MUST NOT confuse them:

- **VerificationObligation** (this contract: a required property that must hold for a candidate) is distinct from `core::semantic::obligation::SemanticObligationRecord` and the adapter's `ObligationResult` (R4 census: whether an extractor observed evidence for a semantic dimension at all -- OBSERVED/UNSUPPORTED/UNKNOWN answers "did we look", never "does this requirement hold").
- **Observation** / **MetricObservation** (this contract: a measured result from a workload/environment/run) is distinct from `SemanticObservation` (the R4 census raw-observation carrier for typed semantic facts).
- **VerificationEvidence** (this contract: producer/run/environment/result-bearing proof for an obligation) is distinct from `core::evidence::Evidence` (an already-implemented, narrower R4 type: id/kind/path/summary/revision only -- no producer, environment, result or counterexample fields). A verification pass MAY reference `core::evidence::Evidence` records as one kind of supporting material, but `VerificationEvidence` itself is the richer, obligation-scoped type this contract defines.

## Logic semantics versus performance semantics

R4's semantic extraction dimensions (SYMBOL, TYPE, FUNCTION_IDENTITY, FUNCTION_SIGNATURE, CALL, CONTROL_FLOW, DATA_FLOW, STATE, EFFECT, OWNERSHIP, CONCURRENCY) answer what a component *does*. This contract's performance semantics answer a separate question: what resources, contention, latency, memory, IO, communication and scaling behavior that logic *implies*. Performance is never reduced to one benchmark number, and a component may be logically fully censused while its performance semantics remain entirely uncharacterized, or vice versa.

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

A MetricContract's semantic identity MUST survive a materialization change to the semantic operation it measures: a REST handler reimplemented as a QUIC RPC handler is a materialization change to the same semantic operation, not a new metric. MetricContract identity is tied to the semantic operation (a FunctionIdentity, component, or ADL operation) it measures, never to the instrumentation call site.

## Objective versus Observation

An objective is a requirement. An Observation is a measured result from a particular workload/environment/run. They must never collapse into one field.

Performance claims are scoped by workload and environment.

An Observation identifies at minimum: the MetricContract it measures, the exact artifact revision measured, the EnvironmentGraph it ran under, the Workload applied, the measurement window, the resulting value or distribution, the collector/producer identity, and provenance sufficient to reproduce or audit the run.

## Workload

`Workload` is the canonical identity this contract, `ATLAS-TO-ATLASX.md`, `COMPILER-IR-PIPELINE.md` and `COMPILER-PRODUCT.md` all reference for what a benchmark or prediction is scoped under (previously an unqualified `WorkloadProfile` bare identity with no defined content). A Workload characterizes, where applicable: arrival rate, concurrency, request mix, read/write ratio, payload size distribution, key distribution, hotspot distribution, session behavior, transaction length, dataset size, access locality, burstiness, client count, regional distribution, and failure/load-transition shape.

`HardwareProfile` and `DeploymentProfile` are Workload's sibling compiler-input identities for codegen/optimization decisions; their content is TARGET (not yet frozen by this contract). Performance-relevant environment parameters that attach per-node to a VerificationWorld/EnvironmentGraph's resources include: CPU architecture/core count/frequency, cache hierarchy, NUMA topology, RAM, memory bandwidth, storage class/latency, network bandwidth/RTT/NIC characteristics, kernel/runtime, hypervisor/container overhead, compiler/build mode, and cloud instance class.

## VerificationWorld

VerificationWorld / EnvironmentGraph describes required services, topology, resources, fixtures, capability boundaries and fault conditions. Docker Compose, containers, VMs, Firecracker, Kubernetes, local processes and CI systems are possible execution backends, not the semantic source of truth.

A benchmark number without its environment is not a fact Atlas may treat as a stable property. An admissible Observation's environment reference resolves at minimum: architecture, OS/platform, runtime, dependency versions, hardware/resource profile, topology, network conditions, fixture identity, materialization backend, configuration, and the exact artifact revision/content hash measured.

## Evidence and obligations

A test is one evidence producer. An obligation is what must hold. Evidence may come from static analysis, tests, property/fuzz campaigns, differential checks, proof/model checking, failure scenarios, benchmarks and runtime observation.

A VerificationObligation MAY be backed by several independent VerificationEvidence records (e.g. one obligation satisfied jointly by a static-analysis pass, a property test, an integration run and a fuzz campaign); the obligation's own state is a function of its evidence set, never a single evidence record's state alone. An obligation with zero admissible evidence is UNVERIFIED, a distinct state from FAILED (evidence exists and contradicts the obligation) or UNRESOLVED (evidence is inconclusive under the active policy).

Verification classes form an extensible, named vocabulary rather than free-form prose: SEMANTIC, STATIC, UNIT, PROPERTY, FUZZ, INTEGRATION, SCENARIO, COMPATIBILITY, FAILURE, SECURITY and BENCHMARK are the classes materializing today. SECURITY (penetration/exploit-shaped evidence) is distinct from STATIC (security scanning) -- a passing static security scan does not discharge a SECURITY-class obligation requiring active exploit attempts.

VerificationEvidence identifies at minimum: producer/tool identity and version, run identity, the exact artifact revision under test, an environment reference, a deterministic run identity/timestamp, inputs, a result (SATISFIED / VIOLATED / UNVERIFIED / UNRESOLVED), the obligation/objective references it supports, measurement data when it is itself an Observation, and a content hash of the evidence artifact. Evidence sources include test runs, benchmark runs, static analyses, formal/model checks, fuzz campaigns, simulations, authorized runtime observations, compatibility-matrix runs, and external attestations.

A **VerificationReport** is the aggregate result of running a VerificationPlan (the planned set of obligations/objectives a policy requires for a candidate): which VerificationObligations/Objectives were evaluated, their outcomes, and references to the VerificationEvidence backing each outcome.

A **VerificationFailure** is the typed record a failed obligation or objective hands back to repair/redesign, reusing the same evidence/decision-record pattern as `ExtractionDiagnostic`/`DecisionProposal` rather than only prose: it identifies the obligation or objective reference, the observed result (metric/value/environment when applicable), a counterexample where one exists, and the evidence references behind it. **PerformanceDiagnostic** is a performance-specialized VerificationFailure: metric, objective, predicted and/or observed value, dominant causes, sensitivity, and non-authoritative recommended search directions for repair.

A pass never means universal correctness beyond the declared scope and environment.

Construction MUST evaluate the exact obligations required by the active seal policy. A required obligation may not disappear merely because no convenient test exists.

A provider claim such as "this should be correct" is not verification evidence.

A benchmark number is not correctness evidence unless the obligation itself is a measured performance/resource property.

A formal proof artifact is evidence for the exact property/model/assumptions it covers; it does not prove unrelated implementation properties.

## Determinism and reproducibility

Three epistemic shapes MUST NOT be collapsed into one "passed" bit:

- **Deterministic checks** (semantic/static/type checks) are bit-for-bit reproducible given pinned inputs.
- **Statistical measurements** (benchmark/performance Observations) are inherently variable; they record a distribution, never a single-number guarantee.
- **Environment-dependent observations** legitimately depend on the recorded environment identity, and are not comparable across a different one without that context.

A Prediction is reproducible given its exact artifact semantic revision, cost-model version, Workload, EnvironmentGraph assumptions, Calibration input and parameters. Stochastic prediction techniques (Monte Carlo, Bayesian search) SHOULD retain their seed/config for that reproducibility; this contract does not promise bitwise-deterministic reproducibility for every numerical method.

## Verification coverage is multi-dimensional

Coverage MUST be tracked per obligation class (VerificationObligation coverage, platform/architecture coverage, failure-mode coverage, compatibility coverage, benchmark coverage, security coverage, dependency-closure coverage), never collapsed into one aggregate percentage. An artifact may be fully unit-tested and entirely unverified for partition tolerance; both facts MUST remain independently visible in inspection output, the same way `CENSUS-CERTIFICATE.md` refuses to manufacture completeness via a prose summary.

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

**PerformanceCharacteristic** is a typed, provenance/confidence-bearing claim about a logic's cost shape -- extracted or inferred, never measured -- distinct from both MetricContract (how to measure) and VerificationEvidence (PASS/FAIL support for an obligation). Its confidence class is drawn from an extensible enum: STATIC_ANALYSIS, AST_IR_ANALYSIS, CONTROL_FLOW_ANALYSIS, COMPILER_METADATA, SOURCE_ANNOTATION, BENCHMARK_EVIDENCE, RUNTIME_PROFILING, KNOWN_ALGORITHM_RECOGNITION, PROVIDER_INFERENCE, EXTERNAL_DOCUMENTATION. A PROVIDER_INFERENCE claim carries lower default trust weight than a STATIC_ANALYSIS or RUNTIME_PROFILING claim and never silently becomes canonical fact until corroborated.

A CostModel carries: subject, named/typed/domain-scoped parameters, a symbolic expression or named technique reference, assumptions, predicted-metric references, a validity domain, and provenance. Atlas remains extensible to whatever technique a subject requires: asymptotic/amortized analysis, queueing theory (Little's Law, M/M/1, M/M/k, finite-capacity variants), graph critical-path and DAG scheduling analysis, max-flow/min-cut, LP/MILP/nonlinear/convex optimization, probability/reliability/Markov/stochastic models, order-statistics tail-latency modeling, control/autoscaling/congestion models, information-theoretic/erasure-code tradeoffs, cache models, and multi-objective optimization.

A Prediction references a CostModel, a Workload and an EnvironmentGraph, and MUST expose the assumptions it depends on (e.g. a cache-hit ratio, network RTT bound, request-rate bound, worker count, or an absence-of-node-failure assumption) rather than hiding them. A Prediction whose assumptions are violated at admission time MUST be marked outside its validity domain, never silently still applied. A Prediction SHOULD prefer an honest range over a falsely precise point value.

A Prediction MAY carry a sensitivity breakdown attributing which parameters dominate its predicted outcome (e.g. network RTT: HIGH, lock-hold time: VERY HIGH, cache-hit ratio: MEDIUM) -- the same mechanism a PerformanceDiagnostic's dominant-causes/recommended-search-directions draws from.

Composing per-stage CostModels into an end-to-end Prediction (critical-path latency, parallel branches, fan-out, queueing, retry amplification, resource consumption, network traffic, memory footprint) MUST account for graph topology: naive summing of per-stage latencies is wrong under parallel execution, async pipelines, batching, speculation, retries, races, cancellation or quorum operations. The already-materialized CONTROL_FLOW/DATA_FLOW/CONCURRENCY semantic graph dimensions carry what a composition engine needs; no new graph representation is required for this.

When Census recognizes a known primitive (hash table, B-tree, LSM-tree, work-stealing queue, Raft quorum, erasure-code stripe, Bloom filter, actor mailbox, consistent-hash ring), Atlas SHOULD attach a reusable, donor-independent CostModel for that primitive rather than re-deriving it per donor -- the primitive's cost model outlives any specific donor implementation. (TARGET: no such reusable-primitive library exists yet.)

Calibration binds a model version to observation sets, error/parameter adjustments, environment family and validity domain. Calibration is scoped by BOTH the environment family AND the workload family it was derived under, and MUST NOT be applied outside that scope merely because it is convenient.

Three revision axes are independent and MUST NOT be conflated: the artifact's own semantic revision (the `*.atlas` content), the cost-model implementation/version that produced a Prediction, and the Calibration dataset/version applied. The same architecture, analyzed later by a better queueing model, produces a new Prediction under a new cost-model version without the artifact's own semantic identity changing.

A benchmark number Census extracts from an OSS donor becomes a MetricObservation/VerificationEvidence record carrying donor revision, benchmark definition, hardware, workload, environment and date/version/configuration -- it is never assumed to transfer directly to an Atlas-native reconstruction, though it may serve as a Calibration prior, scoped like any other Calibration.

## Construction optimization

Atlas may use analytic models, constraint solving, LP/MILP/SMT, numerical methods, Bayesian/evolutionary search or other replaceable solvers to prune candidate architecture space. Solver output is decision evidence, not observed truth.

An **OptimizationProblem** is a typed object with typed objectives (minimize/maximize a MetricContract) and typed constraints (a required Objective, e.g. availability, durability, security or correctness) -- explicitly Pareto/multi-objective, never collapsed into one scalar "performance score". A rejected or accepted candidate's OptimizationProblem result MAY be cited inside a `DecisionProposal.ranking` entry: e.g. a candidate predicted at p99=91ms against an objective of <40ms is rejected before materialization, while a candidate predicted at a 31-45ms range is worth benchmarking.

Final performance admission must use the empirical evidence required by policy.

## Failure semantics

FailureScenario is first-class and may describe node/process loss, storage failure/corruption, network partition/loss/latency, dependency outage, restart/failover, resource exhaustion or clock perturbation. Fault-injection primitives include process termination, machine loss, storage loss/corruption, network partition, packet loss, injected latency, dependency unavailability, service restart, coordinator failover, clock skew, and resource exhaustion.

A FailureScenario has a structured shape: preconditions, an injected fault (e.g. terminate a named node), and assertions the system must still satisfy (e.g. availability stays above objective, no committed data loss, recovery completes within a bound). A production-grade distributed component missing required FailureScenario coverage MAY be denied at seal time.

Steady-state and failure-state performance are separate regimes, each with its own objectives (e.g. a steady-state p99 bound versus a wider one-node-loss p99 bound plus a recovery-time bound). A CostModel, Prediction or VerificationObligation scoped to a FailureScenario MUST declare which regime it describes.

## Artifact identity, EvidenceBundle and mutable observations

Stable semantic definitions, objectives, MetricContracts and verification plans may be part of logical Atlas identity: VerificationObligation definitions, MetricContract definitions, performance Objectives, EnvironmentGraph definitions, FailureScenario definitions, CostModel/Workload/Calibration references and benchmark plans. VerificationEvidence, MetricObservation and VerificationReport instances are run-scoped and content-addressed, referenced from identity rather than embedded in it, alongside the Seal/attestation itself.

Different artifact kinds bind different obligation/metric shapes: a code component typically binds unit/property obligations; a database binds migration/consistency obligations; an image-processing component binds fidelity/quality MetricContracts; a distributed service binds latency/availability MetricContracts plus FailureScenarios; a model binds quality/latency/resource-usage Objectives. A database engine's cost model is dominated by complexity/IO/memory; a network service's by packet-processing/queueing; an image-processing pipeline's by CPU/GPU/memory.

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

A VerificationPolicy is composable and keyed by artifact kind: e.g. a production service requires SEMANTIC, UNIT, INTEGRATION, SECURITY and BENCHMARK classes; distributed storage requires everything a production service requires plus FailureScenario coverage for node loss, partition and recovery; a library package requires only SEMANTIC, UNIT and COMPATIBILITY.

For a performance obligation specifically, sealing means exactly: this exact artifact revision, under this benchmark plan, on this EnvironmentGraph, with this Workload, produced this MetricObservation, and satisfied this obligation -- never "this architecture will always have this performance everywhere."

## Security boundary

This contract's execution model is bound to `EXTERNAL-PROVIDER-TRUST.md` and `DONOR-WORKBENCH-ISOLATION.md`, not exempt from them. Generated tests, benchmarks, fault-injectors and solver code are `CANDIDATE_GENERATED_SOURCE` until admitted; running any of them requires an authorized sandbox policy. Sandboxed execution output becomes OBSERVED evidence only when the executed-artifact identity is pinned, the sandbox/environment identity is recorded, inputs are recorded, the execution ran under an authorized policy, and output integrity/provenance is retained. Census-imported donor CI scripts and tests are census-visible data per `DONOR-WORKBENCH-ISOLATION.md`, never implicitly trusted executable authority. A provider's summary of a hypothetical run is never runtime evidence.

## Census integration

Census extracts verification-relevant and performance-relevant semantics from donors as typed, provenance-bearing facts -- never blindly trusted, and never executed merely to observe them (see Security boundary above).

Verification-relevant extraction includes: tests, benchmark suites, CI topology, runtime assumptions, observability definitions, invariants, failure-handling code, deployment topology and compatibility expectations, becoming VerificationEvidence or hints toward VerificationObligations/MetricContracts.

Performance-relevant extraction includes: algorithmic/amortized complexity, allocation behavior, IO behavior, disk/network intensity and fan-out, batching/retry amplification, serialization cost, critical sections and lock contention, synchronization points, queue structure, thread/process parallelism, dependency latency and network hops, communication volume, cache/data locality, shard/partition/replication behavior, consistency cost, failure-recovery cost, startup cost, and hot-path identification -- each represented as a typed PerformanceCharacteristic (see Performance model above), plus separate extraction from donor benchmark suites, profiler configs, load generators, flamegraph data, perf regression tests, CI benchmark jobs and tuning scripts.

## Extinction integration

Extinction (`SELF-BUILDING-R4-R8.md`'s EXTINCT state) MUST NOT be granted merely because a build succeeded. Extinction policy MAY require VerificationObligation/VerificationEvidence coverage for the extinguished scope, following: donor source → census → semantic extraction → Atlas-native reconstruction → verification (obligations derived from, or exceeding, donor evidence) → extinction certificate → source deletion permitted.

Extinction must also preserve captured performance knowledge -- PerformanceCharacteristics, benchmark VerificationEvidence, known constraints and valid Calibration data -- as durable typed knowledge before donor source deletion. Not all microarchitectural donor behavior is preservable; only what was actually captured as durable typed knowledge survives extinction.

## Implementation status

This contract distinguishes what exists in production code today from what it locks as a schema-frozen target for later waves:

- **CURRENT**: `CandidateChangeSet` (`../schemas/candidate-change-set.schema.json`) already has free-text `tests` and `benchmarks_or_proofs` arrays; `SelectedDesign` already has an untyped "required tests/proofs/benchmarks" field (`SELECTED-DESIGN.md`). The only implemented CLI verbs are `contract`, `systemize`, `docs audit`, `code analyze`, `parse`, `check`, `graph` and `work prepare` (`apps/cli/src/main.rs`) -- `atlas verify`/`atlas bench`/`atlas prove`/`atlas inspect`/`atlas seal` do not exist yet.
- **CONTRACT**: every named type/section in this document (VerificationObligation, VerificationEvidence, VerificationReport, VerificationFailure, MetricContract, Workload, EnvironmentGraph, FailureScenario, CostModel, PerformanceCharacteristic, Prediction, Calibration, OptimizationProblem, PerformanceDiagnostic) is a locked semantic definition other work MUST target, whether or not a matching Rust type or schema file exists yet.
- **TARGET**: `HardwareProfile`/`DeploymentProfile` content, the reusable-primitive CostModel library, and the CLI verbs above are explicitly not implemented; no schema is frozen for them beyond what this contract already states.

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
