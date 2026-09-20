---
id: atlas.contract.format.atlasx
type: contract
status: active
canonical: true
---
# ATLASX Expanded Executable Representation Contract

`<system>.atlasx/` is the deterministic expanded executable representation of a selected design from one pinned logical Atlas root.

ATLASX is executable engineering representation, not expanded prose.

## General shape

```text
<system>.atlasx/
├─ manifest.atlasx
├─ graph/
├─ modules/
├─ interfaces/
├─ runtime/
├─ ui/
├─ tests/
├─ profiles/
│  ├─ deployment.atlasx
│  ├─ hardware.atlasx
│  └─ workload.atlasx
└─ targets/
```

## Digital Organism profile

When `target_kind = digital_organism`, AtlasX extends the general executable representation with:

```text
organism/
├─ genome/
├─ identity/
├─ species_traits/
├─ organs/
├─ circuits/
├─ body/
├─ brain/
│  ├─ model_definitions/
│  ├─ provider_bindings/
│  ├─ training/
│  ├─ inference/
│  └─ checkpoint_manifests/
├─ world/
├─ memory/
├─ learning/
├─ homeostasis/
├─ metabolism/
├─ capabilities/
├─ authority/
├─ lifecycle/
├─ adapters/
└─ evidence/
```

This is the executable Organism Genome/phenotype source representation. It is not equivalent to a weights directory.

## Semantic requirements

General AtlasX carries types, function bodies/CFG, ownership/resource semantics, state/effects, concurrency/transactions/recovery, temporal semantics, bindings, constraints, placement and evidence lineage.

Digital Organism AtlasX additionally carries:

- OrganismId/Genome/Species/Generation identity rules;
- organ/circuit topology;
- body/environment capability bindings;
- model/provider/checkpoint bindings;
- durable memory and learning lineage;
- model/weight admission rules;
- homeostatic variables/responses;
- metabolic resources/objectives;
- lifecycle transitions;
- authority and self-modification constraints.

## Provider independence

An organism may bind cognition to external APIs, self-hosted weights, local embedded models, deterministic algorithms or hybrids. Provider-specific sessions are adapters, not organism identity or canonical memory.

## Compiler input

The same AtlasX semantics may produce multiple physical phenotypes under different DeploymentProfile, HardwareProfile, WorkloadProfile and admitted model/provider bindings. Physical specialization does not change durable organism identity rules or Genome semantics.
