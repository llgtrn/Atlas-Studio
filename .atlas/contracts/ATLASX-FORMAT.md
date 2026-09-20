---
id: atlas.contract.format.atlasx
type: contract
status: active
canonical: true
---
# ATLASX Expanded Executable Representation Contract

`<system>.atlasx/` is the deterministic expanded executable repository representation materialized from a selected `<system>.atlas` design.

"ATLASX" means expanded executable representation, not expanded prose. It is not Markdown, JSON documentation or an LLM summary.

## Shape

ATLASX MUST expose a stable repository-shaped tree so humans, tools and compiler passes can address bounded scopes. A default conceptual shape is:

```text
<system>.atlasx/
├─ manifest.atlasx
├─ graph/
│  ├─ identity.atlasx
│  ├─ nodes.atlasx
│  ├─ edges.atlasx
│  ├─ bindings.atlasx
│  ├─ temporal.atlasx
│  └─ evidence.atlasx
├─ modules/
├─ interfaces/
├─ runtime/
├─ ui/
├─ tests/
└─ targets/
```

Exact physical sharding may evolve, but folder/module scope MUST be stable and derivable from Atlas meaning. Individual `*.atlasx` units may use compact binary encoding; human-readable views are generated projections.

## Semantic requirements

ATLASX carries the selected executable design, including enough information for:

- static type checking;
- ownership/lifetime/resource reasoning where relevant;
- effects/state transitions;
- concurrency/transaction semantics;
- temporal semantics;
- bindings and cross-repository references;
- constraints/invariants;
- target placement;
- verification obligations;
- source/evidence lineage back to the parent `*.atlas`.

## Determinism

ATLASX is not required to contain every rejected donor/design alternative from ATLAS. It is a selected materialization. However, the same pinned `*.atlas` selection, Genome and compiler version MUST produce the same semantic ATLASX hashes.

ATLASX is the input to the compiler pipeline. In early phases it lowers through Rust/TypeScript/C boundaries; in later phases it lowers through Atlas HIR/MIR/LIR and finally machine IR.
