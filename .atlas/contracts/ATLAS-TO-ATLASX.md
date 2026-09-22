---
id: atlas.contract.atlas-to-atlasx
type: contract
status: active
canonical: true
---
# ATLAS → ATLASX Materialization Contract

## Purpose

This contract defines the deterministic transformation from one SEALED logical `*.atlas` root and one explicitly selected design into one canonical `*.atlasx/` executable engineering representation.

The transformation is not decompression.

It is:

~~~text
selection closure
+ required binding resolution
+ deterministic expansion
+ executable obligation checking
+ profile capture
+ lineage preservation
= ATLASX materialization
~~~

Materialization MUST NOT invent new design semantics.

## Roles

~~~text
*.atlas
= complete canonical engineering knowledge for the admitted scope
  including observations, conflicts, alternatives, rejected candidates,
  evidence, unknowns, Technology Genomes and selected design

SelectedDesign
= explicit authoritative design choice inside that Atlas root

*.atlasx/
= deterministic expanded executable representation of that SelectedDesign
  plus the exact semantic/evidence lineage required to compile and verify it
~~~

ATLASX is not a second truth database.

## Required inputs

A materialization invocation MUST identify:

- exact SEALED logical Atlas root identity/hash;
- exact Genome identity/hash;
- exact semantic schema set;
- exact SelectedDesign identity;
- exact materializer identity/version;
- materialization contract/schema version;
- requested scope;
- target kind;
- DeploymentProfile identity if required;
- HardwareProfile identity if required;
- WorkloadProfile identity if required;
- admitted external provider/capability bindings;
- explicit feature/configuration inputs that affect executable semantics.

No input affecting materialized semantics may be ambient/implicit.

Environment variables, filesystem state, network responses or host toolchain discovery MUST NOT silently change canonical ATLASX output.

If an external fact affects materialization, it must enter through an explicit pinned profile/binding/evidence input.

## Precondition gate

Before materialization:

1. verify Atlas wire/root integrity;
2. verify Genome/schema compatibility;
3. verify required shards are present/resolvable;
4. verify CensusCertificate/seal policy required by the selected scope;
5. locate exactly one SelectedDesign identity;
6. verify the SelectedDesign refers only to identities reachable from the pinned Atlas root or explicit admitted external boundaries;
7. verify materialization-critical obligations are closed;
8. reject contradictory selected-design state.

Materialization MUST fail closed on required integrity/schema/selection errors.

## Selection closure

The materializer computes a deterministic transitive closure from the SelectedDesign.

The closure includes all semantic objects required for executable meaning, including where applicable:

- selected modules/components;
- types;
- functions/methods;
- function signatures/bodies;
- call relationships;
- control-flow/data-flow records;
- state definitions/transitions;
- effects;
- interfaces;
- capabilities;
- bindings;
- constraints/invariants;
- ownership/resource semantics;
- concurrency semantics;
- transaction/persistence/recovery semantics;
- deployment-relevant resources;
- target/provider boundaries;
- required tests;
- semantic barriers;
- evidence/provenance lineage required for verification;
- required external dependency/binding declarations.

The closure MUST follow stable semantic identities, never display names.

## What does not become executable authority

The following MAY remain referenced in lineage but do not become active executable design unless the SelectedDesign explicitly includes them:

- rejected designs;
- superseded designs;
- unrelated candidate designs;
- unrelated donor observations;
- unrelated research claims;
- reference-only donor implementations;
- nonselected alternative bindings.

Their omission from active ATLASX does not erase them from the parent Atlas root.

## Selected versus unresolved facts

ATLASX may carry unresolved/non-executable informational facts, but executable-critical ambiguity MUST be handled explicitly.

For every materialization-critical obligation:

~~~text
resolved selected value/binding
OR explicit permitted runtime dynamic binding
OR materialization failure
~~~

Examples that generally block deterministic materialization unless the SelectedDesign explicitly permits runtime resolution:

- conflicting ownership model;
- unknown required function body;
- unresolved mandatory capability provider;
- unknown state layout requirement;
- unresolved ABI boundary;
- contradictory persistence semantics.

UNKNOWN MUST NOT silently become a default.

CONFLICT MUST NOT silently choose a winner.

## Materialization stages

Canonical stage order:

~~~text
M0 Verify parent Atlas + Genome + schemas
M1 Resolve SelectedDesign identity
M2 Compute semantic selection closure
M3 Validate required obligations/barriers
M4 Resolve explicit design-time bindings
M5 Expand reusable/template semantic structures
M6 Freeze requested target/profile descriptors
M7 Build canonical AtlasX module/interface/runtime/test views
M8 Compute file/object content hashes
M9 Build canonical AtlasX manifest
M10 Compute AtlasX root identity
M11 Validate round-trip lineage and deterministic reproduction
M12 Publish transactionally
~~~

An implementation may optimize internal execution but observable results MUST be equivalent to this logical order.

