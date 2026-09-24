---
id: atlas.decision.0005.native-blake3-content-digest
type: decision
status: accepted
canonical: true
---
# ADR 0005 — Native BLAKE3 content digest for inventoried artifacts

## Context

R5 requires "incremental recensus of changed source and affected dependents"
(`.atlas/roadmap/SELF-BUILDING-R4-R8.md` § R5). Generation G35 narrowed the salsa donor's blocking
question to its first executable prerequisite: Atlas cannot tell that a file changed at all.
`ArtifactRecord` carried a path, a size and a disposition, never a content identity; the only hash in
`core` is `stable_id`'s FNV-1a, which is an identity *label* for paths and composite keys, not a
collision-resistant content digest, and must never be used for change detection (a crafted or
accidental collision would make a changed file look unchanged). `IntegrityDigest` existed as a type
but was never constructed anywhere.

The 2026-09-21 genome record (`.atlas/genome/technology/blake3-content-addressing.md`, "Rejected
alternatives" and "Decision") rejected a native reimplementation of BLAKE3's compression function and
prescribed the vetted `blake3` crate as an ordinary dependency. Its reasoning, preserved there
verbatim, was: (1) timing side channels and constant-time comparison bugs in cryptographic code;
(2) correctness defects "that cannot be caught by ordinary unit tests". It also left the trigger at
the binary-format wave.

G36's adversarial security lane re-examined that reasoning against the actual scope needed here and
returned `JUSTIFIED_WITH_CONDITIONS`:

- Concern (1) applies to **secret inputs**: BLAKE3's keyed-hash and derive-key modes. The inventory
  digest hashes public repository bytes with the unkeyed mode; there is no secret to leak, and
  BLAKE3's add-rotate-xor design has no secret-indexed table lookups. The concern stands for the
  keyed/derive modes, which therefore stay outside Atlas.
- Concern (2) is about properties unit tests cannot observe (constant-time behaviour, resistance to
  cryptanalysis). Functional correctness of a deterministic public function is observable: every
  official vector, every update split, and differential equality against the independently
  written, SIMD-optimized upstream crate over boundary-clustered and random inputs up to 1 MiB,
  including a chunk counter beyond 2^32 that no official vector reaches.
- `.atlas/contracts/ATLAS-BINARY-WIRE-FORMAT.md` § Algorithm identifiers already states that
  "Atlas-native or independently maintained implementations may replace bootstrap dependencies while
  preserving wire semantics".
- `core` is pure vocabulary with `serde` as its only dependency; the crate would bring a C/assembly
  build step (`cc`), `cpufeatures`, `arrayvec`, `arrayref` and `constant_time_eq` into it.

## Decision

1. **`core::identity::blake3`** — a section-by-section transcription of the **hash mode only** of
   the BLAKE3 reference implementation (`BLAKE3-team/BLAKE3@6aab490a`,
   `reference_impl/reference_impl.rs`, CC0-1.0 OR Apache-2.0): `Hasher::{new, update, finalize,
   finalize_xof}` and `hash`. The keyed-hash and derive-key flags, key schedules and constructors
   are absent, and a self-scan test fails if any of them appears in the module's non-test source.
2. **`IntegrityDigest`** is validated: the only accepted spelling is `blake3-256:` followed by exactly
   64 lowercase hex digits, enforced on construction and on deserialization, and it maps one-to-one
   onto wire `digest_algorithm = 1`.
3. **`ArtifactRecord.content_digest: Option<IntegrityDigest>`** plus `content_digest_withheld`
   (inventory schema `atlas.inventory-report.v2`), computed by `adapter::source::classify_file`
   through **one** file handle: an `lstat` pre-check, an `fstat` post-open check that the handle is
   still a regular file and (Unix) the same device/inode that was listed, a read bounded by
   `take(len + 1)`, and a 256 MiB cap. An over-cap file, a replaced file, or a file whose read
   length differs from its `fstat` length gets **no digest** and a reason. `None` means "treat as
   changed", never "unchanged". The binary sniff reuses the same read's first 8 KiB.
4. The upstream `blake3` crate is a **`[dev-dependencies]`-only** differential oracle in `core`,
   never a production dependency.

## Consequences

- Atlas owns and executes the content-digest capability; the donor source is not needed at build,
  run or test time (official vectors are pinned in `core/src/identity/blake3_vectors.in` and
  `blake3_xof_vectors.in`; the independent oracle comes from crates.io).
- Throughput: the portable native hash runs at about 450 MiB/s in release on the development
  container, against about 4 GiB/s for the SIMD crate. Inventory is I/O-bound and debug-profile
  `systemize --root .` wall time is unchanged (~2 min). SIMD/rayon parallelism is not absorbed; if a
  measured workload ever needs it, that is a new decision.
- Secret-input hashing (keyed MACs, key derivation) remains `EXTERNAL_BOUNDARY`: any future need
  goes through a vetted implementation, never through `core::identity::blake3`.
- Not yet done, and required before any cache reuses a derivation keyed on this digest (recorded
  on salsa's blocking question): the bytes a parser consumes must be the bytes that were digested
  (today the semantic extractor re-reads the file), and a cache key must also include the
  extractor identity/version.
- `stable_id` (FNV-1a) is unchanged and keeps its identity-label role.

## Supersedes

The genome record's "Rejected alternatives" bullet on reimplementing BLAKE3 natively, **for the
unkeyed hash mode only**. The genome record keeps its original text, with a supersession note
pointing here; its reasoning continues to govern the keyed and derive-key modes.

## Provenance

donor `BLAKE3-team/BLAKE3@6aab490a26124663329dfd3961b8469f8fdb158b`
(`.atlas/provenance/donors/blake3.json`, licenses in `.atlas/licenses/donors/blake3/`) → census
`.atlas/census/donors/blake3.md` → genome `.atlas/genome/technology/blake3-content-addressing.md` →
discovery `.atlas/census/discoveries/r5-content-digest-change-identity.md` → this decision →
`core/src/identity/blake3.rs`, `core/src/identity/mod.rs`, `adapter/src/source/mod.rs` →
evidence `.atlas/evidence/verification/r5-native-blake3-content-digest-absorption.json`.
