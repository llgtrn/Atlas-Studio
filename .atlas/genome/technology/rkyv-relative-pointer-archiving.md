---
id: atlas.genome.technology.rkyv-relative-pointer-archiving
type: technology-genome
status: active
canonical: true
---
# Technology Genome: relative-pointer, mmap-safe archived layout (rkyv)

Donor: rkyv (`rkyv/rkyv`, commit `4845668ae9730a3987966769f6872d86b822dc41`), Wave 6 lane.

This record follows the mandatory schema in `.atlas/contracts/DONOR-TO-LANGUAGE-GENESIS.md`. It
restructures the existing, already-strong deep census at `.atlas/census/donors/rkyv.md` (which
already reads rkyv's own book (`book_src/`) directly, cites concrete files, and reaches an
evidence-backed verdict) into the canonical Technology Genome shape, rather than re-deriving the
mechanism from scratch — the underlying evidence-gathering work was already done well; what was
missing was the typed genome record and an explicit `TECHNOLOGY_GENOME_CAPTURED` state transition.

## Capability / problem

Given an in-memory Rust value, produce a byte buffer such that a *different* process (or the same
process after an `mmap()` at an unpredictable address) can use the value directly from the buffer
— no deserialization pass, no pointer-fixup pass — while still being able to validate the buffer's
integrity before trusting it, if the buffer's origin is untrusted.

## Semantic mechanism (as observed in the donor, evidence already gathered in the existing census)

- **Relative pointers, not absolute pointers** (`book_src/architecture/relative-pointers.md`):
  archived structures store byte *offsets* to their referents rather than addresses. An absolute
  pointer only survives a move if the whole structure is rebased together at the exact same
  address; a relative pointer survives itself and its target moving together as a unit — exactly
  what `mmap()` at a fresh address gives you. This is the single mechanism this genome record
  targets; the two-phase `Serialize`/`Resolve` split and the validation/subtree-range-ownership
  model are real, separately-reusable mechanisms already recorded in the existing census but are
  out of scope for *this* record (see Rejected alternatives).
- **Depth-first, leaves-before-root layout**, root at the *end* of the buffer, specifically so no
  separate "root offset" field needs to be stored — the opposite convention from Cap'n
  Proto/FlatBuffers.
- **Derived types are `#[repr(C)]`/`#[repr(N)]`** with every primitive replaced by an
  endian-explicit wrapper — this is what makes the archived bytes directly interpretable without a
  parse step, at the cost of being tied to a specific layout convention (see Known risk below).

## Required invariants

- A relative offset must remain valid under every operation the format's contract permits
  (specifically: moving the *entire* buffer as one unit) and must not be assumed valid under any
  operation it does not permit (partial relocation, splitting the buffer, appending after the
  fact without re-deriving offsets).
- Any consumer of bytes from outside the current process/trust-boundary must validate
  before use (subtree-range-ownership-style bounds checking), matching this session's own
  established `untrusted_input_default_deny`/`ingestion_is_not_execution` invariants
  (`atlas.genome.toml` `[security]`) — the existing census independently arrived at the same
  principle rkyv itself applies, which is corroborating rather than novel evidence.

## Identity/scope model

Not applicable — this is a byte-layout mechanism, not an identity scheme. (Contrast with the
BLAKE3 genome record, which is specifically about content identity.)

## State/effect/resource model

Pure, in-memory transform: input value → output byte buffer (write side) or byte buffer → typed
reference (read side, after validation). No I/O, no global state, in the mechanism itself.

## Failure and recovery behavior

The one real failure mode this mechanism has to defend against is a *hostile* buffer (bytes
supplied by an untrusted party, not merely a corrupted-by-accident one): `bytecheck`'s
subtree-range-ownership validator is the concrete defense — as the walk descends into a subobject,
the "available" byte range shrinks to that subobject's claimed extent and is restored on ascent,
specifically to catch overlapping/aliasing offsets that would otherwise let one claimed subobject's
bytes overlap another's. The existing census independently cross-references this against Cap'n
Proto's own traversal-limit defense against the same attack class (two unrelated donors converging
on "an adversarial pointer graph inside a flat buffer is a real attack surface" is stronger
evidence than either alone).

## Concurrency/temporal behavior

Not evaluated — the mechanism itself (offset addressing) has no concurrency dimension; reading an
archived buffer is trivially safe to do from multiple threads simultaneously since it is read-only
and requires no synchronization, which is itself worth recording as a property, not just an
omission.

## Performance characteristics

Not benchmarked directly for Atlas's purposes. The existing census notes rkyv's own claim (from its
FAQ) that the validated (`bytecheck`) access path is still faster end-to-end than deserializing
with other high-performance formats, despite the extra validation pass — rkyv's own claim, not
independently verified here.

## Portability/ABI constraints

