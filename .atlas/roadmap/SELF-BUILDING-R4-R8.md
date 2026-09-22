---
id: atlas.roadmap.self-building-r4-r8
type: blueprint
status: canonical
canonical: true
---
# Atlas Studio Self-Building Execution Map — R4 through R8

## Purpose

This document is the canonical execution map for building **Atlas Studio itself** from R4 through R8 by repeatedly censusing approved OSS donors and the complete admitted dependency graphs that actually provide their behavior.

It exists to prevent implementation agents from inventing their own sequencing, redefining census, treating donor repositories as permanent architecture, or declaring absorption/extinction without proof.

This document does not replace the global phase meanings in `ROADMAP.md`, the dependency rules in `../contracts/DEPENDENCY-CENSUS.md`, or the donor state machine in `DONOR-ABSORPTION-ROADMAP.md`. It fixes how those contracts execute together from R4 through R8.

## Scope

The current construction target is **Atlas Studio itself**.

The R4→R8 self-building loop is:

~~~text
current Atlas capability
        ↓
admitted donor roots
        ↓
transitive dependency closure for every admitted resolution context
        ↓
census at the deepest semantic level Atlas currently supports
        ↓
discover mechanisms / invariants / useful ideas / missing capabilities
        ↓
attribute each discovery to its actual provider scope
        ↓
explicit absorption disposition
        ↓
deep census selected provider scope and required dependency subtree
        ↓
Technology Genome / durable evidence
        ↓
Atlas-native design
        ↓
implementation under core/runtime/adapter/apps
        ↓
verification / benchmark where material
        ↓
recensus Atlas
        ↓
prove donor source/runtime dependency is zero for the absorbed scope
        ↓
durable-knowledge gate
        ↓
physical donor-source deletion
        ↓
post-delete recensus
        ↓
EXTINCT
        ↓
stronger Atlas capability
        ↺
~~~

Census is therefore not a one-time import phase. Atlas builds itself by repeatedly improving census, recensusing donors and dependencies, absorbing selected mechanisms, and deleting donor source only after the extinction gate closes.

## Two independent axes

Do not confuse Atlas construction revisions with donor execution waves.

~~~text
R4 / R5 / R6 / R7 / R8
= maturity of Atlas's own native capabilities

W0 / W1 / W2 / ...
= donor-technology execution lanes

dependency closure
= breadth: what engineering world must be censused

R4 semantic dimensions
= depth: how deeply each admitted scope can currently be understood
~~~

A donor wave may span several R revisions. An R revision may use several donor waves. The numbering systems MUST remain distinct.

## Cross-cutting dependency gate — DC1

`DC1 — Dependency Census Runtime` is a cross-cutting implementation gate, not an R4 semantic dimension and not a donor W-wave.

DC1 is production-real only when Atlas can perform, for each admitted dependency-resolution context:

~~~text
root inventory
→ workspace/package/build declarations
→ deterministic direct dependency resolution
→ deterministic transitive dependency expansion
→ stable dependency identities
→ source-backed dependency admission
→ explicit non-source/toolchain/system/service terminals
→ dynamic/build-generated dependency obligations
→ repeat until dependency fixed point
→ expanded federated inventory
~~~

The normative semantics are in `../contracts/DEPENDENCY-CENSUS.md`.

A manifest or lockfile alone does not satisfy DC1.

A root repository boundary does not satisfy DC1.

DC1 is required before W0 may claim `COARSE_CENSUSED` for the full donor corpus.

## When real donor census starts

Atlas MUST NOT wait until R4, R5 or R6 are complete before censusing OSS.

The required sequencing is:

~~~text
R4.4 Function Identity Closure materialized
        ↓
R4.5 CALL semantics may proceed
        │
        └──────────────┐
                       ↓
                    DC1 real
                       ↓
══════════════════════════════════════════════
START W0 REAL DONOR CENSUS ACROSS DEPENDENCIES
══════════════════════════════════════════════
                       ↓
coarse census every admitted donor
+ every active direct/transitive dependency edge
+ every admitted source-backed dependency node
                       ↓
continue R4 semantic depth
                       ↓
recensus targeted donor/dependency scopes after each capability improvement
~~~

R4.x improves semantic depth. DC1 expands census breadth. They are complementary, not sequential substitutes.

## R4 — semantic census depth

R4 owns evidence-producing semantic extraction, typed preservation, deterministic normalization inputs, and the semantic depth required to understand donor implementations.

