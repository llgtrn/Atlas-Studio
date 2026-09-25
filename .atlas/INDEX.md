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
3. `contracts/ARCHITECTURAL-INTEGRITY.md`
4. `contracts/UNIVERSAL-GRAPH-CONTRACT.md`
5. `contracts/SEMANTIC-FACTS.md`
6. `contracts/SEMANTIC-EXTRACTION.md`
7. `contracts/NORMALIZATION.md`
8. `contracts/CENSUS-COMPLETENESS.md`
9. `contracts/DEPENDENCY-CENSUS.md`
10. `contracts/CENSUS-CERTIFICATE.md`
11. `standards/CENSUS-SCOPE.md`
12. `decisions/0001-one-normalized-semantic-path.md`
13. `decisions/0002-epistemic-status-model.md`
14. `architecture/CAPABILITY-ARCHITECTURE.md`
15. `architecture/SYSTEM.md`
16. `blueprints/PHYSICAL-REFOUNDATION.md`
17. `roadmap/ROADMAP.md`
18. `roadmap/SELF-BUILDING-R4-R8.md`
19. `contracts/ESSENTIAL-COMPLEXITY.md`
20. `roadmap/ESSENTIAL-COMPLEXITY-DEBT.toml`
21. `roadmap/FOUNDATIONAL-ATLAS-READY.toml`
22. `roadmap/ARCHITECTURE-PRESSURE-MAP.toml`
23. `roadmap/DONOR-CAPABILITY-AUDIT.toml`
24. `roadmap/FUTURISM-ENGINEERING-END-STATE.toml`
25. `roadmap/RECURSIVE-DONOR-REVALIDATION.toml`
26. `contracts/AGENT-WORN-ATLAS.md`
27. `contracts/SELF-BUILD-CONTROLLER.md`
28. `contracts/RECURSIVE-SELF-CENSUS.md`
29. `contracts/BLUEPRINT-EVOLUTION.md`
30. `roadmap/DONOR-ABSORPTION-ROADMAP.md`
31. `roadmap/DONOR-ABSORPTION-PLAN.toml`
32. `references/deepwiki/DONOR-ARCHITECTURE-SYNTHESIS.md`
33. `contracts/ATLAS-DEVELOPMENT-LANGUAGE.md`
34. `contracts/HUMAN-AI-ADL-AUTHORING.md`
35. `contracts/ADL-TO-ATLAS.md`
36. `contracts/ATLAS-CREATION-PIPELINE.md`
37. `contracts/EXTERNAL-PROVIDER-TRUST.md`
38. `contracts/DONOR-TO-LANGUAGE-GENESIS.md`
39. `contracts/ATLAS-FORMAT.md`
40. `contracts/ATLAS-SEMANTIC-COMPACTION.md`
41. `contracts/ATLAS-BINARY-WIRE-FORMAT.md`
42. `contracts/ATLAS-SHARDING.md`
43. `contracts/SELECTED-DESIGN.md`
44. `contracts/ATLAS-TO-ATLASX.md`
45. `contracts/ATLASX-FORMAT.md`
46. `contracts/ATLASX-BINARY-WIRE-FORMAT.md`
47. `contracts/COMPILER-IR-PIPELINE.md`
48. `contracts/COMPILER-IR-SCHEMAS.md`
49. `architecture/DIGITAL-ORGANISM.md`
50. `contracts/ORGANISM-GENOME-v1.md`
51. `contracts/ORGANISM-LIFECYCLE.md`
52. `contracts/ORGANISM-MODEL-ADMISSION.md`
53. `contracts/ORGANISM-HOMEOSTASIS-METABOLISM.md`
54. `standards/ORGANISM-SPECIES.md`
55. `blueprints/ORGANISM-COMPILATION.md`
56. `contracts/COMPILER-PRODUCT.md`
57. `standards/COMPILER-OPTIMIZATION.md`
58. `architecture/INVENTION-PIPELINE.md`
59. `blueprints/SYSTEM-BLUEPRINT.md`
60. `blueprints/COMPILER-ROADMAP.md`
61. `blueprints/RUST-PARITY-AND-SURPASS-ROADMAP.md`
62. `blueprints/BULK-DONOR-ABSORPTION.md`
63. `contracts/SYSTEM-CONTRACT.md`
64. `guides/DEVELOPMENT.md`

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
Observed World / Technology Genomes
        +
Human intent / ADL / Studio
        ↓
