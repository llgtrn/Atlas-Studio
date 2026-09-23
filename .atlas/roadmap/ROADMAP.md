---
id: atlas.roadmap
type: blueprint
status: active
canonical: true
---
# Atlas Studio Roadmap

## Phase 0 — strict census + logical ATLAS

- Genome loader/validator/hash in Rust;
- universal graph/binding/state/event/temporal/evidence primitives;
- exhaustive root inventory and transitive dependency closure;
- Atlas self-census across its own full admitted dependency contexts, not only Atlas-owned source;
- first-class dependency census usable for arbitrary admitted projects/donors;
- every discovered function represented;
- CFG/call/data/state/effect extraction and explicit dynamic/unknown records;
- reconciliation, adversarial gaps, fixed point and CensusCertificate;
- typed binary `*.atlas`, semantic dedup/compression, logical root manifests and content-addressed shards;
- federated cross-repo references without competing truth.

## Canonical R4→R8 self-building execution

The detailed normative execution map is `SELF-BUILDING-R4-R8.md`.

The central rule is:

~~~text
current Atlas
→ census approved OSS donors
→ resolve/census their admitted direct + transitive dependencies
→ discover and attribute mechanisms
→ explicitly decide ABSORB_NOW / ABSORB_LATER / REFERENCE_ONLY / EXTERNAL_BOUNDARY / REJECT
→ deep-census selected provider scope
→ Technology Genome / evidence
→ Atlas-native implementation
→ verify
→ recensus Atlas and affected donors/dependencies
→ ABSORBED
→ physical source deletion when the durable-knowledge gate is satisfied
→ EXTINCT
→ stronger Atlas
↺
~~~

Atlas MUST NOT wait until R4, R5 or R6 are complete before starting donor census. Census and Atlas construction recursively strengthen each other.

Do not confuse the two sequencing axes:

- `R4..R8` describe maturity of Atlas-native capabilities;
- `W0..W8` in `DONOR-ABSORPTION-ROADMAP.md` describe donor technology lanes.

### Cross-cutting DC1 — Dependency Census Runtime

DC1 is required before W0 can claim full `COARSE_CENSUSED` status. It materializes dependency-resolution contexts, direct/transitive dependency closure, source-backed dependency admission, explicit terminal boundaries and dependency fixed point under `../contracts/DEPENDENCY-CENSUS.md`.

DC1 determines census breadth. R4 semantic dimensions determine census depth.

### R4 — semantic census depth

Current materialized baseline is R4.5.

- **R4.3.x — real Rust semantic bootstrap:** SYMBOL, TYPE, FUNCTION_IDENTITY and FUNCTION_SIGNATURE are real typed observations; extraction is wired through canonical Census; typed records survive normalization; raw observation identity, typed obligation lineage, typed closure and typed graph projection are materialized.
- **R4.4 — Function Identity Closure — materialized:** typed declaration kind, impl/trait owner context and function generics strengthen module/impl/trait/revision-safe declaration identity.
- **R4.5 — Call Semantics — materialized:** real Rust function/method-body call sites flow through canonical Census/Normalization; caller identity is observed while unresolved callees remain explicitly unresolved rather than fabricated.
- **R4.6 — Control Flow:** deterministic basic blocks, branch/loop/return/failure flow.
- **R4.7 — Data Flow:** values, definitions/uses, parameter/result flow, loads/stores and explicit ambiguity.
- **R4.8 — State + Effect:** bootstrap materialized, closure open; partial STATE/EFFECT facts remain useful but dimension obligations stay UNKNOWN until unmodeled state/effect forms and resolution gaps are closed. Compound assignment is read+write; textual panic-like macros are inferred until resolved.
- **R4.9 — Ownership / Resource Semantics:** borrow/move/copy and resource acquisition/transfer/release where evidenced.
- **R4.10 — Concurrency:** tasks/threads/channels/locks/atomics/synchronization and concurrent state interaction.
- **R4.11 — Persistence / Recovery:** durable writes, transactions, logs/checkpoints/recovery and failure ordering where applicable.
- **R4.12 — R4 Semantic Closure:** complete the declared Rust reference profile, deterministic normalization/exact-dedup policy, dynamic/unresolved closure, conflict-input preservation and reference-corpus acceptance.

R4 does not end census. Each R4 improvement MUST trigger recensus of affected donor/dependency scopes.

### R5 — incremental query and fixed-point closure

Implement dependency-aware revision invalidation, incremental recensus, recursive/fixed-point derivation and evidence-preserving query dependencies.

Primary donor lane: W3.

### R6 — reconciliation and CensusCertificate

