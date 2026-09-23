---
id: atlas.contract.recursive-self-census
type: contract
status: active
canonical: true
---
# Recursive Self-Census and Convergence Contract

## Purpose

Atlas self-building is a generational closed loop, not a one-shot donor import.

Each admitted Atlas generation MUST use the strongest already-admitted Atlas capability plus independent evidence to census the candidate next generation, discover newly visible gaps, exercise bounded new scenarios, absorb or invent mechanisms, and then recensus the engineering world with the promoted generation.

This contract prevents two symmetric failures:

- **stagnation** — Atlas keeps using an old census horizon and never notices deeper gaps;
- **circular self-certification** — a candidate declares itself correct because its own new analyzer says so.

It is normative for automated self-building, recursive recensus, convergence claims and donor extinction.

## Related authority

This contract composes with:

- `SELF-BUILD-CONTROLLER.md` — bounded planning authority;
- `CENSUS-COMPLETENESS.md` — census closure;
- `DEPENDENCY-CENSUS.md` — transitive dependency closure;
- `ARCHITECTURAL-INTEGRITY.md` — collapse prevention;
- `BLUEPRINT-EVOLUTION.md` — evidence-authorized redesign;
- `../roadmap/SELF-BUILDING-R4-R8.md` — R4→R8 sequencing;
- `../roadmap/DONOR-ABSORPTION-ROADMAP.md` — donor state machine;
- `../blueprints/BULK-DONOR-ABSORPTION.md` — physical source extinction.

## Generation model

For one transition:

- **G_n** — exact admitted Atlas revision; current stable generation.
- **C_n+1** — untrusted candidate successor.
- **stable validator** — G_n plus independent evidence channels that do not depend solely on C_n+1.
- **S_n** — finite admitted scenario frontier for generation n.
- **Δ_n** — typed generation delta: new/resolved obligations, conflicts, scenarios, dependency changes, donor dependence and verified capabilities.
- **K** — policy/Genome convergence window. Unattended POLICY_AUTO convergence requires K >= 2.

Human-readable generation numbers are labels. Exact revision, Genome identity, evidence roots and admission lineage are authority.

## Canonical loop

~~~text
G_n admitted
  ↓
self-census G_n
+ complete admitted dependency census
+ donor/dependency corpus census
+ scenario frontier S_n
  ↓
closure / UNKNOWN / CONFLICT / blockers
  ↓
capability-gap graph + extinction-gap graph
  ↓
bounded SelfBuildWorkOrder
  ↓
research / donor discovery / alternatives
  ↓
C_n+1 design + untrusted implementation
  ↓
census C_n+1
  ↓
validate with G_n where applicable
+ independent tests/proofs/differential evidence
+ C_n+1 self-census only as supplementary evidence
  ↓
SelectedDesign + AdmissionTransaction
  ↓
promote exact candidate → G_n+1
  ↓
G_n+1 becomes stable
  ↓
MANDATORY stronger recensus
  ↓
revisit affected prior closure / absorption / extinction claims
  ↓
expand bounded S_n → S_n+1 from concrete new gaps
  ↓
Δ_n + convergence test
  ↓
repeat until scoped fixed point
~~~

Promotion does not finish self-building. It changes the instrument used to observe the next generation.

## Stable-validator law

C_n+1 MUST NOT be the sole authority proving C_n+1.

For every material capability introduced by a candidate:

1. run every applicable gate the stable generation can already evaluate;
2. preserve exact evidence for claims outside the stable generation's observation horizon;
3. require at least one evidence channel independent of the candidate implementation for the new material claim;
4. treat candidate self-census as evidence, never sufficient authority by itself;
5. route architecture-changing claims through BlueprintRevisionDecision instead of allowing the candidate to weaken the invariant blocking it.

Independent evidence may include independently implemented extractors, pinned compiler/build metadata, differential execution, deterministic replay, golden/reference corpora, model checking/proofs, binary/runtime observations, or explicitly authorized human/policy evidence where no stronger machine oracle exists.

The old validator need not understand every new capability. The invariant is **no circular self-certification**.

## Planning gap classes

The controller MAY use these planning labels:

- OBSERVATION_GAP;
- REPRESENTATION_GAP;
- RESOLUTION_GAP;
- RECONCILIATION_GAP;
- VERIFICATION_GAP;
- DEPENDENCY_GAP;
- SCENARIO_GAP;
- EXTINCTION_GAP;
- ARCHITECTURE_GAP.

These are not EpistemicStatus values. Canonical semantic evidence continues to use the vocabulary in `SEMANTIC-FACTS.md`.

## Scenario frontier

Static source inspection is not sufficient for all required semantics.

S_n is assembled from concrete obligations such as:

- baseline Genome workloads;
- unresolved semantic obligations;
- newly discovered build/runtime/dependency boundaries;
- observed regressions and failures;
- architecture falsification conditions;
- malformed/partial/external inputs;
- security/adversarial conditions;
- resource pressure/cancellation;
- concurrency/scheduling variation;
- persistence crash/recovery/rollback worlds;
- target/platform/profile variations admitted by policy;
- donor-disappearance and dependency-disappearance probes;
- explicitly retained differential/reference cases.

Every admitted scenario binds pinned inputs/environment, expected invariant/oracle, evidence and disposition.

Scenario generation MUST be bounded and evidence-linked. Atlas may not create infinite random work to avoid convergence. Fuzzing may contribute evidence but cannot by itself prove closure.

## Promotion gate

C_n+1 may become G_n+1 only when the exact candidate:

