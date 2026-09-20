---
id: atlas.blueprint.organism-compilation
type: blueprint
status: active
canonical: true
---
# Digital Organism Compilation Blueprint

Digital Organism is a first-class target kind of the general Atlas compiler.

## Source-to-organism pipeline

```text
repositories / research / model systems / runtime evidence
              ↓
          strict census
              ↓
technology + behavior + algorithm graph
              ↓
        logical *.atlas
              ↓
organism species/trait/genome synthesis
              ↓
selected organism design
              ↓
*.atlasx/ [target_kind = digital_organism]
              ↓
Atlas compiler
              ↓
phenotype repository/product
              ↓
instantiate / birth
              ↓
observe / remember / learn
              ↓
candidate adaptation
              ↓
validate / admit
              ↓
evolve
```

## AtlasX organism profile

A digital-organism AtlasX materialization contains general executable semantics plus:

```text
organism/
├─ genome/
├─ identity/
├─ organs/
├─ circuits/
├─ body/
├─ brain/
├─ world/
├─ memory/
├─ learning/
├─ homeostasis/
├─ metabolism/
├─ capabilities/
├─ lifecycle/
├─ adapters/
├─ evidence/
└─ profiles/
```

This is a semantic layout. Physical source/output folders may be specialized by target language/platform.

## Phenotype repository

A bootstrap generated repository may materialize:

```text
organism/
├─ genome/
├─ kernel/
├─ body/
├─ brain/
├─ memory/
├─ world/
├─ learning/
├─ metabolism/
├─ homeostasis/
├─ capabilities/
├─ lifecycle/
├─ adapters/
├─ evidence/
└─ ui/
```

Backend/runtime code is Rust during bootstrap; UI is TypeScript/TSX; model artifacts may be external, local or absent.

## Species

Species are reusable Genome templates/constraints, for example software-agent, knowledge-worker, coding-organism, robot-organism or research-organism. They are not hardwired product silos.

Species define defaults and required traits/organs; concrete organism Genomes select/override within policy.

## Environment independence

The compiled organism may run standalone or bind to arbitrary environments through adapters. No Chronica dependency is required. Chronica may be one optional world/capability environment among others.

## Training is not compilation completion

Compiler output may include training pipelines/model definitions, but a usable organism substrate exists independently of whether weights are newly trained. External APIs, preexisting checkpoints, deterministic policies or hybrid cognition are valid.