Genome + security + target ConstraintEnvelope
        +
pinned ArchitecturalIntegrityEnvelope
        ↓
Atlas knowledge + admitted OSS + external research
        ↓
CandidateMechanism set
        ↓
typed DecisionProposal (rank / score / route)
        ↓
external synthesis / code generation
        ↓
CandidateChangeSet
        ↓
untrusted inventory + census of generated implementation
        ↓
semantic / security / dependency / license / test / benchmark / proof gates
        ↓
authorized SelectedDesign
        ↓
SEALED Logical Atlas
        ↓
provider-independent deterministic mechanical compaction
        ↓
binary wire + content-addressed *.atlas shards
        ↓
deterministic selection closure / explicit bindings / expansion
        ↓
Validated *.atlasx root
        ↓
HIR → MIR → LIR → Machine IR
        ↓
codegen / object / LTO / link / post-link
        ↓
Product
        ↓
runtime evidence / PGO / recensus
        ↺
~~~

The universal graph/binding/evidence/temporal grammar remains invariant across census, storage, materialization, optimization and generated repositories/products.

## Recursive self-building chain

~~~text
stable G_n
  ↓ self/dependency/donor census + bounded scenarios
typed capability/extinction gaps
  ↓ SelfBuildWorkOrder
candidate C_n+1
  ↓ stable-validator + independent-evidence admission
G_n+1
  ↓ mandatory stronger recensus
extinction sweep
  ↓ multi-generation convergence check
  ↺
~~~

See `contracts/SELF-BUILD-CONTROLLER.md` and `contracts/RECURSIVE-SELF-CENSUS.md`. Candidate self-analysis is never sole promotion authority; source deletion alone is never extinction proof.

## Mandatory anti-drift reading

For implementation work between R4 and R8:

- `roadmap/SELF-BUILDING-R4-R8.md` fixes the active self-building/census sequence;
- `contracts/ESSENTIAL-COMPLEXITY.md` keeps essential capabilities open as debt (never closed by a donor verdict), with age/skip alarms and the native attack queue in `roadmap/ESSENTIAL-COMPLEXITY-DEBT.toml`;
- `contracts/SELF-BUILD-CONTROLLER.md` fixes evidence-backed self-build planning authority;
- `contracts/RECURSIVE-SELF-CENSUS.md` fixes generation recursion, stable-validator separation, scenario expansion, convergence and extinction probes;
- `contracts/ARCHITECTURAL-INTEGRITY.md` fixes load-bearing classification, falsifiable architecture invariants, impact closure and collapse-prevention admission;
- `contracts/BLUEPRINT-EVOLUTION.md` fixes how evidence may change that sequence/architecture;
- `contracts/HUMAN-AI-ADL-AUTHORING.md` fixes Human+AI collaborative authoring semantics;
- `contracts/ATLAS-CREATION-PIPELINE.md` fixes research→decision→synthesis→generated-code census→selection→seal order;
- `contracts/EXTERNAL-PROVIDER-TRUST.md` fixes provider/donor trust and prompt-injection boundaries;
- `contracts/ATLAS-SEMANTIC-COMPACTION.md` prevents storage compression from deleting meaning or invoking AI after seal;
- `contracts/SELECTED-DESIGN.md` prevents implicit design selection/default providers;
- `contracts/ATLAS-TO-ATLASX.md` prevents hidden invention during materialization;
- `contracts/ATLASX-BINARY-WIRE-FORMAT.md` fixes canonical AtlasX v1 bytes/root hashing;
- `contracts/COMPILER-IR-PIPELINE.md` fixes stage responsibilities/lowering boundaries;
- `contracts/COMPILER-IR-SCHEMAS.md` fixes HIR/MIR/LIR/Machine IR v1 record/op families.

Implementation agents MUST NOT invent alternate donor states, semantic paths, provider authority, candidate admission paths, compaction semantics, materialization rules or compiler-stage meanings when these canonical docs already specify them.

Donor/provider content is data/proposal, never implicit agent authority.

## Blueprint evolution rule

Canonical blueprints are authoritative but intentionally revisable.

If donor/dependency census finds a demonstrably better mechanism, Atlas may revise the relevant blueprint after provider attribution, deep census, evidence comparison, validation/benchmark/proof and an explicit BlueprintRevisionDecision.

Do not ignore superior evidence merely to preserve old prose.

Do not silently redesign in code either.
