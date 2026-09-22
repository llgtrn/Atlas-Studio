---
id: atlas.contract.format.atlasx-wire
type: contract
status: active
canonical: true
---
# ATLASX Binary Wire Format Contract

## Purpose

This contract defines the canonical physical encoding for AtlasX v1 objects and `manifest.atlasx`.

ATLASX is an expanded directory representation, but its canonical semantic files are typed binary objects, not JSON/Markdown authority.

The logical object model is defined by `ATLASX-FORMAT.md`.

The materialization algorithm is defined by `ATLAS-TO-ATLASX.md`.

## Canonical directory rule

A canonical AtlasX tree contains:

~~~text
<system>.atlasx/
├─ manifest.atlasx
├─ modules/
├─ types/
├─ functions/
├─ interfaces/
├─ capabilities/
├─ bindings/
├─ state/
├─ effects/
├─ resources/
├─ concurrency/
├─ persistence/
├─ constraints/
├─ external/
├─ tests/
├─ profiles/
├─ targets/
├─ lineage/
├─ graph/
├─ runtime/
└─ ui/
~~~

A directory may be absent when it has no canonical objects for the selected design.

Canonical object filenames are content-addressed:

~~~text
<directory>/<decoded-content-hash-hex>.atlasx
~~~

`manifest.atlasx` is the only canonical non-content-named file.

Human-readable/debug files MAY coexist, but if they are not listed as canonical objects in the manifest they are not compiler authority.

## Byte order

All fixed-width integers are little-endian.

All lengths are unsigned.

Readers MUST bounds-check:

- encoded lengths;
- decoded lengths;
- record lengths;
- field lengths;
- count × element-size calculations;
- nested-record depths;
- decompression limits.

Malformed/truncated input MUST be rejected.

## Fixed object header

Every canonical AtlasX v1 binary object begins with an 80-byte header:

~~~text
offset  size  field
0       8     magic = "ATLSX\0\1\0"
8       2     header_len = 80
10      2     format_major = 1
12      2     format_minor
14      2     flags
16      2     object_class
18      2     object_schema_version
20      2     digest_algorithm
22      2     codec
24      8     encoded_length
32      8     decoded_length
40      32    decoded_content_hash
72      8     reserved = 0
~~~

The encoded payload begins at byte 80.

`encoded_length` counts bytes after the header.

`decoded_length` is the payload size after codec decoding.

`decoded_content_hash` authenticates exactly the decoded canonical payload bytes.

Unknown major versions MUST be rejected.

Unknown minor versions MAY be accepted only when all required object/record/field semantics are understood or explicitly skippable.

## Algorithm identifiers

AtlasX v1 uses the same core identifiers as ATLAS wire v1:

~~~text
digest_algorithm
0 = INVALID
1 = BLAKE3_256
2 = SHA256

codec
0 = NONE
1 = ZSTD
~~~

Supporting a codec implementation does not grant its donor implementation architectural ownership.

## Object classes

AtlasX v1 reserves these canonical object classes:

~~~text
1   ROOT_MANIFEST
2   MODULES
3   TYPES
4   FUNCTIONS
5   INTERFACES
6   CAPABILITIES
7   BINDINGS
8   STATE
9   EFFECTS
10  OWNERSHIP_RESOURCES
11  CONCURRENCY
12  PERSISTENCE_RECOVERY
13  CONSTRAINTS_INVARIANTS
14  EXTERNAL_BOUNDARIES
15  TESTS
16  PROFILES
17  TARGETS
18  LINEAGE_EVIDENCE
19  GRAPH_VIEW
20  RUNTIME
21  UI
~~~

Classes 1–1023 are reserved for canonical AtlasX core.

Extensions use Genome-registered class/schema identifiers and may not redefine core meaning.

## Canonical directory mapping

Core classes map to default directories:

~~~text
MODULES                 → modules/
TYPES                   → types/
FUNCTIONS               → functions/
INTERFACES              → interfaces/
CAPABILITIES             → capabilities/
BINDINGS                 → bindings/
STATE                    → state/
EFFECTS                  → effects/
OWNERSHIP_RESOURCES      → resources/
CONCURRENCY              → concurrency/
PERSISTENCE_RECOVERY     → persistence/
CONSTRAINTS_INVARIANTS   → constraints/
EXTERNAL_BOUNDARIES      → external/
TESTS                    → tests/
PROFILES                 → profiles/
TARGETS                  → targets/
LINEAGE_EVIDENCE         → lineage/
GRAPH_VIEW               → graph/
RUNTIME                  → runtime/
UI                       → ui/
~~~

