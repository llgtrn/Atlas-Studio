---
id: atlas.contract.format.atlas
type: contract
status: active
canonical: true
---
# ATLAS Canonical Semantic Binary Artifact Contract

## Authority

This contract is governed by `ARTIFACT-LAYERING.md`.

A `*.atlas` is the canonical binary representation of exact typed Atlas semantics for its declared scope.

It is not ADL, not source text, not a deployment bundle, and not an AtlasX capsule.

## Purpose

A canonical Atlas artifact answers:

> What exactly is this semantic object/system scope?

The artifact is produced only after intent/research/census/synthesis/validation/selection have converged into a resolved typed semantic world and that world has been logically sealed.

~~~text
ADL / existing reality / generated candidates
        ↓
census + typed elaboration + reconciliation
        ↓
research / decision / synthesis / validation
        ↓
SelectedDesign
        ↓
resolved typed semantic world
        ↓
logical seal
        ↓
deterministic canonicalization + semantic compaction
        ↓
*.atlas binary
~~~

The creation order is governed by `ATLAS-CREATION-PIPELINE.md`.

## Hard role boundary

`*.atlas` is the semantic authority after seal.

It MUST NOT require:

- original ADL prose;
- an LLM interpretation pass;
- provider availability;
- donor repository presence;
- generated source text when typed semantics already carry meaning;
- host-specific defaults.

Human-readable projections are inspection aids only.

## Required knowledge layers

A logical Atlas may contain:

- corpus/repository/global identities and revisions;
- complete scope hierarchy;
- types/functions/methods and required semantic atoms;
- symbols/types/CFG/call/data/state/effect flows;
- nodes/edges/bindings/interfaces/capabilities;
- temporal facts/events;
- constraints/invariants;
- ownership/memory/concurrency semantics;
- source observations and runtime/test evidence;
- donor technology/research claims;
- conflicts/unknowns/hypotheses/gaps;
- candidate/rejected/selected designs;
- selected implementation semantics;
- CandidateChangeSet lineage;
- material DecisionProposal lineage;
- ProviderReceipt lineage;
- ConstraintEnvelope identity;
- security/authority policy;
- dependency identities and relationships;
- deployment/compiler/materialization requirements;
- provenance/license/evidence roots;
- cross-artifact references;
- CensusCertificate and completeness ledger.

The exact required set depends on declared artifact scope/schema.

It MUST NOT merely be a zip of prose, Markdown, JSON, or source files.

## Exact semantics requirement

Every canonical semantic claim MUST be represented by typed records/relations sufficient to recover exact meaning under the declared schema.

Display strings may accompany typed semantics.

Display strings MUST NOT be the only carrier of:

- function identity;
- type identity;
- call edges;
- control/data flow;
- state/effect semantics;
- capability/authority semantics;
- ownership/resource semantics;
- binding state;
- dependency identity;
- obligations.

If a construct cannot yet be represented losslessly, publication policy must mark it explicitly unsupported/unknown or block seal.

## Binary requirement

Canonical publication uses `ATLAS-BINARY-WIRE-FORMAT.md`.

The authoritative direction is:

~~~text
resolved typed Atlas semantics
→ exact semantic interning/factoring/dedup
→ canonical binary records/sections
→ codec compression
→ integrity-bound *.atlas publication
~~~

JSON/YAML/Markdown/source projections are noncanonical unless a future explicit contract changes that rule.

## Logical Atlas and physical units

A logical Atlas may be represented by:

- one canonical root `*.atlas` file; and
- optional immutable content-addressed Atlas shard files when the wire/sharding contracts permit them.

Every physical Atlas unit is binary.

A directory path is not semantic identity.

The canonical root commits to every mandatory shard identity/hash.

Sharding is storage organization, not a different semantic layer.

## Semantic closure

A `*.atlas` MUST be semantically self-describing for its declared scope.

That means a compliant reader can recover the artifact's exact typed semantics without reinterpreting ADL.

Semantic closure does NOT imply whole-system physical closure.

A `*.atlas` may authenticate/reference material that is not embedded, such as:

- source/evidence blobs;
- dependency payload bytes;
- runtime binaries;
- assets;
- model checkpoints;
- build toolchains.

Binding those materials into a selected closed-world system is the role of `*.atlasx`.

## Selected design

A SEALED Atlas intended for system compilation MUST contain or bind an explicit SelectedDesign governed by `SELECTED-DESIGN.md`.

Selection is not inferred from:

- candidate ordering;
- highest provider score;
- only-one-candidate convenience;
- compiler preference;
- file presence.

The selected design identifies executable semantic roots and permitted dynamic/external boundaries.

## Logical completeness before compaction

The logical Atlas is semantically complete before physical compaction begins.

For AI-assisted creation:

~~~text
research
→ alternatives
→ typed decisions
→ generated implementation
→ generated-source census
→ validation
→ authorized selection
→ resolved semantics
→ logical seal
→ mechanical compaction
~~~

A logical Atlas MUST NOT depend on a future provider invocation to discover what selected implementation semantics mean.

## Provider-free post-seal rule

After logical seal:

- no research provider may add truth;
- no decision provider may select a design;
- no synthesis provider may generate missing meaning;
- no coding provider may patch canonical semantics;
- no verification provider may silently rewrite failures into success.

Post-seal compaction/encoding is deterministic mechanical work.

If a missing semantic decision is discovered, return to pre-seal construction and publish a new Atlas identity.

## Canonical contents

A typical artifact may encode:

~~~text
AtlasRoot
├─ Header / schema / Genome
├─ Identity table
├─ Type table
├─ Symbol table
├─ Semantic records
├─ Graph nodes / edges / bindings
├─ Capabilities
├─ Security / authority
├─ Constraints / invariants
├─ Obligations / diagnostics
├─ Dependency semantics
├─ Evidence / provenance
├─ Donor / generated-code lineage
├─ Candidate / decision lineage
├─ SelectedDesign
├─ CensusCertificate
└─ Integrity / content-addressing
~~~

The wire contract determines physical sections.

## THIN and FAT source/evidence modes

THIN Atlas stores canonical semantics plus authenticated external source/evidence references.

FAT Atlas may additionally embed admitted source/evidence blobs.

In both cases:

> source bytes are evidence/reconstruction material; typed semantics remain authority.

THIN/FAT does not change the distinction between Atlas and AtlasX.

Even FAT Atlas does not automatically claim whole-system deployment/build closure.

## Canonical determinism

For a pinned publication profile:

~~~text
same resolved semantic input
+ same Genome/schema set
+ same canonicalization rules
= same canonical semantic identity
~~~

Incidental values MUST NOT perturb semantic identity:

- wall-clock timestamps;
- local absolute paths;
- directory traversal order;
- thread scheduling;
- locale;
- cache location;
- provider prose not admitted as semantic data.

Where physical codec framing is permitted to vary, logical identity MUST be computed from decoded canonical content.

## Immutability and versioning

A SEALED Atlas is immutable in meaning.

Semantic change creates a new artifact identity.

An existing sealed artifact MUST NOT be silently reinterpreted under newer schemas/Genome semantics.

Breaking format/meaning changes require explicit version/compatibility/migration treatment.

## Function-level requirement

Within scopes covered by completeness policy, every discovered function must be accounted for according to the applicable census contract.

Repetition is compressed through stable identity, interning, factoring, shared graph structure, and content addressing — never by deleting required meaning.

## Semantic compaction

Canonical density is governed by `ATLAS-SEMANTIC-COMPACTION.md`.

Allowed examples:

- exact interning;
- exact deduplication;
- DAG sharing;
- graph factoring;
- stable dictionary encoding;
- column/block packing;
- delta encoding;
- content-addressed reuse;
- deterministic codec compression.

Lossy approximation is forbidden for canonical semantic truth.

Approximate search indexes may exist only as regenerable noncanonical accelerators.

## Integrity and publication

The artifact format SHALL support:

- typed binary records;
- bounded section directories;
- stable global IDs;
- content addressing;
- per-section/shard/root hashes;
- Genome/schema/compiler pins;
- bounded/random access;
- transactional publication;
- explicit compatibility negotiation/rejection.

A partially written root MUST never be advertised as SEALED.

## Language convergence

Existing-language census and ADL authoring converge into the same typed semantic world before Atlas publication.

Therefore `*.atlas` is language-independent semantic storage, not a serialized AST of Rust, TypeScript, ADL, or any donor language.

ADL may disappear after compilation when retention policy allows; Atlas meaning must remain intact.

## Relationship to AtlasX

The transition to AtlasX is governed by `ATLAS-TO-ATLASX.md`.

~~~text
*.atlas
= exact semantic truth

*.atlasx
= selected closed-world binary system capsule
~~~

AtlasX may contain one or more Atlas artifacts plus dependency/runtime/resource/reproduction closure.

AtlasX is not decompressed Atlas and not merely "bigger Atlas".

If system closure is not needed, Atlas remains the canonical semantic artifact.

## Provider independence

A sealed Atlas artifact must remain meaningful if every external AI/research/decision/synthesis provider used during creation becomes unavailable.

Provider receipts may remain as lineage.

Provider availability is never required to decode, validate, inspect, or compile already-sealed semantics.

## Source extinction interaction

Donor/source bytes may be physically deleted only under the applicable extinction policy.

Before deletion, Atlas must retain all required:

- semantics;
- provenance;
- license/obligation lineage;
- evidence identities;
- compatibility knowledge.

If donor bytes remain required for build/runtime/reproducibility/legal obligations, they must also be retained/pinned by AtlasX or another explicitly governed artifact boundary.

## Blueprint evolution

Storage/layout/compaction may evolve under `BLUEPRINT-EVOLUTION.md` when census/benchmark/proof evidence demonstrates a better mechanism.

No blueprint revision may silently reinterpret an existing sealed artifact.

## Final invariant

A canonical `*.atlas` is a deterministic binary semantic artifact that remains exact and intelligible without ADL, without donor checkout, and without model interpretation.

It tells Atlas what the declared system scope **means**.

It does not by itself promise that every byte needed to reproduce/deploy that system is physically closed; that promise belongs to `*.atlasx`.
