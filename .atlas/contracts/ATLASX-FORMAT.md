---
id: atlas.contract.format.atlasx
type: contract
status: active
canonical: true
---
# ATLASX Closed-World System Capsule Contract

## Authority

This contract is subordinate to and must be read with `ARTIFACT-LAYERING.md`.

If any older AtlasX document describes `<system>.atlasx/` as canonical semantic authority, that directory model is superseded.

A canonical `*.atlasx` is one binary system capsule.

## Purpose

A `*.atlasx` binds one explicitly selected Atlas system to the complete material closure required by its declared capsule profile.

It is not a second semantic universe.

It does not replace the semantic authority of `*.atlas`.

It exists to make the selected system movable, independently verifiable, reproducible, buildable, and/or executable without undeclared ambient dependencies.

~~~text
SEALED *.atlas semantic root
        +
SelectedDesign
        +
selected-system closure
        +
dependency/runtime/resource closure
        +
supply-chain/security/reproduction material
        ↓
canonical *.atlasx binary capsule
~~~

## Canonical role

~~~text
*.atlas
= exact canonical semantic truth for a declared scope

*.atlasx
= closed-world capsule binding one selected semantic world
  to everything materially required by the declared capsule profile
~~~

A capsule MAY embed multiple `*.atlas` units.

A capsule MUST NOT invent semantics absent from the parent Atlas world and SelectedDesign.

## Capsule profiles

Every AtlasX capsule MUST declare exactly one primary closure profile:

~~~text
VERIFY_ONLY
BUILD_REPRODUCIBLE
EXECUTABLE_PORTABLE
DEPLOYMENT_TARGETED
~~~

A future Genome-registered profile may extend this set.

### VERIFY_ONLY

Must contain everything required to validate:

- capsule integrity;
- parent Atlas integrity;
- selected design;
- dependency identities;
- obligations;
- security/policy closure;
- provenance/evidence required by verification.

It need not contain a directly runnable target binary.

### BUILD_REPRODUCIBLE

Adds the closure required to reconstruct the selected product under the declared build contract, including exact toolchain identities and all build-significant material.

### EXECUTABLE_PORTABLE

Adds the runtime payloads/resources required to execute on the declared portable runtime/ABI class without undeclared host dependencies.

### DEPLOYMENT_TARGETED

Adds target/deployment-specific payloads, configuration, resource contracts, and attestations required by the declared deployment target.

The selected profile participates in capsule identity.

## Closed-world rule

For each required material dependency, exactly one of these states MUST hold:

~~~text
EMBEDDED
PINNED_FETCH
EXPLICIT_RUNTIME_EXTERNAL_BOUNDARY
~~~

### EMBEDDED

The required bytes are present in the capsule and integrity-bound.

### PINNED_FETCH

The bytes are not physically embedded, but the capsule includes:

- immutable content identity;
- expected size/type;
- authenticated retrieval policy;
- provenance/license metadata;
- offline failure semantics;
- cache/admission policy.

PINNED_FETCH is allowed only when the selected capsule profile and security policy permit it.

A mutable URL or package version range is not a pinned fetch.

### EXPLICIT_RUNTIME_EXTERNAL_BOUNDARY

The dependency is intentionally external at runtime and is part of selected semantics, for example:

- remote service;
- user-selected provider;
- approved plugin boundary;
- deployment endpoint.

The capsule MUST preserve the interface, authority, protocol/version, failure, and discovery policy.

An undeclared host library is not an external boundary.

## Forbidden ambient dependencies

The capsule MUST NOT depend silently on:

- arbitrary PATH toolchains;
- system libraries discovered at runtime;
- latest registry packages;
- floating git branches;
- unpinned model checkpoints;
- undeclared environment variables;
- files outside capsule closure;
- host locale/timezone when semantics depend on them;
- model/provider availability to recover missing semantics.

## Logical capsule model

A canonical capsule contains these logical classes when applicable:

~~~text
AtlasXCapsule
├─ RootManifest
├─ AtlasArtifacts
├─ SelectedSystemClosure
├─ DependencyClosure
├─ RuntimePayloads
├─ BuildInputs
├─ AssetsResources
├─ ProfilesTargets
├─ ExternalBoundaries
├─ SupplyChain
├─ SecurityAttestations
├─ Reproducibility
├─ Migration
└─ OptionalEvidenceSourceDebug
~~~

### RootManifest

The root manifest identifies:

- AtlasX wire/schema version;
- capsule root identity;
- capsule profile;
- parent/root Atlas identity;
- all embedded Atlas artifact identities;
- SelectedDesign identity;
- requested scope;
- target/deployment profile identities;
- dependency closure root;
- entry inventory;
- external boundaries;
- toolchain/build contract where applicable;
- security/signature policy;
- compatibility requirements.

### AtlasArtifacts

Contains one or more canonical Atlas semantic artifacts or canonical internal Atlas sections whose identities exactly match the parent semantic roots declared by the manifest.

### SelectedSystemClosure

Contains the deterministic transitive semantic/material selection from SelectedDesign.

It MUST NOT include unrelated alternatives as executable authority.

Historical/rejected evidence may be carried as optional evidence only.

### DependencyClosure

Contains or pins every build/runtime dependency required by the capsule profile.

The dependency closure MUST include transitive dependencies, not merely direct package names.

### RuntimePayloads

May include:

- native executables;
- shared/static runtime payloads;
- WASM;
- bytecode;
- model/checkpoint files;
- firmware/device payloads;
- generated runtime tables.

Their relation to Atlas semantics and target/profile MUST be explicit.

### BuildInputs

For reproducible-build profiles, may include:

- compiler/toolchain artifacts or pinned identities;
- linker/runtime support;
- generated source when used as delegated backend input;
- build graph;
- deterministic build recipe;
- environment contract;
- required patches.

### AssetsResources

Includes selected system assets/static resources/configuration schemas that affect build/runtime behavior.

### SupplyChain

Includes, where policy requires:

- SBOM;
- licenses;
- attribution;
- donor/dependency provenance;
- source hashes;
- admission records;
- vulnerability/security evidence;
- dependency census lineage.

### SecurityAttestations

May include:

- signatures;
- attestations;
- trust roots;
- policy envelopes;
- sandbox/capability declarations;
- verification receipts.

### Reproducibility

Carries enough pinned information to reproduce the declared capsule profile without guessing ambient inputs.

## Canonical binary requirement

The canonical AtlasX artifact is one binary capsule encoded by `ATLASX-BINARY-WIRE-FORMAT.md`.

The canonical identity is the capsule root identity derived from canonical decoded manifest/entry semantics.

Internal entries MAY be individually compressed, content-addressed, and independently verified.

## No canonical directory tree

A filesystem tree is never the AtlasX semantic root.

The following is a tooling projection only:

~~~text
atlasx unpack system.atlasx
→ system.atlasx.unpacked/
   ├─ manifest/
   ├─ atlas/
   ├─ dependencies/
   ├─ runtime/
   ├─ assets/
   ├─ supply-chain/
   └─ ...
~~~

Files in that tree are convenient inspection/workspace material.

The original capsule remains authoritative.

Editing the unpacked tree does not mutate the original capsule.

Repacking requires complete revalidation and produces a new or identical capsule identity depending on canonical content.

## Entry identity

Every canonical capsule entry MUST have:

- entry class;
- schema/version where applicable;
- canonical content identity/hash;
- encoded and decoded length;
- required/optional flag;
- compression/encryption metadata where permitted;
- lineage/role metadata required by its class.

A local extraction path is not semantic identity.

## Atlas artifact identity

Embedded Atlas artifacts MUST retain their own canonical identities.

AtlasX MUST NOT rewrite an Atlas artifact's semantics merely to package it.

If packaging requires a semantic change, a new Atlas artifact must be created and sealed first.

## Dependency identity

A dependency entry MUST identify enough information to prevent substitution, including where applicable:

- ecosystem/package/module identity;
- exact version/revision;
- content hash;
- source/provenance;
- license/obligations;
- transitive parent relation;
- target/profile applicability;
- build/runtime role.

Version range alone is insufficient for canonical closure.

## External boundaries

An external boundary MUST record:

- capability/interface identity;
- protocol/ABI/schema version constraints;
- authority/security policy;
- runtime discovery policy;
- failure semantics;
- ownership/resource transfer;
- allowed provider set or contract;
- evidence/provenance.

External does not mean untracked.

## Source and ADL

ADL MAY be embedded as optional evidence/reconstruction material.

Source MAY be embedded when policy or reproduction requires it.

Neither ADL nor source text replaces typed Atlas semantics.

A valid capsule MUST remain semantically meaningful if optional ADL prose is removed.

## Compiler handoff

The compiler consumes:

~~~text
validated *.atlasx capsule
+ explicit compiler configuration allowed by contract
→ HIR / delegated backend
~~~

The compiler MUST NOT:

- scan arbitrary sibling files for canonical input;
- infer missing dependencies from the host;
- resolve UNKNOWN by default;
- choose a different SelectedDesign;
- call a model to recover missing semantics;
- silently upgrade dependencies;
- treat unpacked debug files as authority.

## Determinism

For identical:

- parent Atlas semantic identities;
- SelectedDesign;
- requested scope;
- capsule profile;
- dependency closure;
- runtime/build/resource entries;
- target/profile identities;
- external boundaries;
- AtlasX schema/canonicalization rules;

two compliant implementations MUST agree on the AtlasX semantic root identity.

Incidental publication timestamp, host path, extraction path, file mtime, traversal order, or thread scheduling MUST NOT change root identity.

## Validation

A validator MUST reject at least:

- invalid magic/version;
- root hash mismatch;
- parent Atlas identity mismatch;
- Genome/schema incompatibility;
- missing required entry;
- duplicate conflicting entry identity;
- content hash mismatch;
- undeclared dependency;
- floating dependency;
- invalid pinned-fetch policy;
- missing license/obligation material required by policy;
- broken transitive dependency edge;
- hidden host dependency declared by no entry/boundary;
- SelectedDesign mismatch;
- external boundary contract violation;
- compiler-contract incompatibility;
- malformed bounds/decompression abuse.

## Extinction interaction

A donor/dependency checkout may be physically extinguished only when required retained material has moved into:

- canonical Atlas semantics/provenance/evidence; and/or
- canonical AtlasX dependency/runtime/source closure;

according to policy.

AtlasX therefore becomes the physical closure boundary that makes donor-source deletion safe for selected systems when all extinction gates pass.

## Migration from AtlasX directory v1 drafts

Older pre-contract AtlasX directory material is migration input, not canonical authority.

Migration:

~~~text
old <system>.atlasx/
→ validate old manifest/object hashes
→ classify canonical objects
→ bind parent Atlas + SelectedDesign
→ compute dependency/runtime/resource closure
→ encode one *.atlasx capsule
→ verify capsule root
~~~

After migration, the directory is disposable projection/workspace material.

## Final invariant

A canonical AtlasX artifact is one independently verifiable closed-world binary capsule for one selected system closure.

It is not:

- arbitrary generated source;
- a canonical directory;
- a donor repository copy;
- a second truth database;
- a place for hidden compiler decisions;
- a package that relies on undeclared ambient host state.
