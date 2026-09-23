---
id: atlas.genome.technology.capnproto-canonicalization-and-untrusted-reader-safety
type: technology-genome
status: active
canonical: true
---
# Technology Genome: canonical-form rules and untrusted-reader safety discipline (Cap'n Proto)

Donor: Cap'n Proto (`capnproto/capnproto`, commit `7fff7b6482a19bdedb18884d13f66a1a4f5e5650`), Wave 6
lane.

This record follows the mandatory schema in `.atlas/contracts/DONOR-TO-LANGUAGE-GENESIS.md`,
restructuring the existing deep census (`.atlas/census/donors/capnproto.md` — verbatim-quoted,
line-ranged citations from `doc/encoding.md`/`doc/language.md`, already the strongest census of the
four binary-format donors examined this session) into the canonical genome shape. Bounded to the
census's own explicitly-named "single most directly relevant" mechanism (canonicalization) plus the
untrusted-reader security discipline, which is inseparable from it in the donor's own documentation.
This is the fourth and, for this session, final binary-format genome record; see Decision for how
it completes the cluster with BLAKE3, rkyv, and FlatBuffers.

## Capability / problem

Two problems the donor solves together, deliberately, because they compose: (1) given a message
that a schema-evolution-tolerant encoder may have produced in more than one valid byte sequence for
the *same logical content*, deterministically reduce it to exactly one canonical byte sequence
(needed for content-hashing/signing to be meaningful at all); (2) given bytes from an untrusted
source, read fields from them directly (no upfront parse pass) without a hostile pointer graph
being able to force unbounded work or a stack overflow.

## Semantic mechanism (as observed in the donor, evidence already gathered in the existing census)

- **Canonical form is a property of the wire format, not a mode an encoder opts into**: the existing
  census (quoting `doc/encoding.md`'s Canonicalization section directly) records five precise rules
  — preorder pointer traversal, single-segment only, trailing-zero-word truncation on struct
  data/pointer sections (and per-column for struct lists), offset `-1` for zero-sized structs, and
  canonical messages are never packed. Rule 3 (trailing-zero truncation) is the mechanism's real
  insight: because every field is stored XOR'd against its schema default (so "unset" is always
  all-zero), **adding a new field to a struct does not change the canonical encoding of old
  messages that never set it** — canonicalization and forward-compatible schema evolution are
  designed as one mechanism, not two mechanisms that happen not to conflict.
- **A historical, named failure mode establishes why the rule must be a format-level prohibition,
  not encoder discretion**: prior to Cap'n Proto 0.5, struct lists could use a narrower element
  encoding for compactness; this made schema-free canonicalization impossible, so 0.5 forced the
  composite (C=7) encoding for *all* struct lists, accepting a real size cost to restore a
  cross-cutting property. This is the single most transferable lesson in this record: **an
  optimization that breaks canonicalization must be forbidden by the format specification, not left
  as an implementation choice** — otherwise a well-intentioned future optimization silently breaks
  Atlas's own content-hashing guarantee, discovered only after the fact.
- **Untrusted-reader safety is the other half of the same design**, because canonicalization's
  benefit (cheap, direct field access with no upfront parse) is exactly what a hostile pointer graph
  can abuse: (a) pointer validation is *lazy* — checked per-getter-call, not via an upfront scan,
  to preserve the O(1)-access property; (b) a **traversal-limit** counter is incremented by the size
  of every dereferenced pointer's target and capped (64 MiB in the C++ reference), specifically
  because cyclic/overlapping/shared pointers can make a tiny message expand to unbounded apparent
  work — with a named, non-obvious special case (zero-sized elements charged a minimum of one word
  each, traced to a specific fixed upstream vulnerability, not a theoretical concern); (c) pointer
  **nesting depth** is separately tracked and capped (64 in the C++ reference) to stop
  recursive-descent readers from being driven into a stack overflow by adversarially deep nesting.

## Required invariants