## Binding resolution boundary

Materialization resolves only bindings whose resolution is part of SelectedDesign semantics.

Examples:

- selected concrete implementation of a capability;
- selected storage provider abstraction;
- selected local versus external service boundary;
- selected runtime component;
- selected ABI adapter;
- selected UI/runtime module connection.

Bindings intentionally left dynamic by the SelectedDesign remain typed dynamic bindings in ATLASX.

Materialization MUST NOT arbitrarily devirtualize or specialize merely for performance. Performance-driven specialization belongs to compiler optimization unless the SelectedDesign itself encodes the choice.

## Template/generic expansion boundary

Materialization MAY expand declarative/template structures when expansion is deterministic and semantically required to make the executable representation explicit.

It MUST preserve lineage from expanded objects to the originating Atlas semantic identities.

Compiler-specific monomorphization may remain later if the representation and target policy assign it to HIR/MIR lowering.

The boundary MUST be documented per construct; an implementation may not duplicate expansion independently in both materializer and compiler.

## Profile handling

ATLASX records exact profile identities used for the materialization request.

Profiles include:

- deployment;
- hardware;
- workload;
- environment/capability policy where canonicalized.

The profile may:

- select an already-admitted SelectedDesign variant;
- bind explicit deployment resources;
- set declared compiler constraints/objectives.

The profile MUST NOT silently rewrite semantic invariants.

Performance optimization based on a profile is primarily compiler work.

If a profile selects materially different semantics, that choice must be explicit in SelectedDesign or a typed design variant before materialization.

## External boundaries

An EXTERNAL_BOUNDARY remains explicit in ATLASX.

Required record content includes:

- provider capability identity;
- interface identity;
- version/protocol/ABI constraints;
- authority/security requirements where applicable;
- runtime discovery policy if dynamic;
- failure semantics;
- evidence/provenance;
- whether provider source is part of Atlas-controlled census.

Wrapping an external dependency does not make it Atlas-native.

## Canonical ATLASX object model

The canonical AtlasX root contains at least these logical classes when applicable:

~~~text
AtlasXRoot
├─ Manifest
├─ Modules
├─ Types
├─ Functions
├─ Interfaces
├─ Capabilities
├─ Bindings
├─ State
├─ Effects
├─ OwnershipResources
├─ Concurrency
├─ PersistenceRecovery
├─ ConstraintsInvariants
├─ Tests
├─ Profiles
├─ Targets
├─ ExternalBoundaries
└─ LineageEvidence
~~~

The directory projection in `ATLASX-FORMAT.md` is a physical organization over this object model.

## Manifest requirements

The canonical AtlasX manifest MUST contain:

- atlasx schema/version;
- AtlasX root identity;
- parent Atlas root identity/hash;
- parent Genome identity/hash;
- SelectedDesign identity;
- requested materialization scope;
- target kind;
- materializer identity/version;
- exact profile identities/hashes;
- exact external binding identities;
- canonical object/file inventory;
- per-object/file content hashes;
- required compiler contract version;
- required IR pipeline version;
- unresolved runtime-dynamic obligations permitted by design;
- semantic barrier set;
- lineage root;
- publication timestamp only if excluded from semantic/root identity;
- compatibility requirements.

Fields affecting semantic identity MUST be included in canonical root hashing.

Incidental timestamps/paths MUST NOT perturb semantic identity.

## AtlasX root identity

The AtlasX root identity is derived from canonical manifest semantics and canonical content identities.

Conceptually:

~~~text
AtlasXRootId =
  H(
    materialization_schema
    parent_atlas_root
    genome
    selected_design
    requested_scope
    target_kind
    profile_identities
    external_binding_identities
    ordered canonical content hashes
    semantic barrier set
  )
~~~

The exact digest/serialization is versioned by the materialization schema.

Local output directory path is not AtlasX identity.

## Directory projection

Canonical default projection:

~~~text
<system>.atlasx/
├─ manifest.atlasx
├─ graph/
├─ modules/
├─ interfaces/
├─ runtime/
├─ ui/
├─ tests/
├─ profiles/
└─ targets/
~~~

Additional Genome-registered directories MAY exist.

Directory names are organization, not semantic authority.

Canonical objects are identified by stable identities and content hashes.

Human-readable debug projections MAY accompany canonical objects but are noncanonical.

## Canonical file encoding

Canonical AtlasX semantic files MUST use a deterministic typed encoding.

JSON/Markdown/YAML source-like documents MAY be debug/export views but MUST NOT become semantic authority unless a future explicit contract changes this rule.

The exact canonical file encoding may evolve under `BLUEPRINT-EVOLUTION.md`, but:

- stable semantic identity must survive;
- reader compatibility/migration must be explicit;
- deterministic root hashing must remain defined.

## No invention during materialization

Materialization MUST NOT:

- invent a missing function;
- choose among conflicting candidate designs;
- infer a provider because it seems likely;
- silently add a dependency;
- invent state placement;
- change ownership semantics;
- add a performance optimization not selected by design/contract;
- use model output to fill an unresolved executable fact.

If executable meaning is missing, materialization fails or preserves an explicitly permitted dynamic boundary.

## No semantic optimization during materialization

Materialization may perform representation normalization and deterministic expansion.

It MUST NOT perform semantic optimizations such as:

- call devirtualization for performance;
- DCE based on target profile;
- inlining;
- CSE;
- loop optimization;
- allocation elimination;
- state relocation;
- concurrency rescheduling;
- data-layout optimization;
- ABI-specific lowering.

Those belong to the compiler pipeline unless the SelectedDesign explicitly changes semantics before materialization.

This separation prevents materializer and compiler from becoming competing optimizers.

## Lineage

Every materialized semantic object MUST retain lineage sufficient to trace back to:

- parent Atlas root;
- SelectedDesign;
- originating semantic record(s);
- originating declaration/observation where required;
- blueprint revision decision where relevant;
- external binding/provider evidence where relevant.

Generated AtlasX IDs may differ from Atlas semantic IDs when representing an expanded/selected executable object, but the mapping MUST be explicit and deterministic.

## Partial materialization

AtlasX may materialize a requested scope smaller than the full SelectedDesign only when:

- the scope is explicitly named;
- external references are represented as typed boundaries;
- omitted required dependencies are not silently treated as absent;
- root identity includes the materialized scope;
- compiler/product claims remain scoped accordingly.

Partial materialization is not evidence that the parent Atlas has fewer semantics.

## Incremental materialization

Unchanged canonical AtlasX objects MAY be reused when:

- parent semantic dependencies are unchanged;
- profile/binding inputs are unchanged;
- materialization schema is unchanged;
- lineage remains valid.

Cache keys MUST include all semantic inputs that can affect the object.

Cache reuse MUST NOT bypass validation.

## Determinism

For identical canonical inputs:

~~~text
same parent Atlas root
same SelectedDesign
same scope
same profiles
same external bindings
same materializer schema/version
=
same AtlasX semantic root identity
~~~

Host path, thread scheduling, hash-map iteration, locale, current time or network ordering MUST NOT affect canonical output.

## Validation

An AtlasX validator MUST verify at least:

- manifest schema/version;
- parent Atlas root identity;
- Genome compatibility;
- SelectedDesign lineage;
- object inventory/content hashes;
- identity uniqueness;
- all required references resolve;
- permitted dynamic boundaries are typed;
- no unresolved required obligation is hidden;
- semantic barriers are present;
- requested profiles/bindings match manifest;
- deterministic ordering constraints;
- target/compiler contract compatibility.

## Compiler handoff

The compiler accepts a validated AtlasX root, not arbitrary files from the directory.

Compiler input identity is:

~~~text
validated AtlasX root
+ exact compiler version/contracts
+ exact target/deployment/hardware/workload profiles
+ admitted toolchain/backend identities
~~~

The compiler MUST NOT silently reinterpret AtlasX semantics.

The next normative stage is defined by `COMPILER-IR-PIPELINE.md`.

## Differential proof

Before declaring the materializer mature, Atlas must prove on reference corpora that:

- two independent runs produce the same root identity;
- object/reference closure is complete;
- selected design is preserved;
- rejected/nonselected designs do not become active accidentally;
- UNKNOWN/CONFLICT blocking rules work;
- permitted dynamic boundaries remain explicit;
- parent lineage is complete;
- materialized behavior matches selected design under differential tests where executable.

## Blueprint evolution

The materialization strategy itself may evolve when census finds a better mechanism.

Examples:

- superior module factoring;
- better deterministic expansion;
- better incremental object reuse;
- better typed canonical encoding.

Changes follow `BLUEPRINT-EVOLUTION.md`.

A better mechanism may revise the blueprint. It may not silently change existing AtlasX semantics or identity.

## Forbidden shortcuts

Forbidden:

- `*.atlas → generate source directly` while bypassing AtlasX when AtlasX is required by the active compiler phase;
- arbitrary directory traversal treated as compiler input;
- materializer inference promoted to selected design;
- unresolved required binding defaulting;
- rejected design leaking into active AtlasX;
- profile/environment facts not recorded in the root;
- human-readable projection becoming canonical authority;
- compiler silently repairing invalid AtlasX.

## Final invariant

ATLAS contains more engineering knowledge than one executable design.

ATLASX is one exact deterministic executable projection.

The only valid transformation is:

~~~text
SEALED Atlas
+ explicit SelectedDesign
+ explicit scope/profiles/bindings
→ validate
→ compute selected semantic closure
→ resolve only design-authorized bindings
→ deterministic expansion
→ preserve lineage/barriers
→ canonical AtlasX root
~~~

No hidden invention is permitted between the two.
