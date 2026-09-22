---
id: atlas.contract.atlas-to-atlasx
type: contract
status: active
canonical: true
---
# ATLAS → ATLASX Closure and Packaging Contract

## Authority

This contract is governed by:

- `ARTIFACT-LAYERING.md`;
- `ATLAS-FORMAT.md`;
- `SELECTED-DESIGN.md`;
- `ATLASX-FORMAT.md`;
- `ATLASX-BINARY-WIRE-FORMAT.md`.

The canonical output is one binary `*.atlasx` capsule.

The legacy `<system>.atlasx/` directory model is migration/debug projection only.

## Purpose

This contract defines the deterministic transformation from one SEALED Atlas semantic root plus one explicit SelectedDesign into one closed-world AtlasX system capsule.

The transformation is not decompression and not ordinary archiving.

It is a closure operation:

~~~text
selected semantic closure
+ transitive dependency closure
+ runtime/build/resource closure
+ target/profile binding
+ supply-chain/security closure
+ reproducibility closure
+ deterministic packaging
= *.atlasx
~~~

No step may invent new design semantics.

## Inputs

A packaging invocation MUST pin:

- exact SEALED Atlas root identity/hash;
- exact Genome identity/hash;
- exact semantic schema set;
- exact SelectedDesign identity;
- requested semantic/system scope;
- AtlasX capsule profile;
- target kind;
- deployment/hardware/workload profiles where applicable;
- explicit external boundaries;
- exact materializer/packer identity/version;
- AtlasX schema/wire version;
- policy governing EMBEDDED versus PINNED_FETCH;
- security/signature policy;
- reproduction/build policy where applicable.

Any input that changes capsule meaning or closure MUST be explicit.

Ambient host state MUST NOT silently affect canonical output.

## Preconditions

Before closure begins, the materializer MUST verify:

1. Atlas root integrity;
2. Genome/schema compatibility;
3. required Atlas shards/sections;
4. logical seal policy;
5. SelectedDesign existence and state = SELECTED;
6. design roots resolve;
7. materialization-critical semantic obligations are closed;
8. every intentionally dynamic/external boundary is typed;
9. no blocking UNKNOWN/CONFLICT remains;
10. required CensusCertificate policy passes.

Failure is closed.

## Stage model

Canonical logical order:

~~~text
X0  Verify parent Atlas / Genome / schemas / seal
X1  Resolve SelectedDesign
X2  Compute selected semantic closure
X3  Compute direct dependency closure
X4  Recursively compute transitive dependency closure
X5  Classify every dependency as build/runtime/resource/external
X6  Bind target/profile inputs
X7  Resolve runtime/build/assets/model/toolchain material
X8  Enforce legal/security/provenance obligations
X9  Choose EMBEDDED / PINNED_FETCH / EXPLICIT_RUNTIME_EXTERNAL_BOUNDARY
X10 Validate closed-world profile
X11 Construct capsule entries
X12 Compute entry content identities
X13 Build canonical root manifest
X14 Compute AtlasX root identity
X15 Encode AtlasX wire v2
X16 Reread + independently verify
X17 Transactionally publish
~~~

An implementation may parallelize internal work but observable semantics MUST match this order.

## Selected semantic closure

The selected closure starts only from semantic roots named by SelectedDesign.

It includes all semantic objects required to understand and compile the selected system, including where applicable:

- modules/components;
- types;
- functions/methods;
- signatures/bodies;
- control/data/state/effect semantics;
- capabilities/interfaces;
- bindings;
- ownership/resources;
- concurrency;
- persistence/recovery;
- invariants/constraints;
- security/authority barriers;
- target/ABI requirements;
- deployment-relevant resources;
- required tests/proofs;
- permitted dynamic runtime decisions;
- external boundaries;
- lineage required for verification.

Rejected/unrelated candidates do not become executable closure merely because they are present in the parent Atlas.

## Dependency closure is recursive

AtlasX MUST close dependencies transitively.

Direct dependencies are not sufficient.

