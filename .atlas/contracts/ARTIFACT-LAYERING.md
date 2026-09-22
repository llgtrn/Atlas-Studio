---
id: atlas.contract.artifact-layering
type: contract
status: active
canonical: true
---
# ADL → ATLAS → ATLASX Artifact Layering Contract

## Purpose

This contract fixes the non-overlapping semantic roles of ADL, `*.atlas`, and `*.atlasx`.

These are not three spellings of the same format and not three levels of "more compiled source".

They are three different artifact classes:

~~~text
ADL      = Intent Artifact
*.atlas  = Canonical Semantic Artifact
*.atlasx = Closed-World System Capsule
~~~

The hard boundary is:

~~~text
WHAT SHOULD EXIST
      ↓
     ADL

WHAT EXACTLY EXISTS
      ↓
   *.atlas

EVERYTHING REQUIRED FOR THAT SELECTED SYSTEM TO EXIST,
VERIFY, REPRODUCE, MOVE, AND ENTER COMPILATION
      ↓
   *.atlasx
~~~

Any document, implementation, CLI, serializer, compiler stage, or provider workflow that collapses these roles is non-conforming.

## Normative lifecycle

The canonical creation path is:

~~~text
Human + AI intent
        ↓
ADL / collaborative authoring
        ↓
research + OSS/dependency census + candidate mechanisms
        ↓
typed decisions + synthesis + generated-code census
        ↓
validation + explicit selection
        ↓
resolved typed Atlas semantic world
        ↓
logical seal
        ↓
deterministic mechanical canonicalization/compaction
        ↓
*.atlas binary
        ↓
selected-system closure + dependency/runtime/resource closure
        ↓
*.atlasx binary capsule
        ↓
HIR → MIR → LIR → Machine IR / delegated backend
        ↓
product
~~~

No external research, decision, synthesis, or coding provider may invent semantic truth after the logical Atlas seal.

## Layer 1 — ADL: Intent Artifact

ADL is the collaborative authoring language/interface for humans and AI.

ADL answers:

> What should exist, what must remain true, what may vary, and what evidence/criteria should guide construction?

ADL is intentionally allowed to be natural-language-first.

A valid ADL authoring experience may contain:

- natural-language goals;
- requirements;
- constraints;
- preferences;
- non-goals;
- acceptance criteria;
- security/privacy requirements;
- policy boundaries;
- performance objectives;
- degrees of freedom;
- explicit unknowns;
- typed references to Atlas knowledge;
- machine-generated structured annotations;
- provider/research/decision lineage.

ADL MAY be conversational, prose-rich, incrementally edited, partially unresolved, and jointly authored by humans and AI.

ADL MUST NOT be treated as exact canonical execution semantics.

### Natural language boundary

Natural language is an authoring/input surface.

It is not the final semantic carrier.

~~~text
natural-language intent
→ semantic extraction/elaboration
→ typed constraints/declarations/decisions
→ validation/reconciliation
→ resolved semantic world
~~~

Ambiguity MUST remain explicit until resolved.

A provider may not convert ambiguity into hidden defaults merely to make compilation continue.

### ADL mutability

ADL is mutable authoring state.

Formatting, comments, conversation structure, prose wording, and provider phrasing MAY change without necessarily changing final semantics.

ADL identity and source lineage may be retained for audit, but ADL bytes are not the semantic identity of the resulting `*.atlas`.

### ADL is not executable authority

ADL MUST NOT directly authorize:

- target opcodes;
- final memory layout;
- resolved dependency closure;
- canonical type/symbol ordinals;
- final CFG/dataflow identities;
- runtime addresses;
- final ABI lowering;
- opaque binary publication without typed resolution;
- hidden provider choice.

ADL may request or constrain these outcomes, but exact meaning must be resolved into typed Atlas semantics first.

### ADL survivability rule

After successful compilation and publication, the original ADL text MAY be deleted when retention policy permits.

Deleting ADL MUST NOT make an already-published `*.atlas` semantically unintelligible.

If the artifact requires original prose to recover its executable meaning, the artifact is invalid.

## Resolution boundary

The boundary between ADL and ATLAS is semantic resolution.

Before canonical `*.atlas` publication, Atlas MUST have enough typed information to answer all publication-critical questions without re-asking a model what the author "probably meant".

The resolved world includes, where applicable:

- identities;
- types;
- symbols;
- function/signature/body semantics;
- control/data/state/effect semantics;
- capabilities;
- ownership/resources;
- concurrency;
- persistence/recovery;
- interfaces/bindings;
- security policy;
- obligations;
- provenance/evidence;
- dependency identities;
- selected design;
- explicit permitted dynamic/external boundaries;
- diagnostics and remaining non-blocking unknowns.

Publication-critical ambiguity blocks the seal.

## Layer 2 — `*.atlas`: Canonical Semantic Artifact

A `*.atlas` artifact is the canonical binary representation of exact Atlas semantics for its declared scope.

It answers:

> What exactly is this semantic object/system scope?

