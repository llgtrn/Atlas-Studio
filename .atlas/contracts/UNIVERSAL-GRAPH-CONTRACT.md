---
id: atlas.contract.universal-graph
type: contract
status: active
canonical: true
---
# Universal Engineering Graph Contract

Atlas may generate hundreds or thousands of independent repositories. Those repositories MUST remain connectable as one engineering world without requiring a shared monorepo, duplicated truth or repository-specific graph dialects.

## Universal semantic spine

Every Atlas-managed/generated repo must support these first-class primitives:

```text
Identity
Scope
Node
Edge
Binding
State
Event
TemporalRevision / Interval
Evidence
Provenance
Claim / EpistemicStatus
Constraint / Invariant
Interface / Capability
Effect
Materialization
```

Domain-specific types extend this spine. They do not replace it.

## Distinctions

A Node is an identifiable thing.

An Edge states a typed relation.

A Binding states an explicit connection/realization between endpoints, including interface/capability/schema/protocol compatibility and constraints. Binding is not merely a generic edge.

Evidence supports or contradicts claims.

FactKind, EvidenceKind, EpistemicStatus and Disposition are distinct taxonomies as defined by `SEMANTIC-FACTS.md`. Graph projections MUST NOT invent a second epistemic vocabulary. If a bootstrap projection cannot carry the full normalized semantic record, that projection is non-canonical and must retain a stable reference to its normalized source rather than becoming truth authority.

Temporal semantics state when an identity, relation, binding, state or claim is valid.

Materialization links semantic meaning to physical code/artifact/runtime representation.

## Global identity

Cross-repository identities must be stable, namespaced and content/revision aware. A repo exports references to owned nodes; another repo imports/references those identities rather than cloning the foreign canonical node.

## Sovereign partitions

Each repository owns its internal canonical partition. Federation is a derived view:

```text
Repo A graph ─┐
Repo B graph ─┼─ bindings/references → Federated Engineering World
Repo C graph ─┘
```

A federated view may be rebuilt. It must not silently become a second mutable source of repo-owned truth.

## Cross-repository contract

Every generated repo publishes a machine-readable boundary containing:

- repository identity;
- temporal revision/head;
- exported node/interface/capability identities;
- imported external identities;
- bindings;
- compatibility constraints;
- evidence/provenance roots;
- materialization identities.

This is how Atlas-generated Ops, development cells, services, libraries, device adapters or future systems can connect exactly as parts of one larger graph while remaining separately versioned repositories.

## Compiler preservation

All compiler phases may optimize representation but may not erase externally observable graph identities/bindings or their required evidence/temporal semantics. Physical implementation is replaceable; semantic connection is durable.
