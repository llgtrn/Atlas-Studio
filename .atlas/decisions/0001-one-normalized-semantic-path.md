---
id: atlas.decision.0001.one-normalized-semantic-path
type: decision
status: accepted
canonical: true
---
# ADR 0001 — One Normalized Semantic Path

## Decision

Atlas has exactly one semantic flow from admitted observations to graph projection:

~~~text
Inventory
→ Structural Frontend
→ Semantic Extractors
→ Raw Typed Observations
→ Census
→ Normalize
→ Reconcile
→ Graph
~~~

No extractor, graph builder, ATLAS serializer, AtlasX compiler, or UI may create a parallel canonical semantic truth store.

## Consequences

- SourceFrontend remains structural.
- semantic extractors produce typed observations, not graph mutations;
- normalization is deterministic and lossless;
- reconciliation owns disagreement handling;
- the engineering graph is rebuildable projection;
- sealed ATLAS artifacts serialize canonical semantic state and evidence, not a second ontology.

Bootstrap shortcuts must be explicitly marked bootstrap and removed before the relevant R4 gate can become complete.

## Supersession

A future ADR may refine stages but may not introduce multiple canonical semantic universes without explicitly superseding this decision.