A canonical `*.atlas` is not source text, not ADL, not Markdown, not JSON authority, and not a compressed source archive.

### Binary requirement

Canonical `*.atlas` publication MUST use the versioned Atlas binary wire format.

Human-readable forms produced by:

~~~text
atlas inspect
atlas disasm
atlas explain
atlas export
~~~

are projections.

They MUST NOT become canonical semantic authority.

### Required semantic properties

Before publication, a `*.atlas` MUST be:

- typed;
- resolved to the declared publication scope;
- validated;
- normalized;
- reconciled according to policy;
- canonically ordered;
- integrity-bound;
- Genome/schema/version pinned;
- provenance-bearing;
- independently decodable without model interpretation.

A `*.atlas` MAY retain alternatives, rejected candidates, evidence, research, and historical semantic material when its declared scope/policy requires them.

However, the selected/executable semantics inside the artifact MUST be explicit and must not depend on future provider interpretation.

### Typical logical contents

A `*.atlas` may contain or commit to:

~~~text
AtlasRoot
├─ Header / Version / Genome
├─ Identity tables
├─ Type tables
├─ Symbol tables
├─ Semantic records
├─ Semantic graph
├─ Bindings
├─ Capabilities
├─ Security policy
├─ Constraints / invariants
├─ Obligations
├─ Diagnostics
├─ Evidence / provenance
├─ Donor / dependency lineage
├─ Candidate / decision lineage
├─ SelectedDesign
└─ Integrity / content identities
~~~

### Semantic closure versus system closure

A `*.atlas` MUST be semantically self-describing for its declared scope.

It is NOT required to physically embed every byte needed to deploy or reproduce a whole selected system.

For example, a `*.atlas` may contain authenticated references to:

- source/evidence blobs;
- external dependencies;
- toolchains;
- runtime payloads;
- assets;
- model checkpoints;
- target resources.

That is why `*.atlasx` exists.

### Determinism

For a pinned canonical publication profile:

~~~text
same resolved semantic input
+ same semantic schemas
+ same Genome
+ same canonicalization rules
= same canonical semantic identity
~~~

Incidental values MUST NOT perturb semantic identity, including:

- wall-clock timestamps;
- host-local absolute paths;
- filesystem traversal order;
- thread scheduling;
- locale;
- provider response wording that was not admitted as semantics.

Where codec framing is permitted to vary, semantic/root identity MUST be defined over decoded canonical content.

### Immutability

A SEALED `*.atlas` is immutable in meaning.

Changing canonical semantics creates a new artifact identity.

Existing sealed bytes are never silently reinterpreted under a newer schema.

### Provider independence

Once SEALED, a `*.atlas` MUST remain decodable, verifiable, and semantically meaningful if all external AI providers used during creation disappear.

Provider receipts are lineage, not runtime semantic dependencies.

## Layer 3 — `*.atlasx`: Closed-World System Capsule

A `*.atlasx` is a canonical binary capsule for one selected system closure.

It answers:

> What complete, pinned set of semantic artifacts, dependencies, runtime payloads, resources, policies, evidence, and reproduction metadata is required to move, verify, reproduce, or compile this selected system?

Canonical `*.atlasx` is ONE binary capsule artifact.

The old model in which `<system>.atlasx/` is itself canonical semantic authority is forbidden.

A directory produced by unpacking a capsule is a projection/workspace only.

### Relationship to `*.atlas`

A `*.atlasx` MAY contain one or more `*.atlas` semantic units.

Conceptually:

~~~text
system.atlasx
├─ capsule manifest
├─ root *.atlas
├─ dependent *.atlas units
├─ dependency closure
├─ runtime payloads
├─ target/profile material
├─ assets/resources
├─ supply-chain records
├─ security/signatures/attestations
├─ reproducibility metadata
└─ optional source/debug material
~~~

`*.atlasx` does not replace the semantic authority of embedded `*.atlas` objects.

It binds them into a closed selected-system closure.

### Closed-world requirement

For the declared capsule mode and target, every material requirement MUST be one of:

1. embedded in the capsule;
2. content-addressed and cryptographically pinned under an explicitly permitted fetch policy;
3. represented as an explicit runtime external boundary whose non-embedded nature is part of selected semantics.

A required build/runtime dependency MUST NOT remain an accidental ambient host dependency.

Examples of forbidden ambient assumptions:

- "whatever rustc is on PATH";
- "the system OpenSSL";
- "the latest package from a registry";
- an unpinned model;
- an unversioned system library;
- an undeclared environment variable that changes semantics;
- an arbitrary file outside the capsule closure.

### Capsule classes

A `*.atlasx` may contain:

- root manifest;
- one or more Atlas semantic artifacts;
- dependency graph/closure;
- runtime binaries/WASM/native payloads where selected;
- model/checkpoint payloads where selected;
- assets/static resources;
- exact profile/target declarations;
- external-boundary contracts;
- SBOM;
- license/attribution material;
- provenance/evidence;
- signatures/attestations;
- toolchain identities;
- build/reproduction recipes;
- migration metadata;
- source/debug/source-map material when policy permits.