Implement independent-observer reconciliation, explicit conflict preservation, cross-scope reconciliation, adversarial gap queries, fixed-point closure, dependency-closure proof and CensusCertificate issuance.

Primary support donor lane: W4 where applicable.

### R7 — research, decision, synthesis and absorption selection

Keep ResearchClaim distinct from ObservedEvidence while adding the mature Human+AI engineering loop:

- Human/AI typed intent and constraint envelopes;
- research providers over Atlas knowledge, admitted OSS and external references;
- multiple candidate mechanisms;
- typed Jev-class decision proposals for rank/score/route decisions;
- SelfBuildController + typed SelfBuildWorkOrder derived from capability gaps;
- external synthesis/code providers;
- CandidateChangeSet;
- generated-code census as untrusted source;
- security/dependency/license/test/benchmark/proof admission;
- explicit Human/Policy/Hybrid SelectedDesign authority;
- AdmissionTransaction for exact-parent, atomic, rollbackable canonical self-modification;
- post-apply recensus and semantic-delta verification;
- semantic metrics / VerificationWorld / failure scenarios / CostModel feedback under the verification-performance contract;
- ProviderReceipt lineage;
- evidence-linked absorption/blueprint revision.

Normative contracts:

- `../contracts/HUMAN-AI-ADL-AUTHORING.md`;
- `../contracts/ATLAS-CREATION-PIPELINE.md`;
- `../contracts/EXTERNAL-PROVIDER-TRUST.md`.

Primary donor lane: W5 plus admitted external provider adapters.

### R8 — real ATLAS and AtlasX

Implement the durable binary Atlas substrate after the selected implementation has already been synthesized/censused/validated: logical seal, provider-independent lossless semantic compaction, content addressing, integrity, transactional publication, sharding, deterministic SelectedDesign→AtlasX materialization, validated AtlasX object closure and lineage needed for census-derived knowledge to survive physical donor-source deletion at scale.

Normative R8 contracts include:

- `../contracts/ATLAS-SEMANTIC-COMPACTION.md`;
- `../contracts/ATLAS-BINARY-WIRE-FORMAT.md`;
- `../contracts/ATLAS-SHARDING.md`;
- `../contracts/SELECTED-DESIGN.md`;
- `../contracts/ATLAS-TO-ATLASX.md`;
- `../contracts/ATLASX-FORMAT.md`;
- `../contracts/ATLASX-BINARY-WIRE-FORMAT.md`.

Compiler handoff is governed by `../contracts/COMPILER-IR-PIPELINE.md` and `../contracts/COMPILER-IR-SCHEMAS.md`.

Primary donor lane: W6.

R8 does not end census. It makes durable large-scale source-independent continuation possible.

## Evidence-driven blueprint evolution

All roadmaps and blueprints are canonical for the current evidence state, not permanently frozen.

If donor/dependency census discovers a materially better mechanism for census, query, reconciliation, ATLAS compaction/storage, AtlasX materialization, compiler IR, verification or another Atlas-native capability, Atlas MAY revise the affected blueprint through `../contracts/BLUEPRINT-EVOLUTION.md`.

A selected blueprint revision may resequence future work, insert a prerequisite, split/merge a wave or replace an implementation strategy.

It MUST remain evidence-backed, explicit, migration-aware when identity/schema changes, and followed by targeted recensus.

Implementation agents MUST NOT silently redesign around the roadmap, and MUST NOT ignore stronger evidence merely to preserve obsolete sequencing.

## Human + AI engineering creation lane

ADL and Studio are not intended to become conventional human-only editors with a chatbot attached.

The target experience is:

~~~text
Human intent / conversation / ADL / Studio
        ↓
Genome + security + target constraint envelope
        ↓
Atlas knowledge + donor/dependency census + OSS/web research
        ↓
multiple mechanism/design candidates
        ↓
typed fast decision/ranking
        ↓
external synthesis provider writes real implementation candidate
        ↓
CandidateChangeSet
        ↓
Atlas inventories/censuses generated code
        ↓
security + dependency + license + semantic + verification gates
        ↓
authorized SelectedDesign
        ↓
SEALED logical Atlas
        ↓
mechanical deterministic compaction
~~~

Search/research may feel like a research assistant; fast typed selection may use a Jev-class provider; code synthesis may use frontier external providers. These are replaceable adapters and do not own canonical semantics.

