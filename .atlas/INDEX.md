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
18. `contracts/BLUEPRINT-EVOLUTION.md`
19. `roadmap/DONOR-ABSORPTION-ROADMAP.md`
20. `roadmap/DONOR-ABSORPTION-PLAN.toml`
21. `references/deepwiki/DONOR-ARCHITECTURE-SYNTHESIS.md`
22. `contracts/ATLAS-DEVELOPMENT-LANGUAGE.md`
23. `contracts/ADL-TO-ATLAS.md`
24. `contracts/DONOR-TO-LANGUAGE-GENESIS.md`
25. `contracts/ATLAS-FORMAT.md`
26. `contracts/ATLAS-SEMANTIC-COMPACTION.md`
27. `contracts/ATLAS-BINARY-WIRE-FORMAT.md`
28. `contracts/ATLAS-SHARDING.md`
29. `contracts/ATLAS-TO-ATLASX.md`
30. `contracts/ATLASX-FORMAT.md`
31. `contracts/COMPILER-IR-PIPELINE.md`
32. `architecture/DIGITAL-ORGANISM.md`
33. `contracts/ORGANISM-GENOME-v1.md`
34. `contracts/ORGANISM-LIFECYCLE.md`
35. `contracts/ORGANISM-MODEL-ADMISSION.md`
36. `contracts/ORGANISM-HOMEOSTASIS-METABOLISM.md`
37. `standards/ORGANISM-SPECIES.md`
38. `blueprints/ORGANISM-COMPILATION.md`
39. `contracts/COMPILER-PRODUCT.md`
40. `standards/COMPILER-OPTIMIZATION.md`
41. `architecture/INVENTION-PIPELINE.md`
42. `blueprints/SYSTEM-BLUEPRINT.md`
43. `blueprints/COMPILER-ROADMAP.md`
44. `blueprints/RUST-PARITY-AND-SURPASS-ROADMAP.md`
45. `blueprints/BULK-DONOR-ABSORPTION.md`
46. `contracts/SYSTEM-CONTRACT.md`
47. `guides/DEVELOPMENT.md`

Canonical chain:

~~~text
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
  ↓ lossless semantic compaction
  ↓ binary wire + content-addressed shards
SelectedDesign
  ↓ deterministic selection closure / explicit bindings / expansion
Validated *.atlasx root
  ↓ explicit compiler lowering
HIR
  ↓
MIR
  ↓ target/layout/ABI lowering
LIR
  ↓ instruction lowering
Machine IR
  ↓ codegen / object / LTO / link / post-link
Product
  ↓ workload profiles / PGO / auto-tuning
Evidence + recensus
  ↺
~~~

The universal graph/binding/evidence/temporal grammar remains invariant across census, storage, materialization, optimization and generated repositories/products.

## Mandatory anti-drift reading

For implementation work between R4 and R8:

- `roadmap/SELF-BUILDING-R4-R8.md` fixes the active self-building/census sequence;
- `contracts/BLUEPRINT-EVOLUTION.md` fixes how evidence may change that sequence/architecture;
- `contracts/ATLAS-SEMANTIC-COMPACTION.md` prevents storage compression from deleting meaning;
- `contracts/ATLAS-TO-ATLASX.md` prevents hidden invention during materialization;
- `contracts/COMPILER-IR-PIPELINE.md` prevents implementation-defined HIR/MIR/LIR/Machine IR.

Implementation agents MUST NOT invent alternate donor states, semantic paths, materialization rules or compiler-stage meanings when these canonical docs already specify them.

## Blueprint evolution rule

Canonical blueprints are authoritative but intentionally revisable.

If donor/dependency census finds a demonstrably better mechanism, Atlas may revise the relevant blueprint after provider attribution, deep census, evidence comparison, validation/benchmark/proof and an explicit BlueprintRevisionDecision.

Do not ignore superior evidence merely to preserve old prose.

Do not silently redesign in code either.
