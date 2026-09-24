---
id: atlas.genome.technology.rust-analyzer-tree-sitter-source-identity
type: technology-genome
status: active
canonical: true
---
# Technology Genome: stable source-artifact identity and incremental change tracking

Donors: rust-analyzer, Tree-sitter (Wave 1, source intelligence lane).

This record follows the mandatory schema in `.atlas/contracts/DONOR-TO-LANGUAGE-GENESIS.md`. It
narrows the existing deep-census evidence in `.atlas/census/donors/source-intelligence-lane-2026-09-20.md`
into one bounded mechanism, comparing it against Atlas's real current implementation rather than
asserting the mechanism is new or fully absorbed.

## Capability / problem

Given a repository's source tree, assign every admitted artifact (file, and eventually
sub-file structural unit) a stable identity that:

1. survives re-running the census over an unchanged file (idempotent identity),
2. can be compared across two runs to determine whether a specific artifact changed, and
3. does not require re-deriving every other artifact's identity when one artifact changes.

## Semantic mechanism (as observed in the donors)

- **rust-analyzer** (`crates/vfs/src/lib.rs`): `FileId(u32)` is an *interned* identity assigned by
  a `Vfs` on first observation (`path_interner.rs`: an `IndexSet<VfsPath>` whose insertion index is
  the id; ids are never freed), stable for the process lifetime. Content state is kept beside it as
  `FileState::{Exists(u64 hash), Deleted, Excluded}` (lib.rs:98-106). `set_file_contents`
  (lib.rs:216-235) turns (stored state, new contents) into `Change::{Create, Modify, Delete}`
  (lib.rs:150-157), short-circuiting when the new `FxHasher` hash equals the stored one; an unseen
  path defaults to `Deleted`, so its first contents are a `Create`. Pending changes are merged per
  file between drains (lib.rs:245-278) and `take_changes()` returns an `IndexMap<FileId,
  ChangedFile>` (lib.rs:284). **There is no rename**: a move is `Delete(old id)` + `Create(new id)`
  with nothing linking them. *(Corrected 2026-09-24, G37: this bullet previously claimed
  `take_changes` returns a `Vec`, named the states `Modified`/`Deleted`, and said a rename is
  representable as one identity -- all three wrong against the pinned source.)* The taxonomy and
  decision rule are absorbed as `core::census::delta` (ADR 0006).
- **Tree-sitter** (`lib/src/tree.c`, `lib/src/get_changed_ranges.c`): `ts_tree_edit` shifts existing
  node byte/point ranges by a described edit instead of re-parsing from scratch, and
  `get_changed_ranges` diffs an old and a new tree to report only the byte ranges that actually
  changed — a smaller, structural analogue of "what changed" at the sub-file level.