### R4.3.x — real Rust semantic bootstrap — materialized

Current materialized sequence:

- R4.3 — first real Rust semantic extractor;
- R4.3.1 — canonical production census wiring;
- R4.3.2 — lossless typed records through Census and normalization;
- R4.3.3 — raw observation identity, typed obligation lineage, typed closure accounting, and typed engineering-graph boundary.

Currently real Rust semantic dimensions:

- SYMBOL;
- TYPE;
- FUNCTION_IDENTITY;
- FUNCTION_SIGNATURE.

These facts are useful but are not sufficient for mechanism absorption by themselves.

### R4.4 — Function Identity Closure — materialized

R4.4 is materialized on canonical main.

The production identity model now distinguishes source-observable declaration context through typed declaration kind, owner/trait context and function generics while preserving repository/revision/scope identity and R4.3.3 raw-observation separation.

Materialized declaration kinds include:

- free function;
- inherent method;
- associated function;
- trait method declaration;
- trait default method;
- trait implementation method.

R4.4 intentionally does not claim compiler DefId-level equivalence, type-alias equivalence, macro-expanded declarations or monomorphized instance identity.

CALL remains the next semantic relation.

### R4.5 — Call Semantics

Materialize typed CALL observations:

- call-site identity;
- caller FunctionIdentity;
- exact static target when evidenced;
- finite partial target sets when evidenced;
- dynamic target placeholders;
- unresolved target obligations;
- FFI/external call boundaries;
- call evidence/provenance.

Name-only call targets are forbidden.

### R4.6 — Control Flow

Materialize CONTROL_FLOW:

- deterministic block identity;
- entry/exit;
- branches;
- loops;
- return edges;
- panic/failure edges;
- explicit unresolved control constructs;
- function-to-CFG closure.

CFG identity MUST be stable for identical pinned input and MUST NOT depend on traversal/hash iteration order.

### R4.7 — Data Flow

Materialize DATA_FLOW:

- values;
- definitions/uses;
- parameter flow;
- return flow;
- load/store relationships;
- local propagation;
- typed unresolved/alias ambiguity where deeper analysis is unavailable.

Do not claim compiler-complete alias analysis unless actually evidenced.

### R4.8 — State and Effect

Materialize STATE and EFFECT:

- state identities;
- reads;
- writes;
- transitions;
- externally observable effects;
- filesystem/network/process/FFI/build/runtime interactions where applicable;
- failure effects;
- explicit unknown/dynamic behavior.

This is the minimum point at which many donor mechanisms become semantically useful for absorption because Atlas can connect implementation behavior to state change and external effect.

### R4.9 — Ownership and Resource Semantics

Materialize OWNERSHIP at the level required for reliable census:

- borrow/move/copy behavior where evidenced;
- allocation/free/resource acquisition/release;
- resource ownership transfer;
- lifetime/region facts only to the level supported by admitted evidence;
- explicit unresolved ownership where compiler-grade analysis is absent.

Do not claim rustc-equivalent borrow checking merely because ownership facts exist.

### R4.10 — Concurrency Semantics

Materialize CONCURRENCY:

- threads/tasks;
- channels;
- locks/unlocks;
- atomics;
- synchronization;
- ordering relationships;
- concurrent state interaction;
- dynamic/unresolved concurrency obligations.

### R4.11 — Persistence and Recovery Semantics

Materialize PERSISTENCE:

- durable writes;
- transaction boundaries;
- log/checkpoint/recovery behavior where applicable;
- durability ordering;
- recovery/failure paths;
- external persistence boundaries;
- explicit unknowns.

### R4.12 — R4 Semantic Closure

R4 closes only after the declared Rust reference profile satisfies the canonical R4 acceptance contract.

R4.12 must include:

- all mandatory R4 dimensions evidence-producing or explicitly accounted;
- every discovered function with stable identity/signature or explicit unresolved state;
- deterministic normalization;
- exact semantic duplicate policy;
- multi-extractor observations preserved;
- conflict candidates preserved for reconciliation;
- dynamic/unresolved facts explicit;
- reference corpus;
- deterministic accounting/closure tests;
- graph construction only from normalized typed truth;
- no compatibility `SemanticFact` authority over typed semantics.

R4 closure is profile-scoped. Do not claim universal language/compiler completeness.

## R5 — incremental query and fixed-point closure

R5 adds the ability to reason and recensus incrementally over the already-growing donor/dependency corpus.

Required capabilities:

- dependency-aware query invalidation;
- revision-scoped cached derivation;
- recursive/fixed-point semantic derivation;
- incremental recensus of changed source and affected dependents;
- deterministic propagation;
- evidence/provenance lineage through derived facts.

Primary donor lane: W3 — salsa, datafrog, differential-dataflow, souffle, buck2.

Every W3 donor is still censused through its admitted transitive dependency closure. If a dependency actually provides a mechanism of interest, attribute the mechanism to that dependency rather than the top-level donor.

After every material R5 capability:

~~~text
implement
→ verify
→ recensus Atlas
→ recensus affected donors/dependencies
→ update Technology Genomes/decisions
~~~

## R6 — reconciliation, adversarial closure and CensusCertificate

R6 turns accounted observations into a closure claim that can be independently checked.

Required capabilities:

- preserve independent extractor identities;
- explicit CONFLICT;
- cross-scope reconciliation;
- top-down claim decomposition;
- bottom-up aggregation;
- adversarial gap queries;
- fixed-point closure;
- dependency closure included in closure proof;
- CensusCertificate issuance;
- Genome policy enforcement for UNKNOWN/UNSUPPORTED.

Primary support donor lane: W4 — kani, miri, verus.

R6 does not mean "choose a winner whenever extractors disagree." Conflict remains conflict until evidence supports reconciliation.

R6 is the first point at which Atlas can make strong proof-producing statements that a declared census scope is closed for a declared policy/profile.

## R7 — research correlation and absorption selection

R7 connects observed donor reality to research and design decisions without allowing research/model claims to impersonate observation.

Required capabilities:

- ResearchClaim distinct from ObservedEvidence;
- Technology Genome comparison;
- Atlas capability-gap graph;
- candidate mechanism/design records;
- evidence-linked absorption decisions;
- explicit selected/rejected/deferred dispositions;
- validation obligations before native implementation becomes selected design.

Primary donor lane: W5 — openrewrite, c2rust, crubit, py2many.

R7 MUST preserve this rule:

~~~text
Observed implementation
≠ ResearchClaim
≠ CandidateDesign
≠ SelectedDesign
~~~

No research page, paper, model answer or README directly upgrades a donor implementation claim to OBSERVED.

## R8 — durable ATLAS / AtlasX substrate

R8 implements the durable semantic/evidence carrier required for Atlas knowledge to outlive donor checkout deletion at scale and makes the Atlas→AtlasX handoff implementable without hidden design invention.

Required capabilities:

- typed binary `*.atlas`;
- lossless semantic compaction under `../contracts/ATLAS-SEMANTIC-COMPACTION.md`;
- content-addressed records/blocks/shards;
- integrity hashes;
- transactional publication;
- logical root manifests;
- stable cross-shard identity/bindings;
- explicit SelectedDesign identity under `../contracts/SELECTED-DESIGN.md`;
- deterministic Atlas→AtlasX materialization under `../contracts/ATLAS-TO-ATLASX.md`;
- canonical AtlasX object/manifest validation under `../contracts/ATLASX-FORMAT.md`;
- canonical AtlasX v1 bytes/root hashing under `../contracts/ATLASX-BINARY-WIRE-FORMAT.md`;
- parent/lineage retention;
- partial materialization without competing truth;
- compiler handoff governed by `../contracts/COMPILER-IR-PIPELINE.md` and v1 IR records/opcodes governed by `../contracts/COMPILER-IR-SCHEMAS.md`.

Primary donor lane: W6 — flatbuffers, arrow, zstd, blake3, object, regalloc2, mold.

R8 does not authorize mechanical donor translation. It provides the durable Atlas-native carrier and deterministic executable projection needed for source-independent continuation.

The R8 storage/materialization blueprint is explicitly revisable if census demonstrates a better mechanism, but revision must follow `../contracts/BLUEPRINT-EVOLUTION.md`.

## Evidence-driven blueprint evolution

Canonical blueprints are authoritative for the current evidence state, but they are not immutable.

During any R4→R8 census/recensus, Atlas may discover a mechanism, representation, compiler strategy, storage layout, verification method or dependency architecture that is materially better than the current blueprint.

When that happens Atlas MUST NOT ignore the evidence merely to preserve an older plan, and MUST NOT silently redesign in implementation code.

The required path is governed by `../contracts/BLUEPRINT-EVOLUTION.md`:

