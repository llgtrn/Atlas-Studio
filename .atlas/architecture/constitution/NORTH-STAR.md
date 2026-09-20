---
id: atlas.constitution.north-star
type: architecture
status: canonical
canonical: true
---
# Atlas Studio North Star

Atlas Studio is a product-neutral engineering-world compiler and system invention environment. Its destination is not prompt-to-code generation and not repository translation. It must understand engineering reality at multiple resolutions, compress that understanding into a dense binary design artifact, expand selected designs into a deterministic executable repository representation, and compile them into verified software.

## Canonical chain

```text
intent + admitted repositories + OSS + specs + DeepWiki + papers + evidence
                                  ↓
                           secure admission
                                  ↓
                  adaptive multi-resolution census
                                  ↓
             mechanism / principle / invariant extraction
                                  ↓
                    compare / synthesize / invent
                                  ↓
                         <system>.atlas
               dense canonical engineering artifact
                                  ↓
                       Atlas materializer
                                  ↓
                       <system>.atlasx/
              expanded executable repo representation
                                  ↓
                         compiler backend
                                  ↓
                      physical executable
                                  ↓
                    tests / benchmark / proof
                                  ↓
                             recensus
                                  ↺
```

## Non-negotiable invariants

- `.atlas/` is the repository control root; `*.atlas` is the dense binary artifact. They are distinct concepts.
- Atlas Genome hard requirements govern census, planning, synthesis, materialization, verification and generated repository shape.
- Every artifact pins genome version/hash and source/evidence lineage.
- Census spans global repository impact down to semantic atoms and uses adaptive depth rather than blindly exploding every line.
- Source code is authority for what an implementation currently does; tests/runtime evidence verify behavior; papers/specs explain algorithms and normative principles; DeepWiki/docs are explanatory evidence; model output is candidate/inference.
- Facts, inference, hypothesis, conflict and unknown remain distinct.
- Donors are evidence, never runtime authority.
- Native technology cannot require donor runtime availability after absorption is complete.
- Backend/compiler/runtime remains Rust during the bootstrap path. Frontend remains TypeScript/TSX. C is a bounded ABI/device/legacy boundary unless explicitly justified.
- `*.atlasx/` is executable representation, not Markdown documentation and not an LLM summary.
- Generated source is a projection/materialization of Atlas meaning, never a competing canonical universe.
- Every Atlas-generated repository uses the universal graph grammar and can federate with other Atlas-generated repositories through stable bindings without surrendering repository sovereignty.
- Node, edge, binding, scope, state, event, temporal, evidence, provenance, constraint/invariant, interface/capability, effect and materialization semantics must survive all compiler phases.
- Optimization may change physical implementation but may not erase authority, state, temporal or evidence semantics required by the source graph.

## Sequencing

Genome and graph contract -> secure source intelligence -> census scope engine -> ATLAS binary writer/reader -> synthesis/design representation -> ATLASX materializer -> Phase 1 delegated compiler -> Phase 2 typed IR/optimizer -> Phase 3 external native backends -> Phase 4 Atlas native backend -> hardware-aware whole-system compilation.

Compiler ambition must not outrun semantic correctness. A native register allocator is irrelevant if Atlas cannot first identify a state mutation, binding, provenance chain or cross-repository identity correctly.
