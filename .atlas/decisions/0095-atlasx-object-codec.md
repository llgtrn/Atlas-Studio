---
id: atlas.decision.0095.atlasx-object-codec
type: decision
status: accepted
canonical: true
---
# ADR 0095 — The AtlasX object codec: one class, FUNCTIONS, canonical and content-addressed (G183)

## Context

`ATLASX-BINARY-WIRE-FORMAT.md` defines how AtlasX objects are encoded: an 80-byte header, record and tagged-field framing, wire types, a content-addressed filename rule and a reader verification order. Nothing implemented it. Construction node M12 (the object codec, DEBT-ATLASX) was MISSING. M10, the precondition gate, exists since G179 (ADR 0093). M11 (materializer) and M13 (validator) need an object codec first.

NA-ATLASX-CODEC asks for the smallest slice of M12: one object kind, with a header and one typed record section, encoded canonically, addressed by its BLAKE3 digest and read back. It is falsified if a fixture does not round-trip byte for byte, if a flipped byte is not refused by its digest, or if an undeclared field is not refused.

## Decision

1. **One class: FUNCTIONS (4), one top-level record kind: FUNCTION_SIGNATURE.** The records come from the construction IR (`construction::IrFunction`, `atlas.construction-ir.v1`). This is the one class Atlas can produce from real input today: G181 lifts real function signatures from census records (SR1-3). Types would also be possible, but a signature has more wire shapes: an identity, text, an enum code, a hash, an ordered sequence and a set. The schema table (`codec::FUNCTIONS_SCHEMA`, object schema 1, record schema 1) is the only place where a field name meets its tag, wire type and required flag:

   | tag | field | wire type | required |
   |---|---|---|---|
   | 1 | function_id | GLOBAL_ID (UTF-8 bytes of the IR id) | yes |
   | 2 | name | UTF8 | yes |
   | 3 | owner | UTF8 | no |
   | 4 | dispatch | UVARINT (1 FREE_FUNCTION, 2 INHERENT_METHOD, 3 ASSOCIATED_FUNCTION) | yes |
   | 5 | visibility | UTF8 | yes |
   | 6 | documentation | UTF8 | no |
   | 7 | params | RECORD, one or more PARAM (2) records in declared order: 1 name, 2 type_spelling (UTF8) | no |
   | 8 | result | UTF8 | no |
   | 9 | body_fingerprint | HASH32 (the 32 bytes of a `blake3-256:` digest) | no |
   | 10 | lineage | RECORD, one or more LINEAGE_REF (3) records: 1 record (GLOBAL_ID) | yes |

   `schema_digest()` hashes the table. A test pins that digest to schema version 1, so a changed table needs a new version. A function's HIR body is not a field of this record kind and is not encoded. `encode_module` names every function whose body was left out (`bodies_not_encoded`), so nothing is dropped silently.

   `encode_module` writes only a module that verifies. Its id must be its content identity (SOURCE_MODULE_UNVERIFIED). `construction::validate_module` must find nothing against the census records the module declares as inputs (SOURCE_MODULE_INVALID, with the violation code). The writer also refuses an empty function name (MALFORMED_VALUE). Whether those inputs resolve in the container the module names is not checked here, because the codec is not given the container.
2. **Canonical writing.**
   - Header: magic `ATLSX\0\1\0`, header_len 80, major 1, minor 0, flags 0, class 4, schema 1, digest BLAKE3_256, codec NONE, `encoded_length == decoded_length`, reserved 0.
   - `decoded_content_hash` is Atlas's own BLAKE3 (`identity::blake3`, ADR 0005) over the payload.
   - Records are ordered by their stable identity bytes, the function id. The contract's further keys (record kind, then record-content hash) never decide here: the object has one top-level kind, and two records with one identity are refused (DUPLICATE_IDENTITY).
   - Fields ascend by tag. Each field's `field_flags` bit 0 is the table's required flag. Parameters keep their order. Lineage is a set: sorted bytewise and deduplicated. An empty optional sequence is an absent field. The same set of records in any input order gives the same bytes.