~~~text
selected system
→ direct dependency set
→ each dependency's dependencies
→ each dependency's build/runtime dependencies
→ target-specific dependency branches
→ fixed point
~~~

Closure ends only when every required dependency is:

- embedded;
- cryptographically pinned under permitted fetch policy; or
- intentionally external by SelectedDesign.

Unknown transitive dependency state blocks profiles that require closure.

## Dependency census requirement

A dependency that affects canonical closure MUST have enough census/admission evidence to establish:

- identity/revision;
- source/provider;
- transitive dependency relationship;
- build/runtime role;
- license/obligations;
- security/admission state;
- selected target/profile relevance;
- expected content identity.

Package metadata alone is not equivalent to full semantic census.

The required census depth follows `DEPENDENCY-CENSUS.md` and capsule policy.

## Generated implementation

Selected AI-generated code does not bypass closure.

If selected implementation was generated externally:

~~~text
CandidateChangeSet
→ untrusted ingestion
→ census
→ validation
→ SelectedDesign
→ Atlas seal
→ AtlasX closure
~~~

AtlasX packages admitted selected results, never raw provider authority.

## Build closure

For BUILD_REPRODUCIBLE and stricter profiles, every build-significant input MUST be closed.

Examples:

- compiler;
- linker;
- code generator;
- build script/interpreter;
- generated source;
- target libraries;
- sysroot;
- runtime support;
- build configuration;
- codegen flags that alter semantics;
- patches;
- schemas consumed at build time.

"Installed on the machine" is not closure.

A system package MAY be an explicit pinned boundary only if the profile contract permits it and its exact identity/ABI/content requirements are declared.

## Runtime closure

For EXECUTABLE_PORTABLE and DEPLOYMENT_TARGETED profiles, every runtime-significant material dependency MUST be closed.

Examples:

- native/shared libraries;
- WASM runtime requirements;
- bytecode runtime;
- model/checkpoint;
- configuration;
- assets;
- device data;
- certificates/trust roots where required;
- runtime schemas;
- migration data.

Dynamic services remain external only when explicitly selected as runtime boundaries.

## Resource closure

Resources affecting selected behavior MUST be captured or pinned.

Examples:

- templates;
- static assets;
- embedded SQL/schema;
- tokenizer/model vocab;
- migrations;
- UI bundles;
- firmware blobs;
- policy tables;
- protocol descriptors.

Documentation that has no runtime/build/verification role need not enter canonical closure.

## External boundary rule

An explicit runtime external boundary is not a failure of closed-world reasoning.

It is a selected fact that the system intentionally depends on an external capability.

Each boundary MUST declare:

- interface/capability;
- version/protocol/ABI constraints;
- allowed provider class/set;
- authority/security policy;
- discovery/binding policy;
- failure behavior;
- ownership/resource semantics;
- evidence/provenance.

The materializer MUST NOT create a new external boundary merely because embedding a dependency is inconvenient.

## EMBEDDED versus PINNED_FETCH

The packer may choose between EMBEDDED and PINNED_FETCH only within the declared capsule profile/policy.

The choice itself is canonical capsule state.

### EMBEDDED

Preferred when:

- offline verification/build/execute is required;
- supply-chain isolation is required;
- dependency bytes are small/reasonable;
- policy forbids network resolution;
- extinction requires preserved source/runtime bytes.

### PINNED_FETCH

Allowed only with immutable cryptographic identity and explicit resolver policy.

Forbidden forms include:

- latest;
- semver range without content pin;
- floating branch;
- mutable URL;
- provider alias with no version/content identity.

## Legal and attribution closure

Before publication the materializer MUST verify required:

- license texts;
- attribution;
- NOTICE obligations;
- source-offer obligations;
- redistribution restrictions;
- patent/export obligations where policy models them;
- generated-code provenance.

A capsule may not become "closed" by dropping legal obligations.

## Security closure

Security policy may require:

- signature;
- attestation;
- vulnerability evidence;
- capability manifest;
- sandbox contract;
- approved trust roots;
- artifact transparency evidence.

Missing mandatory security material blocks publication.

## Reproducibility closure

