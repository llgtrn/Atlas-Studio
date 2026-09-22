---
id: atlas.contract.format.atlasx-wire
type: contract
status: active
canonical: true
---
# ATLASX Binary Capsule Wire Format Contract

## Authority and version break

This contract defines the canonical physical encoding for AtlasX **wire v2**.

It is governed by `ARTIFACT-LAYERING.md` and `ATLASX-FORMAT.md`.

AtlasX wire v1 encoded canonical semantics as a directory containing `manifest.atlasx` and content-addressed object files.

That physical model is superseded.

Because the canonical root changed from a directory/object set to one binary capsule, this is a wire-major break.

~~~text
AtlasX wire v1 = legacy directory/object encoding; migration input only
AtlasX wire v2 = canonical single-file *.atlasx capsule
~~~

A v2 reader MUST NOT interpret v1 bytes as v2.

A v1 reader MUST reject v2 by magic/major version.

## Canonical physical unit

The canonical publication unit is exactly one binary file:

~~~text
<system>.atlasx
~~~

The file contains:

~~~text
fixed capsule header
root manifest section
entry directory
canonical entry payloads
optional signature/attestation material
~~~

An unpacked filesystem tree is not canonical wire representation.

## Byte order and bounds

- All fixed-width integers are little-endian.
- All offsets are unsigned absolute offsets from the start of the capsule file.
- Readers MUST bounds-check every offset, length, count, addition, multiplication, and decode operation before allocation/read.
- Readers MUST reject required regions that overlap illegally.
- Readers MUST enforce decoded-size and nesting limits before decompression.
- Readers MUST reject truncated input.
- No entry is trusted before required integrity checks pass.

## Fixed 96-byte capsule header

Every AtlasX v2 capsule begins with:

~~~text
offset  size  field
0       8     magic = "ATLSX\0\2\0"
8       2     header_len = 96
10      2     format_major = 2
12      2     format_minor
14      2     flags
16      2     digest_algorithm
18      2     default_codec
20      2     capsule_profile
22      2     reserved0 = 0
24      32    genome_hash
56      8     root_manifest_offset
64      8     root_manifest_length
72      8     entry_directory_offset
80      8     entry_directory_length
88      8     file_length
~~~

`file_length` MUST equal the actual capsule byte length.

Unknown major versions MUST be rejected.

A newer minor version MAY be accepted only when every required entry/field encountered is understood or explicitly skippable.

## Capsule profile identifiers

Wire v2 reserves:

~~~text
0 = INVALID
1 = VERIFY_ONLY
2 = BUILD_REPRODUCIBLE
3 = EXECUTABLE_PORTABLE
4 = DEPLOYMENT_TARGETED
~~~

The header value MUST match the profile declared by the root manifest.

Mismatch invalidates the capsule.

## Algorithm identifiers

Wire v2 reserves:

~~~text
digest_algorithm
0 = INVALID
1 = BLAKE3_256
2 = SHA256

codec
0 = NONE
1 = ZSTD
~~~

Additional algorithms require an explicit compatible schema revision.

A codec implementation is an implementation detail; its donor does not become architectural authority.

## Entry directory

The entry directory is an array of fixed 80-byte entries:

~~~text
size  field
2     entry_class
2     entry_flags
2     codec
2     entry_schema_version
8     offset
8     encoded_length
8     decoded_length
8     logical_record_count
32    decoded_content_hash
8     reserved = 0
~~~

Directory entries MUST be sorted canonically by:

~~~text
entry_class
→ decoded_content_hash
→ offset
~~~

for SEALED publication.

Offsets/lengths refer to encoded payload bytes.

`decoded_content_hash` authenticates decoded canonical payload bytes, not compression framing.

## Core entry classes

AtlasX v2 reserves:

~~~text
1   ROOT_MANIFEST
2   ATLAS_ARTIFACT
3   SELECTED_SYSTEM_CLOSURE
4   DEPENDENCY_CLOSURE
5   RUNTIME_PAYLOAD
6   BUILD_INPUT
7   ASSET_RESOURCE
8   PROFILE_TARGET
9   EXTERNAL_BOUNDARY
10  SBOM
11  LICENSE_ATTRIBUTION
12  PROVENANCE_EVIDENCE
13  SECURITY_ATTESTATION
14  REPRODUCIBILITY
15  MIGRATION
16  OPTIONAL_SOURCE
17  OPTIONAL_DEBUG
18  SIGNATURE_BLOCK
~~~

