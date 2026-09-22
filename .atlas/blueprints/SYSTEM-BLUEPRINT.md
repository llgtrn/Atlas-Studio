---
id: atlas.blueprint.system
type: blueprint
status: active
canonical: true
---
# Atlas Studio System Blueprint

## Objective

Build an independent engineering-world compiler capable of strict corpus accounting, invention, dense Atlas publication, deterministic AtlasX expansion and verified product compilation. Digital Organism is a first-class target kind: Atlas can compile a complete organism substrate around external, self-hosted, hybrid or absent neural models rather than treating weights as the product.

## Blueprint authority and evolution

This document is a canonical blueprint for the current evidence state.

It is NOT an immutable constitution.

If donor/dependency census discovers and validates a materially better system mechanism, this blueprint may be revised through `../contracts/BLUEPRINT-EVOLUTION.md`.

Any revision must preserve higher-level contracts/Genome invariants or explicitly revise them through a separate compatibility/migration decision.

Implementation agents may not silently redesign around this blueprint.

## Construction sequence

1. Atlas Genome source, versioning and lock/hash semantics.
2. Universal graph, global identity, binding, evidence and temporal primitives.
3. Secure corpus admission and complete inventory accounting.
4. Multi-engine Rust Census Engine with S0→S10 scope lattice.
5. Function-level semantic accounting and explicit unknown/dynamic records.
6. Reconciliation, adversarial gap queries, fixed point and CensusCertificate.
7. Dense binary ATLAS writer/reader, lossless semantic compaction and logical shard federation.
8. Research correlation, gap graph, invention candidates and selected-design graph.
9. Donor Technology Genome extraction and cross-donor mechanism comparison.
10. Atlas Development Language semantic genesis: evolve ADL0 declarations into typed language constructs that lower into the same canonical semantic world.
11. Deterministic ADL → typed records → logical ATLAS compilation path.
12. Deterministic SelectedDesign → validated ATLASX executable representation under `../contracts/ATLAS-TO-ATLASX.md`.
13. Organism Genome v1 and Digital Organism target profile: Genome → Organ → Circuit → Trait → Model → Weight → Memory → Body → Lifecycle.
14. Organism persistent identity and lifecycle substrate.
15. Model/provider binding plus candidate weight/model admission.
16. Memory/learning/adaptation substrate.
17. Homeostasis and metabolism substrate.
18. Body/environment capability adapters and authority boundaries.
19. Phase 1 delegated Rust/TypeScript/bounded-C compiler over validated AtlasX.
20. Phase 2 HIR/MIR semantic/memory/concurrency optimizer under `../contracts/COMPILER-IR-PIPELINE.md`.
21. Phase 3 LIR → LLVM/Cranelift/WASM/accelerator backends with explicit target/ABI contracts.
22. Phase 4 Machine IR plus Atlas-native codegen under the same explicit compiler-stage contract.
23. Whole-program/LTO/link/post-link optimization.
24. Deployment/hardware/workload specialization, PGO and empirical auto-tuning.
25. Organism birth/instantiate/observe/learn/evaluate/admit/evolve lifecycle.
26. Product/organism lineage evidence and recensus.
27. World Canvas as projection only.
28. Compiler self-hosting after semantic/runtime maturity.

## Language/ATLAS boundary

The current `.atlas/declared/*.adl` implementation is ADL0, not the completed development language. Full ADL is built only after typed census semantics are trustworthy enough to preserve the mechanisms learned from donors.

Existing-language reconstruction and ADL authoring must converge before ATLAS publication:

```text
existing source → census ─┐
                          ├→ typed semantic world → *.atlas
ADL source → elaborate ───┘
```

See `../contracts/ATLAS-DEVELOPMENT-LANGUAGE.md`, `../contracts/ADL-TO-ATLAS.md` and `../contracts/DONOR-TO-LANGUAGE-GENESIS.md`.

## ATLAS / ATLASX / compiler boundary

The system blueprint does not leave the executable path implementation-defined.

Canonical responsibilities are:

~~~text
SEALED logical Atlas
→ lossless semantic compaction / wire / shards
→ explicit SelectedDesign
→ deterministic AtlasX materialization
→ validated AtlasX root
→ HIR
→ MIR
→ LIR
→ Machine IR
→ physical product
~~~

Normative contracts:

- `../contracts/ATLAS-SEMANTIC-COMPACTION.md`;
- `../contracts/ATLAS-TO-ATLASX.md`;
- `../contracts/ATLASX-FORMAT.md`;
- `../contracts/COMPILER-IR-PIPELINE.md`.

No implementation may bypass these boundaries by inventing direct graph/source/codegen truth.

## Product target kinds

Atlas may compile ordinary software artifacts, libraries, WASM/UI products and Digital Organisms. A target kind specializes semantics without changing the universal graph substrate.

## Digital Organism invariant

A generated Digital Organism repository/product must account for:

```text
persistent identity
organism genome
body
brain/model bindings
world model
memory
learning/adaptation
homeostasis
metabolism
capabilities
authority constraints
lifecycle
evidence/model/memory lineage
```

Weights are optional organs. External API cognition is valid. Self-hosted cognition is valid. Hybrid cognition is valid.

## Independence invariant

No Organism Genome may require Chronica unless that specific target explicitly selects a Chronica adapter/environment. Chronica is one possible external environment, not Atlas's host or organism identity substrate.

## Performance invariant

Optimization begins at graph/world/organ level before machine IR. Faster output that breaks graph, authority, temporal, evidence, lifecycle, model admission, safety, transaction or recovery semantics is invalid.
