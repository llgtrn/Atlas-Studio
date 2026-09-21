---
id: atlas.decision.0001.one-normalized-semantic-path
type: decision
status: accepted
canonical: true
---
# ADR 0001 — One Normalized Semantic Path

## Context

R4 has multiple potential producers: inventory, SourceFrontend, parsers, compiler metadata, ADL, tests and later runtime traces.

If any producer writes the engineering graph or ATLAS directly, Atlas can acquire parallel truth systems whose identities, normalization and conflict behavior diverge.

## Decision

All source-derived semantics follow one path:

~~~text
Inventory
→ Structural Frontend
→ Semantic Extractor
→ Raw typed observations
→ Census
→ Normalize
→ Reconcile
→ Graph / ATLAS projection
~~~

Declared ADL and other authored claims enter the same census/normalization/reconciliation path with DECLARED status.

The engineering graph is a derived projection of normalized/reconciled semantics. It is not an independent mutable semantic source.

## Consequences

- extractor implementations may vary without creating new truth languages;
- graph code cannot reinterpret raw parser output independently;
- normalization becomes a hard semantic boundary;
- conflicts remain visible before graph projection;
- later ATLAS binary encoding consumes the same semantic world rather than rebuilding meaning.

## Forbidden alternatives

- parser → graph direct writes;
- compiler metadata → graph direct writes;
- ADL → graph bypass;
- one normalization dialect per language;
- UI/editor state as semantic truth;
- donor-specific native semantic universes.

## Migration

The existing R4 spine already routes census → normalization → graph for bootstrap facts. Future deep extractors MUST attach before census and may not bypass this path.