This lane is governed by `../contracts/HUMAN-AI-ADL-AUTHORING.md`, `../contracts/ATLAS-CREATION-PIPELINE.md`, `../contracts/EXTERNAL-PROVIDER-TRUST.md`, `../contracts/SELF-BUILD-CONTROLLER.md`, `../contracts/ADMISSION-TRANSACTION.md` and `../contracts/VERIFICATION-METRICS-PERFORMANCE.md`.

## Language Genesis lane — after typed census semantics are trustworthy

Atlas Development Language is synthesized from the same semantic world; it is not designed as a syntax-first side project.

Sequence:

- retain and extend the R4.4 lossless typed Census/normalization carrier beyond the currently-real SYMBOL/TYPE/FUNCTION_IDENTITY/FUNCTION_SIGNATURE dimensions and strengthened declaration identity;
- deep-census selected donors and their admitted dependency closures and emit Technology Genomes;
- compare mechanisms/invariants/trade-offs across donors and research;
- admit Atlas-native semantic primitives under `../contracts/DONOR-TO-LANGUAGE-GENESIS.md`;
- evolve current ADL0 declaration syntax into the full language without creating a parallel truth model;
- implement deterministic ADL → typed semantic records → ATLAS lowering;
- add type/effect/resource/ownership/state/concurrency semantics by explicit contract and evidence;
- validate against reference workloads through differential compilation/behavior tests;
- retain donor runtime dependencies only as explicit adapters/oracles, never hidden language ownership.

The current `.atlas/declared/*.adl` implementation remains ADL0 until the readiness gates in `../contracts/ATLAS-DEVELOPMENT-LANGUAGE.md` are satisfied.

Donor execution order, per-wave census targets, Atlas ownership targets, recensus gates, discovery dispositions and physical source-extinction rules are governed by `SELF-BUILDING-R4-R8.md`, `DONOR-ABSORPTION-ROADMAP.md` and `DONOR-ABSORPTION-PLAN.toml`.

## Organism Foundation — after universal semantics are real

Before model training ambitions:

- implement Organism Genome v1 types;
- persistent OrganismId / GenomeId / SpeciesId / Generation / RuntimeInstance lineage;
- organ/circuit/trait semantics;
- body/environment capability model;
- durable memory lineage;
- learning/adaptation candidate model;
- authority constraints and self-modification admission;
- model/provider/checkpoint bindings supporting API, self-hosted, hybrid and none;
- homeostasis and metabolism;
- lifecycle/birth/suspend/retire/recovery;
- species templates as reusable constraints, not product silos.

## Phase 1 — deterministic AtlasX + delegated compiler

- general `*.atlas → *.atlasx/`;
- digital-organism AtlasX profile;
- Rust backend / TypeScript frontend / bounded C boundary lowering;
- phenotype repository/product generation;
- compile/test/benchmark/recensus and product lineage.

## Phase 2 — HIR/MIR semantic optimizer

- graph/binding/organ/circuit specialization;
- devirtualization and policy partial evaluation;
- ownership/lifetime/region/escape/alias analysis;
- state/memory/model placement;
- data layout/locality;
- concurrency scheduling;
- classic scalar/control/loop optimization;
- semantics barriers for authority/evidence/temporal/lifecycle/model admission.

## Phase 3 — external native backends

- LIR;
- LLVM/Cranelift/WASM/accelerator paths;
- stable Atlas ABI/object layout;
- vectorization/SIMD/target specialization;
- GPU/model runtime integration where selected.

## Phase 4 — Atlas native backend

- Machine IR, instruction selection, register allocation, scheduling, object emission;
- x86-64, ARM64 and justified targets;
- native linker/object tooling only after sequencing supports it.

## Organism learning/evolution lane

After organism substrate and authority are mature:

~~~text
Experience
→ Durable Observation/Event
→ Memory
→ Dataset Compiler
→ Training/Adaptation
→ Candidate Model/Weights/Rule
→ Evaluation
→ Simulation/Regression/Safety
→ Genome Compatibility
→ Admission
→ Activation
→ Evidence/Rollback
~~~

No learning artifact activates directly.

## UI

World Canvas remains a projection over the same graph. It may visualize organism organs/circuits/memory/model lineage but does not own organism truth.

## Rust parity/surpass maturity lane

The compiler destination is not merely native codegen. Follow `../blueprints/RUST-PARITY-AND-SURPASS-ROADMAP.md` through ownership/borrow safety, diagnostics, codegen correctness, LLVM integration, ABI/platform support, debug info, native backend parity, whole-world optimization, PGO/auto-tuning and sustained compiler maturity.

Atlas may only claim a scoped Rust-surpass result after parity gates are green and the same semantic workload is demonstrably faster/more efficient under locked safety, hardware and workload constraints.
