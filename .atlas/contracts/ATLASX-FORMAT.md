---
id: atlas.contract.format.atlasx
type: contract
status: active
canonical: true
---
# ATLASX Expanded Executable Representation Contract

`<system>.atlasx/` is the deterministic expanded executable representation of one explicitly selected design from one pinned SEALED logical Atlas root.

ATLASX is executable engineering representation, not expanded prose and not a second semantic universe.

The selected design model is governed by `SELECTED-DESIGN.md`.

The materialization algorithm is governed by `ATLAS-TO-ATLASX.md`.

The canonical v1 binary object/manifest encoding is governed by `ATLASX-BINARY-WIRE-FORMAT.md`.

The compiler handoff is governed by `COMPILER-IR-PIPELINE.md`.

## Role

~~~text
*.atlas
= full engineering world
  observations + conflicts + alternatives + evidence + selected design

*.atlasx/
= one deterministic executable projection
  of one SelectedDesign
  with explicit scope/profiles/bindings/lineage
~~~

ATLASX MUST NOT contain an independently invented architecture.

## Required canonical root

Every AtlasX tree MUST contain exactly one canonical root manifest:

~~~text
<system>.atlasx/
└─ manifest.atlasx
~~~

The manifest is authoritative for the AtlasX object/file inventory.

Arbitrary files present in the directory but absent from the manifest are not canonical compiler input.

## Required manifest fields

The canonical manifest MUST identify:

- AtlasX schema/version;
- AtlasX root identity/hash;
- parent Atlas root identity/hash;
- parent Genome identity/hash;
- SelectedDesign identity;
- requested materialization scope;
- target kind;
- materializer identity/version;
- exact deployment/hardware/workload profile identities when present;
- exact admitted external binding identities;
- canonical object/file inventory;
- per-object/file content hashes;
- required compiler-pipeline contract/version;
- required target/ABI contract versions;
- semantic barrier set;
- explicitly permitted runtime-dynamic obligations;
- lineage root;
- compatibility requirements.

Values affecting executable semantics MUST participate in AtlasX root identity.

Local output path, wall-clock time and incidental build-host data MUST NOT affect semantic root identity.

## Canonical logical object classes

ATLASX supports these canonical classes when applicable:

- Module;
- Type;
- Function;
- FunctionBody;
- Interface;
- Capability;
- Binding;
- State;
- Effect;
- OwnershipResource;
- Concurrency;
- PersistenceRecovery;
- ConstraintInvariant;
- ExternalBoundary;
- Test;
- Profile;
- Target;
- LineageEvidence;
- GraphView.

A target profile may add typed domain-specific classes through Genome-registered schemas.

An extension may extend this set. It may not redefine a core class with different meaning.

## Default physical projection

The default canonical projection is:

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
│  ├─ deployment.atlasx
│  ├─ hardware.atlasx
│  └─ workload.atlasx
└─ targets/
~~~

Directory placement is not semantic identity.

Objects are identified by stable semantic identity and content hash.

A future blueprint MAY change directory/layout strategy under `BLUEPRINT-EVOLUTION.md` without changing logical meaning.

## Canonical encoding

Canonical AtlasX semantic objects MUST use the deterministic typed encoding defined by `ATLASX-BINARY-WIRE-FORMAT.md` for v1.

Human-readable JSON/YAML/Markdown/source projections MAY exist for debugging, review or interoperability but are noncanonical unless a future explicit contract revision changes this rule.

A compiler consumes validated canonical AtlasX objects referenced by `manifest.atlasx`, not arbitrary human-readable files.

## Deterministic object identity

Each AtlasX object identity derives from:

- parent semantic lineage;
- object kind;
- selected-design coordinate;
- deterministic materialization coordinate when one parent object expands into several executable objects;
- schema/version.

Names are display metadata, not sufficient identity.

## Object closure

Every canonical AtlasX reference MUST resolve to:

- another canonical AtlasX object in the same root;
- a typed external boundary;
- an explicitly permitted runtime-dynamic binding.