A future blueprint may change physical grouping under `BLUEPRINT-EVOLUTION.md`, but v1 canonical publication uses this mapping.

## Record framing

Decoded object payloads consist of deterministic length-delimited records.

Each record begins:

~~~text
size  field
2     record_kind
2     record_schema_version
4     record_flags
8     payload_length
~~~

Then:

~~~text
payload[payload_length]
~~~

Records MUST be emitted in canonical order defined by the object-class schema.

For identity-bearing records, default ordering is:

~~~text
stable semantic identity bytes
→ record_kind
→ decoded record-content hash
~~~

A class-specific contract may define a stronger order.

## Tagged field framing

Record payloads use fields:

~~~text
size  field
2     field_tag
1     wire_type
1     field_flags
4     field_length
N     field_bytes
~~~

Fields MUST be emitted in ascending `field_tag`.

Repeated values preserve semantic order when order matters. Otherwise the field schema defines deterministic sorting.

Unknown optional fields may be skipped.

Unknown required fields reject the record.

Field tags are never repurposed with different meaning within a schema lineage.

## Wire types

AtlasX v1 defines:

~~~text
0   INVALID
1   UVARINT
2   SVARINT
3   FIXED32
4   FIXED64
5   BYTES
6   UTF8
7   HASH32
8   GLOBAL_ID
9   LOCAL_INDEX
10  RECORD
11  PACKED
12  BOOL
~~~

`GLOBAL_ID` carries a stable identity defined by the semantic schema.

`LOCAL_INDEX` is allowed only within the current object payload and MUST NOT become a cross-object identity.

## String rules

UTF8 fields preserve their exact semantic UTF-8 byte sequence unless the field schema explicitly defines normalization.

Generic Unicode normalization MUST NOT be applied silently to source spellings or identity-bearing strings.

Canonical sorting of unconstrained UTF8 strings is bytewise UTF-8 lexicographic order.

## Root manifest object

`manifest.atlasx` MUST use `object_class = ROOT_MANIFEST`.

It MUST contain exactly one root-manifest record.

Required root-manifest fields:

~~~text
tag  meaning
1    atlasx_root_id
2    parent_atlas_root_id
3    genome_hash
4    selected_design_id
5    materialized_scope_id
6    target_kind
7    materializer_identity
8    materializer_version
9    materialization_schema_version
10   compiler_ir_contract_version
11   profile_ref           repeated
12   external_binding_ref  repeated
13   semantic_barrier_ref  repeated
14   object_entry          repeated
15   dynamic_obligation    repeated
16   compatibility_requirement repeated
~~~

Tag 1 is excluded from the root-ID preimage to avoid self-reference.

All other semantic fields are included.

## ObjectEntry

Each manifest `object_entry` nested record MUST contain:

~~~text
tag  meaning
1    relative_path
2    object_class
3    object_schema_version
4    decoded_content_hash
5    decoded_length
6    required
7    logical_record_count
~~~

Relative paths MUST:

- be UTF-8;
- use `/`;
- be relative to the AtlasX root;
- not begin with `/`;
- not contain `.` or `..` path segments;
- match the canonical directory mapping for core classes;
- match the content-addressed filename rule.

Object entries are sorted by:

~~~text
object_class
→ decoded_content_hash
→ relative_path
~~~

## AtlasX root identity

The root identity is computed from the canonical decoded root-manifest payload with field tag 1 omitted.

Conceptually:

~~~text
atlasx_root_id =
H(
  canonical_manifest_fields_2_through_16
)
~~~

The digest algorithm is the one declared by `manifest.atlasx`.

After computing the root ID, tag 1 stores that exact digest/identity.

A reader MUST recompute and compare it.

## Object content identity

A canonical object file's physical filename is the lowercase hexadecimal form of its `decoded_content_hash`.

Because manifest entries also include `object_class` and schema version, identical payload bytes under two classes do not become the same semantic object accidentally.

Object semantic identities inside the payload remain independent of the filename/hash.

## Canonical record families

Each core object class carries only its corresponding logical records:

