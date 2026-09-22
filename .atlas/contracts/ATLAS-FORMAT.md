---
id: atlas.contract.format.atlas
type: contract
status: active
canonical: true
---
# ATLAS Dense Binary Artifact Contract

A `*.atlas` is the dense canonical engineering/design artifact produced by admitted census, reconciliation, research correlation, invention, candidate implementation synthesis, generated-code census, validation and design selection.

The creation order is governed by `ATLAS-CREATION-PIPELINE.md`.

Atlas aims for lossless engineering meaning, not small summaries. A logical Atlas may be much larger than source even after compression.

## Required knowledge layers

A logical Atlas may contain:

- corpus/repository/global identities and revisions;
- complete scope hierarchy;
- every discovered type/function/method and required blocks/semantic atoms;
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
- constraint-envelope identity;
- deployment/compiler/materialization hints;
- provenance/license/evidence roots;
- cross-repository references;
- CensusCertificate and completeness ledger.

It MUST NOT merely be a zip of prose/Markdown/JSON summaries.

It also MUST NOT be treated as "compressed source code." The canonical payload is typed engineering meaning. THIN mode may reference authenticated source externally; FAT mode may embed source/evidence blobs, but those blobs remain evidence rather than substitutes for typed semantics.

## Function-level requirement

Every discovered function is represented. Repetition is compressed semantically using stable IDs, interning, content-addressed records and shared graph structure rather than by deleting meaning.

## Logical completeness before compaction

The logical Atlas is semantically complete before the physical compactor starts.

For AI-assisted creation this means, where applicable:

~~~text
research
→ alternatives
→ typed decision
→ generated implementation
→ generated-source census
→ validation
→ SelectedDesign
→ logical seal
→ compaction
~~~

A logical Atlas MUST NOT depend on a future provider invocation to discover what the selected implementation means.

Provider-generated source may be embedded/referenced according to THIN/FAT policy, but selected implementation semantics and their lineage must already be canonical.

## Semantic compaction before physical encoding

Canonical density is governed by `ATLAS-SEMANTIC-COMPACTION.md`.

The required layering is:

```text
logical typed Atlas
→ semantic interning/factoring/exact-dedup
→ graph/column/block packing
→ binary physical encoding
→ per-section/shard codec compression
→ content-addressed publication
```

Lossy approximation is forbidden for canonical semantic truth. Approximate/quantized search accelerators are allowed only as explicitly noncanonical, regenerable indexes.

The canonical post-seal compaction run MUST NOT invoke research, decision, synthesis, coding or verification models. It is a deterministic/mechanical transform over the already-sealed logical Atlas.

## Physical encoding

The normative wire-v1 container structure is defined by `ATLAS-BINARY-WIRE-FORMAT.md`. This document defines logical requirements; the compaction contract defines lossless density rules; the wire contract defines headers, section framing, typed record framing, integrity and reader validation.

The format SHALL support:

- typed binary records;
- section/chunk directories;
- dictionary/string/type/symbol interning;
- stable global IDs and content addressing;
- deduplicated DAG structures;
- compact graph adjacency encoding;
- delta/varint/bit packing where useful;
- semantic revision deltas;
- strong compression;
- per-chunk/shard/root integrity hashes;
- Genome/schema/compiler/version pins;
- bounded/random access;
- transactional publication;
- explicit compatibility negotiation or rejection.

## Logical versus physical Atlas

One logical Atlas MAY be represented by one file or multiple immutable content-addressed shards. A root manifest commits to the required shard identities/hashes. Shards may live across repositories/artifact stores/object stores without changing semantic identities.

Cross-shard references use global IDs. Unchanged shards may be reused across revisions and, when ownership/policy permits, across logical Atlases.

## Portable modes

THIN Atlas stores semantics plus authenticated source/evidence references. FAT Atlas may additionally embed compressed admitted source/evidence blobs. Either may be sharded.

## Language convergence

Existing-language census and Atlas Development Language compilation converge into the same typed semantic world before ATLAS publication. ATLAS is therefore language-independent semantic storage, not a serialized AST of Rust, TypeScript or ADL.

See `ATLAS-DEVELOPMENT-LANGUAGE.md` and `ADL-TO-ATLAS.md`.

## Canonical role

ATLAS retains more knowledge than a single materialization.

The transition from SEALED Atlas to executable AtlasX is governed by `ATLAS-TO-ATLASX.md`.

ATLASX is not decompressed Atlas. It is one deterministic executable projection selected from the richer Atlas world through explicit SelectedDesign identity, scope, profiles, bindings, lineage and materialization validation.

Artifact size is never justification for silent semantic omission.

## Provider independence

A sealed Atlas artifact must remain meaningful if every external AI/research/decision provider used during creation becomes unavailable.

Provider receipts/evidence may remain as lineage. Provider availability is not required to decode, validate or compile the already-selected canonical semantics.

## Blueprint evolution

The logical/physical design described here is canonical for the current evidence state, not permanently frozen.

If donor/dependency census demonstrates a materially better compaction, storage, identity, materialization or access mechanism, Atlas MAY revise the blueprint through `BLUEPRINT-EVOLUTION.md`.

A revision MUST preserve higher-level contracts or explicitly revise them with migration/compatibility evidence. No implementation may silently reinterpret existing sealed artifacts.
