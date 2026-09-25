---
id: atlas.contract.semantic-compaction
type: contract
status: active
canonical: true
---
# ATLAS Semantic Compaction Contract

## Purpose

ATLAS is a dense canonical engineering artifact.

Density MUST come from removing representational redundancy, not deleting engineering meaning.

This contract defines the lossless semantic-compaction layer between the canonical logical Atlas and its physical wire/shard encoding.

~~~text
SEALED logical Atlas semantic world
        ↓
canonical semantic compaction
        ↓
physical record/layout encoding
        ↓
section/shard codec compression
        ↓
content-addressed *.atlas
~~~

Physical codec compression such as ZSTD is the final layer, not the primary semantic compression strategy.

## Core invariant

For canonical Atlas semantics:

~~~text
decode(encode(logical_atlas))
≡ logical_atlas
~~~

and for the same sealed logical Atlas + same canonical compaction profile:

~~~text
semantic compaction decisions are provider-independent and deterministic
~~~

where equivalence means preservation of all canonical engineering meaning required by the active schemas/Genome, including identity, provenance, evidence, epistemic status, obligations, conflicts and unresolved states.

Canonical compaction is lossless with respect to meaning.

## Meaning that may not be approximated

The following MUST NOT be subjected to lossy/approximate canonical encoding:

- stable identities;
- repository/revision identity;
- scope identity;
- graph node/edge/binding identity;
- semantic record identity;
- RawObservationId lineage;
- epistemic status;
- evidence references;
- provenance;
- diagnostics/obligations;
- UNKNOWN;
- UNSUPPORTED;
- CONFLICT;
- constraints/invariants;
- authority/safety semantics;
- dependency identities/edges;
- Technology Genome evidence;
- selected-design lineage;
- extinction evidence;
- compiler semantic barriers.

Approximate equivalence is not canonical equivalence.

## Post-seal mechanical boundary

Canonical compaction begins only after logical Atlas seal.

CandidateAtlas and SealEligibleAtlas are not legal canonical-compaction inputs.

The compactor MUST fail closed if the input does not carry/verifiably reference the required seal identity, SelectedDesign identity, active seal-policy identity and evidence/attestation commitment required by the publication profile.

During a canonical compaction run the following are forbidden:

- research-provider calls;
- web/OSS search;
- decision-provider calls;
- synthesis/code-provider calls;
- semantic invention;
- heuristic deletion of "unimportant" records;
- model-based approximate deduplication;
- model summaries substituted for records.

The compactor receives:

- exact sealed logical Atlas identity;
- exact seal/policy identity;
- exact SelectedDesign identity;
- exact required evidence/attestation root(s);
- exact compaction profile;
- exact schema/wire versions;
- deterministic configuration.

It emits the canonical physical representation without changing selected meaning.

External AI may help design a future compaction algorithm, but only through the ordinary blueprint-revision process before that algorithm becomes the selected deterministic compaction implementation.

## Compaction stages

Canonical semantic compaction MAY use the following stages when deterministic and schema-governed:

1. vocabulary/enum packing;
2. string/path/repository/revision interning;
3. identity factoring;
4. repeated-structure factoring;
5. exact semantic deduplication;
6. evidence/provenance factoring;
7. graph adjacency packing;
8. columnar grouping;
9. sorted integer delta encoding;
10. unsigned/signed varint encoding;
11. bit packing;
12. run-length encoding where semantics permit;
13. shared immutable DAG nodes;
14. content-addressed record/block reuse;
15. shard-level codec compression.

The selected physical strategy is a blueprint and MAY evolve under `BLUEPRINT-EVOLUTION.md` when census/benchmark evidence demonstrates a better mechanism.

Once selected, a compaction profile is executable policy, not an invitation for a provider/model to make per-artifact semantic choices.

## Vocabulary packing

Stable closed enumerations SHOULD use compact numeric codes under versioned schemas.

Examples include:

- EpistemicStatus;
- SemanticDimension;
- fact/record kind;
- edge/binding kind;
- effect kind;
- obligation state;
- compiler barrier kind.

A numeric code is a physical encoding, not the semantic identity itself.

Existing numeric meanings MUST NOT be repurposed within a schema version.

## Interning

Repeated values SHOULD be interned when doing so reduces size/locality cost without changing global identity.

Candidate intern tables include:

