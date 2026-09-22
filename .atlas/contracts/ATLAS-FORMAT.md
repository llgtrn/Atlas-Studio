---
id: atlas.contract.format.atlas
type: contract
status: active
canonical: true
---
# ATLAS Dense Binary Artifact Contract

A `*.atlas` is the dense canonical engineering/design artifact produced by admitted census, reconciliation, research correlation, invention and design selection.

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
- deployment/compiler/materialization hints;
- provenance/license/evidence roots;
- cross-repository references;
- CensusCertificate and completeness ledger.

It MUST NOT merely be a zip of prose/Markdown/JSON summaries.

It also MUST NOT be treated as "compressed source code." The canonical payload is typed engineering meaning. THIN mode may reference authenticated source externally; FAT mode may embed source/evidence blobs, but those blobs remain evidence rather than substitutes for typed semantics.

## Function-level requirement

Every discovered function is represented. Repetition is compressed semantically using stable IDs, interning, content-addressed records and shared graph structure rather than by deleting meaning.

## Physical encoding

The normative wire-v1 container structure is defined by `ATLAS-BINARY-WIRE-FORMAT.md`. This document defines logical requirements; the wire contract defines headers, section framing, typed record framing, integrity and reader validation.

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

ATLAS retains more knowledge than a single materialization. ATLASX selects one executable design projection. Artifact size is never justification for silent semantic omission.
