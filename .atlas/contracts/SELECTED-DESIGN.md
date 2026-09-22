---
id: atlas.contract.selected-design
type: contract
status: active
canonical: true
---
# Selected Design Contract

## Purpose

A SelectedDesign is the explicit canonical design choice inside one logical Atlas root that is eligible for deterministic AtlasX materialization.

It is not an inference, not a compiler preference and not a directory layout.

~~~text
Observed world / research / Technology Genomes
        ↓
candidate designs
        ↓ validation
validated candidate(s)
        ↓ explicit selection
SelectedDesign
        ↓
ATLAS → ATLASX materialization
~~~

## Distinction from epistemic status

Design lifecycle is not EpistemicStatus.

The design lifecycle states are:

~~~text
CANDIDATE
VALIDATED
SELECTED
REJECTED
SUPERSEDED
~~~

These MUST NOT be encoded as OBSERVED/DECLARED/etc.

Evidence supporting the design retains its own EpistemicStatus.

## Required SelectedDesign fields

A canonical SelectedDesign MUST contain at least:

- design identity;
- design schema/version;
- parent logical Atlas root identity;
- exact scope identity;
- target kind;
- design lifecycle state = SELECTED;
- root semantic identities included by the design;
- capability selections;
- interface selections;
- design-time binding decisions;
- intentionally dynamic binding decisions;
- state model decisions;
- effect requirements;
- ownership/resource requirements;
- concurrency requirements;
- persistence/recovery requirements;
- external boundaries;
- constraints/invariants;
- semantic barriers;
- deployment constraints;
- target/ABI constraints known at design time;
- allowed profile variation;
- required tests/proofs/benchmarks;
- unresolved items explicitly permitted at runtime;
- evidence references;
- selection rationale;
- BlueprintRevisionDecision references when the selection changes an active blueprint;
- supersedes/superseded-by lineage when applicable.

No executable-significant field may be implicit.

## Stable design identity

Design identity MUST NOT be based only on display name.

Conceptually it commits to:

~~~text
design schema
+ parent Atlas root
+ scope
+ target kind
+ selected semantic roots
+ bindings
+ external boundaries
+ required invariants/barriers
+ permitted dynamic decisions
~~~

The exact canonical encoding/hash is versioned.

Two designs with materially different executable semantics MUST have different identities.

## One selected design per coordinate

Within one logical Atlas root, a design coordinate is:

~~~text
(scope, target_kind, declared design variant/profile class)
~~~

At most one non-superseded design may be SELECTED for one exact coordinate.

Multiple variants are allowed only when their distinguishing coordinate is explicit.

Example:

~~~text
server/high-throughput
embedded/low-memory
browser/wasm
~~~

They are different selected design variants, not an ambiguous multi-winner state.

## Root semantic identities

The design explicitly names its semantic roots.

Examples:

- system/application capability root;
- executable module root;
- service root;
- library export root;
- Atlas target root.

AtlasX materialization computes transitive executable closure from these roots.

The materializer MUST NOT decide roots from directory traversal.

## Binding decisions

Every required binding is one of:

~~~text
STATIC_SELECTED
DYNAMIC_PERMITTED
EXTERNAL_BOUNDARY
UNRESOLVED_BLOCKER
~~~

A SELECTED design MUST NOT contain UNRESOLVED_BLOCKER for a materialization-critical binding.

STATIC_SELECTED names the exact target semantic identity.

DYNAMIC_PERMITTED names the interface/capability plus the runtime selection policy.

EXTERNAL_BOUNDARY names the external provider contract/version/ABI constraints.

## Capability/interface selection

If multiple implementations satisfy one capability/interface, the SelectedDesign explicitly records:

- which implementation is selected; or
- that runtime dynamic selection is intentional.

The compiler/materializer MUST NOT choose a provider because it is first, cheapest, local or easiest to compile.

## State model

For state required by the selected executable semantics, the design identifies:

- state identity;
- ownership domain;
- lifecycle;
- required consistency;
- persistence class when applicable;
- concurrency constraints;
- transition invariants;
- external storage boundary if applicable.

Physical memory address/layout is normally compiler work unless the selected design makes it semantically significant.

## Effect model

The design identifies required/allowed externally observable effects and their semantic barriers.

Examples:

- filesystem;
- network;
- process;
- FFI;
- persistence;
- authority-sensitive action;
- device/OS operation;
- provider/model invocation.

