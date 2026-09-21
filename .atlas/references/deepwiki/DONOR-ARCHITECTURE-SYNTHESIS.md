---
id: atlas.reference.deepwiki.donor-architecture-synthesis
type: reference
status: active
canonical: false
---
# DeepWiki Donor Architecture Synthesis

## Status and trust rule

This is research synthesis, not canonical observation. The canonical donor registry remains `../donor-corpus.toml`.

```text
DeepWiki / paper / RFC / explanatory docs
→ ResearchClaim
→ correlate with Atlas need
→ PINNED DONOR SOURCE @ exact SHA
→ source/build/test/runtime census
→ ObservedEvidence
→ Mechanism + Invariant
→ Atlas-native CandidateDesign
→ validation
→ SelectedDesign
```

DeepWiki may index a different revision than Atlas's pinned donor SHA. It is therefore a navigation and hypothesis source, never direct proof of pinned implementation.

## Cross-donor pattern

The researched donors repeatedly converge on:

```text
incremental structure
→ typed semantic identity
→ normalized graph/facts
→ dependency-aware queries
→ fixed-point derivation
→ controlled transformation
→ IR lowering
→ verification / bounded execution
→ evidence
→ incremental recensus
```

Atlas should absorb this pattern rather than clone donor module trees.

## Lanes

### Structural parsing

Tree-sitter and ast-grep suggest incremental concrete structure plus structural query/rewrite behind a common source boundary.

DeepWiki: https://deepwiki.com/tree-sitter/tree-sitter/2-core-parsing-system

Atlas owner: `adapter/source` + `runtime/census`.

### Semantic source intelligence

rust-analyzer, SCIP, Kythe, Glean, Joern, Semgrep and Sourcetrail suggest stable symbol identity, typed occurrences/relationships and normalization of syntax/control/dataflow/type knowledge into queryable graphs.

DeepWiki:
- https://deepwiki.com/sourcegraph/scip/1-overview
- https://deepwiki.com/joernio/joern/1.1-architecture

Atlas owner: `core/identity/schema/graph` + `runtime/normalize/query`.

### Incremental reasoning

Salsa, Datafrog, Differential Dataflow, Soufflé and Buck2 suggest revision/dependency tracking, memoized derivation and fixed-point graph evaluation.

DeepWiki:
- https://deepwiki.com/salsa-rs/salsa/1-salsa:-an-incremental-computation-framework
- https://deepwiki.com/facebook/buck2

Atlas owner: `runtime/query/closure`.

### Rewrite and translation

OpenRewrite, C2Rust, Crubit and py2many suggest typed transformation candidates, repeatable rewrite cycles, before/after lineage and semantic lowering instead of text replacement.

DeepWiki: https://deepwiki.com/openrewrite/rewrite/1.1-architecture-overview

Atlas owner: `runtime/invention/materialize` + exchange/language adapters.

### Compiler/backend

Rust, LLVM, Buck2, regalloc2, object and mold suggest explicit IR stages, dependency-aware builds, machine constraints, object emission and link/post-link optimization with differential oracles.

Atlas owner: `runtime/compile` + `adapter/build/compiler/binary`.

### Verification and execution

Kani, Miri and Verus suggest model checking, execution-semantics validation and proof obligations as separate evidence modes. Wasmtime/wasm-tools suggest bounded executable capability interfaces and explicit sandbox/runtime configuration.

DeepWiki:
- https://deepwiki.com/verus-lang/verus
- https://deepwiki.com/bytecodealliance/wasmtime/1-overview

Atlas owner: `runtime/verify` + `adapter/execution/exchange`.

### Binary/data/storage

FlatBuffers and Arrow suggest schema-driven compact binary records and columnar derived analytics. Git, BLAKE3, FastCDC, Borg, Restic, LZ4, Zstd and XZ suggest content identity, chunking, deduplication, compression, indexing and recovery ordering.

Atlas owner: `core/atlas` + `runtime/seal` + `adapter/storage/compression/exchange`.

### Studio

Zed, OpenDesign, XYFlow, Cytoscape.js and ELK.js suggest separating editor/workspace models, graph interaction, layout and design authoring from canonical semantic state.

DeepWiki: https://deepwiki.com/zed-industries/zed/4.1-collaboration-architecture

Atlas owner: `apps/studio`.

### Security/trust

The staged security family (containers/image, Podman, SELinux, OpenSCAP, Keycloak, Keylime, Clair) separates supply-chain admission, isolation, least privilege, identity/authorization, attestation, compliance and vulnerability intelligence.

Atlas owner: typed semantics in `core`, policy/state machines in `runtime`, OS/provider mechanics in `adapter`.

## Final synthesis

```text
adapter/source
→ runtime/inventory+census
→ core typed identities/facts
→ runtime/normalize+query+closure
→ runtime/reconcile
→ CensusCertificate
→ runtime/research (ResearchClaim only)
→ runtime/invention+selection
→ runtime/seal+materialize
→ runtime/compile+verify
→ adapters for execution/binary/storage
→ apps/studio projections
```

Reject designs that make DeepWiki/LLM output canonical evidence, create donor-shaped native subsystems, create lane-specific truth universes, silently drop artifacts, let UI state become truth, retain permanent donor runtime ownership for native technology, or jump to compiler/IDE breadth before inventory closure.
