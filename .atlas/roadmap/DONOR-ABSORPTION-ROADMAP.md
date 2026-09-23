---
id: atlas.roadmap.donor-absorption
type: blueprint
status: active
canonical: true
---
# Donor Absorption and Extinction Roadmap

## Purpose

This roadmap turns the donor registry into an execution order for Atlas self-building.

Canonical inputs:

- `SELF-BUILDING-R4-R8.md` — canonical R4→R8 self-building execution and discovery/absorption policy;
- `../references/donor-corpus.toml` — admitted donor identity, pinned revisions, licenses, provenance and current status;
- `DONOR-ABSORPTION-PLAN.toml` — machine-readable wave order, donor membership and policy flags;
- `../blueprints/BULK-DONOR-ABSORPTION.md` — bulk staging and absorption/extinction gates;
- `../contracts/DEPENDENCY-CENSUS.md` — transitive dependency census requirements;
- `../contracts/DONOR-TO-LANGUAGE-GENESIS.md` — Technology Genome and language-genesis rules.

This roadmap is about building **Atlas Studio itself**. Donor repositories are temporary evidence/workbenches, not permanent Atlas architecture.

## State machine

Canonical scope-level progression:

~~~text
STAGED
  ↓ pin SHA/license/provenance
COARSE_CENSUSED
  ↓ capability/dependency targeting
DEEP_CENSUS_ACTIVE
  ↓ typed mechanisms/invariants/evidence
TECHNOLOGY_GENOME_CAPTURED
  ↓ Atlas-native design + implementation
NATIVE_IMPLEMENTED
  ↓ differential tests/benchmarks + Atlas recensus
ABSORBED
  ↓ all non-deletion extinction gates closed
EXTINCTION_READY
  ↓ durable-knowledge carrier ready + physical source deletion
SOURCE_DELETED
  ↓ verify absence + post-delete recensus
EXTINCT
~~~

`ABSORBED` is not `EXTINCT`.

`EXTINCTION_READY` is not `EXTINCT`.

`SOURCE_DELETED` is mandatory before `EXTINCT`.

An EXTINCT scope MUST NOT retain its donor OSS source files under Atlas control. The active source root is `.atlas/temporary/donors/<donor>/`. Renaming, ignoring, unreferencing, archiving or moving the same source into another Atlas-controlled cache/vendor/snapshot directory does not satisfy extinction.

A donor retained locally as an explicit oracle/reference is not extinct.

A scope that only removes imports but fails a clean donor-disappearance/rebuild probe is not EXTINCTION_READY.

Historical Git objects are outside this active-tree deletion rule unless a separate history-scrubbing policy is adopted.

## Recursive extinction sweeps

After every material Atlas capability generation, recompute extinction readiness with the stronger observer.

Before SOURCE_DELETED, run the isolated extinction probe from `../contracts/RECURSIVE-SELF-CENSUS.md`: donor source and Atlas-controlled substitutes unavailable, silent network re-fetch disabled, affected build/test/scenarios/recensus still pass.

If stronger census later exposes a hidden dependency or semantic obligation, create corrective evidence and a new state lineage under this existing state machine. Do not invent a donor state and do not rewrite the old proof.

The self-building target is retained donor source → 0 only through proven independence. Explicit REFERENCE_ONLY / EXTERNAL_BOUNDARY retention prevents the stronger "all donor source extinct" claim.

### Corpus extinction accounting

At each generation Atlas SHOULD report, separately:

- source scopes still STAGED/COARSE_CENSUSED/DEEP_CENSUS_ACTIVE;
- source scopes waiting on native implementation;
- ABSORBED scopes not yet EXTINCTION_READY;
- EXTINCTION_READY scopes waiting on destructive proof;
- EXTINCT scopes;
- explicit REFERENCE_ONLY / EXTERNAL_BOUNDARY exceptions.

A decreasing donor-source count is progress evidence, not correctness evidence by itself.

## Dependency closure rule

Every donor is censused through its resolved dependency closure under `../contracts/DEPENDENCY-CENSUS.md`.

For every admitted dependency-resolution context Atlas MUST:

- resolve every active direct dependency edge;
- resolve every active transitive dependency edge;
- assign stable concrete dependency identity;
- inventory/census every source-backed dependency node according to policy;
- record non-source/toolchain/system/service dependencies as explicit typed terminals;
- account for build/proc-macro/codegen/native/dynamic dependency obligations;
- continue until dependency fixed point.

The top-level donor repository is not assumed to own every mechanism observed in its behavior. Atlas attributes mechanisms to the actual provider scope.

A transitive dependency is automatically a census subject when active in an admitted context. It is **not** automatically an absorption donor.

