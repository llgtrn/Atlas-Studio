---
id: atlas.contract.format.atlas
type: contract
status: active
canonical: true
---
# ATLAS Dense Binary Artifact Contract

A `*.atlas` file is the dense canonical engineering/design artifact produced by admitted census, research correlation and design synthesis.

It is conceptually similar to a large weights/model artifact in packaging and density, but unlike opaque neural weights it is typed, deterministic, evidence-linked and auditable.

## Required content classes

A `*.atlas` artifact may contain:

- repository/global identities and revisions;
- scope hierarchy;
- symbols/types;
- nodes, edges and bindings;
- control/data/state/effect flows;
- temporal facts and events;
- constraints/invariants;
- interfaces/capabilities;
- source observations;
- donor technology primitives;
- research claims and evidence anchors;
- conflicts/unknowns/hypotheses;
- candidate/rejected/selected designs;
- compiler/materialization hints;
- provenance/license/evidence roots;
- cross-repository references;
- target selection state.

It MUST NOT merely be a zip of Markdown/JSON summaries.

## Physical requirements

The format SHALL support:

- binary typed records;
- section/chunk directory;
- dictionary/string interning;
- stable IDs and content addressing;
- deduplicated DAG structures;
- delta/varint/bit packing where suitable;
- strong compression;
- per-chunk and root integrity hashes;
- format, schema, Genome and compiler version pins;
- bounded/random access without full decompression;
- transactional publication;
- forward-compatible version negotiation or explicit rejection.

## Canonical role

`*.atlas` is the durable compressed design world. It can retain more knowledge than any single generated implementation: donor alternatives, research, rejected designs and historical evidence may remain inside while `*.atlasx/` materializes only the selected executable design.

The mapping to ATLASX must preserve all semantics required for the selected design and its proof obligations.