- Canonical-form determinism: the same logical content must always canonicalize to the same bytes,
  regardless of which conforming encoder produced the original (possibly non-canonical) message —
  this is the entire point, and it only holds if every optimization the format spec permits is
  provably canonicalization-preserving (the 0.5 struct-list lesson above is the cautionary
  counter-example).
- Traversal-limit and depth-limit checks must be enforced on **every** pointer dereference during
  reading from an untrusted source, with no path that skips them for a "trusted-looking" pointer —
  the two limits exist specifically because a hostile message is, by construction, not going to
  announce itself.

## Identity/scope model

Directly relevant to `core::identity`'s eventual role once a real `.atlas` binary format exists:
canonicalization is the prerequisite that makes "hash the bytes and call it the artifact's content
identity" a meaningful, collision-safe operation at all — without it, two byte-identical-in-meaning
messages could hash differently purely due to encoder-choice noise (padding placement, segment
splitting, packing), which would make BLAKE3's collision-resistance property (see the BLAKE3 genome
record) moot: a cryptographically strong hash of an under-specified byte layout still gives
different hashes for the same logical content.

## State/effect/resource model

Canonicalization itself is a pure transform (arbitrary valid message → canonical bytes). The
traversal-limit/depth-limit counters are the one piece of *mutable* state in this mechanism: a
per-read-operation counter that must be threaded through every recursive descent into the message —
worth flagging because it is easy to accidentally reset or fail to propagate such a counter across
an API boundary (e.g. a helper function that recurses without receiving the parent's remaining
budget), silently reintroducing the amplification vulnerability the counter exists to prevent.

## Failure and recovery behavior

Both limits are designed to fail *closed*: exceeding the traversal limit or the nesting-depth cap
is documented as an explicit, recoverable read error, not a panic or unbounded resource consumption
— directly consistent with this session's own `untrusted_input_default_deny` posture
(`atlas.genome.toml` `[security]`), independently arrived at by an unrelated donor for an unrelated
reason (network/IPC message safety, not repository census), which strengthens rather than merely
repeats the principle.

## Concurrency/temporal behavior

Not evaluated in this pass — canonicalization and reading are single-message, single-thread
operations in the donor's own documentation; no concurrency dimension was identified as essential
to this specific mechanism.

## Performance characteristics

Not benchmarked. The existing census records the donor's own quantified trade-off: forcing
composite-list encoding for canonicalizability cost real wire-size compactness relative to the
pre-0.5 design — a concrete, donor-acknowledged example of paying a measurable performance cost to
restore a correctness property, worth citing directly if a future Atlas format-design discussion
needs precedent for "yes, this canonicalization rule costs bytes, and that's the right trade."

## Portability/ABI constraints

Not evaluated — the existing census explicitly notes no Rust implementation exists in this donor
(`capnproto-rust` is a separate, un-donated project), so no Rust-specific ABI lessons could be drawn
here; FlatBuffers' and rkyv's genome records carry that responsibility for this cluster instead.

## Evidence references

- `.atlas/census/donors/capnproto.md` (existing deep census — primary evidentiary source; quotes
  `doc/encoding.md` (428 lines, read in full) and `doc/language.md` lines 714-813 directly)
- `.atlas/genome/technology/rkyv-relative-pointer-archiving.md` and
  `.atlas/genome/technology/flatbuffers-vtable-schema-evolution.md` — both already independently
  cite Cap'n Proto's evolution/canonicalization rules as cross-donor corroboration before this
  record formally captured Cap'n Proto's own genome; this record closes that citation loop.
- `.atlas/genome/technology/blake3-content-addressing.md` — canonicalization is the missing
  precondition that BLAKE3's genome record assumed without stating (see Identity/scope model above).

## Donor revisions/licenses

capnproto/capnproto, commit `7fff7b6482a19bdedb18884d13f66a1a4f5e5650`, MIT license
(`.atlas/licenses/donors/capnproto/LICENSE`, full text verified by the existing census).

