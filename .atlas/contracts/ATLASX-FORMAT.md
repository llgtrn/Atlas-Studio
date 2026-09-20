---
id: atlas.contract.format.atlasx
type: contract
status: active
canonical: true
---
# ATLASX Expanded Executable Representation Contract

`<system>.atlasx/` is the deterministic expanded executable representation of a selected design from one pinned logical Atlas root.

ATLASX is executable engineering representation, not expanded prose.

## Default shape

```text
<system>.atlasx/
├─ manifest.atlasx
├─ graph/
│  ├─ identity.atlasx
│  ├─ nodes.atlasx
│  ├─ edges.atlasx
│  ├─ bindings.atlasx
│  ├─ state.atlasx
│  ├─ temporal.atlasx
│  └─ evidence.atlasx
├─ modules/
├─ interfaces/
├─ runtime/
├─ ui/
├─ tests/
├─ profiles/
│  ├─ deployment.atlasx
│  ├─ hardware.atlasx
│  └─ workload.atlasx
└─ targets/
```

Physical sharding may evolve, but scope/module addressing and semantic hashes are deterministic for pinned inputs.

## Semantic requirements

ATLASX carries enough selected meaning for:

- static types and layouts;
- function bodies/CFG and executable semantics;
- ownership/lifetime/resource reasoning;
- state transitions/effects;
- concurrency/transactions/recovery;
- temporal semantics;
- bindings/cross-repository references;
- constraints/invariants;
- verification obligations;
- placement and deployment intent;
- evidence lineage to parent Atlas;
- compiler optimization barriers and freedoms.

## Partial materialization

Atlas may materialize bounded scopes from a sharded logical Atlas without expanding the whole corpus, provided dependencies/bindings and proof obligations are resolved.

## Compiler input

In Phase 1 AtlasX lowers through Rust/TypeScript/C boundaries. Later it lowers through Atlas HIR/MIR/LIR/Machine IR.

The same AtlasX semantics may produce multiple target-specialized physical products under different explicit DeploymentProfile, HardwareProfile and WorkloadProfile inputs. Physical specialization does not change the semantic identity of the selected design.
