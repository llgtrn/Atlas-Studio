---
id: atlas.decisions.index
type: reference
status: active
canonical: true
---
# Atlas Decisions

Only durable architecture choices belong here. Temporary implementation history, migration notes and progress reports stay in Git history or non-canonical evidence.

A superseded decision must identify its successor. No historical fleet/mirror decision is currently canonical.

Current durable decisions:

1. 0001-one-normalized-semantic-path.md — all semantic observations converge through one normalization/reconciliation path before graph projection.
2. 0002-epistemic-status-model.md — one EpistemicStatus vocabulary; FactKind, EvidenceKind, status and disposition stay orthogonal.
