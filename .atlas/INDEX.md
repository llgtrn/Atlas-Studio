---
id: atlas.docs.index
type: reference
status: active
canonical: true
---
# Atlas Studio Index

Read in this order:

1. `architecture/constitution/NORTH-STAR.md`
2. `contracts/ATLAS-GENOME.md`
3. `contracts/UNIVERSAL-GRAPH-CONTRACT.md`
4. `contracts/SEMANTIC-FACTS.md`
5. `contracts/SEMANTIC-EXTRACTION.md`
6. `contracts/NORMALIZATION.md`
7. `contracts/CENSUS-COMPLETENESS.md`
8. `contracts/CENSUS-CERTIFICATE.md`
9. `standards/CENSUS-SCOPE.md`
10. `decisions/0001-one-normalized-semantic-path.md`
11. `decisions/0002-epistemic-status-model.md`
12. `architecture/CAPABILITY-ARCHITECTURE.md`
13. `architecture/SYSTEM.md`
14. `blueprints/PHYSICAL-REFOUNDATION.md`
15. `roadmap/ROADMAP.md`
16. `roadmap/DONOR-ABSORPTION-ROADMAP.md`
17. `roadmap/DONOR-ABSORPTION-PLAN.toml`
18. `references/deepwiki/DONOR-ARCHITECTURE-SYNTHESIS.md`
19. `contracts/ATLAS-DEVELOPMENT-LANGUAGE.md`
20. `contracts/ADL-TO-ATLAS.md`
21. `contracts/DONOR-TO-LANGUAGE-GENESIS.md`
22. `contracts/ATLAS-FORMAT.md`
23. `contracts/ATLAS-BINARY-WIRE-FORMAT.md`
24. `contracts/ATLAS-SHARDING.md`
25. `contracts/ATLASX-FORMAT.md`
26. `architecture/DIGITAL-ORGANISM.md`
27. `contracts/ORGANISM-GENOME-v1.md`
28. `contracts/ORGANISM-LIFECYCLE.md`
29. `contracts/ORGANISM-MODEL-ADMISSION.md`
30. `contracts/ORGANISM-HOMEOSTASIS-METABOLISM.md`
31. `standards/ORGANISM-SPECIES.md`
32. `blueprints/ORGANISM-COMPILATION.md`
33. `contracts/COMPILER-PRODUCT.md`
34. `standards/COMPILER-OPTIMIZATION.md`
35. `architecture/INVENTION-PIPELINE.md`
36. `blueprints/SYSTEM-BLUEPRINT.md`
37. `blueprints/COMPILER-ROADMAP.md`
38. `blueprints/RUST-PARITY-AND-SURPASS-ROADMAP.md`
39. `blueprints/BULK-DONOR-ABSORPTION.md`
40. `contracts/SYSTEM-CONTRACT.md`
41. `guides/DEVELOPMENT.md`

Canonical chain:

```text
Reality / Corpus
  ↓ exhaustive inventory
Strict Census S0→S10
  ↓ typed OBSERVED records
  ↓ normalize / reconcile / adversarial gaps / fixed point
CensusCertificate
  ↓
Observed World ────────────────────────────┐
                                          │
Authored ADL                               │
  ↓ typed DECLARED records                 │
  ↓ canonical Census / normalize/reconcile ├─→ comparison / invention / selected design
                                          │
Donor Technology Genomes / research ──────┘
  ↓
SEALED Logical *.atlas
  ↓ content-addressed shards as needed
Deterministic *.atlasx/
  ↓ world/semantic optimization
HIR → MIR → LIR → Machine IR
  ↓ target codegen / LTO / link / post-link
Product
  ↓ workload profiles / PGO / auto-tuning
Evidence + recensus
  ↺
```

The universal graph/binding/evidence/temporal grammar remains invariant across census, storage, materialization, optimization and generated repositories/products.