- came from an authorized bounded work order/change set;
- was censused as untrusted implementation;
- preserves all applicable HARD architecture invariants or carries an explicit selected blueprint revision;
- passes security, dependency, provenance/license and generated-code admission gates;
- passes applicable regression/falsification scenarios from S_n;
- has independent evidence for new material capability claims;
- does not hide policy-forbidden UNKNOWN/CONFLICT/blockers;
- records exact selection and admission lineage.

Provider confidence and self-reported success are not promotion authority.

## Mandatory post-promotion recensus

After G_n+1 is admitted, Atlas MUST recensus every scope whose previous conclusion could change because observation, representation, resolution or verification became stronger.

At minimum consider:

- Atlas itself;
- invalidated dependency scopes;
- donor/provider scopes previously inspected in the changed semantic dimension;
- affected Technology Genomes and absorption decisions;
- negative-evidence claims in the changed dimension;
- affected architecture falsification scenarios;
- scopes previously considered ABSORBED or EXTINCTION_READY.

A stronger generation finding a new obligation is progress. It MUST reopen work instead of hiding the obligation to preserve an old closure claim.

Old CensusCertificates remain historical evidence for their exact pinned inputs and observer generation. They are never silently reinterpreted.

## Monotonic evidence, revisable conclusions

Evidence/provenance SHOULD accumulate monotonically.

Conclusions may be superseded by stronger evidence.

If a later generation exposes a hidden dependency or semantic obligation behind an ABSORBED/EXTINCT conclusion, create corrective evidence and a new state lineage under the existing donor state machine. Never rewrite historical proof and never invent a new donor state.

## Extinction probe

Before physical donor-source extinction, run an isolated donor-disappearance probe:

~~~text
eligible donor scope D
  ↓
clean probe environment
  ↓
make D source unavailable
+ remove Atlas-controlled D archives/vendor copies/caches
+ forbid silent network re-fetch of D
  ↓
build / test / verify affected Atlas
  ↓
run relevant S_n scenarios
  ↓
self-census + dependency census
  ↓
prove no runtime/build/test/source read reaches D
  ↓
PASS → deletion may proceed
FAIL → create EXTINCTION_GAP; do not delete
~~~

A hidden build tool, generated asset, helper binary, service, reference database, shell-out, environment lookup or cached source is dependence if the claimed native capability requires it.

When R8 durable/source-independent materialization exists, the probe MUST also exercise the applicable rebuild/continuation path.

## Post-delete proof

After deletion:

1. prove donor source and substitute local copies are absent;
2. rerun dependency closure;
3. rerun build/test/proof/material benchmark gates;
4. rerun affected scenarios;
5. recensus with the current stable Atlas;
6. verify required capability/semantic roots;
7. only then record EXTINCT.

## Corpus-level extinction

Atlas SHOULD drive retained source for the self-building native-absorption donor corpus toward zero.

A claim of **total donor-source extinction** is valid only when:

- every required selected native-absorption scope is EXTINCT;
- runtime/build/test dependence on donor source is zero;
- no Atlas-controlled vendor/cache/archive substitute remains;
- required behavior is reproducible from Atlas-native implementation plus durable knowledge;
- no locally retained REFERENCE_ONLY / EXTERNAL_BOUNDARY / other explicit exception remains.

Atlas MUST NOT delete a donor merely to make the counter reach zero. Zero is an outcome of proven independence.

## Convergence

One census internal fixed point is not a self-build fixed point.

A generation is convergence-clean only when, for the admitted self-build scope:

- policy-blocking capability gaps = 0;
- policy-blocking census UNKNOWN/UNSUPPORTED/CONFLICT = 0;
- dependency closure is stable;
- S_n produces no new mandatory capability class;
- no new material discovery requires ABSORB_NOW;
- no selected native-absorption scope remains dependent on retained donor source;
- architecture falsification suite passes;
- the current generation proposes no required correctness/closure change to itself.

Semantic self-build convergence requires K consecutive convergence-clean promoted generations with no reopened hard blocker or new mandatory capability class.

For unattended POLICY_AUTO, K >= 2.

Convergence does not forbid later optimization, optional features, new hardware targets, new external requirements or newly admitted corpora.

## False convergence

None of these proves convergence:

- one clean generation;
- all current unit tests passing;
- no authored roadmap items remaining;
- candidate self-census says CLOSED;
- donor source count reaches zero by deletion;
- no new random fuzz finding;
- research found nothing new;
- benchmark improvement plateau;
- an analyzer reports "no findings" for a class it cannot observe.

## Failure and liveness

The loop MUST be progressive and bounded.

Repeated equivalent candidate failure without new evidence requires strategy change, scope reduction, authorized blueprint revision, automation downgrade or human escalation. Atlas MUST NOT endlessly regenerate equivalent candidates merely to remain active.

A generation may stop blocked without pretending to be converged.

## R4 through R8

- **R4** increases semantic perception; each material new dimension triggers targeted recensus.
- **R5** makes recursive/incremental recensus and fixed-point maintenance efficient.
- **R6** strengthens independent verification and falsification.
- **R7** operationalizes work-order → research → synthesis → selection → admission generations.
- **R8** makes durable Atlas knowledge strong enough for clean-room/source-independent extinction and continuation tests.

The loop begins before R8; later stages increase autonomy and proof strength.

## Hard invariants

1. No one-shot self-build.
2. No circular self-certification.
3. No silent semantic horizon.
4. No stale closure reuse after observation capability changes.
5. No infinite scenario theater.
6. No extinction by rename/cache/archive.
7. No zero-donor vanity metric.
8. No candidate architecture self-waiver.
9. No generation without exact revision/Genome/evidence lineage.
10. No false finality: fixed point is scoped evidence, not omniscience.
