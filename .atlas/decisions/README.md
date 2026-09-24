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
