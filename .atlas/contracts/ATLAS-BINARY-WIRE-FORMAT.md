---
id: atlas.contract.format.atlas-wire
type: contract
status: active
canonical: true
---
# ATLAS Binary Wire Format Contract

## Purpose

This contract turns the logical requirements of `ATLAS-FORMAT.md` into an implementable binary container family.

A logical Atlas may be one file or a root plus immutable shards. Every physical file uses the same bounded, self-describing container rules.

This document defines **wire v1 structure**. Individual semantic record schemas remain governed by their own typed contracts and Genome/schema pins.

Lossless semantic factoring/compaction before wire encoding is governed by `ATLAS-SEMANTIC-COMPACTION.md`. The wire layer MUST NOT be used as an excuse to discard canonical meaning.

## Byte order and bounds

- All fixed-width integers are little-endian.
- All offsets are unsigned absolute byte offsets from the start of the containing file.
- Readers MUST bounds-check every offset, length, count and multiplication before allocation/read.
- Readers MUST reject overlapping mandatory regions unless a section type explicitly permits shared content.
- No section payload is trusted before its declared integrity hash is verified when policy requires verification.

## Fixed file header

Every physical ATLAS v1 container begins with a 72-byte header:

```text
offset  size  field
0       8     magic = "ATLAS\0\1\0"
8       2     header_len = 72
10      2     format_major = 1
12      2     format_minor
14      2     flags
16      2     digest_algorithm
18      2     default_compression
20      32    genome_hash
52      8     section_directory_offset
60      8     section_directory_length
68      4     reserved = 0
```

Unknown major versions MUST be rejected.

A reader MAY accept a newer minor version only when every required section/record it encounters is understood or explicitly skippable.

## Algorithm identifiers

Wire v1 reserves:

```text
digest_algorithm
0 = INVALID
1 = BLAKE3_256
2 = SHA256

compression
0 = NONE
1 = ZSTD
```

The exact algorithm used is therefore explicit on the artifact. Supporting an algorithm does not make its donor implementation a runtime owner; Atlas-native or independently maintained implementations may replace bootstrap dependencies while preserving wire semantics.

## Section directory

The section directory is an array of fixed 80-byte entries:

```text
size  field
2     section_type
2     section_flags
2     codec
2     reserved = 0
8     schema_id
8     offset
8     encoded_length
8     decoded_length
8     record_count
32    decoded_content_hash
```

Directory entries MUST be sorted by:

```text
section_type → schema_id → decoded_content_hash → offset
```

for canonical publication.

Offsets/lengths refer to encoded payload bytes. `decoded_content_hash` authenticates the canonical decoded section content, not incidental compression framing.

## Core section types

Wire v1 assigns:

```text
1   ROOT_MANIFEST
2   STRING_TABLE
3   IDENTITY_TABLE
4   TYPE_TABLE
5   SYMBOL_TABLE
6   SEMANTIC_RECORDS
7   GRAPH_NODES
8   GRAPH_EDGES
9   BINDINGS
10  EVIDENCE
11  DIAGNOSTICS
12  OBLIGATIONS
13  TEMPORAL
14  DESIGN
15  SOURCE_BLOBS
16  SHARD_MANIFEST
17  CENSUS_CERTIFICATE
```

Types 1-1023 are reserved for canonical Atlas core. Extensions use Genome-registered schema IDs and may not redefine a core type.

A logical Atlas does not need every section in every shard. The root manifest declares which semantic classes are mandatory for that logical root.

## Root manifest

A root container MUST contain exactly one `ROOT_MANIFEST`.

The manifest commits to at least:

- logical Atlas root identity;
- seal identity and seal-policy identity;
- SelectedDesign identity when the root is materialization-capable;
- required verification/evidence/attestation root identities used by the seal;
- Atlas wire version;
- Atlas semantic schema set;
- exact Genome identity/hash;
- corpus/design identities and revisions;
- required shard identities/content hashes;
- CensusCertificate identity when applicable;
- THIN/FAT mode;
- required/optional section classes;
- compiler/publication tool identity;
- compatibility requirements.

A shard manifest MUST bind the shard to its logical root and content identity.

A root that is candidate/unsealed state MUST NOT be advertised as canonical sealed `*.atlas`. Debug or migration containers, when supported, require an explicit unsealed marker/profile and are outside canonical publication identity.

## Record framing

Record-oriented sections use deterministic length-delimited records:

```text
u16 record_kind
u16 record_schema_version
u32 payload_length
payload[payload_length]
```

Payloads use canonical tagged fields:

```text
u16 field_tag
u8  wire_type
u8  field_flags
u32 field_length
field_bytes[field_length]
```

Fields are emitted in ascending `field_tag` order. Repeated fields preserve semantic order when order matters; otherwise their contract defines canonical sorting.

Unknown optional field tags are skippable. Unknown required field tags cause rejection.

JSON, Markdown and language-specific source text are not canonical payload encodings for semantic records.

## Interning and references

Strings, identities, types and symbols SHOULD be interned.

Within one physical container, compact references may use unsigned varint table indices.

Canonical global identity remains independent of local table index. Cross-shard references MUST resolve through globally stable identity/content IDs, never by another shard's local ordinal.

Reordering an intern table may change physical bytes but MUST NOT change semantic identity. Canonical publication therefore uses deterministic first-ordering defined by the relevant table schema.

## Semantic record preservation

Binary encoding may compress representation but MUST preserve the complete typed semantic record required by contracts, including where applicable:

- record kind/id;
- repository/revision/scope identity;
- typed payload;
- epistemic status;
- extractor/compiler identity;
- provenance/span;
- evidence references;
- unresolved obligations and diagnostic linkage.

Flattening a typed function signature, call edge, CFG block or effect record into display text is not a valid wire encoding.

## Graph encoding

Graph nodes, edges and bindings remain distinct record classes.

Adjacency compression, delta encoding and shared attribute dictionaries are allowed only when exact node/edge/binding identity and provenance remain reconstructable.

Graph is a projection of normalized/reconciled semantics; the wire format must not invent graph-only truth.

## THIN and FAT source modes

THIN mode stores canonical semantics plus authenticated external source/evidence references.

FAT mode MAY additionally include compressed admitted source/evidence blobs in `SOURCE_BLOBS`.

Source blobs are evidence/reconstruction material. They do not replace typed semantic sections.

## Compression

Physical codec compression is the final layer after semantic compaction.

The canonical layering is:

~~~text
typed semantic records
→ exact semantic compaction/factoring
→ wire records/sections
→ section codec
→ shard/root publication
~~~

Each section independently declares its codec.

Compression MUST be deterministic for canonical publication under the pinned publication profile, or the logical content hash/root identity MUST be defined over decoded canonical content so codec nondeterminism cannot change semantic identity.

Readers enforce decoded-length limits before decompression to prevent resource-exhaustion attacks.

## Content addressing and sharding

A shard identity is derived from canonical decoded content according to the digest algorithm declared by its root/profile.

The root manifest commits to every mandatory shard hash.

Unchanged shards may be reused across revisions when all semantic identity/policy constraints allow it.

Shard storage location is never semantic identity.

## Transactional publication

Writers publish to a temporary identity/path, fully write and verify all sections, compute directory/root hashes, fsync according to platform policy, then atomically publish the root reference.

A partially written file MUST never be advertised as a valid sealed Atlas root.

## Reader verification order

A reader MUST conceptually verify:

```text
magic/version/header bounds
→ directory bounds
→ section-entry bounds/non-overlap policy
→ algorithm support
→ root manifest
→ Genome/schema compatibility
→ required shard/section presence
→ section hashes
→ record framing/bounds
→ semantic schema constraints
→ seal identity / seal-policy identity
→ required verification/evidence/attestation roots
→ CensusCertificate / obligation closure required by seal policy
```

Failure at any required step rejects trust in the artifact.

## Canonical publication versus readable publication

Readers may support noncanonical physical ordering for migration/debug artifacts when explicitly marked unsealed.

SEALED publication MUST use canonical ordering, schema/version pins and integrity rules so equivalent semantic inputs have reproducible logical identities.

## Evolution

New semantic record kinds/fields use schema versions and compatibility rules. Existing field tags are never repurposed with different meaning.

A breaking change requires a new major wire version or a new incompatible schema identity explicitly rejected by older readers.

## Blueprint evolution and wire stability

The chosen compaction/layout/codec strategy may evolve under `BLUEPRINT-EVOLUTION.md` when census or benchmark evidence finds a better mechanism.

However, a blueprint change does not permit silent wire reinterpretation.

A change that modifies any of the following requires explicit schema/version compatibility treatment:

- header meaning;
- section type meaning;
- record/field meaning;
- identity derivation;
- canonical ordering;
- logical content hash definition;
- required/optional semantics.

Physical improvements beneath unchanged logical/wire semantics may evolve without a major version when compatibility remains provable.

Old SEALED artifacts MUST remain either:

- readable under the declared compatibility policy; or
- explicitly rejected/migrated by version, never silently misread.
