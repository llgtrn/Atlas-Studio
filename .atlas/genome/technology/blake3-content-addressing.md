---
id: atlas.genome.technology.blake3-content-addressing
type: technology-genome
status: active
canonical: true
---
# Technology Genome: cryptographic content-addressing (BLAKE3)

Donor: BLAKE3 (`BLAKE3-team/BLAKE3`, commit `6aab490a26124663329dfd3961b8469f8fdb158b`), Wave 6
(binary/storage/data-layout lane) per `DONOR-ABSORPTION-ROADMAP.md`, pulled forward here only for
Technology Genome capture (understanding the mechanism), not for native implementation — see
Decision below for why implementation stays deferred.

This record follows the mandatory schema in `.atlas/contracts/DONOR-TO-LANGUAGE-GENESIS.md`,
advancing BLAKE3 from `COARSE_CENSUSED` (the existing `.atlas/census/donors/blake3.md`, an
admission-stage inventory only) to a real deep-census-grade mechanism record, read directly from
`.atlas/temporary/donors/blake3/src/lib.rs` (not from README/API-shape alone, per the deep-census
gate in `DONOR-ABSORPTION-ROADMAP.md`).

Directly motivated by the prior genome record
(`rust-analyzer-tree-sitter-source-identity.md`), which flagged that Atlas's current
`core::identity::stable_id` (FNV-1a, non-cryptographic) is a forward risk against
`atlas.genome.toml`'s own `[atlas_format] integrity_hashes_required = true` /
`content_addressed_records_required = true` for the eventual `.atlas` binary format. This record
investigates the donor named for exactly that gap.

## Capability / problem

Given an arbitrary byte sequence (a census artifact, a serialized `.atlas` shard, a provenance
record), produce a fixed-size digest such that: two different byte sequences are computationally
infeasible to map to the same digest (collision resistance), the digest cannot be reversed to
recover the input (preimage resistance), and the digest can be produced incrementally (streaming,
without holding the whole input in memory) and, for large inputs, in parallel.

## Semantic mechanism (as observed in the donor)

Read directly from `src/lib.rs`:

- **Chunked Merkle tree, not a single serial pass.** `CHUNK_LEN: usize = 1024` (line 167): input is
  split into 1024-byte chunks; each chunk is hashed independently, then chunk hashes are combined
  pairwise up a binary tree (`parent_node_output`, ~line 1055) to a single root hash. This is what
  makes BLAKE3 embarrassingly parallelizable (`update_rayon`, gated behind the `rayon` feature) and
  why the *tree structure itself*, not just the compression function, is the mechanism worth
  recording — a single-pass hash (what `stable_id`'s FNV-1a is) cannot be parallelized or
  incrementally re-verified the same way.
- **Three distinct, domain-separated modes over the same tree mechanism** (`hash`/`keyed_hash`/
  `derive_key`, lines 967/996/1050): plain hashing, a keyed MAC mode (replaces HMAC, explicit
  warning at line ~974 that MAC comparison must be constant-time), and a key-derivation mode that
  takes a free-form, application-chosen *context string* and mixes it into the tree's initial
  chaining value (`hazmat::hash_derive_key_context`) so the same key material produces independent,
  non-interchangeable subkeys per context. This context-string pattern is directly reusable
  vocabulary for Atlas: a `derive_key("atlas-studio <schema-version> shard-identity", material)`
  call is a ready-made way to make Atlas's own future shard/provider/provenance identities
  cryptographically independent of each other without inventing a new domain-separation scheme.
- **Fixed digest and block sizes**: `OUT_LEN = 32` (256-bit digest), `KEY_LEN = 32`,
  `BLOCK_LEN = 64`. `MAX_DEPTH: usize = 54` documents the tree's own scaling limit
  (`2^54 * CHUNK_LEN = 2^64` bytes — the input-length ceiling the 64-bit chunk counter supports).
- **Extendable-output function (XOF)**: `finalize_xof`/`OutputReader` (line 1730) let a caller
  request more (or less) than 32 bytes of output from the same tree — relevant if Atlas ever needs
  variable-length content identifiers.
- Platform-specific SIMD backends (`avx2`/`avx512`/`neon`/`sse2`/`sse41`/`portable`, selected via
  `build.rs`-set `cfg` flags) implement the actual compression function; this is accidental
  performance-engineering detail, not essential mechanism, per
  `DONOR-TO-LANGUAGE-GENESIS.md`'s "essential vs accidental" framing.

## Required invariants

- Collision/preimage/second-preimage resistance are cryptographic security properties, not
  functional-correctness properties Atlas can verify by writing its own tests. This is the single
  most important fact this record exists to make explicit and durable (see Decision).
- Domain separation: two different context strings (or a keyed vs. unkeyed call) must never be
  able to produce a colliding/related output — BLAKE3 achieves this via distinct internal flag
  words (`KEYED_HASH`, `DERIVE_KEY_MATERIAL`) mixed into the tree, not by convention alone; any
  Atlas usage must preserve using the *mode itself* to separate domains, not string-concatenation
  tricks.

## Identity/scope model

A `Hash` (`[u8; 32]`) is a pure function of (mode, key-or-context, full input bytes) — unlike
`core::identity::ArtifactId` (a function of *path only*) or `ContentFingerprint` (already a
content hash, but FNV-1a and already-adopted for extraction-batch dedup in
`runtime::census::extraction`). This genome record does not propose replacing either existing type;
see Decision.

## State/effect/resource model

`Hasher` (line 1113) is a plain in-memory streaming accumulator (chunk buffer + partial tree state)
with no I/O, no allocation beyond the input(besides SIMD-aligned stack buffers), and no global
state — a pure, side-effect-free data transform. `update_mmap`/`update_reader` (feature-gated) are
the only I/O-touching entry points, and they are thin wrappers that still feed the same pure
`Hasher::update`.

## Failure and recovery behavior

None to speak of at this API layer — hashing cannot fail for well-formed in-memory input; the only
fallible surface is `Hash::from_hex` (parsing a hex string back into a `Hash`, which validates
length and hex-ness).

## Concurrency/temporal behavior

The tree structure is what enables `update_rayon`'s data-parallelism (splitting large inputs across
threads along chunk-tree boundaries) — not evaluated further here since Atlas has no current
multi-threaded census path to plug it into (single-threaded walk, confirmed in the prior genome
record).

