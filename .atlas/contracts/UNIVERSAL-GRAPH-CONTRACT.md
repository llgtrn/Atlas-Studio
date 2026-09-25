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

**Implementation (G135, ADR 0054).** `Edge.kind` is a closed `EdgeKind`, serialized to the same names the free string used. Each kind declares:

- its category: structural, declared, semantic or control flow;
- its cardinality;
- whether the source owns the target.

A declared ADL relation is one of `depends_on`, `provides` or `contains`. Any other name is an untyped relation: the ADL compiler rejects it (`ATLAS-E065`, which blocks admission), and it states no edge.

No kind is causal. A call is an invocation and an effect site is where an effect is performed, never a cause. A causal relation needs a counterfactual record, which Atlas does not yet have.

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

## Implementation status

`EngineeringGraph.evidence` was declared on the type (`core::graph::EngineeringGraph`) but no constructor populated it -- `build_source_graph`/`build_repository_graph` both left it `Vec::new()` unconditionally, and `build_system_graph` never touched it either, even though it receives a `NormalizationReport` whose own `evidence` field (`normalize_evidence(&census.evidence)`) is real evidence backing the exact facts that function projects into graph nodes/edges. Fixed: `build_system_graph` now carries `normalization.evidence` onto `graph.evidence`, so a consumer of the graph can trace which evidence backs it without a separate lookup into `NormalizationReport`. `.atlas/contracts/DEPENDENCY-CENSUS.md#implementation-status` separately documents the resolved dependency closure's own projection into this graph (`Package`/`Dependency` nodes, `RESOLVES_DEPENDENCY` edges).

A `ConstraintResult`/`Diagnostic` fact's graph node id was computed by joining `subject`/`predicate`/`object` unescaped (`format!("diagnostic:{}:{}:{}", ...)`). `subject` in particular is not restricted to a fixed, code-controlled charset -- it can be ADL-authored text (a materialization target name) extracted by simple substring splitting, not a restrictive lexer -- so two genuinely different diagnostics could compute the identical node id and silently collapse into one node via `ensure_node`'s id-based dedup. Fixed the same way as the sibling collisions closed this session in `DEPENDENCY-CENSUS.md#implementation-status` and `ADL-TO-ATLAS.md#implementation-status`: each field is escaped (`core::identity::escape_identity_field`) before joining. Found via falsification-first testing: the regression test was run against the unfixed code and confirmed to fail with two genuinely different diagnostics collapsing into one graph node before the fix was written.
