---
id: atlas.architecture.system
type: architecture
status: canonical
canonical: true
---
# Atlas Studio System Architecture

## Responsibilities

- `core/` owns pure typed engineering semantics: global identities, graph primitives, scope, state/event/temporal models, bindings, evidence/provenance, claim status, constraints/invariants, Atlas Genome semantics, ATLAS/ATLASX contracts and compiler IR types.
- `runtime/` owns admission, census orchestration, corpus/design construction, synthesis, materialization, compiler passes, query, verification, recensus and incremental invalidation.
- `adapter/` owns Git/filesystem/parsers/storage/provider/DeepWiki/research/benchmark/OS/toolchain mechanics. Adapters never become semantic authority.
- `apps/ui/` owns TypeScript/TSX visualization and editing projections over bounded engine APIs. UI state is not engineering truth.
- `.atlas/` owns authored architecture/control knowledge, genome source, evidence/provenance references and durable contracts.
- `.atlas/artifacts/` owns durable compiled artifacts once the binary formats are implemented.

## Canonical dataflow

```text
untrusted/admitted evidence
       ↓
source observations
       ↓
multi-resolution census
       ↓
universal engineering graph
       ↓
technology/design synthesis
       ↓
*.atlas
       ↓
deterministic materialization
       ↓
*.atlasx/
       ↓
compiler
       ↓
physical target
       ↓
verification / recensus / evidence
```

## Universal graph substrate

Every repo, donor, paper claim, target design, compiler unit and generated system is represented on the same graph grammar. Repositories are sovereign graph partitions, not isolated semantic universes.

A repository boundary must expose:

```text
RepositoryIdentity
Revision/TemporalHead
ExportedNodes
ImportedNodeReferences
Bindings
Interfaces/Capabilities
Constraints/Invariants
EvidenceRoot
ProvenanceRoot
MaterializationRoot
```

Cross-repository linkage references stable identities rather than copying foreign canonical state. Federation is a derived graph over repo-owned partitions.

## Format boundaries

`*.atlas` is the dense binary engineering/design artifact produced by census plus synthesis. It may retain donor alternatives, rejected designs, research claims, source observations, target decisions and evidence.

`*.atlasx/` is the selected, deterministic, expanded executable repository representation derived from `*.atlas`. It contains typed executable units and graph metadata organized in a stable repo-shaped tree. Human-readable exports are projections only.

## Current compiler bootstrap

Until Phase 3/4 mature, AtlasX lowers primarily to Rust backend and TypeScript frontend, with C only for explicit ABI/device boundaries. Rust/TypeScript source remains a generated/reference backend, not the final semantic authority.

Core performs no filesystem, network, subprocess, provider or UI work. Ingestion is never execution. Mutation and integration remain explicit authorized repository operations.