## Known trade-offs

- Canonicalization-preserving encoding rules (forced composite-list encoding, single-segment
  requirement) cost real wire compactness versus an encoder free to choose the most compact valid
  representation — the donor's own historical 0.5 change is direct evidence this trade-off was
  judged worth making for a real, shipped format.
- Lazy pointer validation trades a larger, harder-to-reason-about validation surface (every getter
  is a potential validation site) for the format's headline O(1)-access performance property,
  compensated for by the traversal-limit/depth-limit discipline rather than by upfront validation.

## Rejected alternatives (for this pass)

- Adopting Cap'n Proto's exact bit-widths, tag values, or pointer encoding — explicitly rejected by
  the existing census: the *principles* (ordinal-based field identity, preorder canonicalization,
  forbidding non-canonicalizable optimizations, lazy validation with dual limits) are the
  transferable asset, not the specific 2-bit-tag/30-bit-offset layout, which was tuned for Cap'n
  Proto's own 32-vs-64-bit segment-addressing constraints.
- The full RPC/capability layer (`rpc.capnp`, `persistent.capnp`, promise pipelining) — out of
  scope, not evaluated, explicitly flagged as a gap rather than silently assumed irrelevant.
- Taking `capnp`/`kj` as a runtime or build dependency — rejected; this is format-design study, and
  unlike BLAKE3 there is no cryptographic-property argument here either (canonicalization rules and
  traversal-limit accounting are both ordinary, independently-testable logic).

## Dependency/extinction status

`STUDY_ONLY_NOT_A_DEPENDENCY` per `donor-corpus.toml`, unchanged by this record. No runtime/build
dependency exists today.

## Decision

**`ABSORB_LATER`**, and specifically: absorb this as a **design-rule constraint on Atlas's own
future `.atlas`/`.atlasx` format specification**, not merely as an implementation technique — this
is a different absorption shape than the other three records in this cluster. BLAKE3 contributes
"what hash function," rkyv contributes "how pointers survive relocation," FlatBuffers contributes
"how optional fields stay compatible across schema versions"; Cap'n Proto contributes the
*governing rule* that ties them together: whatever specific layout `.atlas` ends up using, its
specification must name, explicitly and in advance, which encoder freedoms are forbidden because
they would break canonical-form determinism — chosen post hoc (Cap'n Proto 0.5's own retrofit) is
possible but costly and a real historical near-miss, not a hypothetical risk.

Combined with the traversal-limit/depth-limit discipline (needed regardless of which specific
layout is chosen, since any pointer-based format read from untrusted bytes has the same
amplification/recursion attack surface), this record's practical output is a checklist for the
actual `.atlas` binary-format specification work, once it starts: (1) canonical form must be
defined precisely enough that two conforming encoders of the same logical content always agree
byte-for-byte; (2) every format-level optimization must be checked against that definition before
being permitted, not after; (3) any untrusted-source reader must enforce a traversal-limit and a
nesting-depth cap on every pointer dereference, both failing closed with a typed, recoverable error.

Not `ABSORB_NOW`: same reasoning as the rest of this genome cluster — the trigger is the
`.atlas`/`.atlasx` binary format work itself, not yet built (compiler phase 0).

`capnproto`'s `census_status` in `donor-corpus.toml` remains `DEEP_CENSUSED` (this record covers
canonicalization and untrusted-reader safety; the RPC layer and the C++ implementation's actual
behavior, as opposed to its documented contract, remain explicitly un-censused per the existing
census's own "Known Risks/Gaps" section), with this genome record added as new evidence.

This completes a coherent four-record binary-format genome cluster (BLAKE3, rkyv, FlatBuffers,
Cap'n Proto) for this session — all four independently converge on "defer implementation to the
actual `.atlas` format work," and this record is the first to name the cross-cutting rule (governed
canonicalization) that the other three's mechanisms must each be checked against once that work
begins.