## Performance characteristics

Not benchmarked directly; not currently relevant since nothing in Atlas today hashes content large
enough for BLAKE3's parallel/SIMD advantage over FNV-1a to matter (`ContentFingerprint` inputs are
single source files, typically well under a megabyte).

## Portability/ABI constraints

The `c/` directory ships a C implementation with its own CMake build — evidence that BLAKE3 is
designed for cross-language embedding, relevant if Atlas's compiler backend work (`c_default_role =
"ffi_device_os_vendor_boundary"` per `atlas.genome.toml`) ever needs a non-Rust hash implementation
at a native boundary. Not evaluated further; out of scope for R4.

## Evidence references

- `.atlas/temporary/donors/blake3/src/lib.rs` (read directly for this record: lines 151-169 for
  constants, 967-1057 for the three public hash functions, ~1055 for `parent_node_output`, 1113 for
  `Hasher`, 1730 for `OutputReader`)
- `.atlas/census/donors/blake3.md` (existing coarse census — this record supersedes it for the
  content-addressing mechanism specifically, not for BLAKE3's full surface, e.g. the `b3sum` CLI
  and C bindings remain uncensused at this depth)
- `core/src/identity/mod.rs` (`stable_id`, `ArtifactId`) and `runtime/src/census/extraction.rs`
  (`ContentFingerprint`) — Atlas's real current comparison points, read directly

## Donor revisions/licenses

BLAKE3-team/BLAKE3, commit `6aab490a26124663329dfd3961b8469f8fdb158b`. Triple-licensed: Apache-2.0
(`LICENSE_A2`), Apache-2.0 LLVM exception (`LICENSE_A2LLVM`), CC0-1.0 (`LICENSE_CC0`) — permissive
enough for either a vendored dependency or a reference reimplementation; license is not the
constraint here (see Decision).

## Known trade-offs

- BLAKE3 is measurably slower than FNV-1a for tiny inputs (cryptographic mixing has real fixed
  cost); irrelevant for Atlas's current per-file identity use, since no performance problem exists
  today (same conclusion as the prior genome record).
- A 256-bit output is 4x the storage of FNV-1a's 64-bit output; only matters once Atlas has a
  durable on-disk format storing many hashes, which does not exist yet.

## Rejected alternatives (for this pass)

- **Reimplementing BLAKE3's compression function natively in `core`** — *(superseded for the
  unkeyed hash mode by ADR 0005, 2026-09-24; still governs keyed/derive-key modes — see Decision
  update below)* considered and explicitly
  rejected, not merely deferred. This is the one place in this record where `ABSORB_LATER` means
  "absorb the mechanism as *dependency justification*, never as *dependency elimination*." Hand
  reimplementing a cryptographic primitive is a recognized, serious security anti-pattern
  (subtle timing side-channels, constant-time comparison bugs, and correctness bugs in cryptographic
  code are exactly the class of defect that cannot be caught by ordinary unit tests — they require
  the kind of specialized cryptanalysis and audit history the upstream `blake3` crate already has
  and a from-scratch Atlas reimplementation would not). This directly qualifies as an
  `EXTERNAL_BOUNDARY` disposition under `DONOR-ABSORPTION-ROADMAP.md` ("Atlas intentionally retains
  an external capability through an explicit adapter/capability boundary"), not a `REWRITE`: when
  Atlas eventually needs cryptographic content-addressing, the correct native absorption is
  `core::identity` depending on the real, vetted `blake3` crate as an ordinary Cargo dependency
  (a build/runtime dependency on a *cryptographic library*, explicitly distinct from and not a
  violation of `ZERO-RUNTIME-DEPENDENCY.md`'s concern, which is about not depending on *donor
  workbench source trees* like `.atlas/temporary/donors/blake3/` for production execution) — not
  a hand-copied/transpiled reimplementation of `.atlas/temporary/donors/blake3/src/lib.rs`.
- Upgrading `core::identity::stable_id` to BLAKE3 (or any cryptographic hash) right now — deferred,
  not rejected. See Decision.

## Dependency/extinction status

`REFERENCE_ONLY` per `donor-corpus.toml`, unchanged by this record. No runtime/build dependency
exists today. This record does not claim `ABSORBED` or propose immediate `EXTINCT` for the donor
workbench source — see Decision for why deletion is explicitly premature.

## Decision

**`ABSORB_LATER`** for actually adding the `blake3` crate as a dependency and re-deriving
`core::identity`'s hashing strategy — not `ABSORB_NOW`. Per the pull-forward criteria in
`DONOR-ABSORPTION-ROADMAP.md`, `ABSORB_NOW` requires a concrete prerequisite/blocker for active R4
work; nothing in R4 requires cryptographic-strength identity today (same reasoning as the prior
genome record's incremental-census conclusion, applied here to hashing strength instead of
incremental computation). The correct trigger is the `.atlas`/`.atlasx` binary format work itself
(compiler phase 0 today, Wave 6 in the roadmap) — that is where
`integrity_hashes_required`/`content_addressed_records_required` actually become load-bearing.

What *is* recorded now, durably, so it is not rediscovered later: (1) the mechanism (chunked Merkle
tree + domain-separated modes) is understood and evidenced from real source, satisfying the
deep-census gate for this scope; (2) the correct absorption shape when the trigger arrives is an
ordinary vetted `blake3` crate dependency, not a native reimplementation — cryptographic primitives
are the one class of donor technology where "depend on it directly" is the *better* engineering
decision than "rewrite it as Atlas-native source," and `DONOR-TO-LANGUAGE-GENESIS.md`'s general
REWRITE-by-default posture should not be applied here without this explicit carve-out; (3) no
change to `core::identity::stable_id` follows from this record — changing a pervasively-used
internal hash function ahead of an actual consumer that needs its cryptographic guarantee would be
exactly the kind of out-of-sequence, unmotivated change the roadmap's wave discipline exists to
prevent.

`census_status` for BLAKE3 in `donor-corpus.toml` is left as `COARSE_CENSUSED` (this record covers
one mechanism — content-addressing/hashing — not BLAKE3's full surface: the `b3sum` CLI, the C
bindings, and the SIMD platform-detection machinery in `platform.rs` remain uncensused at this
depth), with this genome record added as new evidence.

## Decision update (2026-09-24, G36) — superseded for the unkeyed hash mode

`.atlas/decisions/0005-native-blake3-content-digest.md` supersedes the native-reimplementation
rejection above **for the unkeyed hash mode only**; the text above is kept as the original reasoning.
The trigger arrived earlier than the binary-format wave: R5 incremental recensus needs a
collision-resistant content identity per artifact, and Atlas had none (`stable_id` FNV-1a is a label,
never a change detector).

- The timing-side-channel objection concerns secret inputs. Keyed-hash and derive-key modes stay
  `EXTERNAL_BOUNDARY`; `core::identity::blake3` contains no keyed/derive machinery, enforced by a
  self-scan test.
- The "cannot be caught by unit tests" objection concerns constant-time and cryptanalytic
  properties, not functional correctness of a deterministic public function. Correctness is pinned
  by all 35 official vectors (hash and 131-byte XOF, every update split), differential equality
  against the upstream crate (dev-dependency only), and a >2^32 chunk counter via the crate's
  `hazmat` API.
- `stable_id` is unchanged, as this record's Decision required.

Donor disposition after absorption: hash mode `ABSORBED`; keyed/derive `EXTERNAL_BOUNDARY`; SIMD,
`platform.rs`, rayon, C implementation and `b3sum` `REFERENCE_ONLY`.