A compiler may optimize around an effect only under the compiler barrier/equivalence contracts.

## Ownership/resource model

A SelectedDesign records resource requirements that must survive materialization/compiler lowering.

Examples:

- unique/shared ownership requirement;
- explicit transfer;
- linear resource;
- acquisition/release pairing;
- lifetime/region constraint when design-significant;
- external resource ownership contract.

It does not need to fix target-level register/stack placement.

## Concurrency model

The design records where concurrency is semantic rather than incidental.

Possible requirements:

- single-threaded;
- actor/task ownership;
- parallelizable region;
- required synchronization;
- ordering relation;
- atomicity;
- cancellation;
- backpressure;
- deterministic execution requirement.

Compiler/runtime strategy may vary within those constraints.

## Persistence/recovery model

Where applicable the design records:

- durable state identities;
- transaction boundaries;
- commit/abort requirements;
- ordering/durability guarantees;
- checkpoint/log/recovery invariants;
- external storage boundary.

The compiler/runtime may select a physical mechanism only if permitted by the selected design/blueprint.

## External boundaries

Every retained external technology is explicit.

A boundary specifies:

- capability/interface;
- provider identity class;
- version/protocol/ABI constraints;
- authority/security requirements;
- failure behavior;
- ownership/resource transfer;
- runtime discovery policy;
- census/provenance references.

An external boundary is not Atlas-native ownership.

## Permitted runtime dynamics

Runtime dynamics must be explicitly admitted.

Examples:

- provider selected from approved set;
- plugin selected by typed capability contract;
- user-selected backend;
- environment endpoint supplied under deployment policy.

Each dynamic decision includes:

- decision identity;
- allowed candidates or contract;
- authority/policy;
- failure behavior;
- evidence/lineage.

UNKNOWN is never implicitly converted into permitted runtime dynamics.

## Constraints and semantic barriers

All selected design invariants that constrain implementation MUST be explicit.

At minimum classify barriers such as:

- authority;
- safety;
- effect;
- transaction;
- persistence;
- recovery;
- concurrency/order;
- temporal/lifecycle;
- external interface;
- admission/policy.

The compiler receives these through AtlasX.

## Evidence and selection

A design may become SELECTED only when the validation evidence required by policy exists.

Evidence may include:

- census observations;
- Technology Genomes;
- differential tests;
- prototypes;
- benchmarks;
- proofs/model checking;
- runtime experiments;
- research corroboration.

Model preference alone cannot select a design.

## Blueprint interaction

SelectedDesign and blueprint are related but distinct.

~~~text
Blueprint
= reusable/current architecture and execution strategy

SelectedDesign
= one concrete selected system design under that architecture
~~~

A candidate may fit the existing blueprint.

If selecting it requires changing Atlas's architecture, storage strategy, materialization rules, compiler stages or other blueprint-level design, a BlueprintRevisionDecision under `BLUEPRINT-EVOLUTION.md` is required first or alongside selection according to that contract.

## Supersession

When a new SelectedDesign replaces an old one:

- new design gets new identity;
- old design remains historical;
- supersession lineage is explicit;
- prior AtlasX/products remain traceable to the old design;
- migration/compatibility obligations are recorded;
- affected donor absorption/extinction state is reassessed when required.

Do not mutate an old selected design in place and pretend lineage did not change.

## Materialization gate

ATLAS → ATLASX may begin only when:

- parent Atlas root is SEALED under required policy;
- SelectedDesign state is SELECTED;
- required semantic roots resolve;
- all materialization-critical bindings resolve or are explicitly permitted dynamic/external boundaries;
- required constraints/barriers exist;
- required evidence/admission gates pass;
- no blocking conflict/unknown remains.

The detailed algorithm is `ATLAS-TO-ATLASX.md`.

## Forbidden shortcuts

Forbidden:

- "selected" because only one candidate exists;
- materializer choosing a candidate;
- compiler choosing a candidate;
- implicit default provider;
- converting UNKNOWN to default;
- changing selected design during codegen;
- mutating selected design identity in place after a blueprint revision;
- treating a donor implementation as selected design merely because it was censused.

## Final invariant

A SelectedDesign answers, before materialization:

> Exactly what executable engineering design has Atlas chosen, what remains dynamic/external, which invariants must survive, and which evidence justifies that choice?

If that question cannot be answered from the selected design record and its referenced Atlas semantics, AtlasX materialization is not allowed.
