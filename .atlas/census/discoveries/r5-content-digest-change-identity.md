---
id: atlas.discovery.r5-content-digest-change-identity
type: discovery-record
status: absorbed
canonical: true
---
# Discovery record: native BLAKE3 content digest as change identity (R5 slice)

Fields follow `.atlas/roadmap/DONOR-ABSORPTION-PLAN.toml`'s `[discovery_record]` schema. Third record
filed under it.

## discovery_identity

`r5-content-digest-change-identity`: the unkeyed BLAKE3 hash mode, owned natively as
`core::identity::blake3`, gives every inventoried regular file a collision-resistant content
identity (`ArtifactRecord.content_digest`). This is the first executable prerequisite for R5's
incremental recensus.

## provider_identity

`blake3` (`BLAKE3-team/BLAKE3`), per `.atlas/references/donor-corpus.toml`.

## provider_revision_or_version

`6aab490a26124663329dfd3961b8469f8fdb158b` (pinned commit, per `.atlas/provenance/donors/blake3.json`).
The independent differential oracle is the published crate `blake3 = "=1.8.7"`, used only as a
dev-dependency.

## provider_scope

The following parts of `reference_impl/reference_impl.rs`, all in hash mode: `g`, `round`,
`permute`, `compress`, `Output`, `ChunkState`, `parent_output`, and `Hasher::{new, update,
finalize_xof}` with its lazy-merge chaining-value stack.

Explicitly **not** in scope:

- `keyed_hash` and `derive_key`. These take secret inputs, and they carry the `KEYED_HASH`,
  `DERIVE_KEY_CONTEXT` and `DERIVE_KEY_MATERIAL` flags. Disposition: `EXTERNAL_BOUNDARY`.
- The optimized crate's SIMD backends, `platform.rs` dispatch, `rayon` join, and `hazmat` subtree
  API. Disposition: `REFERENCE_ONLY`.
- The C implementation and the `b3sum` CLI. Disposition: `REFERENCE_ONLY`.

## evidence_refs

- `.atlas/census/donors/blake3.md`
- `.atlas/genome/technology/blake3-content-addressing.md` (the mechanism, and the superseded
  rejection)
- `.atlas/decisions/0005-native-blake3-content-digest.md`
- `.atlas/evidence/verification/r5-native-blake3-content-digest-absorption.json`
- The G36 security lane's `JUSTIFIED_WITH_CONDITIONS` verdict. Its six conditions are recorded in
  the evidence file.

## root_donor_or_dependency_attribution

This is a root donor. It is not reached through any other donor's dependency graph for this
mechanism.

## atlas_capability_target

- R5: "incremental recensus of changed source and affected dependents".
- The first half of salsa's narrowed blocking question: content-addressed inventory change
  detection.
- `ATLAS-BINARY-WIRE-FORMAT.md` `digest_algorithm = 1 = BLAKE3_256`.

## atlas_native_owner

- `core::identity::blake3`: the hash.
- `core::IntegrityDigest`: the validated digest type.
- `core::census::ArtifactRecord.content_digest`: the digest field.
- `adapter::source::classify_file` / `read_file_content`: the single-handle, bounded read that
  produces the digest.

## disposition

`ABSORB_NOW`, now absorbed at mechanism level. The remaining donor surface is `EXTERNAL_BOUNDARY`
for secret-input modes and `REFERENCE_ONLY` for everything else.

## decision_rationale

Atlas had no content identity at all. The existing FNV `stable_id` is a label and must not be used
for change detection. The genome record rejected a native port because of timing side channels
and cryptographic correctness risk. Both concerns are handled for this scope:

- **Timing side channels:** these only matter for secret-input modes, and those modes are excluded
  and guarded by a self-scan test.
- **Correctness:** the unkeyed function is deterministic and public, so its correctness can be
  observed and was verified three ways:
  - all 35 official vectors (hash and 131-byte extended output), under every update split;
  - differential equality with the independent SIMD crate;
  - a chunk counter above 2^32, compared through the crate's `hazmat` API.

A crate dependency would have added a C/assembly build step to `core`, which is otherwise
serde-only.

## required_semantic_depth

Byte-exact equality with the official vectors and with the independent implementation, including
the extended output.

## prerequisites_or_blockers

None for this slice. The following remain open, carried on salsa's blocking question:

- parsed bytes must equal digested bytes before any cache reuse;
- cache keys must include extractor identity;
- inventory diffing (Created/Modified/Deleted) must exist.

## verification_plan

Executed. The evidence file records:

- 24 mutants, all killed: 13 in the hash core and 11 in the production wiring and parser;
- the production path through `atlas-cli systemize --root .`;
- full workspace gates.

## dependency_removal_plan

None needed. The donor was never a build or runtime dependency. The crate is dev-only, pinned
with `=1.8.7`, and exists solely as a test oracle.

## extinction_implications

The absorbed scope needs no donor source:

- the vectors are pinned in-tree;
- the oracle comes from crates.io;
- no code, test or script references the checkout.

The rest of the donor surface is dispositioned, and its knowledge is kept in the genome and census
records. The checkout is therefore eligible for physical extinction, which is evaluated and
performed as a separate unit.