~~~text
current canonical blueprint
→ donor/dependency census discovers better mechanism
→ attribute actual provider
→ deep census
→ compare current vs candidate vs alternatives
→ evidence / benchmark / proof
→ BlueprintRevisionDecision
→ SELECTED
→ update canonical blueprint/roadmap/contracts if required
→ implement
→ recensus / verify
~~~

A blueprint revision MAY alter future sequencing, insert prerequisites, split/merge waves or change implementation strategy when evidence justifies it.

A blueprint revision MUST NOT silently violate higher-level contracts/Genome invariants. If the better design requires a contract change, that contract change is explicit and compatibility/migration analysis is mandatory.

"Canonical" therefore means "currently selected authoritative design", not "frozen forever".

## Continuous census and recensus rule

Every material Atlas capability improvement from R4 through R8 MUST consider recensus impact.

The default loop is:

~~~text
stronger Atlas capability
→ recensus Atlas itself
→ recensus affected donor scopes
→ recensus affected source-backed dependency scopes
→ compare new evidence with prior observations
→ deepen Technology Genome where justified
→ reconsider pending absorption decisions
~~~

A capability wave that changes what Atlas can observe but never recensuses relevant donors is incomplete as a self-building wave.

## Discovery is allowed; uncontrolled drift is not

Census may reveal technology not listed in the authored donor plan.

Examples include:

- an unexpected algorithm;
- a better incremental strategy;
- an important data structure;
- a compiler or query technique;
- a verification method;
- a storage/layout mechanism;
- a hidden provider dependency;
- an Atlas capability gap not previously recognized.

Discovery does not automatically change Atlas architecture and does not automatically create a donor.

Every material discovery receives an explicit disposition.

## Discovery dispositions

The canonical planning dispositions are:

~~~text
ABSORB_NOW
ABSORB_LATER
REFERENCE_ONLY
EXTERNAL_BOUNDARY
REJECT
~~~

### ABSORB_NOW

Use only when the discovered technology:

- removes or materially reduces a current R4→R8 blocker or capability gap;
- is a prerequisite for the active self-building path, or is clearly high-leverage and bounded;
- has an identifiable actual provider scope;
- has a plausible Atlas-native owner;
- can be deep-censused to the semantics required for safe absorption;
- has a credible verification path;
- has a credible dependency-removal/extinction path.

### ABSORB_LATER

Use when the mechanism is valuable to Atlas but:

- depends on capabilities not yet mature;
- would violate current sequencing;
- has too large an unresolved scope;
- is not required for the active construction path.

The discovery remains durable and queued; it is not silently forgotten.

### REFERENCE_ONLY

Use when the donor/dependency is useful as:

- oracle;
- comparison;
- test reference;
- explanatory implementation evidence;

but Atlas has not selected its technology for native ownership.

Local source retained for reference means the scope is not EXTINCT.

### EXTERNAL_BOUNDARY

Use when Atlas intentionally keeps a technology external through an explicit adapter/capability boundary.

External technology MUST remain explicitly identified, versioned and censused according to policy. It MUST NOT be renamed Atlas-native.

### REJECT

Use when the mechanism is:

- irrelevant;
- redundant;
- inferior for the active constraints;
- incompatible with Atlas invariants;
- unjustified by evidence;
- legally/provenance constrained in a way that prevents the intended absorption;
- outside the current Atlas self-building objective.

Rejection retains the evidence and rationale needed to avoid rediscovering and re-evaluating the same dead end without cause.

## Required discovery decision record

Every material discovery selected for planning MUST retain enough information to answer:

- what was discovered;
- exact provider identity/revision/version;
- provider scope;
- how it was discovered;
- evidence references;
- whether the provider is the root donor or a dependency;
- proposed Atlas capability;
- proposed native owner;
- current disposition;
- decision rationale;
- required semantic depth;
- prerequisites/blockers;
- verification plan;
- dependency-removal plan;
- extinction implications.

The physical data type may be added in a later implementation wave. The semantic obligations are fixed here.

## Discovered dependency is not automatically an absorption donor

All active dependencies are census subjects.

They are not automatically donor-admission subjects.

Example:

~~~text
Donor A
→ dependency B
→ dependency C
~~~

A, B and C are all dependency-census nodes when active in an admitted context.

If census discovers that C owns a mechanism Atlas wants to absorb, C MUST be explicitly promoted through donor admission before absorption work begins.

Promotion requires:

- exact provider identity;
- exact version/revision;
- license;
- provenance;
- selected mechanism/scope;
- Atlas capability target;
- native owner;
- required semantic depth;
- verification plan;
- extinction/dependency plan.

