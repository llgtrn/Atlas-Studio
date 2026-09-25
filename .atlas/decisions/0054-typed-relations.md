---
id: atlas.decision.0054.typed-relations
type: decision
status: accepted
canonical: true
---
# ADR 0054 — Typed relations in the engineering graph (G135, NA-TYPED-RELATIONS)

## Context

UNIVERSAL-GRAPH says an edge states a typed relation, yet `Edge.kind` was a free `String`:

- every Atlas-built edge wrote its own literal;
- a declared ADL relation became `predicate.to_ascii_uppercase()`, whatever it was.

`DEBT-CAUSALITY` ("typed causal and relation schema", `M0_ABSENT` since G38) asked that an untyped relation kind be rejected. It was blocked until G134 decided the futures states.

## Decision

1. **`core::graph::EdgeKind`** is the closed set of relations the graph states.
   - **Coverage.** It holds 35 kinds: structural, declared, semantic and control flow, including the control-flow successor kinds mapped by `EdgeKind::control_flow`.
   - **Schema.** Each kind has a category, a cardinality (one-to-one, one-to-many, many-to-one, many-to-many) and whether its source owns its target (containment and the `HAS_*` parts do).
   - **Wire compatibility.** It is a `vocabulary_enum!`, so the graph serializes exactly as before.
2. **Declared ADL relations are typed.**
   - The names `depends_on`, `provides` and `contains` map to `DEPENDS_ON`, `PROVIDES` and `CONTAINS`.
   - Any other name is an untyped relation. The ADL compiler rejects it with `ATLAS-E065` (a diagnostic, which blocks admission), and the graph states no edge for it.
3. **Causation is decided.**
   - No edge kind is causal: `CALLS` is invocation and `PRODUCES_EFFECT` marks where an effect is performed.
   - A causal relation needs a counterfactual record. Atlas has none, so `causes` is an untyped relation today.

## Evidence

Tests cover:

- every kind's schema;
- the declared-name mapping;
- wire compatibility;
- an untyped `causes` relation diagnosed as `ATLAS-E065`;
- an untyped declared edge stating no graph edge;
- the identity-collision test, rewritten for typed relations with the colliding text now in subjects and objects.

Every Atlas edge site uses a typed kind. The self census's declared relations are all typed, and admission stays allowed.

Five mutants were each caught by a test:

- any name declaring an edge;
- untyped relations not diagnosed;
- an untyped relation still becoming an edge;
- containment not owning;
- a branch mapped to a fallthrough.

## What stays open

`DEBT-CAUSALITY` rises from `M0_ABSENT` to `M2_PARTIAL` and stays open:

- causal kinds wait for a counterfactual record;
- social and permission relations are absent;
- the declared vocabulary is three names;
- cardinality is declared, not yet checked against the graph.
