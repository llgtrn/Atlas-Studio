---
id: atlas.constitution.north-star
type: architecture
status: canonical
canonical: true
---
# Atlas Studio North Star

Atlas Studio is a product-neutral engineering-world compiler and system invention environment.

Its destination is not prompt-to-code generation, repository translation or a graph viewer. Atlas must account for engineering reality without silent omission, preserve that reality and its evidence in a dense universal graph, invent and validate better target designs, materialize one selected executable world, and compile it into verified physical products specialized for their deployment and hardware.

## Canonical chain

```text
intent + repositories + OSS + builds/tests + specs + papers + evidence
                                  ↓
                           secure admission
                                  ↓
                         full inventory
                                  ↓
                 strict multi-resolution census
                                  ↓
              function + required semantic atoms
                                  ↓
             reconcile / adversarial gaps / fixed point
                                  ↓
                         CensusCertificate
                                  ↓
              observed + research + invention graphs
                                  ↓
                       selected target design
                                  ↓
                     SEALED Logical *.atlas
                                  ↓
                 content-addressed shards if needed
                                  ↓
                       deterministic *.atlasx/
                                  ↓
                   world / semantic optimization
                                  ↓
                    HIR → MIR → LIR → Machine IR
                                  ↓
                 codegen / LTO / link / post-link
                                  ↓
                           physical product
                                  ↓
                  workload profile / PGO / tuning
                                  ↓
                         evidence + recensus
                                  ↺
```

## Non-negotiable invariants

- `.atlas/` is the repository knowledge/control directory; `*.atlas` is the dense binary artifact.
- Atlas Genome governs admission, census, invention, materialization, optimization and verification.
- Every admitted artifact and discovered function is accounted for; adaptive census changes depth, never permits silent omission.
- UNKNOWN, UNSUPPORTED, DYNAMIC and CONFLICT are explicit states.
- Important facts retain evidence/provenance/temporal scope.
- A production Atlas root requires Genome-defined closure and CensusCertificate.
- Completeness outranks compression ratio; a logical Atlas may be very large.
- One logical Atlas may span many immutable content-addressed physical shards without becoming multiple truth systems.
- `*.atlasx/` is deterministic selected executable meaning, not documentation.
- Universal Identity/Scope/Node/Edge/Binding/State/Event/Temporal/Evidence/Provenance/Constraint/Invariant/Interface/Capability/Effect/Materialization semantics survive all stages.
- Generated repositories remain sovereign graph partitions yet are cross-repository composable through stable identities/bindings.
- Optimization may aggressively change physical representation but may not weaken required authority, safety, tenant, temporal, evidence, transaction, recovery or external-interface semantics.
- Donors are evidence, not permanent runtime authority.
- Backend/compiler/runtime bootstrap is Rust; frontend bootstrap is TypeScript/TSX; C is bounded to explicit low-level boundaries by default.
- Runtime profiles and benchmark results are evidence tied to target/workload/hardware, never timeless universal facts.

## Sequencing

Genome → universal graph → strict Rust Census Engine → census closure/sealing → real ATLAS binary/sharding → invention/selection → AtlasX materializer → delegated compiler → typed HIR/MIR → external native backends → Machine IR/native backend → production optimization/profile loop → UI/World Canvas as projection.

Compiler ambition must not outrun semantic correctness. Native code generation is not mature if Atlas can still silently miss a function, binding, state mutation, temporal constraint or evidence obligation.