Both donors solve the same underlying problem (avoid re-deriving everything when only part of the
input changed) at two different granularities: whole-file (rust-analyzer's VFS) and sub-file syntax
node (Tree-sitter's edit/diff).

## Atlas's current mechanism (verified against real code, not assumed)

- `core::identity::ArtifactId` (`core/src/identity/mod.rs`): a stable, content-independent identity
  derived as `stable_id("artifact", relative_path)` — an FNV-1a hash of the *path string*, not an
  interned counter. Two consequences worth naming explicitly (never previously written down):
  1. Because identity is a pure function of path, a **rename is a different `ArtifactId`, not the
     same identity under a new path** — the opposite of rust-analyzer's `FileId`/path decoupling.
     This is a real, current difference in what "identity" means, not a bug: Atlas's census has no
     concept of "the same artifact under a new name" today, and nothing in the R4 contracts
     requires one yet.
  2. `ArtifactId` says nothing about *content* — two different runs where the file's bytes changed
     produce the identical `ArtifactId`. Content identity is instead achieved by the *separate*
     `ContentFingerprint` type (`stable_id("content", source_text)`, used in
     `runtime::census::extraction`), not fused with artifact identity the way rust-analyzer fuses
     `FileId` + content revision inside one `SourceDatabase` input.
- **No incremental change-stream exists.** `adapter::source::visit_inventory` and
  `runtime::census::extraction::extract_semantics` both re-walk and re-parse the *entire* admitted
  tree on every invocation; there is no `ChangedFile`-equivalent, no prior-run snapshot compared
  against the current one, and no sub-file edit/diff mechanism analogous to Tree-sitter's changed-range
  tracking anywhere in the codebase (confirmed by grep: no persisted prior-census state is read by
  `systemize`/`code_analyze`/`graph`). Every census this session, including the ones in this very
  generation, has been a cold full walk.

## Required invariants

- An artifact's identity must never silently collide with a different artifact's identity (already
  enforced for the *separator-collision* class this session — see `escape_identity_field` — but
  `ArtifactId`'s single-field `stable_id("artifact", path)` has no multi-field join to collide in
  the first place, so that specific fix does not apply here).
  `stable_id` is FNV-1a 64-bit, not cryptographic; two distinct paths could in principle collide
  within a 64-bit space, which is an accepted, currently-undocumented bootstrap trade-off (see
  Trade-offs below) — not addressed by this genome record.
- Whatever incremental mechanism Atlas eventually adopts must not weaken
  `CENSUS_PRECEDES_NORMALIZATION`/`UNKNOWN_OR_OVERSIZED_ARTIFACTS_CANNOT_DISAPPEAR`: a changed-file
  fast path must still produce the same typed facts a full re-walk would for that file, or must be
  provably equivalent, never merely faster-and-hopefully-right.

## Identity/scope model

`ArtifactId` is currently repository-root-relative-path-scoped, non-interned, not content-aware.
rust-analyzer's `FileId` is process-lifetime-interned, decoupled from path, and paired with a
separate content/revision axis inside the same `SourceDatabase` row.

## State/effect/resource model

Atlas: none — every run starts from zero prior state. rust-analyzer: `Vfs` holds live in-memory
state (`FileState` per file) that must itself be initialized from a full scan before any diff is
meaningful; the *first* observation of a file is not incremental either.

## Failure and recovery behavior

Not yet applicable to Atlas — there is no persisted incremental state to fail or recover. This is
itself the honest gap: adopting an incremental mechanism would introduce a new failure class
(stale/corrupted prior-state cache) that a full-rewalk design structurally cannot have.

## Concurrency/temporal behavior

Not evaluated in this pass — out of scope for a single-mechanism genome record; would need its own
deep-census pass against rust-analyzer's salsa integration (Wave 3 territory, not Wave 1).

## Performance characteristics

Not benchmarked. Real repository (`Atlas-Studio` itself) census over ~250 source files currently
completes in well under a second per the `systemize` timings observed this session; no performance
problem currently motivates incremental adoption. This weakens, not strengthens, the case for
`ABSORB_NOW` right now (see Decision below).

## Portability/ABI constraints

None — this mechanism is pure in-process data structure design, no ABI surface.

## Evidence references

- `.atlas/census/donors/source-intelligence-lane-2026-09-20.md` (existing deep census, Tree-sitter
  and rust-analyzer sections)
- `.atlas/census/rust-analyzer.md`, `.atlas/census/tree-sitter.md`
- `core/src/identity/mod.rs` (`ArtifactId`, `stable_id`) — read directly for this record
- `runtime/src/census/extraction.rs` (`ContentFingerprint`) — read directly for this record
- `adapter/src/source/mod.rs` (`visit_inventory`) — read directly for this record; confirmed no
  incremental/prior-state read exists

## Donor revisions/licenses

- rust-analyzer: commit `aaddfb73fd95f2c0bf001b474dca91ae28bcce3a`, dual Apache-2.0/MIT
  (`.atlas/licenses/rust-analyzer/`)
- Tree-sitter: commit `5b951eff4f8b1431e933ed0fe45e48fcd4036a38` (license per
  `.atlas/licenses/tree-sitter/`)

## Known trade-offs

- Path-derived, non-interned identity (Atlas today) is simpler and requires zero mutable state, at
  the cost of not representing renames as identity-preserving and not supporting incremental
  recomputation.
- FNV-1a is fast but non-cryptographic; acceptable for internal census bookkeeping today, but this
  genome record flags it as a real, unresolved tension with `atlas.genome.toml`'s own
  `[atlas_format] integrity_hashes_required = true` / `content_addressed_records_required = true`
  for the eventual durable `.atlas` binary format (Wave 6, `blake3`/`rkyv`/`flatbuffers` territory,
  compiler phase 0 today — not yet built, so not yet a live contradiction, but a forward risk worth
  recording rather than discovering later).

## Rejected alternatives (for this pass)

- Importing rust-analyzer's `Vfs`/salsa machinery directly as an Atlas dependency: forbidden by
  `runtime_dependency_status = REFERENCE_ONLY` in `donor-corpus.toml` and by
  `DONOR-TO-LANGUAGE-GENESIS.md`'s anti-pattern list ("wrapping donor APIs and calling the wrapper
  Atlas-native").
  Not evaluated as a real candidate this pass: renaming detection via content-hash matching
  (treat two artifacts with equal `ContentFingerprint` at different paths across runs as "the same
  artifact, renamed") — flagged `ABSORB_LATER`, not designed here, because it needs a persisted
  prior-run snapshot to compare against, which does not exist yet (see State/effect model above).

## Dependency/extinction status

Both donors remain `REFERENCE_ONLY` per `donor-corpus.toml`; no runtime/build dependency exists or
is proposed. This genome record does not claim `ABSORBED` for either donor — see Decision below.

## Decision

**`ABSORB_LATER`**, not `ABSORB_NOW`, for the incremental-identity mechanism specifically. Per
`DONOR-ABSORPTION-ROADMAP.md`'s pull-forward criteria, `ABSORB_NOW` requires the mechanism to be a
"concrete prerequisite/blocker" for the active R4 work or "clearly high-leverage and bounded" —
neither holds today: nothing in R4's own closure requirements needs incremental census, and no
performance problem motivates it (see Performance characteristics). Incremental census belongs
naturally with Wave 3 (`salsa`/`datafrog`, R5's own "incremental query, fixed point" target) where
the query-side machinery this mechanism would actually plug into is scheduled to be built.

`rust-analyzer`'s `census_status` advances from `DEEP_CENSUSED` to `TECHNOLOGY_GENOME_CAPTURED` for
this specific mechanism scope (stable source-artifact identity / incremental change tracking) —
not for rust-analyzer's full surface (name resolution, type inference, diagnostics remain
uncensused at this depth and are out of scope for this record). Tree-sitter's changed-range
mechanism is recorded as evidence here but Tree-sitter's own `census_status` in `donor-corpus.toml`
already reads `DEEP_CENSUS_ACTIVE`/broader and is not modified by this record, since this genome
only captures its *changed-range* mechanism, not its full parsing/query surface.

No Atlas source code changes follow from this record. `ArtifactId`/`ContentFingerprint` remain
exactly as they are; this record makes explicit, evidence-backed, and durable a gap (no incremental
census) and a risk (non-cryptographic identity hash meeting a future cryptographic-hash
requirement) that were previously unrecorded, rather than implementing a change ahead of its
correctly-sequenced wave.
