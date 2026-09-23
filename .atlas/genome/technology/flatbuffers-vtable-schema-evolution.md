---
id: atlas.genome.technology.flatbuffers-vtable-schema-evolution
type: technology-genome
status: active
canonical: true
---
# Technology Genome: vtable-indirected fields with implicit schema evolution (FlatBuffers)

Donor: FlatBuffers (`google/flatbuffers`, commit `b8431fbcd7a5c71817f314e18b332c0648554efa`), Wave 6
lane.

This record follows the mandatory schema in `.atlas/contracts/DONOR-TO-LANGUAGE-GENESIS.md`. Like
the rkyv genome record, it restructures an already-strong existing deep census
(`.atlas/census/donors/flatbuffers.md`, which reads `docs/source/internals.md`,
`docs/source/evolution.md`, and the Rust crate's `vtable.rs`/`verifier.rs` directly, cross-checks
against Cap'n Proto's independently-censused evolution rules, and reaches a disposition table)
into the canonical genome shape, bounded to its single most load-bearing mechanism.

## Capability / problem

Given a binary record format, let old code read data written by a newer schema and new code read
data written by an older schema, **without a version number anywhere in the file**, and without
storing wasted space for fields that were never set.

## Semantic mechanism (as observed in the donor, evidence already gathered in the existing census)

- **Tables are reached by offset and prefixed with a vtable offset**, never laid out with a fixed
  field-to-byte-offset mapping the way a C struct or FlatBuffers' own `structs` are. The vtable
  itself is a small `[u16]` array: `[vtable_byte_size, object_byte_size, field_0_offset, ...]`.
  Verified directly against `rust/flatbuffers/src/vtable.rs`: `num_fields()` computes
  `(vtable_byte_size / SIZE_VOFFSET) - 2`.
- **One bounds check *is* the entire compatibility mechanism, in both directions.** Every generated
  accessor's field index is checked against the vtable's own recorded field count before reading:
  index out of range → field absent → return the schema's default. Old code reading new data:
  new fields the old code doesn't know about are simply never accessed. New code reading old data:
  a field the old data's vtable doesn't include reads as its default via the same bounds check.
  FlatBuffers' own docs state this explicitly (quoted in the existing census): *"the format itself
  does not need a version number ... versioning is something that is intrinsically part of the
  format."*
- **Normative evolution rules that make the mechanism safe** (`docs/source/evolution.md`, read in
  full by the existing census): fields must be appended, never removed (deprecate + stop writing
  instead); renames are free (names aren't on the wire); a field's default value must never change
  once shipped. A dedicated `flatc --conform` tool mechanically enforces these rules against a base
  schema.
- **Sparse storage as the direct consequence**: because absence is "vtable entry says 0 / entry
  doesn't exist," an unset optional field costs nothing on the wire beyond the (shared,
  deduplicated) vtable entry — a different trade-off from Cap'n Proto's approach (store every
  field's slot, rely on packing/compression), independently confirmed by the existing census's
  cross-reference to that donor's own record.

## Required invariants

- The bounds-check-implies-default rule must be enforced at **every** field read, unconditionally
  — this is not an optimization that can be selectively skipped, since skipping it is exactly what
  would turn "old data missing a new field" into an out-of-bounds read instead of a safe default.
- Schema evolution safety depends entirely on the append-only/never-reuse-a-slot discipline being
  followed by whatever generates or hand-authors the schema — the mechanism enforces *reading*
  safety, not *authoring* discipline; `flatc --conform` is the donor's own answer to that gap
  (mechanically checking authoring discipline rather than trusting it).

## Identity/scope model

Not applicable to this specific mechanism — this genome record is about field presence/versioning,
not record identity. (Consistent with the pattern across all three genome records so far: identity,
cryptographic content-addressing, and layout/evolution are three genuinely separate concerns this
session has now recorded independently rather than conflating.)

## State/effect/resource model

Pure read-side transform: `&[u8]` + a byte offset → field value or default, via one bounds check.
No allocation, no mutation, on the read path. The write side (`builder.rs`) is a separate, stateful
accumulator (vtable deduplication requires remembering previously-emitted vtables to detect
matches) — not evaluated in depth here; the existing census flags `builder.rs` as only partially
read.

## Failure and recovery behavior

Directly relevant to untrusted input: the existing census's read of `verifier.rs` (629 lines) found
a real, worth-recording design split — a cheap, `unsafe`, precondition-documented fast
vtable-construction path (`VTable::init`) kept separate from a dedicated safe verification layer
with a typed `InvalidFlatbuffer` error enum. That verifier distinguishes, *in source*, between
richly-detailed errors for ordinary data mistakes (`MissingRequiredField`, with a full
`ErrorTrace`/field-name/position) and deliberately minimal-detail errors for inputs that look
adversarial — to avoid the error trace itself becoming a resource-exhaustion vector on hostile
input. This is a directly reusable idea for any future Atlas binary-format reader, independent of
whether Atlas ever adopts FlatBuffers' specific vtable format.

## Concurrency/temporal behavior

Read access requires no synchronization (immutable bytes, no shared mutable state) — same property
already recorded for rkyv's relative-pointer mechanism; two independently-censused donors
converging on "read-only zero-copy formats are trivially thread-safe to read from" is itself a
useful, corroborated fact worth carrying forward rather than assuming per-format.

## Performance characteristics

Not benchmarked. The existing census records the named, quantified trade-off directly from the
donor's own comparison surface (cross-referenced via the rkyv census's citation of rkyv's own
feature-comparison table): FlatBuffers trades one vtable-indirection hop per field access for
genuinely-zero wire cost on unset fields, versus formats that store every field's slot and rely on
compression.