Classes 1–1023 are reserved for canonical AtlasX core.

Extensions use Genome-registered identifiers and may not redefine core classes.

## Root manifest

Every capsule MUST contain exactly one `ROOT_MANIFEST` entry.

The root manifest commits to at least:

- AtlasX root identity;
- wire major/minor;
- capsule schema version;
- capsule profile;
- Genome identity/hash;
- root/parent Atlas identity;
- embedded Atlas artifact identities;
- SelectedDesign identity;
- requested scope;
- target/profile identities;
- dependency-closure identity;
- exact canonical entry inventory;
- required versus optional entries;
- external boundaries;
- compiler contract/version;
- build/reproduction contract when applicable;
- compatibility requirements;
- permitted pinned-fetch records;
- signature/attestation policy.

The manifest MUST NOT depend on extraction paths.

## Canonical record framing

Record-oriented decoded entries use:

~~~text
u16 record_kind
u16 record_schema_version
u32 record_flags
u64 payload_length
payload[payload_length]
~~~

Record payload fields use:

~~~text
u16 field_tag
u8  wire_type
u8  field_flags
u32 field_length
field_bytes[field_length]
~~~

Fields MUST be emitted in ascending `field_tag` order.

Unknown optional fields may be skipped.

Unknown required fields reject the record.

Existing field tags are never repurposed with different meaning.

## Wire types

Wire v2 defines:

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

A `LOCAL_INDEX` is valid only within its containing decoded entry.

Cross-entry references MUST use stable global/content identities.

## Manifest entry records

For each canonical directory entry other than the root manifest, the root manifest MUST contain one corresponding `CapsuleEntry` record including at least:

~~~text
entry_class
entry_schema_version
decoded_content_hash
decoded_length
required
semantic_role
atlas_lineage_ref     optional by class
dependency_role       optional by class
target_profile_ref    optional by class
~~~

The manifest entry inventory and physical entry directory MUST match exactly for canonical entries.

An unexplained physical entry is rejected unless explicitly marked noncanonical padding under a future compatible rule.

## Atlas artifact entries

An `ATLAS_ARTIFACT` entry may contain:

1. exact canonical bytes of a sealed `*.atlas`; or
2. an Atlas-native capsule embedding form that is provably identity-equivalent to the referenced Atlas semantic root.

The default v2 rule is exact canonical `*.atlas` bytes.

The capsule MUST preserve the embedded artifact's own identity.

Packaging MUST NOT silently rewrite Atlas semantics.

## Dependency closure entries

Dependency records MUST support transitive closure.

Each dependency record includes, where applicable:

- dependency identity;
- exact version/revision;
- decoded content hash;
- parent dependency relation;
- build/runtime role;
- target/profile applicability;
- source/provenance;
- license/obligations;
- admission status;
- payload reference or pinned-fetch record.

A mutable package range is not canonical closure.

## Pinned-fetch records

A `PINNED_FETCH` record MUST contain at least:

- expected content hash;
- expected decoded length when known;
- immutable identity/revision;
- allowed retrieval origins or resolver class;
- transport integrity/authentication requirements;
- offline/failure behavior;
- admission/cache policy;
- provenance/license references.

A URL alone is insufficient.

The root manifest MUST declare whether pinned fetch is permitted for the capsule profile.

## Runtime payload entries

Runtime payloads remain opaque bytes only with explicit typed metadata that binds them to:

- content identity;
- payload class;
- target/ABI/runtime;
- originating Atlas semantics;
- SelectedDesign;
- required loader/runtime;
- integrity policy.

Opaque bytes without semantic lineage are not valid canonical runtime payloads.

## Build-input entries

BUILD_REPRODUCIBLE and stricter profiles may carry:

- delegated generated source;
- compiler/toolchain payload;
- linker/runtime support;
- build graph;
- deterministic recipe;
- environment contract;
- patches.

Every build-significant input must be embedded or pinned.

## Asset/resource entries

Assets/resources that affect selected behavior MUST be in the canonical inventory.

Optional documentation/media not required for selected behavior may be noncanonical or optional according to policy.

## Supply-chain entries

SBOM, license, attribution, provenance, security, and evidence entries may be factored independently but remain content-addressed and root-bound.

A required legal/security obligation may not be hidden in an unmanifested sidecar.

## Signature block

Signature semantics are policy/schema governed.