3. **The address.** An object's address is the lowercase hex of its `decoded_content_hash` (`object_address`). Its file is `<address>.atlasx`. `decode_named` refuses a file named otherwise (ADDRESS_MISMATCH). The digest form is `blake3-256:<address>`.
4. **The reader follows the contract's order for one object.**
   1. The header. Every header defect is reported together, sorted: BAD_MAGIC, BAD_HEADER_LEN, UNSUPPORTED_MAJOR, UNSUPPORTED_MINOR, UNKNOWN_HEADER_FLAGS, CLASS_MISMATCH, UNSUPPORTED_SCHEMA_VERSION, UNSUPPORTED_DIGEST_ALGORITHM (SHA256 is declared but not supported), UNSUPPORTED_CODEC (ZSTD is declared but not supported), RESERVED_NONZERO.
   2. Bounds, before anything the lengths describe is read: LENGTH_LIMIT (over 64 MiB), LENGTH_MISMATCH (encoded and decoded lengths differ under NONE, or bytes follow the payload), TRUNCATED.
   3. The decoded hash: DIGEST_MISMATCH.
   4. Record and field framing, recursively, with no schema: RECORD_FRAMING, FIELD_FRAMING, FIELD_ORDER (tags not strictly ascending, including duplicates), INVALID_WIRE_TYPE, DEPTH_LIMIT (nesting deeper than 8).
   5. The schema: UNDECLARED_RECORD_KIND, UNSUPPORTED_RECORD_VERSION, RECORD_FLAGS, FIELD_FLAGS, UNDECLARED_REQUIRED_FIELD, UNDECLARED_OPTIONAL_FIELD, WIRE_TYPE_MISMATCH, MISSING_REQUIRED_FIELD, MALFORMED_VALUE (not UTF-8, a varint that is not minimal, an unknown dispatch code, a hash that is not 32 bytes, a sequence with no element).
   6. The canonical form. The reader re-encodes what it decoded through the writer and requires the same bytes (NON_CANONICAL). Records out of order and lineage out of order or repeated cannot be accepted. The writer's own refusals apply to what is read by this same rule: EMPTY_OBJECT, DUPLICATE_IDENTITY, and MALFORMED_VALUE for an empty identity or lineage id. There is one rule for both sides, not two copies of it.

   No buffer is sized from a length or a count read from the input. Every length is checked with overflow-safe arithmetic before a slice is taken. Arbitrary input is refused, never a panic. `CodecDefect` and `CodecVerdict` (DECODED, REFUSED) are in the vocabulary map.
5. **Every undeclared field is refused.** The contract lets a reader skip an unknown optional field and requires it to reject an unknown required one. Here minor 0 is the only minor, and the reader refuses any other minor. So no field outside the table can be a compatible extension, and skipping one would accept bytes that the object's identity covers but no reader understands. The reader therefore refuses every undeclared field: UNDECLARED_REQUIRED_FIELD when `field_flags` marks it required, UNDECLARED_OPTIONAL_FIELD otherwise. A later minor that adds optional fields must change this rule and the re-encode check together.
6. **Every result says what it does not verify** (`CODEC_NOT_VERIFIED`):
   - the root manifest and this object's entry in it;
   - the AtlasX root identity and parent compatibility;
   - cross-object references: owner names and type spellings are carried, not resolved;
   - lineage against the parent Atlas root;
   - that the records materialize an admitted parent;
   - function bodies;
   - every other class, ZSTD and SHA256;
   - the source module's inputs against the container it names, and its lineage against the census and the parent Atlas.

   DECODED means only that the bytes are the canonical FUNCTIONS object they claim to be.
7. **Runtime and CLI.** `runtime::atlasx::encode_module` and `decode_object` read and write the files. `decode_object` reads at most a header, the largest payload and one byte more (`OBJECT_READ_LIMIT`), so an over-long file is refused LENGTH_MISMATCH without being read whole.

   - `atlas-systemizer atlasx codec --encode <module.json> --out <dir>` writes `<dir>/<address>.atlasx` and prints the address and digest. A refused module prints `{"verdict": "REFUSED", "defects": [...]}` and writes nothing.
   - `atlas-systemizer atlasx codec --decode <file> [--out <report.json>]` prints the typed verdict.
   - Either refusal exits `ATLASX_CODEC_REFUSED: <n> defects`. As in §4, the defects are every header defect together, and otherwise the first defect found.

