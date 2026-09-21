---
id: atlas.architecture.capability
type: architecture
status: canonical
canonical: true
---
# Atlas Studio Capability Architecture

## Constitutional shape

Atlas Studio is one capability-first engineering system. Donor repositories, DeepWiki, models, UI surfaces and bootstrap tools may inform the system, but they do not define native ownership.

Long-lived production ownership is limited to:

```text
core      = typed product-neutral meaning
runtime   = algorithms, state machines and orchestration
adapter   = external mechanics and trust boundaries
apps      = Studio / CLI / MCP projections and invocation
```

`tools/` is bootstrap/migration-only. Mature behavior moves into one of the four native owners and the superseded tool path is retired.

## Epistemic planes

Atlas keeps observation, research, design and execution distinct:

```text
OBSERVED WORLD
pinned source / build / tests / runtime / binary evidence
        ↓ census + reconcile
ObservedEvidence + UNKNOWN / UNSUPPORTED / DYNAMIC / CONFLICT
        │
        ├──────────────────────────────┐
        │                              │
RESEARCH WORLD                         │
DeepWiki / papers / RFCs / docs        │
        ↓                              │
ResearchClaim                          │
        ↓ corroborate when donor-specific
        └──────────────────────────────┘
                       ↓
DESIGN WORLD
gap / comparison / invention
        ↓
CandidateDesign
        ↓ proof / test / benchmark / simulation
        ↓
SelectedDesign
        ↓
EXECUTABLE WORLD
SEALED *.atlas → deterministic *.atlasx/ → compiler → product
        ↓
runtime evidence → recensus
```

DeepWiki can tell Atlas where to look and can explain mechanisms or trade-offs. It cannot directly create ObservedEvidence. Donor implementation claims require verification against the exact revision pinned by Atlas.

## Technical spine

```text
Secure Admission
→ Inventory Ledger
→ Structural Frontends
→ Semantic Frontends
→ Normalized Typed Fact Graph
→ Incremental Dependency / Query Engine
→ Fixed-Point Derivation
→ Multi-Engine Reconciliation
→ Adversarial Gap Queries
→ CensusCertificate
→ Research Correlation
→ Invention / Selection
→ ATLAS Seal + Sharding
→ AtlasX Materialization
→ HIR → MIR → LIR → Machine IR
→ Verification / Sandboxed Execution / Codegen / Link
→ Product
→ Profile / Evidence / Recensus
```

This spine absorbs mechanisms from donors without copying their package topology.

## Native module targets

### core

```text
core/src/
├─ identity/
├─ scope/
├─ graph/
├─ schema/
├─ state/
├─ temporal/
├─ evidence/
├─ provenance/
├─ constraint/
├─ capability/
├─ genome/
├─ census/
├─ atlas/
├─ atlasx/
└─ ir/
```

`core` performs no filesystem, Git, subprocess, provider, network or UI work.

Foundational semantics converge away from open-ended string maps. Bootstrap compatibility structures may remain temporarily, but stable primitives require typed identities, relations, schemas, epistemic status and revision scope.

### runtime

```text
runtime/src/
├─ admission/
├─ inventory/
├─ census/
├─ normalize/
├─ reconcile/
├─ query/
├─ closure/
├─ research/
├─ invention/
├─ selection/
├─ seal/
├─ materialize/
├─ compile/
├─ optimize/
├─ verify/
├─ profile/
└─ recensus/
```

### adapter

```text
adapter/src/
├─ filesystem/
├─ vcs/
├─ source/
├─ build/
├─ binary/
├─ exchange/
├─ compiler/
├─ execution/
├─ storage/
├─ compression/
├─ security/
├─ research/
└─ model/
```

Adapters translate external mechanics into typed Atlas inputs/effects. They never become semantic authority.

### apps

```text
apps/
├─ studio/
│  ├─ workspace/
│  ├─ editor/
│  ├─ world-canvas/
│  ├─ graph-view/
│  ├─ design-authoring/
│  ├─ timeline/
│  ├─ evidence-inspector/
│  └─ compiler-view/
├─ cli/
└─ mcp/
```

Editor buffers, rendered graph positions, browser state and UI caches are projections, never canonical engineering truth.

## Inventory is stronger than scanning

Production inventory accounts for every admitted artifact before semantic depth is chosen.

```text
ArtifactDisposition
= Parsed
| BinaryDescribed
| Generated
| IgnoredByExplicitPolicy
| Unsupported
| Unknown
| ExternalizedWithEvidence
```

Unknown extension, large file, generated form, binary content or parser failure may reduce semantic depth; none may make an artifact disappear.

## Incremental and fixed-point reasoning

Atlas combines:

1. dependency-aware query invalidation for semantic facts and derived views;
2. differential/fixed-point derivation for recursive graph relations.

Every derived result carries revision and evidence lineage. Cache validity never comes from UI or model-session state.

## Multi-engine census

Independent observers may include syntax parsers, compiler/semantic frontends, build graphs, symbol indexes, CFG/dataflow engines, tests, runtime traces and binary metadata. Their outputs normalize into typed facts.

Agreement increases confidence. Disagreement creates explicit CONFLICT and triggers deeper census.

## Transformation

```text
ObservedWorld + DesiredConstraint
→ TransformCandidate
→ Preview / Diff
→ Verify
→ Apply as new revision
→ Recensus
```

Observed history is never rewritten in place.

## Compiler boundary

```text
SelectedDesign
→ AtlasX semantic world
→ HIR
→ MIR
→ LIR
→ Machine IR / delegated backend
→ object
→ link
→ product
```

External compilers/runtimes remain adapters and differential oracles until Atlas-native stages are mature.

## Technology lanes

| Lane | Donor examples | Native destination |
| --- | --- | --- |
| Structural parsing | Tree-sitter, ast-grep | `adapter/source`, `runtime/census` |
| Semantic intelligence | rust-analyzer, SCIP, Kythe, Glean, Joern, Semgrep, Sourcetrail | `core/schema/identity`, `runtime/normalize/query` |
| Incremental reasoning | Salsa, Datafrog, Differential Dataflow, Soufflé | `runtime/query/closure` |
| Rewrite/translation | OpenRewrite, C2Rust, Crubit, py2many | `runtime/invention/materialize` |
| Build/compiler | Buck2, Rust, LLVM | `adapter/build/compiler`, `runtime/compile` |
| Backend/link | regalloc2, object, mold | `runtime/compile`, `adapter/binary` |
| Verification | Kani, Miri, Verus | `runtime/verify` |
| Bounded execution | Wasmtime, wasm-tools | `adapter/execution/exchange`, `runtime/verify` |
| Binary/data | FlatBuffers, Arrow | `core/atlas`, `adapter/exchange`, derived analytics |
| CAS/storage | Git, BLAKE3, FastCDC, Borg, Restic, LZ4, Zstd, XZ | `runtime/seal`, `adapter/storage/compression` |
| Studio | Zed, OpenDesign, XYFlow, Cytoscape.js, ELK.js | `apps/studio` |
| Security/trust | containers/image, Podman, SELinux, OpenSCAP, Keycloak, Keylime, Clair | `core/runtime/adapter` security capabilities |

These lanes are absorption routes, not permanent subsystem names.

## Sequencing

```text
typed identity/schema
→ exhaustive inventory ledger
→ structural census
→ semantic normalization
→ incremental query + fixed point
→ reconciliation + CensusCertificate
→ research claim correlation
→ invention/selection
→ binary Atlas
→ AtlasX
→ compiler/verification
→ Studio breadth
```

Do not build the final IDE, native backend or autonomous invention loop ahead of inventory closure and typed semantic correctness.
