---
id: atlas.decisions.index
type: reference
status: active
canonical: true
---
# Atlas Decisions

Only durable architecture choices belong here. Temporary implementation history, migration notes and progress reports stay in Git history or non-canonical evidence.

A superseded decision must identify its successor. No historical fleet/mirror decision is currently canonical.

## Accepted decisions

- `0001-one-normalized-semantic-path.md` — all semantic producers converge through census → normalize → reconcile before graph/ATLAS projection.
- `0002-epistemic-status-model.md` — one canonical EpistemicStatus vocabulary; FactKind, EvidenceKind and Disposition remain separate.
- `0003-constraint-derivation-provenance.md` — Souffle's provenance pattern absorbed as an additive `ConstraintResult.derivation` field, projected into the engineering graph via `add_constraint_derivations`.
- `0004-semi-naive-dependency-closure.md` — datafrog's semi-naive delta-driven fixed point absorbed as `core::closure`, computing origin-labelled transitive dependency reachability in `census_cargo_workspace` (`DependencyClosureReport.reachability`, schema v4).
- `0005-native-blake3-content-digest.md` — BLAKE3's unkeyed hash mode absorbed natively as `core::identity::blake3`; every inventoried regular file carries a validated `blake3-256:` `ArtifactRecord.content_digest` from one bounded, verified read (inventory schema v2). Supersedes the genome record's native-port rejection for the unkeyed mode only.
- `0006-inventory-change-detection.md` — rust-analyzer vfs's Create/Modify/Delete taxonomy absorbed as `core::census::delta::diff_inventories` over BLAKE3 content identity; `systemize --previous` emits `inventory_delta` (report schema v13).
- `0007-three-valued-constraint-verdict.md` — `ConstraintVerdict { Satisfied, Violated, Unknown }` (strong Kleene conjunction) on every ADL constraint result; an undeclared required attribute is UNKNOWN (`ATLAS-E055`), not a violation; admission blocks on `ADL_CONSTRAINT_VIOLATED` and `ADL_CONSTRAINT_UNKNOWN` separately.
- `0008-extraction-derivation-cache.md` — persisted per-artifact `ExtractionBatch` memo keyed on BLAKE3 over the full extraction input, the digest of the bytes read, and a build digest of all extractor/core sources + `Cargo.lock`; used only when read bytes equal the inventoried digest; `systemize --cache`. Measured: reports byte-identical, but graph summarization (~95% of run time) — not extraction — is the recensus cost centre.
- `0009-indexed-graph-node-dedup.md` — `ensure_node` deduplicates through an incremental, non-serialized node-id index instead of a linear scan (append-only contract, shrink rebuild, debug sentinel); graph output byte-identical, `systemize` 187 s → 12.9 s (debug).
- `0010-physical-quantities-and-dimensional-constraints.md` — `core::quantity`: exact rational SI quantities over 8 base dimensions (angle separate); units with irrational/affine factors refused; ADL `require x.attr (==|>=|<=|>|<) <quantity>` compares by dimension and exact value (E056 dimension mismatch = VIOLATED, E057 unsupported unit = UNKNOWN, E058 ordering over non-quantity = UNKNOWN); 75/75 units agree with pint.
- `0011-visual-observation-record.md` — Creator Fabric's first primitive: hermetic browser observation (`atlas-systemizer observe`) → OBSERVED element geometry/style → DERIVED ratio-first layout relations with resolution-bounded uncertainty and interval-stated responsive rules; `contracts/CREATOR-FABRIC.md` fixes authorization/originality/epistemic rules.
- `0012-planar-arm-physical-model.md` — physical milestone 1: `core::physical::PlanarArm` from ADL Arm/Link/Joint entities with dimension-checked quantities; exact reach, inner radius and worst-case static shoulder torque (DERIVED, assumptions stated); requirement verdicts; FK verified vs closed form; `atlas-systemizer physical`; `contracts/PHYSICAL-ENGINEERING.md`.
- `0013-browser-as-instrument-breakpoint-bisection.md` — `observe --bisect`: responsive changes located to one pixel by re-observation (INFERRED, monotone-transition assumption stated); the fixture's `max-width: 699px` measured at exactly (699, 700) in 10 probes.
- `0014-cross-domain-drivetrain-power.md` — physical milestone 2: joint torque → motor torque (gear, efficiency) → holding current (k_t) → heat (I²R) → rail total and power, all exact and dimension-checked, every value naming its inputs; a heavier payload that keeps reach SATISFIED is caught VIOLATED at the power rail.