- MODULES: module/composition records;
- TYPES: type records;
- FUNCTIONS: function/signature/body records;
- INTERFACES: interface records;
- CAPABILITIES: capability records;
- BINDINGS: binding records;
- STATE: state/transition records;
- EFFECTS: effect records;
- OWNERSHIP_RESOURCES: ownership/resource records;
- CONCURRENCY: task/thread/channel/lock/atomic/order records;
- PERSISTENCE_RECOVERY: transaction/durability/recovery records;
- CONSTRAINTS_INVARIANTS: constraints/barriers/invariants;
- EXTERNAL_BOUNDARIES: explicit external capability/provider boundaries;
- TESTS: selected verification test records;
- PROFILES: deployment/hardware/workload/profile records;
- TARGETS: target/ABI requirement records;
- LINEAGE_EVIDENCE: Atlas/SelectedDesign/evidence mappings;
- GRAPH_VIEW: derived executable graph projection;
- RUNTIME: runtime composition records;
- UI: UI projection semantics.

A record MUST NOT be placed in an unrelated class merely for convenience.

## Cross-object references

Cross-object semantic references MUST use GLOBAL_ID or another schema-defined stable content identity.

A LOCAL_INDEX from one object MUST NEVER reference another object.

The validator MUST verify every required cross-object reference resolves to:

- a canonical object record in the same AtlasX root;
- an explicit external boundary;
- an explicitly permitted dynamic runtime binding.

## Graph view

GRAPH_VIEW is a deterministic projection over canonical AtlasX objects.

It MUST NOT introduce semantics absent from those objects.

Node/edge/binding identity must remain reconstructable.

A compiler MAY use graph view as an acceleration structure, but truth comes from validated canonical AtlasX semantics.

## Compression

Each object independently declares a codec.

The root/object identity is based on decoded canonical payload content, so codec framing MUST NOT change semantic identity.

Canonical publication SHOULD use deterministic codec settings when practical.

Readers MUST enforce decoded-length/resource limits before decompression.

## Publication transaction

AtlasX publication is transactional at root-manifest level.

Required order:

~~~text
1. write all canonical object files to staging
2. fsync/verify each object according to platform policy
3. compute object hashes
4. build canonical manifest payload
5. compute AtlasX root identity
6. write/verify manifest.atlasx in staging
7. atomically publish/switch root directory/reference
8. only then advertise the AtlasX root
~~~

A manifest MUST NOT reference missing/unverified required objects.

## Reader verification order

A reader/compiler MUST conceptually verify:

~~~text
manifest header/magic/version
→ manifest bounds/codec/hash
→ recompute AtlasX root ID
→ parent Atlas / Genome / SelectedDesign compatibility
→ object-entry path validity
→ required object presence
→ each object header/bounds
→ each object decoded hash
→ record framing
→ field framing
→ class/schema constraints
→ cross-object reference closure
→ dynamic/external boundary validity
→ semantic barrier presence
→ compiler-contract compatibility
~~~

Failure at a required step invalidates the root.

## Canonical versus noncanonical files

Files not listed as canonical object entries are noncanonical projections/cache/debug material.

Examples:

- pretty JSON;
- Markdown;
- generated diagrams;
- temporary source renderings;
- search indexes;
- compiler caches.

They MUST NOT affect AtlasX root identity.

The compiler MUST NOT rely on them for semantic truth.

## Evolution

A backward-compatible field may be added as optional under a compatible schema version.

Existing tag meanings are never repurposed.

Breaking changes require:

- incompatible object schema version; or
- new wire major version;

plus explicit migration/compatibility rules.

## Blueprint evolution

This v1 physical layout is canonical now, not permanently frozen.

If census/benchmark evidence demonstrates a better AtlasX encoding or object organization, Atlas MAY revise the blueprint through `BLUEPRINT-EVOLUTION.md`.

A revision may improve physical representation.

It may not silently reinterpret old object bytes, stable semantic IDs or root lineage.

## Forbidden shortcuts

Forbidden:

- canonical JSON manifest substituted for `manifest.atlasx`;
- filename/path used as semantic identity;
- compiler scanning arbitrary files instead of the manifest;
- unbounded decompression;
- cross-file LOCAL_INDEX references;
- object hash computed over incidental codec framing as semantic identity;
- hidden required object outside manifest;
- silently accepting unknown required fields/classes;
- human-readable debug projection used as compiler authority.

## Final invariant

A canonical AtlasX root is independently verifiable from bytes.

Two compliant implementations receiving the same materialization inputs and using the same v1 schemas must agree on:

- canonical object semantics;
- object content hashes;
- manifest object inventory/order;
- AtlasX root identity.

Physical codec bytes may differ only where the declared publication policy permits decoded-content identity to remain stable.