Broken references invalidate the AtlasX root.

## Function representation

An executable function object MUST preserve or reference:

- stable function identity;
- signature;
- body/CFG semantics where available/required;
- call relationships relevant to executable lowering;
- type dependencies;
- state/effect semantics;
- ownership/resource semantics;
- concurrency semantics where relevant;
- persistence/recovery semantics where relevant;
- failure behavior;
- external boundaries;
- constraints/barriers;
- parent Atlas lineage.

Display source text is optional and nonauthoritative.

## Type representation

A canonical type object MUST preserve enough information for deterministic compiler lowering, including where applicable:

- identity;
- structure;
- generic parameters;
- variants/fields;
- ownership/resource semantics;
- representation/layout constraints explicitly selected by design;
- ABI/interface requirements;
- parent Atlas lineage.

Physical target layout need not be fixed in AtlasX unless it is part of selected design semantics. Target-specific layout is normally compiler LIR work.

## Binding representation

Bindings MUST remain first-class.

A binding object identifies:

- binding identity;
- source capability/interface;
- selected target/provider when design-time resolved;
- dynamic-binding policy when intentionally deferred;
- authority/security constraints;
- lifecycle/temporal requirements;
- failure semantics;
- evidence/lineage.

A dynamic binding MUST NOT be serialized as if it were statically resolved.

## State/effect representation

State and effects remain typed, distinct concepts.

State objects/relations identify:

- state identity;
- legal transitions;
- initialization/lifecycle constraints;
- persistence policy where selected;
- ownership/concurrency constraints.

Effect objects identify:

- effect kind;
- subject/function;
- ordering requirements;
- external boundary if any;
- authority/transaction barriers;
- lineage.

## Ownership/resource representation

ATLASX carries ownership/resource semantics required for compilation.

This may include:

- move/copy/borrow requirements;
- resource acquisition/release;
- ownership transfer;
- region/lifetime constraints when part of selected semantics;
- external resource boundaries.

Compiler optimization may refine physical lifetime/placement but MUST preserve these requirements.

## Concurrency representation

Concurrency objects may include:

- tasks/threads;
- synchronization;
- channels;
- locks;
- atomics;
- ordering requirements;
- shared-state relationships;
- cancellation/failure semantics.

Compiler lowering may specialize the implementation, not remove required ordering.

## Persistence/recovery representation

Persistence/recovery semantics may include:

- durable state;
- transaction boundaries;
- commit/abort;
- checkpoints/logs;
- recovery requirements;
- durability ordering;
- external storage boundary.

Compiler/runtime lowering MUST preserve selected durability semantics.

## Constraint and semantic-barrier representation

Constraints/invariants that affect executable legality MUST survive into AtlasX.

Materialization MUST emit typed semantic barriers for at least:

- authority;
- safety;
- effect;
- transaction;
- persistence;
- recovery;
- concurrency/order;
- temporal/lifecycle;
- external interface;
- provider/model admission where applicable.

Generic unstructured strings are not sufficient for mature canonical barriers.

## Tests

Tests selected as part of executable verification MAY be canonical AtlasX objects.

Each canonical test identifies:

- test identity;
- target scope;
- required inputs/environment profile;
- expected semantic property or observable behavior;
- lineage/evidence;
- required compiler/product admission role.

Tests not selected for executable/product verification may remain only in parent Atlas knowledge.

## Profiles

Profiles are typed canonical inputs, not ambient environment descriptions.

A profile MUST have:

- identity/hash;
- schema/version;
- declared constraints/objectives;
- provenance;
- deterministic serialization.

Profile changes that affect executable semantics change the relevant AtlasX/compiler identity.

## Targets

A Target object identifies:

- target kind;
- required runtime model;
- required compiler/backend class;
- external boundaries;
- target-specific semantic requirements;
- target/ABI constraints if already selected.

Detailed physical instruction/ABI lowering is governed by `COMPILER-IR-PIPELINE.md`.