For reproducible profiles AtlasX MUST capture enough information to rebuild without guessing.

This includes:

- exact toolchain identities;
- environment contract;
- canonical build graph/recipe;
- target/profile;
- deterministic seeds where relevant;
- all generated inputs;
- all build-significant dependencies.

Wall-clock timestamps and host-local paths are not reproducibility inputs unless explicitly semantically required.

## Atlas embedding

The root Atlas semantic artifact MUST be present or identity-equivalently embedded according to AtlasX wire rules.

Additional Atlas semantic units MAY be included when the selected system closure composes multiple canonical modules/artifacts.

Their identities remain independent.

Packing does not mutate Atlas semantics.

## Source retention

Source may be:

- omitted when semantic/runtime/reproduction policy allows;
- embedded when required for reproducibility, legal obligation, audit, or extinction safety;
- pinned externally under a permitted policy.

ADL is optional evidence.

Neither source nor ADL is semantic authority after Atlas seal.

## Extinction gate interaction

Physical donor-source extinction is permitted only when every required retained role has a canonical home.

Conceptually:

~~~text
donor source knowledge
→ *.atlas semantics/provenance/evidence

donor bytes still required for build/runtime/reproduction/legal duty
→ *.atlasx closure
~~~

If neither artifact contains/pins a required role, extinction is forbidden.

## No AI after seal

AtlasX closure is deterministic/mechanical with respect to canonical meaning.

The materializer MUST NOT call a research, decision, synthesis, or coding model to:

- choose an implementation;
- invent a dependency;
- repair missing semantics;
- infer an ABI;
- choose a license interpretation;
- fabricate a build recipe;
- resolve an UNKNOWN.

If new reasoning/design is required, return to pre-seal Atlas construction and create a new semantic artifact.

## No semantic optimization

AtlasX closure may:

- select already-selected closure;
- deduplicate identical bytes;
- content-address entries;
- compress;
- factor metadata;
- bind explicit target/profile facts;
- package runtime/build resources.

It MUST NOT perform compiler semantic optimization such as:

- inlining;
- DCE;
- speculative devirtualization;
- state model rewriting;
- ownership rewriting;
- concurrency transformation.

Those belong to the compiler unless already fixed by SelectedDesign.

## Determinism

Pinned identical inputs MUST produce the same AtlasX semantic root identity.

Host filesystem order, locale, wall clock, staging path, thread schedule, or cache state MUST NOT change capsule identity.

Codec bytes may vary only where wire/profile rules define identity over decoded canonical content.

## Output

The only canonical output is:

~~~text
<system>.atlasx
~~~

Optional outputs:

~~~text
<system>.atlasx.unpacked/   # noncanonical inspection projection
reports/                   # noncanonical
logs/                      # noncanonical
debug-source/              # noncanonical unless explicitly embedded in capsule
~~~

Tooling MUST label these projections clearly.

## Verification before publish

Before publication Atlas MUST independently verify:

- wire/header bounds;
- root identity;
- embedded Atlas identities;
- SelectedDesign identity;
- entry inventory;
- transitive dependency closure;
- build/runtime/resource closure for selected profile;
- pinned-fetch records;
- external boundaries;
- legal obligations;
- security obligations;
- compiler compatibility;
- deterministic regeneration where policy requires.

Publication occurs only after all required checks pass.

## Forbidden shortcuts

Forbidden:

- renaming a directory to `*.atlasx`;
- treating unpacked files as canonical authority;
- copying direct dependencies but ignoring transitive closure;
- assuming host toolchain/library availability;
- floating package/model versions;
- silently fetching latest;
- materializer selecting a new design;
- provider/model filling missing meaning;
- omitting license/provenance to reduce size;
- replacing exact closure with prose instructions;
- compiler scanning outside the capsule for undeclared inputs.

## Final invariant

ATLAS → ATLASX is the boundary where exact semantic truth becomes a closed selected-system material closure.

`*.atlas` answers what the system means.

`*.atlasx` answers exactly what must travel with or be cryptographically bound to that selected system so it can be verified, reproduced, built, or executed under its declared capsule profile.