## Evidence

- **Fixture** (`core/src/atlasx/codec/`). `fixture_module.json` is the SR1-3 construction module (G181, `evidence/self-reconstruction/SR1/attempt3/module.json`), copied verbatim: one function, `MaterializationMode::is_local`. It encodes to the committed `fixture.bin` (465 bytes) byte for byte. The fixture reads back to the same record, and writing that record again gives the same bytes. Its address is `5b8223605edbd24c18a1033e9007ffef0c7d7d8385a56a4ce6ca3cf92a1241ed`. The native BLAKE3 of the payload equals the reference `blake3` crate's (the test-only oracle of ADR 0005). An independent parse of the bytes (outside Rust) matches the contract's offsets.
- **Falsification** (25 core codec tests, 1 runtime test, 1 CLI test):
  - every single-byte flip of the fixture (465 positions, 3 masks each) is refused with exactly its defect: DIGEST_MISMATCH for every payload byte and for the hash field (bytes 40 to 71), and each other header field's own defect for the rest of bytes 0 to 79;
  - every truncation is refused TRUNCATED, and any extension LENGTH_MISMATCH;
  - an undeclared field is refused before the first tag, after the last and inside an embedded record, whether marked required or optional;
  - out-of-order and duplicate tags, wrong and invalid wire types, wrong field flags, missing required fields, undeclared, versioned and flagged record kinds, and malformed values are each refused with their defect;
  - an unknown major, a nonzero reserved field, codec ZSTD, digest SHA256 and every other class are refused;
  - lengths up to `u64::MAX` are refused LENGTH_LIMIT without reading, and nesting deeper than 8 is refused DEPTH_LIMIT;
  - records or lineage out of order are refused NON_CANONICAL, and a repeated identity DUPLICATE_IDENTITY;
  - all 24 permutations of a four-record object encode identically;
  - a forged source module is refused before anything is written: a forged id, an edit under the old id, lineage outside its inputs, no inputs, an empty name;
  - an object file one byte past the read limit is refused LENGTH_MISMATCH from exactly its first `OBJECT_READ_LIMIT` bytes;
  - 4,000 rounds of pseudo-random bytes and edited payloads (raw, and re-hashed behind a valid header) are refused or read back canonically, never a panic.
- **Through the CLI.** The module is encoded to its address and decoded. A flipped byte exits `ATLASX_CODEC_REFUSED` with DIGEST_MISMATCH. The same bytes under another name exit with ADDRESS_MISMATCH. A forged module is refused and nothing is written.
- **Mutation testing** (`evidence/verification/G183-atlasx-codec.json`). The final record has 56 hand mutants over the header, bounds, digest, framing, schema, canonical-form, writer, address, source-module, bounded-read and CLI rules. 55 were killed. The survivor is equivalent: dropping the writer's field sort leaves the bytes unchanged, because the writer already emits fields in ascending tag order. The reader enforces the order independently.
  - The first run left three more survivors. They were reader copies of the writer's empty-object, duplicate-identity and empty-identity checks, which the re-encode step already applied with the same defect. The copies were removed.
  - Two off-by-one mutants of the header bounds were first killed only by the random test. Each now also has an exact-boundary test.

## Consequences

- **M12 is EXISTS for one class.** One FUNCTIONS object, carrying function signatures from the construction IR, is written canonically, addressed by its BLAKE3 digest and read back fail-closed. The other 20 classes (ROOT_MANIFEST included), function-body records, ZSTD and SHA256 stay unimplemented and are refused.
- **M11 and M13 stay MISSING.** No manifest, AtlasX root identity, object inventory, cross-object reference closure or parent lineage check exists. A DECODED object is not a validated AtlasX root, and nothing here may be reported as one.
- **The GLOBAL_ID encoding is provisional.** An IR id is carried as its UTF-8 bytes. The contract leaves the GLOBAL_ID byte form to the semantic schema, and the materializer (M11) may fix another form. That form would be a new object schema version, not a reinterpretation of schema 1.
