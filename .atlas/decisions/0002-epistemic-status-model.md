---
id: atlas.decision.0002.epistemic-status-model
type: decision
status: accepted
canonical: true
---
# ADR 0002 — Epistemic Status Model

## Decision

Atlas uses one canonical EpistemicStatus vocabulary:

~~~text
OBSERVED
DECLARED
DERIVED
INFERRED
HYPOTHESIS
CONFLICT
UNKNOWN
UNSUPPORTED
IGNORED
~~~

FactKind, EvidenceKind, EpistemicStatus, and Disposition are orthogonal concepts.

## Rationale

The previous Genome vocabulary and runtime census vocabulary overlapped but were not equivalent. Allowing both to persist would create incompatible truth languages across Genome, Census, Graph, and ATLAS storage.

## Rules

- OBSERVED is evidence-backed observation, not general truth.
- DECLARED remains distinct from observation.
- DERIVED is deterministic computation from admitted facts.
- INFERRED and HYPOTHESIS never auto-promote.
- CONFLICT preserves unresolved incompatibility.
- UNKNOWN means the answer is not known.
- UNSUPPORTED means the obligation is known but the current extractor cannot satisfy it.
- IGNORED requires explicit policy and still remains accounted.

No subsystem may define a competing status enum for canonical semantic truth.