- strings;
- repository identities;
- revisions;
- source paths;
- scope prefixes;
- type shapes;
- symbol names;
- extractor/compiler identities;
- evidence source descriptors.

Local table ordinals MUST NOT become cross-shard semantic identities.

## Identity factoring

Repeated identity components SHOULD be factored.

For example, thousands of function records from the same repository/revision/module may reference compact repository/revision/scope table entries rather than repeat full strings/hashes.

Factoring MUST preserve reconstruction of the exact full typed identity.

## Exact semantic deduplication

Atlas may deduplicate representation only when the relevant canonical schema proves two records are the same semantic object/claim.

This must preserve the distinction:

~~~text
RawObservationId
≠ SemanticRecordId
~~~

Multiple independent raw observations MAY reference one semantic claim identity while retaining all extractor/evidence/provenance lineage.

The following are forbidden dedup keys:

- display text;
- function name alone;
- type name alone;
- similar signature;
- approximate graph shape;
- embedding similarity;
- model judgment.

Exact-dedup rules MUST be explicit and deterministic.

## Evidence factoring

Many semantic records may share evidence/provenance structures.

Atlas SHOULD factor shared evidence objects rather than duplicate them.

Factoring MUST preserve:

- exact evidence identity;
- evidence kind;
- provenance;
- source span/location where required;
- revision;
- extractor/compiler source;
- diagnostic/obligation references.

Evidence may be shared physically without becoming less attributable semantically.

## Graph compaction

Graph nodes, edges and bindings remain distinct semantic classes.

Allowed compaction includes:

- sorted adjacency blocks;
- local ordinal tables;
- delta-coded sorted IDs;
- shared attribute dictionaries;
- edge-kind grouping;
- source-node grouping;
- destination block factoring.

Compaction MUST preserve exact reconstructability of:

- node identity;
- edge identity where identity-bearing;
- binding identity;
- direction;
- kind;
- endpoints;
- provenance/evidence;
- temporal/revision context.

Graph packing MUST NOT create graph-only truth absent from normalized/reconciled semantics.

## Columnar organization

High-volume homogeneous record families MAY use columnar physical representation.

Example conceptually:

~~~text
CALL block
caller_id[]
callsite_id[]
target_kind[]
target_ref[]
status[]
evidence_ref[]
~~~

Columnar encoding is allowed only when the logical typed record can be reconstructed exactly.

The logical semantic schema remains authoritative over the column layout.

Measured on the census container (G88): columnar grouping shrinks the census-facts section 5.0x raw, but only about 1.4x once generic codec compression is applied, and it leaves the string table, half the container, untouched. When container size becomes a measured cost, shard-level codec compression (stage 15) is the cheaper first lever. Columnar grouping is justified only by a family the codec cannot already compact.

## Delta/varint/bit packing

Sorted/local numeric references SHOULD use delta/varint/bit packing where beneficial.

Examples:

- local record ordinals;
- sorted adjacency destinations;
- span offsets;
- enum codes;
- boolean flags;
- repeated small cardinalities.

Physical compactness MUST NOT cause integer truncation, sentinel overloading or ambiguous decoding.

## Progressive semantic resolution

A logical Atlas MAY be physically partitioned by semantic resolution.

Example:

~~~text
coarse
repository / subsystem / module / function summaries

medium
call / state / effect / dependency relationships

deep
CFG / data flow / semantic atoms / detailed evidence
~~~

This is a sharding/access strategy, not three semantic universes.

A coarse shard MUST NOT assert that deep facts do not exist merely because they are not loaded.

Lazy fetch/partial materialization MUST preserve UNKNOWN versus not-yet-loaded distinction.

## THIN/FAT interaction

THIN Atlas:

- keeps canonical semantic records;
- references authenticated source/evidence externally.

FAT Atlas:

- may additionally embed admitted compressed source/evidence blobs.

Source blobs are never substitutes for semantic records.

Semantic compaction requirements apply in both modes.

## Content addressing

Content identity SHOULD be computed over canonical decoded semantic content, not incidental compressor framing, unless a schema explicitly defines otherwise.

This permits physical codec evolution without changing semantic identity.

Unchanged canonical blocks/shards MAY be reused across Atlas revisions when identity/policy permits.

## Canonical versus accelerator data

Atlas distinguishes canonical truth from regenerable accelerators.

### Canonical

Must be lossless and authority-bearing:

- semantic records;
- graph/bindings;
- evidence/provenance;
- obligations/diagnostics;
- selected design;
- Technology Genomes;
- dependency closure;
- certificates.

### Regenerable accelerator

MAY be lossy/approximate:

- embedding indexes;
- ANN/vector indexes;
- learned indexes;
- approximate similarity clusters;
- quantized search vectors;
- summary caches;
- heuristic query accelerators.

Accelerators MUST:

- be clearly marked noncanonical;
- never overwrite canonical identity/status;
- be rebuildable from canonical Atlas plus declared external model/profile inputs;
- never be required to decode canonical truth.

Deleting all accelerators MUST leave the logical Atlas intact.

## Learned/quantized indexes

Aggressive quantization is allowed for noncanonical search acceleration.

Example:

~~~text
canonical typed records
        ↓
derived embeddings
        ↓
INT8 / INT4 / PQ / other approximate index
        ↓
candidate retrieval
        ↓
canonical verification before truth claim
~~~

Approximate retrieval may find candidates. Canonical records decide meaning.

## Determinism

A canonical compaction profile MUST pin enough information to reproduce the same logical block identities for the same canonical semantic input.

At minimum pin:

- schema versions;
- ordering rules;
- intern-table ordering;
- exact-dedup rules;
- canonical block partition rules;
- digest algorithm;
- logical content hashing rules.

Codec bytes may vary only when semantic/root identity is defined over canonical decoded content and the publication profile permits such variation.

SEALED artifacts SHOULD prefer reproducible physical output when practical.

## Compaction profiles

Atlas MAY define different physical compaction profiles, for example:

- query-optimized;
- archive-optimized;
- build-optimized;
- mobile/embedded;
- network-streaming.

Profiles MUST preserve identical canonical semantics unless explicitly representing different selected scopes/designs.

A profile choice is physical/layout policy, not permission to omit required truth.

## Benchmark selection

Compaction strategy may be revised when evidence demonstrates improvement.

Benchmark relevant metrics:

- encoded size;
- decode throughput;
- random lookup latency;
- graph traversal latency;
- memory mapping locality;
- incremental rewrite cost;
- shard reuse rate;
- publication time;
- integrity-verification cost;
- working-set memory.

Optimization MUST remain subordinate to semantic preservation.

## Blueprint evolution

The current compaction/layout strategy is a blueprint.

If donor/dependency census discovers a better mechanism:

~~~text
discover
→ deep-census actual mechanism/provider
→ benchmark/prove against current strategy
→ BlueprintRevisionDecision
→ update compaction blueprint/schema if selected
→ migrate/read old artifacts where required
→ verify semantic round-trip
~~~

Do not freeze an inferior layout merely because it was documented first.

Do not change the wire/schema silently.

## Compatibility

A compaction change that only changes physical layout beneath the same semantic/wire contract may remain compatible.

A change that alters:

- record meaning;
- field meaning;
- identity;
- required section semantics;
- canonical ordering;
- logical hash definition;

requires explicit schema/version/contract migration.

## Required tests before R8 closure

R8 compaction implementation MUST include:

- semantic encode/decode round-trip;
- deterministic canonical ordering;
- exact identity preservation;
- raw-observation lineage preservation;
- evidence/provenance preservation;
- UNKNOWN/UNSUPPORTED/CONFLICT preservation;
- graph node/edge/binding reconstruction;
- cross-shard reference reconstruction;
- corrupted/truncated input rejection;
- decompression bounds enforcement;
- old/new compatible-reader tests where required;
- benchmark evidence for selected compaction strategy.

## Forbidden shortcuts

Forbidden:

- giant JSON as canonical semantic payload;
- "compress the JSON with zstd" as the complete Atlas density strategy;
- lossy canonical record compression;
- approximate semantic dedup;
- merging UNKNOWN with false/absent;
- dropping evidence to save space;
- dropping raw independent observations because claims dedup;
- deriving identity from local table position;
- making lossy accelerators authoritative.

## Final invariant

ATLAS compression is primarily semantic factoring, then binary/layout compaction, then codec compression.

The desired direction is:

~~~text
full engineering meaning
→ less repeated representation
→ denser exact structure
→ efficient physical layout
→ strong codec compression
~~~

not:

~~~text
full engineering meaning
→ discard detail
→ small file
~~~
