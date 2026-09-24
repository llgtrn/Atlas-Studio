---
id: atlas.decision.0006.inventory-change-detection
type: decision
status: accepted
canonical: true
---
# ADR 0006 — Inventory change detection between systemize snapshots

## Context

R5 requires "incremental recensus of changed source and affected dependents". ADR 0005 gave every inventoried regular file a BLAKE3-256 content identity. Nothing yet compared two inventories, so Atlas could not say what changed between two runs. Salsa's blocking question, differential-dataflow's workload question and the G36 frontier lane's single `CANDIDATE_ABSORB_NOW` (rust-analyzer `crates/vfs`) all point at this capability.

A read-only G37 lane checked every claim against the pinned rust-analyzer source (`aaddfb73`). It also found three factual errors in the earlier genome record, which have been corrected there.

## Decision

1. **`core::census::delta::diff_inventories(previous, current)`** is a pure function and returns `InventoryDelta` (`atlas.inventory-delta.v1`). Absorbed from vfs:
   - the `Create`/`Modify`/`Delete` taxonomy (`crates/vfs/src/lib.rs:150-157`);
   - the rule that an unseen path is a creation (lib.rs:300-305);
   - the equal-content short circuit (lib.rs:227-230).
2. **Adapted, not copied:**
   - Identity is the artifact path (`ArtifactId = stable_id(path)`), not a process-local interned `FileId`.
   - Content identity is BLAKE3-256, not the 64-bit `FxHasher`. An FxHasher collision would silently drop a real modification.
   - There is no within-cycle merge table. A two-snapshot diff has no intermediate events, so create-then-delete is simply absent, and A→B→A is unchanged. Vfs would report the latter as `Modify`.
   - A missing digest on either side is `Modified` (`CONTENT_IDENTITY_UNAVAILABLE`), never unchanged.
   - A kind change at the same path is `Modified` (`KIND_CHANGED`).
   - A `PolicyBoundary` directory present on both sides is unchanged, because its contents are outside the census by policy.
   - A move is `Deleted` plus `Created`, and no rename is inferred. Case-only path differences are distinct identities.
   - Unchanged content with a different classification (`bytes`/`disposition`/`language`/`reason`) is reported as `classification_drift`, not as a content change.
3. **Refusals, never an empty delta.** `diff_inventories` refuses to diff when:
   - the two roots differ;
   - the inventory schemas differ;
   - either snapshot contains a duplicate path.
4. **Production wiring.**
   - `runtime::systemize_since(root, Some(&previous))` and `runtime::read_previous_inventory`.
   - `atlas-systemizer systemize --root R --out O --previous <earlier --out report>` emits `SystemizeReport.inventory_delta` (report schema v13).
   - A first run has no baseline and therefore no delta.

## Consequences

- Atlas can name the changed set of files between two runs of itself.
- The next R5 step is a derivation cache for one per-file derivation. It must meet two prerequisites carried from ADR 0005:
  - the parsed bytes equal the digested bytes;
  - the cache key includes the extractor identity and version.
  It must also be validated against a full recompute.
- The rust-analyzer checkout **cannot** go extinct from this absorption. It is also a DC1 real-donor dependency-census corpus, and its HIR and name-resolution surface is not absorbed.

## Provenance

- Donor: `rust-lang/rust-analyzer@aaddfb73fd95f2c0bf001b474dca91ae28bcce3a` (`.atlas/provenance/donors/rust-analyzer.json`).
- Genome: `.atlas/genome/technology/rust-analyzer-tree-sitter-source-identity.md`.
- Discovery: `.atlas/census/discoveries/r5-inventory-change-detection.md`.
- This decision.
- Code: `core/src/census/delta.rs`, `runtime/src/lib.rs`, `apps/cli/src/main.rs`.
- Evidence: `.atlas/evidence/verification/r5-inventory-change-detection-absorption.json`.