## External boundaries

External boundaries remain explicit.

AtlasX MUST identify:

- provider/capability/interface;
- version/protocol/ABI constraints;
- authority/security requirements;
- runtime binding/discovery policy;
- failure semantics;
- ownership/resource transfer where applicable.

An external dependency remains external. Materialization does not relabel it Atlas-native.

## Lineage and evidence

Every canonical object MUST be traceable to:

- AtlasX root;
- SelectedDesign;
- parent Atlas root;
- originating Atlas semantic records;
- relevant blueprint revision decision when design changed;
- external-provider evidence where applicable.

Lineage may be factored/deduplicated physically, but must remain exactly reconstructable.

## Partial materialization

Partial AtlasX roots are allowed when the requested scope is explicit.

A partial root MUST:

- identify the exact materialized scope;
- represent omitted dependencies as typed external/reference boundaries when required;
- preserve parent Atlas lineage;
- avoid claims of whole-system completeness;
- include all executable dependencies required by the partial scope.

## Dynamic runtime resolution

ATLASX may intentionally preserve runtime-dynamic behavior.

Examples:

- plugin/provider discovery;
- user-selected adapter;
- runtime service endpoint;
- dynamic capability binding.

Such behavior MUST be explicitly typed as dynamic.

UNKNOWN is not dynamic.

UNRESOLVED is not automatically permitted runtime resolution.

## Digital Organism profile

When `target_kind = digital_organism`, AtlasX extends the general executable representation with:

~~~text
organism/
├─ genome/
├─ identity/
├─ species_traits/
├─ organs/
├─ circuits/
├─ body/
├─ brain/
│  ├─ model_definitions/
│  ├─ provider_bindings/
│  ├─ training/
│  ├─ inference/
│  └─ checkpoint_manifests/
├─ world/
├─ memory/
├─ learning/
├─ homeostasis/
├─ metabolism/
├─ capabilities/
├─ authority/
├─ lifecycle/
├─ adapters/
└─ evidence/
~~~

This remains an extension of the same AtlasX object/lineage rules.

It is not a separate compiler universe and not merely a weights directory.

## Compiler input

The compiler accepts a validated AtlasX root plus explicit compiler/target/profile inputs.

The compiler MUST NOT:

- infer missing selected-design semantics;
- accept arbitrary unmanifested directory files as canonical input;
- silently repair broken references;
- silently resolve UNKNOWN/CONFLICT;
- reinterpret a dynamic binding as static;
- use host defaults for semantic target decisions.

The next stage is HIR or an explicitly contracted delegated backend under `COMPILER-IR-PIPELINE.md`.

## Determinism

For identical:

- parent Atlas root;
- SelectedDesign;
- scope;
- target kind;
- profiles;
- external bindings;
- materializer version/schema;

AtlasX MUST have the same canonical semantic root identity.

Directory path, file timestamp, traversal order, thread schedule and host locale MUST NOT affect that identity.

## Validation

An AtlasX validator MUST reject at least:

- manifest/schema incompatibility;
- parent Atlas mismatch;
- Genome mismatch;
- missing required object;
- unresolved required reference;
- duplicate conflicting identity;
- object hash mismatch;
- hidden required obligation;
- invalid dynamic boundary;
- missing required semantic barrier;
- compiler-contract incompatibility.

## Blueprint evolution

The ATLASX blueprint may change when census demonstrates a better executable representation or materialization strategy.

Examples:

- better object factoring;
- better module structure;
- better deterministic expansion;
- better incremental reuse;
- better canonical encoding.

Any change follows `BLUEPRINT-EVOLUTION.md`.

A layout improvement may change physical organization while preserving logical object meaning.

A semantic/identity change requires explicit schema/contract migration.

## Final invariant

ATLASX is one validated deterministic executable projection of a selected Atlas design.

It is neither:

- arbitrary generated source;
- prose;
- a donor repository copy;
- a place for hidden compiler invention;
- an alternate semantic truth system.