## Donor promotion rule

If a dependency contains technology Atlas may absorb, it must be explicitly promoted through donor admission before absorption work begins.

Promotion requires:

- exact provider identity;
- exact version/revision;
- license;
- provenance;
- selected mechanism/scope;
- Atlas capability target;
- intended native owner;
- required semantic depth;
- verification plan;
- dependency-removal/extinction plan.

No agent may silently promote a dependency into donor status.

## Discovery dispositions

Census is allowed to discover useful mechanisms and ideas beyond the authored donor plan, but every material discovery MUST receive one explicit disposition:

~~~text
ABSORB_NOW
ABSORB_LATER
REFERENCE_ONLY
EXTERNAL_BOUNDARY
REJECT
~~~

### ABSORB_NOW

Use only when the discovery is a concrete prerequisite/blocker for the active Atlas construction path, or clearly high-leverage and bounded, and:

- actual provider scope is identified;
- sufficient evidence is available;
- Atlas-native owner is known;
- required semantic depth is achievable;
- verification is definable;
- a dependency-removal/extinction path is credible.

### ABSORB_LATER

Use when the technology is valuable but current sequencing, prerequisites, unresolved scope or cost makes immediate absorption incorrect.

It remains a durable queued discovery.

### REFERENCE_ONLY

Use for an oracle/comparison/reference that Atlas does not currently select for native ownership.

Locally retained reference source is not extinct.

### EXTERNAL_BOUNDARY

Use when Atlas intentionally retains an external capability through an explicit adapter/capability boundary.

External technology remains explicitly identified and censused. It is not Atlas-native.

### REJECT

Use when the technology is irrelevant, redundant, unjustified, incompatible with Atlas invariants, legally/provenance constrained for the intended use, or outside current Atlas self-building objectives.

The rationale remains durable.

## Blueprint revision from donor discovery

A donor/dependency discovery may do more than fill an existing Atlas capability gap.

It may demonstrate that the **current Atlas blueprint itself is inferior**.

When the discovered mechanism would change Atlas architecture, representation, storage, materialization, compiler staging, identity, query strategy or another selected design rule, the discovery MUST enter the blueprint-evolution path defined by `../contracts/BLUEPRINT-EVOLUTION.md`.

~~~text
census discovery
→ actual provider attribution
→ explicit discovery disposition
→ deep census
→ compare current blueprint vs candidate vs alternatives
→ evidence / benchmark / proof
→ BlueprintRevisionDecision
→ SELECTED
→ update canonical blueprint/roadmap/contracts if required
→ Atlas-native implementation
→ recensus / verification
~~~

`ABSORB_NOW` may therefore trigger a blueprint revision when the mechanism is not merely an implementation detail but a better design for Atlas itself.

Do not force a superior mechanism into an obsolete blueprint.

Do not silently rewrite the blueprint in code either.

## Absorption decision criteria

A discovery may be selected for absorption only after considering:

- current Atlas capability gap;
- active R4→R8 prerequisite/blocker relevance;
- correctness/completeness/determinism/performance/verification/independence benefit;
- generality suitable for Atlas-native ownership;
- actual provider attribution;
- required semantic depth and unresolved facts;
- ability to understand and preserve invariants;
- native owner under `core/runtime/adapter/apps`;
- dependency burden introduced or removed;
- alternative mechanisms already present in admitted donors;
- license/provenance constraints;
- verification/benchmark/proof path;
- credible source/runtime dependency removal and extinction path.

Novelty alone is not a reason to absorb.

## Idea-driven pull-forward rule

W-wave order is the default donor execution order, not an absolute prohibition against discovering a prerequisite in a later wave.

A later-wave mechanism may be pulled forward only when all of the following hold:

- concrete prerequisite/blocker or clear high leverage for the active R-wave;
- bounded scope;
- actual provider identified;
- evidence available;
- native owner known;
- required semantic depth achievable;
- verification definable;
- decision recorded as `ABSORB_NOW`.

Otherwise record `ABSORB_LATER`.

## Deep-census gate before absorption

Atlas MUST NOT select a mechanism for native implementation from README/API shape/function names/model description alone.

Before native absorption, census the actual provider scope deeply enough to establish the essential mechanism, including where applicable:

- participating types/functions;
- calls;
- control flow;
- data flow;
- state reads/writes/transitions;
- external effects;
- ownership/resource assumptions;
- concurrency;
- persistence/recovery;
- failure paths;
- tests/runtime/binary evidence;
- dependency-provided behavior;
- constraints/invariants;
- trade-offs;
- unresolved facts.

Depth is mechanism-specific. Essential semantics and unknowns may not be silently omitted.

## Recursion rule