### Binary requirement

Canonical `*.atlasx` publication MUST use the versioned AtlasX capsule wire format.

The capsule has one canonical root identity.

Internal entries/sections may be independently content-addressed and compressed.

### Unpack rule

~~~text
atlasx unpack system.atlasx
→ system.atlasx.unpacked/
~~~

The unpacked tree is noncanonical.

Editing unpacked files MUST NOT mutate the identity of the original capsule.

Repacking is a new canonicalization operation that must revalidate all identities and closure constraints.

### Operational closure

A valid `*.atlasx` MUST remain meaningful without ADL.

It MUST also remain meaningful without the donor source checkout used during construction unless that source is explicitly part of the capsule's selected reproduction policy.

### Reproducibility versus execution

A capsule may declare profiles such as:

- VERIFY_ONLY;
- BUILD_REPRODUCIBLE;
- EXECUTABLE_PORTABLE;
- DEPLOYMENT_TARGETED.

The profile changes which physical payload classes are mandatory, but never changes the layer definitions.

For example, VERIFY_ONLY may not embed a deployable native binary, while EXECUTABLE_PORTABLE may require it.

The exact profile is part of capsule identity.

## Hard non-overlap rules

### ADL MUST NOT become ATLAS by renaming

A text file containing prose or structured authoring declarations is not a `*.atlas`.

### ATLAS MUST NOT require ADL reinterpretation

A `*.atlas` that requires an LLM to understand what a field "really means" is invalid.

### ATLASX MUST NOT be merely a larger ATLAS

A `*.atlasx` exists to bind a selected semantic world to complete system closure.

If it adds no dependency/runtime/resource/reproduction closure beyond `*.atlas`, it has no architectural reason to exist.

### ATLASX MUST NOT be a canonical directory

Filesystem layout is not capsule identity.

Directory projections are tooling views.

### Compiler MUST NOT treat arbitrary unpacked files as authority

Compiler input begins from a validated `*.atlasx` capsule root plus explicit compiler options.

Unmanifested files are ignored or rejected according to policy.

## Canonical authority table

| Property | ADL | `*.atlas` | `*.atlasx` |
| --- | --- | --- | --- |
| Primary role | intent/constraints | exact semantic truth | selected-system closure |
| Natural language allowed | yes, first-class | only as nonauthoritative evidence/display | only as optional evidence/docs |
| Canonical binary | no requirement | yes | yes |
| Mutable authoring | yes | no after seal | no after seal |
| Exact typed semantics required | partial during authoring | yes | yes via embedded/bound Atlas semantics |
| Must survive without ADL | n/a | yes | yes |
| Must contain deployment/build closure | no | not necessarily | yes according to capsule profile |
| May call external AI to decide meaning | during creation under trust policy | no after seal | no |
| Human-readable view authoritative | authoring authority for intent | no | no |
| Canonical identity content-addressed | optional source identity | yes | yes |

## Command semantics

The intended CLI distinction is:

~~~text
# authoring / construction
atlas build project.adl -o project.atlas

# semantic artifact operations
atlas verify project.atlas
atlas inspect project.atlas
atlas disasm project.atlas
atlas explain project.atlas

# system closure / transport
atlasx pack project.atlas -o project.atlasx
atlasx verify project.atlasx
atlasx inspect project.atlasx
atlasx unpack project.atlasx
atlasx reproduce project.atlasx
~~~

Exact command names may evolve, but their authority boundaries may not silently collapse.

## Extinction and donor implications

Donor/source extinction is evaluated against these artifact layers.

A donor checkout MAY be physically deleted only after Atlas has retained every required semantic, provenance, license, evidence, reproduction, and compatibility obligation in canonical Atlas/AtlasX forms according to policy.

Extinction MUST NOT rely on undocumented memory of the donor repository.

If the selected system requires donor bytes at runtime/build time, those bytes must be admitted into the AtlasX closure or remain an explicit non-extinct external boundary.

## Migration from the old AtlasX directory model

Any document that describes `<system>.atlasx/` as the canonical artifact is superseded by this contract.

The migration rule is:

~~~text
old canonical AtlasX directory tree
→ validate canonical objects + manifest
→ ingest as migration input
→ encode one canonical *.atlasx binary capsule
→ treat any later unpacked directory as noncanonical projection
~~~

Old AtlasX object semantics may survive as internal capsule sections/entries.

Their filesystem placement does not.

## Final invariants

The following statements are normative:

> ADL MUST NOT be treated as canonical execution semantics.

> A canonical `*.atlas` MUST encode exact typed semantics and MUST remain meaningful without reinterpreting ADL.

> A canonical `*.atlasx` MUST be a binary closed-world selected-system capsule, not a canonical directory tree.

> A `*.atlasx` MUST contain or cryptographically close every build/runtime/resource dependency required by its declared capsule profile, except explicitly typed external runtime boundaries.

> No research/decision/synthesis provider may change canonical meaning after the Atlas logical seal.
