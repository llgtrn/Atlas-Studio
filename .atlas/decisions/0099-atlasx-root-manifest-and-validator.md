---
id: atlas.decision.0099.atlasx-root-manifest-and-validator
type: decision
status: accepted
canonical: true
---
# ADR 0099 — The staged AtlasX root gets a ROOT_MANIFEST and a root identity, and a validator judges it against its parent (G187)

## Context

NA-ATLASX-VALIDATE (G187, CREATION, DEBT-ATLASX) asks for M13, the smallest slice: a ROOT_MANIFEST over the staged objects of a materialization, the AtlasX root identity computed from it, and a validator that follows the contract's reader verification order over one root. It is falsified by the materialized G161 root validated, and by tampered roots refused.

G185 (ADR 0097) left the materializer at publication step 3. It stages one FUNCTIONS object at `functions/<address>.atlasx` and nothing more. ROOT_MANIFEST had no schema (RES-G183-SINGLE-CLASS). The parent root, seal and design an object came from were bound only in the noncanonical JSON result (RES-G185-LINEAGE-ROOT-IN-RESULT-ONLY). A DECODED object was never validated (RES-G183-DECODED-IS-NOT-VALIDATED).

## Decision

1. **The ROOT_MANIFEST object (class 1, schema 1).** `core::atlasx::manifest` writes and reads `manifest.atlasx` through the object codec's own framing: the 80-byte header, record and tagged-field framing, BLAKE3 decoded hash, bounds, and the canonical re-encode rule. It uses its own schema table (`MANIFEST_SCHEMA`), pinned by `schema_digest` to its version. The codec is generalized, not copied:
   - record lookup and the writer take a schema table;
   - the header check takes the class and schema it expects;
   - steps 1 to 3 of the per-object order (header, bounds, decoded hash) are `payload_of`, shared by both readers;
   - a BOOL reader, a required HASH32 reader and a UVARINT writer are added.

   FUNCTIONS bytes, defects and tests are unchanged.

   The object holds exactly one ROOT_MANIFEST record (zero is EMPTY_OBJECT, two or more DUPLICATE_IDENTITY). Every declared field is required, and an undeclared field is refused, as the codec refuses one:

   | tag | field | wire |
   |---|---|---|
   | 1 | atlasx_root_id | HASH32 |
   | 2 | parent_atlas_root_id | HASH32 |
   | 3 | genome_hash (the parent container's, carried) | HASH32 |
   | 4 | selected_design_id | GLOBAL_ID |
   | 5 | materialized_scope_id (`<sealed scope>/FUNCTIONS/design-function-roots`) | UTF8 |
   | 6 | target_kind (`NONE`: M6 is not done, no target is frozen) | UTF8 |
   | 7 | materializer_identity (`atlas_core::atlasx::materialize`) | UTF8 |
   | 8 | materializer_version (`1`, moved with any change to what is written) | UTF8 |
   | 9 | materialization_schema_version (`atlas.atlasx-materialization.v2`: bumped by G187, since the result gained `manifest` and `root_id`) | UTF8 |
   | 10 | compiler_ir_contract_version (`UNKNOWN`: none is defined) | UTF8 |
   | 14 | object_entry, one OBJECT_ENTRY record each, at least one | RECORD |
   | 17 | parent_census_digest | HASH32 |
   | 18 | parent_revision | UTF8 |
   | 19 | parent_seal_id | GLOBAL_ID |

   An OBJECT_ENTRY carries the contract's seven fields: relative_path (UTF8), object_class, object_schema_version, decoded_content_hash (HASH32), decoded_length, required (BOOL: one byte, 0 or 1), logical_record_count. The class and schema must fit a u16. Entries are sorted by class, then decoded hash, then path, and two entries with one path are refused. An empty identity or text field is refused.

   **Declared deviations from `ATLASX-BINARY-WIRE-FORMAT.md`:**
   - **Tags 17 to 19 are new, and required.** The contract's 16 tags have no place for the parent's census digest, revision or seal. Without them, the parent binding the precondition admitted would stay outside the root identity. No existing tag is repurposed.
   - **The repeated tags 11, 12, 13, 15 and 16 are undeclared** (profiles, external bindings, semantic barriers, dynamic obligations, compatibility requirements). This slice materializes none of them, and an absent repeated field is an empty one. A manifest that carries one is refused UNDECLARED_*_FIELD, never read as understood.
   - **Wire types are this schema's choice.** The contract names the fields, not their wire types.
   - **Root identity preimage.** "Fields 2 through 16" is read as the canonical record payload without field 1, so it includes tags 17 to 19. The record header is not in the preimage.
2. **The AtlasX root identity.** `root_identity` is the BLAKE3-256 of the canonical ROOT_MANIFEST record payload with field 1 omitted: every other field, framed, in ascending tag order. BLAKE3-256 is the digest the manifest header declares. Field 1 carries those 32 bytes, and a reader recomputes them. The identity does not depend on the order objects are given in, nor on the root id the manifest carries. Any other field moves it, including an entry's length or required flag, the Genome hash and the revision. The output path is not an input.
3. **Materialization writes the manifest after its objects.** This covers publication steps 1 to 6, in staging.
   - `materialize` builds the manifest (`manifest_for`) once the FUNCTIONS object is staged and its lineage checked. It lists one required entry per object and binds the admitted parent: root, Genome hash, census digest, revision, seal, design and scope. It computes the root identity (steps 4 and 5).
   - `Materialization` gains `manifest` (the staged `manifest.atlasx`) and `root_id`, both only when STAGED.
   - `runtime::atlasx::materialize` writes the objects first, each created new, synced and read back under its address. Then it creates `manifest.atlasx` new, syncs it, and reads it back with a bounded read. The manifest must decode, its fields must give the root identity it carries, and that must be the materialization's (step 6). Any failure removes everything staging created, objects included.
   - **Steps 7 and 8 are not done:** no atomic publish or switch, no advertisement. `MATERIALIZE_NOT_DONE` says so. A staged root is never reported as published.
4. **The validator (`core::atlasx::validate`, M13, FUNCTIONS only).** It is a pure function of the root's files (`RootFiles`) and the parent's inputs (`PreconditionInputs`), in the contract's reader order.
   1. **Limits.** A root the runtime stopped reading is `ROOT_LIMIT`.
   2. **The manifest.** Header, magic and version, then bounds, codec and hash, then framing, schema and canonical form (`read_manifest`). No `manifest.atlasx` regular file is `MANIFEST_ABSENT`. A refusal is `MANIFEST_REFUSED`, carrying the codec's defect. A manifest that does not read stops validation, because nothing it names can be trusted.
   3. **The root identity, recomputed** (`ROOT_ID_MISMATCH`).
   4. **Parent compatibility.** Validation requires the parent's inputs. A root alone cannot show that its parent is admitted, nor that its lineage resolves there, and UNKNOWN is not VALID. The precondition is re-run over those inputs: the G185 admission, with its re-run seal gate, under the repository's declaration. A refused parent is `PARENT_NOT_ADMITTED`, once for each reason. Against an admitted parent, the manifest's binding (parent root, Genome hash, census digest, revision, seal, design, scope) must equal what the materializer writes for it (`PARENT_MISMATCH`, per field). Its materializer, version, schema, target kind and compiler contract must be this materializer's (`SCHEMA_INCOMPATIBLE`).
   5. **Object-entry paths.** An entry's path must be exactly `functions/<decoded hash hex>.atlasx`. A path that is absolute, holds a backslash, or has an empty, `.` or `..` segment is `ENTRY_PATH_INVALID`, and so is one that is not the canonical path. An entry of any class other than FUNCTIONS is `ENTRY_CLASS_UNSUPPORTED`: it is never validated, so it is never VALID. Paths are looked up among the files the runtime listed and are never opened, so a traversal path reaches nothing.
   6. **Presence.** A required entry whose file is absent or not a regular file is `OBJECT_MISSING`. An absent entry that is not required is skipped. An entry of the root the manifest does not list is `UNLISTED_FILE`: a file, a directory holding no listed object (an empty `types/`, a `.git/`), or a name that is not UTF-8. **Deviation:** the contract lets noncanonical debug files coexist. This validator refuses them, so a staged root holds exactly its canonical files and the class directories that hold them.
   7. **Each object**, through `codec::decode_named` under its file name: header, bounds, decoded hash, framing, schema, canonical form and address. A refusal is `OBJECT_REFUSED`, carrying the codec's defect. The entry must describe the object: its schema version, decoded length and record count (`ENTRY_MISMATCH`).
   8. **Identity and lineage.** A function identity carried by two objects is `DUPLICATE_IDENTITY`. Lineage closure against the admitted parent reuses `materialize::check_lineage`: `LINEAGE_OUTSIDE_PARENT` and `LINEAGE_NOT_SELECTED`. Without an admitted parent, lineage is not judged, and the root is already INVALID.
   9. **Cross-object references.** FUNCTIONS has none beyond lineage: owner names and parameter and result type spellings are text, not references.
   10. **Reproduction** (added in the G187 review). This is a same-code, same-process re-materialization, not M11's independent deterministic reproduction (a second, independent materialization, which `MATERIALIZE_NOT_DONE` still lists). It runs last, and only over a root every earlier step accepts. The admitted parent is materialized again in memory: `materialize_over` reuses the admission already decided, so the gate is not re-run and nothing is staged. The root must be exactly that root: every record, every object's bytes and the manifest's bytes. Any difference is `ROOT_NOT_REPRODUCED`, naming the first difference: a record's differing field class (name, owner, dispatch, visibility, documentation, params, result, body_fingerprint, lineage), a record the parent does not materialize, a record the root lacks, an object, or the manifest.

      The earlier, finer defects keep firing first. Reproduction is what refuses a record edited beyond its lineage, re-encoded canonically and re-rooted. Without it, such a root passed every structural and lineage check.

      Cost on the fixture: the reproduction takes 0.42 ms per validation in a debug test build and 0.08 ms in release. A whole validation takes 91 ms and 15 ms, dominated by the precondition's re-run gate and container re-encode.

   The verdict is VALID or INVALID (`ValidationVerdict`), with sorted, typed `ValidationDefect`s. Both enums are in the vocabulary map. `root_id` is reported only when the verdict is VALID. The embedded precondition verdict is absent when the manifest does not read. Every verdict lists what VALID does not claim (`VALIDATE_NOT_VERIFIED`):
   - the precondition's own unverified steps, Genome compatibility among them;
   - dynamic and external boundaries, semantic barriers, profiles and bindings, of which the manifest carries none;
   - compiler-contract compatibility (UNKNOWN, target NONE);
   - cross-object references beyond lineage;
   - that the materializer's own mapping from census records to record fields is right: VALID means the root is byte for byte the one materializer version 1 writes for the admitted parent, and that mapping is checked by the materializer's tests, not independently here;
   - the design's full closure;
   - function bodies;
   - every other class;
   - publication.
5. **Reading a root (runtime).** `read_root` lists the root and one directory level below it.
   - Every entry counts toward the 1,024-entry limit before it is used, directories included. Reading stops past that or past four maximum-size objects of bytes (`ROOT_LIMIT`).
   - A top-level directory is listed as a directory. A link, a special file or a deeper directory is listed as not a file and never followed. A name that is not UTF-8 is listed as such under its lossy spelling, and the validator refuses it.
   - Types and identity are checked when an entry is listed (`symlink_metadata`, never following) and again when it is opened: the opened handle's metadata must be a regular file with the listed device and inode, or the entry is not a file. A file replaced after listing, by a link or by another file, is therefore not read. Each file is read with the codec's bounded read (`OBJECT_READ_LIMIT`).
   - A writer changing the directory while it is read is outside the model. A FIFO swapped in between the two checks can still block the open, since no `O_NONBLOCK` is hand-coded (RES-G187-CONCURRENT-WRITER).
   - VALID judges the bytes read. Link structure, such as hard links from a root's files to files elsewhere, is not judged.
6. **CLI.** `atlas-systemizer atlasx validate --root <staging dir> --atlas <parent> --design <d> --verification <v> --integrity <i> [--root-declared <repo root>] [--out <file>]`.
   - The parent arguments are those of `atlasx precondition` and `atlasx materialize`.
   - The declarations are read from `--root-declared`, the current directory by default, because `--root` names the AtlasX root. `--policy` and `--envelope` are refused, naming `--root-declared`.
   - It prints the typed verdict, or writes it to `--out`, and exits `ATLASX_ROOT_INVALID: <n> defects` unless VALID, as the other parent commands exit REFUSED.

## Evidence

- **The G161 root is VALID.** The fixture is sealed by the real gate for a design signed by a fixture principal with a test key; Atlas signs nothing. It materializes into `functions/6e6af317….atlasx` and an 834-byte `manifest.atlasx`. The validator accepts the root end to end. Its root identity, pinned in `validate::tests`, is the following. It moved from the first pass's `6c091b70…` when `MATERIALIZATION_SCHEMA_VERSION` went to v2, because the result gained `manifest` and `root_id` and the version is manifest tag 9: `blake3-256:7d9e6cf960320931703ad3d9b767af1f188ea866ac2185096c19c349ed463393`.
  - The manifest binds the admitted parent's root, census digest, revision, seal, design, Genome hash (`07…07`) and scope `fixture/FUNCTIONS/design-function-roots`.
  - A second materialization gives the same manifest and root identity.
  - Through the CLI: `atlasx materialize` stages the object and `manifest.atlasx`, and `atlasx validate` prints VALID with the root id and the embedded ADMITTED precondition. An exact copy of the root is also VALID.
- **Tampered roots are INVALID, each with its typed defect:**
  - a flipped payload byte in the object: `OBJECT_REFUSED` (DIGEST_MISMATCH), and every byte of the object flipped: `OBJECT_REFUSED`;
  - a flipped payload byte in the manifest: `MANIFEST_REFUSED` (DIGEST_MISMATCH), with the parent not judged, and every byte of the manifest flipped: `MANIFEST_REFUSED`;
  - a missing object, or a link where it belongs: `OBJECT_MISSING`;
  - an extra unlisted file (`functions/extra.atlasx`, `notes.md`, `types/x.atlasx`): `UNLISTED_FILE`;
  - an object moved to a name that is not its address, the manifest listing that name and hash consistently: `OBJECT_REFUSED` (ADDRESS_MISMATCH);
  - an entry naming another hash than its own: `ENTRY_PATH_INVALID` plus `UNLISTED_FILE`;
  - eight bad paths, each `ENTRY_PATH_INVALID` plus `UNLISTED_FILE`: `../`, `/…`, `functions/../`, `./`, `functions//`, backslashes, `types/`, `.bak`;
  - another class: `ENTRY_CLASS_UNSUPPORTED`;
  - an entry lying about its length, record count or schema: `ENTRY_MISMATCH`;
  - a lineage entry outside the parent, the object and manifest rebuilt canonically and re-rooted: `LINEAGE_OUTSIDE_PARENT`; lineage without the selected root: `LINEAGE_NOT_SELECTED`;
  - two objects with one function identity: `DUPLICATE_IDENTITY`;
  - a manifest naming another seal, design, parent root, census digest, Genome hash, revision or scope, re-rooted: `PARENT_MISMATCH`, one per field;
  - another materializer, version, schema, target kind or compiler contract: `SCHEMA_INCOMPATIBLE`;
  - a manifest re-hashed after editing its root id: `ROOT_ID_MISMATCH`; after editing its seal: `ROOT_ID_MISMATCH` plus `PARENT_MISMATCH`;
  - no manifest, or a manifest that is not a file: `MANIFEST_ABSENT`; a root over the read limits: `ROOT_LIMIT`.
- **A root of a parent that is not admitted is INVALID.**
  - Against the unsealed container: `PARENT_NOT_ADMITTED` (NOT_SEALED); through the CLI, `ATLASX_ROOT_INVALID: 1 defects`.
  - Against the G179 self-certified seal: `PARENT_NOT_ADMITTED` (SEAL_RECORD_NOT_DECIDED).
  - The rest of the root is still judged: a missing object is reported beside it.
  - A root materialized from another admitted parent (the same container sealed for another design) is VALID against that parent and `PARENT_MISMATCH` (design, parent root, seal) against the G161 one.
- **The manifest codec.**
  - The schema table is pinned (`blake3-256:2a37c35f…`).
  - The manifest round-trips, and the root identity is BLAKE3 of the record payload after field 1.
  - Every flipped byte and every truncation is refused.
  - Tags 11 and 13 are refused UNDECLARED_OPTIONAL_FIELD and UNDECLARED_REQUIRED_FIELD.
  - A dropped required field, a 31-byte hash, two or zero records, and an embedded or undeclared kind at the top are refused, as are entries out of order (NON_CANONICAL), a BOOL of 2, empty or two bytes, a class or schema over u16, and a non-minimal varint.
  - The writer refuses no entry, two entries with one path, an empty path, and an empty identity or text field.
- **A record edited beyond its lineage is INVALID** (the review's falsification; before the reproduction step it was VALID). Ten edits were each re-encoded canonically and re-rooted, and each is refused with exactly `ROOT_NOT_REPRODUCED` naming its field class: name, owner, dispatch, visibility, documentation, params, result, body_fingerprint, lineage (another census record of the parent beside the selected root), and the function id (`not a function the parent materializes`).
  - The lineage in reversed order, framed and re-hashed, is refused earlier by the codec (`OBJECT_REFUSED`, NON_CANONICAL).
  - A manifest naming materializer version 2, re-rooted, is refused `SCHEMA_INCOMPATIBLE`.
  - An entry marked not required, with its object present or absent, is `ROOT_NOT_REPRODUCED` (the manifest, or the record the root lacks).
  - The unedited records, re-encoded and re-rooted, give the same bytes and are VALID.
- **No panic on arbitrary bytes.** 4,000 rounds of noise and edits run through the manifest reader: whatever reads is canonical. 600 rounds of garbage manifests, objects and entry names run through the validator: always INVALID, with no root id.
- **The runtime.**
  - A staged manifest that does not read back, that carries another root id than its fields give, or that is not the materialization's, is refused, and staging removes every file, objects included.
  - `read_root` lists one level down and does not follow links: a linked directory and a linked object are not files. It lists a non-UTF-8 name as such. A file replaced after listing (by a link, by another file renamed over it, or by a directory) is not read. It stops at the byte limit and at the entry limit, and more than 1,024 empty directories give `ROOT_LIMIT`.
  - In the validator, an empty `types/`, a `.git/`, a `func/` and a non-UTF-8 name, at the top or in `functions/`, are each `UNLISTED_FILE`.
- **The self scope stays refused.** `atlasx materialize` over the G184 self census container, under the repository's declarations and with the G186 self-scope reports, is refused (`NOT_SEALED`, `DESIGN_ABSENT`) and stages nothing, so there is no self root to validate. `atlasx validate` of an empty directory against it is `MANIFEST_ABSENT`. Atlas itself is not materializable.
- **Mutation testing** (`evidence/verification/G187-atlasx-validate.json`). 41 hand mutants were run on the new code, one at a time, each restored and checked by sha256: 32 on the manifest, root identity, materializer and validator, 7 on the runtime, 2 on the CLI. All 41 were killed. The review added 9 mutants on the reproduction step:
  - drop it;
  - compare only the manifest;
  - compare only object addresses;
  - run it over roots already refused;
  - leave a differing params field unnamed;
  - leave a missing record unnamed;
  - skip the object byte comparison;
  - skip the manifest comparison;
  - accept a differing record.

  8 were killed. The survivor, the object byte comparison, is equivalent because the manifest bytes are compared too. Equal manifests list equal entry hashes, and each object has already decoded under its address name, so the object bytes are equal. It is kept because the review asked for the bytes of every entry to be compared. The 21 earlier mutants on the materializer's manifest and the validator (M12 to M32) were re-run on the final code.

  The second review added 10 mutants:
  - the identity at open not checked;
  - the type at open not checked;
  - identity compared by device only;
  - directories not counted toward the limit;
  - a listing that follows links;
  - a non-UTF-8 name not typed;
  - top-level directories not listed;
  - a non-UTF-8 name accepted;
  - a directory holding no listed object accepted;
  - a directory held by a path that only shares its prefix.

  All 10 were killed. The unlisted-entry, object-presence and entry-limit mutants whose text changed were rewritten against the final code and killed. Those, the reproduction mutants, the runtime mutants and the validator mutants make 42 re-run after the second review. On the final code there are 59 mutants: 58 killed, 1 equivalent (MR7).

## Consequences

- **M13 exists, as a first slice.** A staged AtlasX root of the FUNCTIONS class is validated against its parent in the contract's reader order. VALID means all of the following:
  - the manifest and every object read canonically;
  - the root identity recomputes;
  - the parent is admitted by the re-run precondition and is the one the manifest binds;
  - every entry is at its canonical path, present and described truly;
  - no unlisted file is present;
  - every record's lineage resolves in the parent and includes a selected root;
  - the root is byte for byte the one the materializer reproduces for the admitted parent.
- **MIN_ATLASX is not recorded as reached.** Its mechanism is demonstrated end to end on the test fixture only: the precondition admits, the materializer stages, and the validator accepts the root and rejects every tampered root tested, a re-rooted record edit included.
  - **The seal rests on synthetic inputs:** verification evidence whose producer tool is `fixture`, a hand-set satisfied ADL constraint result, the placeholder census digest `[9; 32]`, and a test-key principal.
  - **The root is narrow:** staged, not published, and FUNCTIONS only, carrying the design's FUNCTION_IDENTITY roots with no bodies, no other class, no profiles, bindings or barriers, and compiler contract UNKNOWN.
  - **As a construction milestone, MIN_ATLASX stays MISSING** until a real repository parent is sealed, admitted, materialized and validated: a declared principal and a SELECTED design over a real census. `.atlas/declared/principals.json` declares no principal, and the self scope is NOT_SEALED and DESIGN_ABSENT. M13 EXISTS, for the FUNCTIONS class.
- **Residuals closed:**
  - RES-G185-LINEAGE-ROOT-IN-RESULT-ONLY: the parent root, census digest, revision, seal and design are canonical manifest fields, bound by the root identity and checked by the validator.
  - RES-G185-STAGED-NOT-PUBLISHED, superseded by RES-G187-STAGED-ROOT-NOT-PUBLISHED: steps 4 to 6 are done, and 7 and 8 remain.
  - RES-G183-SINGLE-CLASS: ROOT_MANIFEST has a schema. The other nineteen classes are RES-G187-FUNCTIONS-ONLY-ROOT.
  - RES-G183-DECODED-IS-NOT-VALIDATED: its trigger, the validator, exists. A DECODED object is judged inside a root, down to reproduction from the parent. RES-G187-VALID-NOT-REPRODUCED, opened in the first pass, is fixed in this generation by the reproduction step. What remains is RES-G187-VALIDATION-BY-REPRODUCTION: fidelity rests on the materializer's own mapping.
- **Residuals opened:** RES-G187-STAGED-ROOT-NOT-PUBLISHED, RES-G187-VALIDATION-BY-REPRODUCTION, RES-G187-FUNCTIONS-ONLY-ROOT, RES-G187-MANIFEST-SCHEMA-DEVIATIONS, RES-G187-UNLISTED-FILES-REFUSED, RES-G187-ROOT-READ-MEMORY, RES-G187-CONCURRENT-WRITER (the report of G187 carries their full text).