No implementation agent may silently promote a dependency to donor status.

## Idea-driven pull-forward rule

W-wave order is the default donor execution order, not an excuse to ignore a prerequisite.

A mechanism from a later donor lane may be pulled forward only when all of the following are true:

- it is a concrete prerequisite/blocker for the active R-wave, or its leverage on the active Atlas capability is clear;
- the scope is bounded;
- the actual provider is identified;
- evidence is available;
- the Atlas-native owner is known;
- required semantic depth can be achieved;
- verification is definable;
- the decision is recorded as ABSORB_NOW.

Otherwise use ABSORB_LATER.

"Interesting" alone is never sufficient reason to pull a donor/mechanism forward.

## Deep-census gate before absorption

Atlas MUST NOT select a mechanism for native implementation from README/API shape/function names/model description alone.

Before absorption, the provider scope must be censused to the depth required to identify the mechanism's essential semantics.

Depending on the mechanism this may require:

- participating types/functions;
- call relationships;
- control flow;
- data flow;
- state reads/writes;
- effects;
- ownership/resource assumptions;
- concurrency;
- persistence/recovery;
- failure paths;
- tests/runtime/binary evidence;
- dependency-provided behavior;
- constraints/invariants;
- trade-offs;
- unresolved facts.

The required depth is mechanism-specific. Not every mechanism requires every S10 atom. Essential dependencies and unknowns may not be silently omitted.

## Absorption state machine

Canonical scope-level progression:

~~~text
STAGED
  ↓
COARSE_CENSUSED
  ↓
DEEP_CENSUS_ACTIVE
  ↓
TECHNOLOGY_GENOME_CAPTURED
  ↓
NATIVE_IMPLEMENTED
  ↓
ABSORBED
  ↓
EXTINCTION_READY
  ↓
SOURCE_DELETED
  ↓
EXTINCT
~~~

`EXTINCTION_READY` is a non-terminal state. It exists so Atlas can truthfully distinguish "native replacement and dependency-removal proof are complete" from "the donor source has actually been deleted and post-delete verification succeeded."

A scope may skip a long residence in EXTINCTION_READY when all deletion prerequisites are already satisfied, but it may not skip SOURCE_DELETED.

## ABSORBED definition

A scope is ABSORBED only when:

- required mechanism/invariants are durably captured;
- Atlas-native implementation exists in the intended owner;
- required tests/proofs/benchmarks pass;
- Atlas recensus agrees with selected design within declared policy;
- donor runtime/build/test source dependency is zero for the absorbed scope;
- remaining donor-only knowledge for the scope is captured, explicitly deferred, or explicitly rejected.

ABSORBED is not EXTINCT.

## EXTINCTION_READY definition

A scope is EXTINCTION_READY when the ABSORBED conditions hold and deletion can proceed once the durable-knowledge and physical-deletion gates are satisfied.

Before R8, a scope may remain EXTINCTION_READY when the canonical durable Atlas carrier required to survive source deletion is not yet mature for that scope.

Do not weaken the durable-knowledge requirement merely to report faster extinction.

## EXTINCT definition

EXTINCT is physical.

An extinct scope MUST satisfy all of:

- exact extinct donor/revision/scope recorded;
- relevant dependency closure accounted;
- durable Technology Genome/evidence/native replacement retained;
- zero runtime/build/test dependency on the donor source for the extinct scope;
- donor OSS source files for the extinct scope physically removed from Atlas-controlled active storage;
- no substitute Atlas-controlled source archive/cache/snapshot/vendor copy retained as a hidden equivalent;
- source-path absence explicitly checked;
- post-delete Atlas recensus passes;
- required tests/proofs/benchmarks still pass.

Renaming, ignoring, unreferencing, vendoring elsewhere or archiving the same source does not satisfy extinction.

Historical Git objects are governed separately by repository-history policy and do not make active-tree deletion false.

## Scope-level versus repository-level extinction

Absorption/extinction is fundamentally scope-level.

A repository may contain:

~~~text
scope A  EXTINCT
scope B  ABSORBED but source retained for another unresolved scope
scope C  REFERENCE_ONLY
scope D  not yet deep-censused
~~~

Do not label the whole donor repository EXTINCT while required donor source remains under Atlas control.

Repository-level EXTINCT is valid only when all retained scopes satisfy the repository-level deletion rule.

## Dependency-aware extinction

Donor A does not own mechanisms merely because A depends on them.

If:

