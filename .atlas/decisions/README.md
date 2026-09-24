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
