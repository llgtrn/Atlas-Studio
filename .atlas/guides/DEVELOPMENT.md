---
id: atlas.guide.development
type: runbook
status: active
canonical: true
---
# Atlas Development Guide

Before coding, read North Star, Genome, Census Completeness, Universal Graph, ATLAS/ATLASX/Sharding, Compiler Product, Compiler Optimization, system blueprint/roadmap and resolve the exact target revision.

## End-to-end work sequence

1. Resolve exact repositories/revisions and Genome identity.
2. Admit corpus/evidence and build exhaustive inventory.
3. Account for every artifact and discovered function.
4. Run syntax/symbol/build/type/control/data/state/effect/binding/temporal/evidence census passes.
5. Lower critical/triggered scopes to blocks and semantic atoms.
6. Run independent extractor reconciliation.
7. Resolve dynamic behavior or record explicit UNKNOWN/DYNAMIC/UNSUPPORTED states.
8. Reconcile bottom-up facts with top-down architecture claims.
9. Run adversarial gap/orphan/unbound/unevidenced queries.
10. Iterate until Genome-required fixed point.
11. Produce CensusCertificate.
12. Correlate donor/research evidence and build gap/conflict/opportunity graphs.
13. Invent candidates without rewriting observed facts.
14. Validate candidates with proof/test/simulation/benchmark as appropriate.
15. Select target design and publish SEALED logical `*.atlas`.
16. Shard/content-address the logical Atlas as required; size is not a reason to discard meaning.
17. Deterministically materialize selected `*.atlasx/`, possibly by bounded scope/lazy shard fetch.
18. Compile under explicit DeploymentProfile, HardwareProfile and WorkloadProfile.
19. Apply world/graph, semantic, memory, concurrency and target optimizations.
20. Lower through active HIR/MIR/LIR/Machine IR/backend path.
21. Apply whole-program/LTO, link and post-link optimization where supported.
22. Run representative workload, collect profile evidence and optionally PGO/auto-tune.
23. Verify product lineage/hash/tests/benchmarks.
24. Recensus materialized/generated implementation/product evidence against intended design.
25. Record evidence and extinguish donor checkout/scope only after the absorption gate.

## Research evidence boundary

For DeepWiki, papers, RFCs, external docs or model-assisted research:

1. create a ResearchClaim with source URL/revision/date when available;
2. keep it outside the observed-world census;
3. for donor implementation claims, resolve the exact pinned SHA from `references/donor-corpus.toml`;
4. verify against pinned source/build/test/runtime evidence;
5. preserve disagreement as CONFLICT or UNKNOWN;
6. only then use verified observation for mechanism extraction or design selection.

Research may trigger deeper census. It cannot bypass census.

## Bootstrap language policy

Backend/compiler/runtime is Rust. Frontend is TypeScript/TSX. C is bounded to explicit FFI/device/OS/vendor boundaries unless a target contract justifies otherwise.

Do not mechanically translate donors. Extract mechanisms/invariants, synthesize Atlas-native design, then materialize.

## Cross-repository rule

Analysis may span the federated graph. A work run mutates one canonical repository target at a time. Cross-repo changes are explicit linked runs using stable identities/bindings/evidence.

## Generated state

Human-readable Atlas/AtlasX exports and graph dumps are projections. Binary/sharded artifacts plus their manifests/hashes carry the compiled semantic state.


## Digital Organism target work

When `target_kind = digital_organism`:

1. define/pin Organism Genome and Species/trait lineage;
2. define persistent identity/lifecycle semantics before provider/model integration;
3. define organs/circuits/body/environment capability bindings;
4. define memory/world/learning/homeostasis/metabolism semantics;
5. bind cognition to external API, self-hosted checkpoint, deterministic code or hybrid provider set;
6. compile the phenotype substrate;
7. instantiate/birth with durable OrganismId;
8. treat experience/learning outputs as candidates;
9. evaluate/simulate/regression-test candidate models/rules;
10. admit/activate explicitly with rollback lineage.

Do not make provider session history, hidden model state or a weights file the organism's identity, memory or authority.