This is the mechanism's real limitation, already established with strong evidence in the existing
census: rkyv's own maintainer-published feature-comparison table (`book_src/feature-comparison.md`)
marks rkyv "no" for schema evolution and "no" for cross-language support, in direct contrast to
both Cap'n Proto and FlatBuffers (both "yes"/"yes"). Combined with the crate being pre-1.0
(`0.8.18`) and its own validation dependency (`bytecheck`) pinned to an unreleased git commit
rather than a release, the existing census's conclusion — rkyv's archived representation is tied to
a specific build/toolchain/version, not a stable long-term interchange contract — is well-evidenced
and this record adopts it unchanged.

## Evidence references

- `.atlas/census/donors/rkyv.md` (existing deep census — primary evidentiary source for this
  record; itself cites `book_src/architecture/relative-pointers.md`, `book_src/format.md`,
  `book_src/validation.md`, `book_src/feature-comparison.md`, and `rkyv/Cargo.toml` directly)
- Cross-referenced against the BLAKE3 genome record's own decision framework
  (`blake3-content-addressing.md`) for the general shape of "when is depending on a donor crate
  directly the right call vs. absorbing only the mechanism" — the two records reach *opposite*
  crate-dependency conclusions for structurally similar reasons (see Decision), which is itself
  useful cross-donor comparison evidence per `DONOR-TO-LANGUAGE-GENESIS.md`'s "Cross-donor
  comparison" section.

## Donor revisions/licenses

rkyv/rkyv, commit `4845668ae9730a3987966769f6872d86b822dc41`, MIT license
(`.atlas/licenses/donors/rkyv/LICENSE`, verified directly by the existing census).

## Known trade-offs

- Relative-pointer addressing costs an extra indirection/arithmetic step per pointer dereference
  compared to absolute pointers, in exchange for mmap-safety; not benchmarked, but the trade-off
  direction (small constant cost for a structural safety property) is the same shape as BLAKE3's
  cryptographic-mixing-cost-for-security-property trade-off.
- The layout is version-pinned by construction (see Portability/ABI constraints) — durability
  requires either pinning rkyv's exact version forever for any persisted bytes, or re-deriving only
  the *relative-pointer idea* natively without depending on rkyv's own evolving derive macro output.

## Rejected alternatives (for this pass)

- The two-phase `Serialize`/`Resolve` construction discipline and the `bytecheck` subtree-range
  validation model are both real, separately-reusable mechanisms the existing census already
  identified (`STUDY`/`ADAPT` dispositions) — deliberately left out of this record's scope, which
  is bounded to the relative-pointer mechanism specifically, so this genome record stays a single
  coherent unit rather than trying to capture three mechanisms at once. They remain durably queued
  (`ABSORB_LATER`) in the existing census's own disposition table, which this record does not
  supersede for those two mechanisms.
- Adopting rkyv itself as the `.atlas`/`.atlasx` portable format: already rejected, with strong
  evidence, by the existing census (see Portability/ABI constraints) — this record does not revisit
  that conclusion, only formalizes the *mechanism-extraction* half of it.

## Dependency/extinction status

`STUDY_ONLY_FOR_PORTABLE_FORMAT / POSSIBLE_FUTURE_CARGO_DEPENDENCY_FOR_INTERNAL_CACHING_ONLY` per
`donor-corpus.toml`, unchanged by this record. No runtime/build dependency exists today.

## Decision

**`ABSORB_LATER`**, native-implementation-only (no crate dependency), for the relative-pointer
mechanism specifically — the opposite dependency shape from the BLAKE3 genome record's conclusion,
and worth stating explicitly why: BLAKE3's value is a *cryptographic property* (collision/preimage
resistance) that cannot be independently re-verified by ordinary tests, so depending on an audited
implementation is the safer engineering choice. Relative-pointer offset addressing is an ordinary,
fully-specifiable data-layout technique with no cryptographic content — Atlas can implement its own
offset type, own `#[repr(C)]` conventions, and own bounds-checked accessor with ordinary unit and
property tests providing adequate assurance, without inheriting rkyv's version-churn/no-schema-
evolution limitation. This is exactly the "mechanism before syntax" principle the contract
describes for LLVM/Wasmtime/Arrow: absorb the *idea* (offsets survive `mmap()` moves; absolute
pointers don't), not rkyv's own derive-macro-generated types.

Not `ABSORB_NOW`: like BLAKE3, the actual trigger is the `.atlas`/`.atlasx` binary format work
(compiler phase 0 today, not yet built) — nothing in current R4 work reads or writes a persisted
binary format yet, so there is no concrete consumer for this mechanism today. Recording it now,
evidence-backed, means a future generation building the real format does not have to rediscover
"use relative offsets, not absolute pointers, for anything meant to be memory-mapped" from scratch.

`rkyv`'s `census_status` in `donor-corpus.toml` is left as `DEEP_CENSUSED` (this record covers one
mechanism; the two-phase construction and validation mechanisms the existing census also
identified remain queued, not genome-captured, in this pass), with this genome record added as new
evidence.
