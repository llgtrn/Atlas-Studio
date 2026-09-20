---
id: atlas.standard.organism-species
type: contract
status: active
canonical: true
---
# Organism Species Standard

Species is a reusable Organism Genome template/constraint family. It is not a runtime silo and does not create a separate graph universe.

## Species semantics

A Species may define:

- required/optional organs;
- required circuits;
- trait defaults and allowed variation;
- body/environment capability expectations;
- memory classes;
- learning/adaptation policy;
- model/provider requirements or optionality;
- homeostatic variables and safe ranges;
- metabolic objectives;
- lifecycle rules;
- authority/safety constraints;
- validation requirements.

Example conceptual species:

```text
software-agent
knowledge-worker
coding-organism
research-organism
robot-organism
factory-organism
```

These names are templates only. Concrete organisms remain identified by OrganismId and OrganismGenomeId.

## Derivation

```text
Species Genome Template
   ↓ select traits / environment / capabilities
Concrete Organism Genome
   ↓ compiler
Phenotype
```

A Species update does not silently mutate existing organism Genomes. Adoption creates an explicit derived Genome revision with lineage/evidence.

## Provider neutrality

Species may specify cognition capability requirements without naming a provider. A concrete Genome may bind those capabilities to external APIs, self-hosted models, deterministic algorithms or hybrids.

## Cross-species graph

Species, traits, organs and circuits use the same universal Atlas graph primitives and may share immutable semantic components by stable identity.