~~~text
A
→ B
→ C
~~~

and B provides the selected mechanism, absorption/extinction attribution belongs to B's provider scope.

If Atlas intentionally retains B as an external dependency, B remains an explicit dependency.

If Atlas selects B for native absorption, B receives its own donor admission, Technology Genome, native implementation, verification, recensus and extinction lifecycle.

Third-party dependency code MUST NOT be silently relabeled Atlas-native.

## R4→R8 donor-lane map

The default mapping is:

| Atlas maturity | Primary donor lane | Purpose |
| --- | --- | --- |
| R4 + DC1 | W0, W1, W2 | corpus breadth, source intelligence, semantic/compiler understanding |
| R5 | W3 | incremental query, fixed point, build/dependency reasoning |
| R6 | W4 where relevant | verification/safety evidence supporting reconciliation and closure |
| R7 | W5 | transformation/migration mechanisms and evidence-linked selection |
| R8 | W6 | durable binary/storage/data-layout/backend mechanisms |
| later/default | W7 | security/trust breadth unless a bounded prerequisite is pulled forward |
| later/default | W8 | Studio/editor projection unless a bounded prerequisite is pulled forward |

This table is a default execution map, not permission to skip dependency closure and not permission to pull work forward without the recorded ABSORB_NOW criteria.

## Required self-building loop at every implementation wave

Before coding, the agent MUST identify:

- exact canonical main SHA;
- active R-wave or cross-cutting gate;
- Atlas-native capability being made real;
- donor/dependency evidence that motivates it, if applicable;
- native owner: core/runtime/adapter/apps;
- semantic truth that becomes more complete;
- remaining UNKNOWN/UNSUPPORTED states;
- donor scopes to recensus afterward;
- whether any discovery decision changes;
- whether any absorbed scope becomes EXTINCTION_READY;
- whether any EXTINCTION_READY scope may safely progress through SOURCE_DELETED to EXTINCT.

After coding, the agent MUST report the same items with evidence.

An agent MUST NOT invent a new architectural owner, new semantic path, new donor status, or extinction claim to make a wave appear complete.

## Implementation truth vocabulary

Use these maturity words precisely:

- DOCUMENTED — contract/blueprint exists;
- TYPED — type/API exists;
- IMPLEMENTED — working logic exists;
- CALLED_IN_PRODUCTION — canonical runtime path invokes it;
- TESTED — behavior is covered by verification;
- VERIFIED — exact candidate/revision has durable verification evidence.

Do not use "done" to collapse these distinctions.

## Prohibited shortcuts

The following are forbidden:

- waiting until R4/R5/R6 finish before starting donor census;
- stopping census at a donor repository boundary;
- treating a lockfile as dependency closure;
- silently omitting build/proc-macro/native/dynamic dependencies;
- automatically admitting every transitive dependency as an absorption donor;
- absorbing a mechanism from README/API/prose alone;
- copying donor topology into Atlas architecture;
- wrapping a donor library permanently and calling it Atlas-native;
- allowing donor-named production ownership outside provenance/reference roles;
- using model output as OBSERVED donor implementation truth;
- implementing a native replacement without recensus;
- claiming ABSORBED when donor runtime/source dependence remains;
- claiming EXTINCT while donor source remains under Atlas control;
- moving donor source into another cache/vendor/archive and calling it deleted;
- declaring an entire repository extinct because one mechanism is extinct;
- pulling an interesting later donor forward without prerequisite/high-leverage evidence;
- creating a second semantic truth path.

## Completion through R8

By the end of R8, Atlas should have progressed from typed Rust source observations to a system that can:

~~~text
expand donor roots through admitted dependency closure
→ census implementation semantics deeply
→ incrementally query and recensus
→ reconcile and prove closure
→ compare observed mechanisms and research
→ explicitly select what Atlas should absorb
→ encode durable Technology Genomes/evidence in real ATLAS
→ implement Atlas-native replacements
→ recensus those replacements
→ physically extinguish eligible donor source scopes
→ continue with a stronger Atlas
~~~

R8 is not the end of census. It is the point where census-derived knowledge has a durable native carrier suitable for large-scale source-independent continuation.

## Final invariant

Atlas Studio is not built first and used to census OSS later.

Atlas Studio is built **by** repeatedly censusing OSS donors and their complete admitted dependency graphs, learning from them under evidence discipline, selectively absorbing mechanisms into Atlas-native ownership, proving the replacements, and physically deleting donor source only when extinction is true.