A signature MUST bind at least the AtlasX root identity and relevant policy/domain separator.

Signatures are not included in the preimage in a self-referential way.

Multiple signatures/attestations may be present.

## AtlasX root identity

The root identity is computed over canonical decoded root-manifest semantics with the `atlasx_root_id` field omitted from its own preimage.

Conceptually:

~~~text
AtlasXRootId =
H(
  domain_separator("ATLASX-V2")
  + canonical_manifest_without_root_id
  + ordered required entry identities
)
~~~

The exact field tags and domain separator bytes are fixed by the machine schema for v2.

After computation, the root ID field stores the exact result.

A reader MUST recompute and compare it.

Codec framing and physical extraction path MUST NOT affect root identity.

## Canonical entry identity

Each entry's physical integrity identity is its `decoded_content_hash`.

Semantic identities carried inside the entry remain distinct from the physical content hash where the semantic schema defines them separately.

Two entries with equal bytes but different semantic classes are not automatically the same semantic object.

## Compression

Compression occurs after canonical decoded entry construction.

~~~text
canonical typed content
→ entry framing
→ deterministic/declared codec
→ capsule placement
~~~

The root/entry semantic identity is based on decoded canonical content.

Readers MUST enforce decoded-length/resource limits before decompression.

## Physical layout

Canonical v2 publication order is:

~~~text
Header
RootManifest payload
EntryDirectory
EntryPayloads sorted by canonical directory order
Signature/Attestation entries where declared by directory
~~~

Padding is forbidden in canonical v2 unless a future minor-version rule explicitly defines deterministic padding.

The header's offsets make physical scanning unnecessary.

## Transactional publication

Writers MUST:

~~~text
1. construct all canonical decoded entries
2. validate closure
3. compute decoded entry hashes
4. construct canonical root manifest
5. compute AtlasX root identity
6. encode/compress entries
7. build entry directory
8. write complete capsule to staging
9. reread/verify bounds + hashes + root
10. fsync according to platform policy
11. atomically publish final *.atlasx
~~~

A partially written capsule MUST never be advertised as valid.

## Reader verification order

A reader/compiler MUST conceptually verify:

~~~text
magic / major version
→ file_length
→ header bounds
→ root-manifest bounds
→ entry-directory bounds
→ directory entry bounds / overlap
→ algorithm support
→ decode root manifest
→ Genome/schema compatibility
→ capsule-profile agreement
→ recompute AtlasX root identity
→ manifest/directory inventory agreement
→ decode required entries
→ per-entry decoded hashes
→ embedded *.atlas integrity
→ SelectedDesign relationship
→ dependency transitive closure
→ pinned-fetch policy
→ external-boundary policy
→ supply-chain/security obligations
→ compiler contract compatibility
~~~

Any required failure invalidates the capsule.

## Unpack projection

`atlasx unpack` may project entries to directories/files.

Extraction paths are derived convenience names.

They are NOT part of semantic identity unless an entry's own typed semantics explicitly includes a path requirement.

A compiler MUST NOT establish authority by rescanning an unpacked directory.

## Migration from wire v1

Wire v1 directory/object roots are accepted only by an explicit migration reader.

Migration MUST:

- verify the v1 root/object hashes under v1 rules;
- recover parent Atlas + SelectedDesign lineage;
- classify every retained object into v2 entry classes;
- compute missing dependency/runtime/resource closure required by the target v2 profile;
- reject ambiguous/unmanifested semantic authority;
- produce a new v2 capsule root identity.

Migration is not an in-place reinterpretation.

## Forbidden shortcuts

Forbidden:

- treating `<system>.atlasx/` as canonical v2;
- JSON/YAML/Markdown root manifest as v2 authority;
- compiler directory scanning for hidden inputs;
- path/filename as semantic identity;
- floating dependency versions;
- cross-entry LOCAL_INDEX references;
- hashing codec framing as semantic identity;
- unbounded decompression;
- undeclared external dependency;
- model invocation to fill missing capsule semantics;
- silent v1→v2 reinterpretation.

## Final invariant

A canonical AtlasX v2 artifact is one independently verifiable binary capsule.

Two compliant implementations given identical canonical inputs MUST agree on:

- required entry semantics;
- decoded content hashes;
- canonical entry ordering;
- AtlasX root identity.

Any implementation that requires the old directory tree as semantic authority is not AtlasX v2 compliant.