## Portability/ABI constraints

The Rust crate (`rust/flatbuffers/src/`) is a **from-scratch reimplementation**, not an FFI wrapper
over the C++ reference implementation — directly relevant precedent: if Atlas wants a native Rust
reader/writer for its own eventual binary format, "reimplement the mechanism natively in Rust
rather than binding a C library" is exactly what this donor's own maintainers chose to do, and it
has a dedicated `no_std`-compilation test (`tests/rust_no_std_compilation_test`) proving the claim
is actually exercised, not merely documented.

## Evidence references

- `.atlas/census/donors/flatbuffers.md` (existing deep census — primary evidentiary source; cites
  `docs/source/internals.md`, `docs/source/evolution.md` (277 lines, read in full), and
  `rust/flatbuffers/src/{vtable.rs, verifier.rs, builder.rs, follow.rs, table.rs}` directly)
- `.atlas/census/donors/rkyv.md` and `.atlas/genome/technology/rkyv-relative-pointer-archiving.md`
  — cross-donor comparison point (see Decision)
- `.atlas/census/donors/capnproto.md` (cited by the existing FlatBuffers census as an independent
  corroboration for the append-only/never-remove evolution rule and the union-discriminant escape
  hatch converging across two unrelated donors)

## Donor revisions/licenses

google/flatbuffers, commit `b8431fbcd7a5c71817f314e18b332c0648554efa`, Apache License 2.0
(`.atlas/licenses/donors/flatbuffers/LICENSE`, full text verified by the existing census — this
record also fixed a data-integrity bug in `donor-corpus.toml` where a duplicated donor entry had
regressed this same license field to the literal string `"UNVERIFIED"`; see the immediately
preceding commit).

## Known trade-offs

- Vtable indirection costs one extra pointer-chase per field read versus a fixed-offset struct
  layout, in exchange for implicit forward/backward compatibility with no version field. Same
  general shape as rkyv's relative-pointer cost-for-safety trade-off, applied to a different
  property (schema evolution vs. mmap-relocation-safety).
- Sparse-storage-of-absent-fields is a real space win only when a meaningful fraction of optional
  fields are actually typically unset; a record type where nearly every field is always present
  gets less benefit and pays the vtable-indirection cost regardless.

## Rejected alternatives (for this pass)

- **Taking `flatc` or the `flatbuffers`/`flatbuffers-derive` Rust crates as an actual dependency**:
  rejected by the existing census, unchanged here — this is architecture study of the binary-layout
  and evolution-rule *design*, not adoption of FlatBuffers' own toolchain. Unlike the BLAKE3 case,
  there is no cryptographic-property argument for depending on the donor implementation here: vtable
  bounds-checking is an ordinary, fully-specifiable, independently-testable mechanism, so the
  "mechanism before syntax, native reimplementation, not donor dependency" default from
  `DONOR-TO-LANGUAGE-GENESIS.md` applies without the BLAKE3-style carve-out.
- Copying FlatBuffers' exact integer widths (`u16` `voffset_t`, etc.) into a future `.atlas` format
  — explicitly rejected by the existing census: the *mechanism* (indirection table +
  bounds-check-implies-default) is the transferable asset, not FlatBuffers' own size/reach-tuned
  integer choices, which would need an independent Atlas-specific sizing decision.
- The full multi-language `flatc` codegen architecture (Java/Go/Swift/Kotlin/Dart/PHP/etc.
  backends) — out of scope; Atlas's own need is a Rust-native reader/writer.

## Dependency/extinction status

`REFERENCE_ONLY` per `donor-corpus.toml`, unchanged by this record. No runtime/build dependency
exists today.

## Decision

**`ABSORB_LATER`**, native-implementation-only, same shape as the rkyv genome record's conclusion
and for the same underlying reason: vtable bounds-checking has no cryptographic content, so a
native Atlas reimplementation with ordinary tests provides adequate assurance without inheriting
FlatBuffers' own C++-first, ~15-language-backend project surface (Atlas needs exactly one backend:
Rust).

**Cross-donor comparison, now that both rkyv and FlatBuffers are genome-captured**: the two donors
solve *different* problems that a future `.atlas`/`.atlasx` format may both need. rkyv's
relative-pointer mechanism answers "how do I make a structure directly usable after an unpredictable
`mmap()`"; FlatBuffers' vtable mechanism answers "how do I let old and new schema versions read each
other's data without a version field." Nothing observed in either census suggests these are
mutually exclusive — a hypothetical Atlas binary record could use relative offsets *for* pointers
(rkyv's idea) while using a vtable-style indirection table *for* optional-field presence/evolution
(FlatBuffers' idea). This combination is not designed or committed to here (that decision belongs
to the actual `.atlas` binary-format work, not to this genome-capture pass), but recording that the
two mechanisms compose rather than compete is itself a useful, durable finding this record makes
explicit for the first time.

Not `ABSORB_NOW`: same reasoning as every genome record so far this session — the real trigger is
the `.atlas`/`.atlasx` binary format work, not yet built.

`flatbuffers`'s `census_status` in `donor-corpus.toml` remains `DEEP_CENSUSED` (this record covers
the vtable/evolution mechanism; `flexbuffers` and the `reflection/` runtime-reflection subsystem,
which the existing census explicitly flagged as inventoried-but-not-deep-censused, remain queued),
with this genome record added as new evidence.
