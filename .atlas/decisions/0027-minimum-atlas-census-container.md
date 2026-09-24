---
id: atlas.decision.0027.minimum-atlas-census-container
type: decision
status: accepted
canonical: true
---
# ADR 0027 — Minimum `.atlas`: an unsealed census container on ATLAS wire v1

## Context

Exit criterion D of the first phase is "minimum .atlas writer/reader/validator works". P2 needs census → validated semantic state → packaged `.atlas`.

`ATLAS-BINARY-WIRE-FORMAT.md` (wire v1) defines the header, the section directory, record and field framing, and the section types. No code wrote or read it.

The contract leaves several things undefined:
- the varint encoding;
- the field wire types;
- the flag bits, including any unsealed marker;
- a schema-id registry;
- record kinds and field tags;
- the root-identity formula.

No seal gate exists. The contract forbids advertising unsealed state as canonical sealed `*.atlas`.

## Decision

1. **One container shape: the unsealed census container.** `atlas_core::atlas` writes and reads exactly this shape:
   - header flag bit 0 `UNSEALED` is set;
   - the root manifest's seal field is `UNSEALED_CENSUS_CONTAINER`.

   The writer refuses any other seal status, and the reader rejects a container with any other seal status or a missing flag.

2. **Sections.** The first is ROOT_MANIFEST (1). The rest carry the census's validated semantic state:

   | section | contents |
   |---|---|
   | STRING_TABLE (2) | every string, sorted and unique; records reference it by index |
   | SEMANTIC_RECORDS (6) | every census fact: id, kind, status, subject, predicate, object, source path, revision, extractor, span |
   | GRAPH_NODES (7) | declared ADL nodes |
   | GRAPH_EDGES (8) | declared ADL edges |
   | OBLIGATIONS (12) | typed semantic obligations |
   | CENSUS_CERTIFICATE (17) | the certificate as issued for this container: id, state, blockers |

   Epistemic statuses (UNKNOWN, UNSUPPORTED, CONFLICT) and absent optional fields round-trip exactly. Nothing is JSON or Markdown.

3. **Pinned encodings** (the gaps the contract leaves):
   - **Varints:** unsigned LEB128, minimal, at most 10 bytes.
   - **Wire types:** the AtlasX table (1 UVARINT, 6 UTF8, 7 HASH32, 9 LOCAL_INDEX).
   - **Field flags:** bit 0 is REQUIRED. Unknown required fields reject; unknown optional fields are skipped.
   - **Schema ids:** `schema_id` is the first 8 bytes (little-endian) of BLAKE3(`atlas.wire.v1/<section>`).
   - **Ordering:** records are in a canonical order, and the reader enforces it.
   - **Layout:** the header, sections and directory tile the file exactly.

4. **Root identity.** The root identity is BLAKE3 over the ROOT_MANIFEST content. The manifest commits to:
   - the Genome schema and hash (equal to the header's);
   - the self-recensus census digest;
   - the revision and the certificate id;
   - the seal status, tool and THIN mode;
   - every other section's type, schema id, content hash and record count.

   Any semantic change in any section therefore moves the root identity.

5. **Reader verification** follows the contract's order, and any failure rejects trust. The checks, in order:
   - magic, header length, major version, flags and reserved bytes;
   - digest and compression algorithms;
   - directory and entry bounds, canonical directory order, and exact tiling;
   - section hashes and schema ids;
   - exactly one of each section;
   - that the manifest commits to exactly the directory's sections;
   - record framing (strictly ascending tags, wire types, bounds, varints, string indices) and record counts;
   - canonical order;
   - that the certificate id matches.

6. **Publication** (`runtime::atlas::publish`):
   1. encode;
   2. prove the bytes decode back to exactly the packaged state;
   3. write a temporary file and fsync it;
   4. rename it atomically, then fsync the directory.

   `atlas verify F --root R` also requires the container to package R's current census.

7. **Certificate.** `census certificate --atlas F` verifies F against the fresh census and passes its root identity. `CertificateInputs::atlas_root_sealed` is false for this container, so the certificate:
   - replaces ATLAS_ROOT_ABSENT with ATLAS_ROOT_UNSEALED;
   - cannot become SEALED on an unsealed root.

## Consequences

- The self-census packs into about 2.8 MB (11,887 facts, 1,296 obligations, 9 nodes, 12 edges). Re-packing is byte-identical. A census change in scope makes `atlas verify --root` refuse the old container.
- The container is derived and reproducible from its commit. Evidence records its root identity; the binary is not committed.
- Deferred:
  - sealing (needs a seal gate, SelectedDesign and seal policy);
  - compression;
  - sharding;
  - typed semantic records beyond facts;
  - evidence and source blobs (FAT mode);
  - graph nodes and edges beyond the declared ADL.

  Each is an additional section or flag on the same framing.
- Falsification: 21 mutants, all killed. The first battery's two survivors were real test gaps and were closed:
  - a wrong-wire-type case whose bytes were also an out-of-range string index;
  - no test for census facts that collapse under canonicalization.