Every material Atlas capability wave closes this loop:

~~~text
implement stronger Atlas census/query/reconciliation/storage capability
  ↓
verify exact candidate
  ↓
re-census Atlas itself
  ↓
re-census affected donor scopes
  ↓
re-census affected source-backed dependency scopes
  ↓
update discoveries / Technology Genomes / decisions
  ↓
deep-census selected ABSORB_NOW mechanisms
  ↓
SelfBuildWorkOrder / candidate mechanism set where R7 self-build is active
  ↓
synthesize or implement Atlas-native candidate
  ↓
verify + benchmark/prove where applicable
  ↓
SelectedDesign
  ↓
AdmissionTransaction when canonical Atlas source changes
  ↓
re-census exact admitted Atlas implementation
  ↓
ABSORBED
  ↓
EXTINCTION_READY when non-deletion gates close
  ↓
physically delete eligible donor source scope
  ↓
verify absence
  ↓
post-delete recensus
  ↓
EXTINCT
~~~

A wave may partially absorb/extinguish a donor. Unabsorbed scopes remain staged, queued, external or reference material according to their explicit disposition.

## R-wave versus W-wave map

Do not confuse Atlas maturity with donor lanes.

| Atlas maturity | Default donor lanes | Primary purpose |
| --- | --- | --- |
| R4 + DC1 | W0, W1, W2 | dependency breadth, source intelligence, semantic/compiler understanding |
| R5 | W3 | incremental query, fixed point, build/dependency reasoning |
| R6 | W4 where relevant | verification/safety evidence for closure |
| R7 | W5 | transformation/migration, self-build work orders, synthesis, evidence-linked selection and admission |
| R8 | W6 | durable ATLAS/storage/data/backend knowledge carrier |
| later/default | W7 | security/trust, unless a bounded prerequisite is pulled forward |
| later/default | W8 | Studio/editor projection, unless a bounded prerequisite is pulled forward |

The detailed R4→R8 requirements live in `SELF-BUILDING-R4-R8.md`.

## Wave 0 — corpus accounting

All donors in `donor-corpus.toml` participate.

Goals:

- exact revision/license/provenance pinned;
- cheap whole-repository inventory;
- full direct + transitive dependency closure for admitted contexts;
- repository/language/module/build/dependency/capability map across the expanded dependency corpus;
- source regions mapped to Atlas capabilities;
- no donor or active dependency edge silently omitted.

W0 may claim full `COARSE_CENSUSED` only after DC1 Dependency Census Runtime is production-real.

This wave is breadth-first. It does not require deep semantics for every donor.

## Wave 1 — source intelligence and semantic graph

Donors:

`tree-sitter`, `rust-analyzer`, `scip`, `kythe`, `glean`, `joern`, `semgrep`, `ast-grep`, `sourcetrail`.

Primary learning:

- structural parsing and incremental syntax;
- stable symbol/type/function identity;
- cross-reference representation;
- call/control/data-flow graph construction;
- queryable fact/schema models;
- structural/semantic search and rewrite boundaries.

Atlas targets:

- `adapter/source`;
- `core/identity`, `core/schema`, universal graph;
- `runtime/census`, `runtime/normalize`, `runtime/query`.

Extinction is per absorbed mechanism/scope. UI-only or unsupported scopes remain non-extinct until separately handled.

## Wave 2 — compiler semantic core

Donors:

`rust`, `llvm-project`, `wasmtime`, `wasm-tools`.

Primary learning:

- function/call/control/data-flow semantics;
- type/layout/ABI constraints;
- ownership/resource/concurrency semantics from Rust where applicable;
- SSA/IR legality and lowering;
- sandbox/capability boundaries;
- WASM validation/component representation.

Atlas targets:

- typed semantic families beyond R4.3.3;
- HIR/MIR/LIR foundations where sequenced;
- resource/effect/authority semantics;
- bounded execution and exchange boundaries.

The locally retained rustc/LLVM/Wasmtime source may remain `REFERENCE_ONLY` while still required as an oracle. Such retained scopes are not EXTINCT.

## Wave 3 — incremental reasoning and build graph

Donors:

`salsa`, `datafrog`, `differential-dataflow`, `souffle`, `buck2`.

Primary learning:

- dependency invalidation;
- memoized/incremental queries;
- fixed-point and relational inference;
- differential graph maintenance;
- build/target identity and scheduling.

Atlas targets:

- `runtime/query`, `runtime/closure`;
- incremental recensus;
- dependency-aware materialization/build execution.

## Wave 4 — verification and safety

Donors:

`kani`, `miri`, `verus`.

Primary learning:

- model checking and proof obligations;
- execution-semantics validation;
- alias/UB/resource safety evidence;
- proof-oriented constraints.

Atlas targets:

- `runtime/verify`;
- semantic obligation/evidence models;
- safety gates for ownership/effect/concurrency semantics.

## Wave 5 — transformation and migration

Donors:

`openrewrite`, `c2rust`, `crubit`, `py2many`.

Primary learning:

- typed repeatable transformations;
- migration IR and semantic preservation;
- inter-language boundaries;
- before/after lineage.

Atlas targets:

- invention/refactor/migration planning;
- deterministic ChangeSet and transformation evidence;
- ADL/native semantic lowering where admitted.

## Wave 6 — binary, storage, data layout and native-backend mechanics

Donors:

`flatbuffers`, `arrow`, `zstd`, `blake3`, `object`, `regalloc2`, `mold`.

Primary learning:

- schema/layout and zero-copy trade-offs;
- columnar data and IPC representation;
- compression and content identity;
- object formats/relocations/debug structures;
- register allocation/spilling;
- linking and parallel binary layout.

Atlas targets:

- logical/wire ATLAS storage;
- content-addressed shards;
- evidence/data-layout optimization;
- Machine IR/backend/link stages when sequenced.

## Wave 7 — security and trust

Donors:

`containers-image`, `podman`, `selinux`, `openscap`, `keycloak`, `keylime`, `clair`.

Primary learning:

- supply-chain admission;
- sandbox/rootless isolation;
- least privilege/access control;
- compliance policy;
- identity/authorization;
- attestation/integrity;
- vulnerability intelligence.

Atlas targets:

- typed security/authority semantics in `core`;
- policy/state machines in `runtime`;
- OS/provider mechanics in `adapter`.

## Wave 8 — Studio/editor projection

Donors:

`zed`, `opendesign`, `xyflow`, `cytoscape-js`, `elkjs`.

Primary learning:

- editor/workspace architecture;
- design authoring;
- graph interaction/rendering;
- automatic layout;
- agent/editor interaction.

Atlas targets:

- `apps/studio` projections over canonical Atlas semantics.

These donors must not become alternate owners of canonical truth.

## ABSORBED gate

Before a scope is ABSORBED, the verification record must establish:

- exact donor/revision/provider scope;
- complete relevant dependency-census accounting;
- Technology Genome/mechanism/invariant evidence durable;
- Atlas-native replacement exists in the intended owner;
- runtime/build/test dependency on donor source is zero for that absorbed scope;
- required tests/proofs/material benchmarks pass;
- Atlas recensus agrees with selected design within declared policy;
- remaining donor-only knowledge is captured, explicitly deferred, externalized or rejected.

Writing replacement code alone does not prove absorption.

## EXTINCTION_READY gate

A scope is EXTINCTION_READY only after all ABSORBED conditions hold and all non-deletion extinction prerequisites are proven.

If the durable canonical knowledge carrier required to survive deletion is not mature for that scope, keep the scope EXTINCTION_READY. Do not weaken the knowledge-retention requirement to accelerate deletion.

## Physical extinction proof

Before a scope can transition from SOURCE_DELETED to EXTINCT, the verification record must establish all of the following:

- exact extinct donor/revision/scope;
- Technology Genome/evidence/native replacement that supersedes it;
- complete census accounting for the relevant transitive dependency closure;
- zero runtime/build/test dependency on the donor source for that scope;
- deletion of donor source files from the active Atlas-controlled donor path;
- absence of substitute local source archives/caches/snapshots/vendor copies;
- post-delete check proving the source path is absent;
- Atlas recensus after deletion still passes required invariants/tests/benchmarks.

Durable license/provenance/revision metadata remains after extinction. The source tree does not.

## Scope-level extinction

Absorption/extinction is scope-level first.

A donor repository may simultaneously contain extinct, absorbed, reference-only and unresolved scopes.

Do not call the entire repository EXTINCT while any required donor source remains under Atlas control.

Repository-level EXTINCT is valid only when the complete retained-source rule is satisfied.

## Agent non-discretion rules

Implementation agents MUST NOT:

- invent a new donor state;
- invent a new absorption disposition;
- skip explicit donor promotion for a discovered dependency;
- claim W0 closure before DC1 is real;
- treat root-repo census as dependency closure;
- absorb from prose/API surface alone;
- pull a later-wave donor forward without the documented ABSORB_NOW criteria;
- create permanent donor-named native ownership;
- claim ABSORBED before recensus and dependency removal;
- claim EXTINCT before physical deletion and post-delete recensus;
- move donor source elsewhere and call it extinct.

If evidence is insufficient, use the explicit unresolved/deferred state rather than inventing certainty.
