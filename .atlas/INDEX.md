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
8. `contracts/DEPENDENCY-CENSUS.md`
9. `contracts/CENSUS-CERTIFICATE.md`
10. `standards/CENSUS-SCOPE.md`
11. `decisions/0001-one-normalized-semantic-path.md`
12. `decisions/0002-epistemic-status-model.md`
13. `architecture/CAPABILITY-ARCHITECTURE.md`
14. `architecture/SYSTEM.md`
15. `blueprints/PHYSICAL-REFOUNDATION.md`
16. `roadmap/ROADMAP.md`
17. `roadmap/SELF-BUILDING-R4-R8.md`
18. `roadmap/DONOR-ABSORPTION-ROADMAP.md`
19. `roadmap/DONOR-ABSORPTION-PLAN.toml`
20. `references/deepwiki/DONOR-ARCHITECTURE-SYNTHESIS.md`
21. `contracts/ATLAS-DEVELOPMENT-LANGUAGE.md`
22. `contracts/ADL-TO-ATLAS.md`
23. `contracts/DONOR-TO-LANGUAGE-GENESIS.md`
24. `contracts/ATLAS-FORMAT.md`
25. `contracts/ATLAS-BINARY-WIRE-FORMAT.md`
26. `contracts/ATLAS-SHARDING.md`
27. `contracts/ATLASX-FORMAT.md`
28. `architecture/DIGITAL-ORGANISM.md`
29. `contracts/ORGANISM-GENOME-v1.md`
30. `contracts/ORGANISM-LIFECYCLE.md`
31. `contracts/ORGANISM-MODEL-ADMISSION.md`
32. `contracts/ORGANISM-HOMEOSTASIS-METABOLISM.md`
33. `standards/ORGANISM-SPECIES.md`
34. `blueprints/ORGANISM-COMPILATION.md`
35. `contracts/COMPILER-PRODUCT.md`
36. `standards/COMPILER-OPTIMIZATION.md`
37. `architecture/INVENTION-PIPELINE.md`
38. `blueprints/SYSTEM-BLUEPRINT.md`
39. `blueprints/COMPILER-ROADMAP.md`
40. `blueprints/RUST-PARITY-AND-SURPASS-ROADMAP.md`
41. `blueprints/BULK-DONOR-ABSORPTION.md`
42. `contracts/SYSTEM-CONTRACT.md`
43. `guides/DEVELOPMENT.md`

Canonical chain:

```text
Reality / Root Corpus
  ↓ exhaustive root inventory
Transitive Dependency Closure
  ↓ expanded source/binary/toolchain/system corpus
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

For any implementation work between R4 and R8, `roadmap/SELF-BUILDING-R4-R8.md` is mandatory reading. It fixes the R4.4→R4.12 sequence, DC1 dependency-census gate, continuous donor/dependency recensus loop, discovery dispositions, donor-promotion rule, absorption gates and physical extinction semantics. Implementation agents MUST NOT invent alternate sequencing or donor states when these docs already specify them.
