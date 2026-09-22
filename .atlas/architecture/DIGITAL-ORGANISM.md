---
id: atlas.architecture.digital-organism
type: architecture
status: canonical
canonical: true
---
# Digital Organism Architecture

Atlas Studio is independent of any target runtime. A Digital Organism is a first-class Atlas compilation target, not Atlas itself and not any specific external platform.

A Digital Organism is not equivalent to a model, weights file, agent loop or API client. Model/weights are optional organs inside a larger executable organism substrate.

## Organism equation

```text
Digital Organism
=
Persistent Identity
+ Genome
+ Body
+ Brain / Models
+ World Model
+ Memory
+ Learning
+ Homeostasis
+ Capabilities
+ Authority Constraints
+ Metabolism
+ Lifecycle
+ Evidence Lineage
```

A valid organism MAY use:

- external model/LLM APIs;
- locally/self-hosted models and weights;
- multiple specialized models;
- deterministic algorithms without neural weights;
- hybrid combinations.

Provider sessions, prompt history or model weights do not define organism identity.

## Compilation model

```text
Repositories / research / behavior evidence
        ↓
Strict Census
        ↓
Technology / Behavior / Algorithm Graph
        ↓
Logical *.atlas
        ↓
Organism genome synthesis + selected design
        ↓
*.atlasx binary capsule with target_kind = digital_organism
        ↓
Atlas Compiler
        ↓
Digital Organism Repository / Product
        ├─ genome
        ├─ kernel
        ├─ body
        ├─ brain
        ├─ memory
        ├─ world
        ├─ learning
        ├─ metabolism
        ├─ homeostasis
        ├─ capabilities
        ├─ lifecycle
        ├─ adapters
        ├─ evidence
        └─ ui (optional)
```

## Genome to phenotype

For organism targets:

```text
OrganismGenome + EnvironmentProfile + Compiler
                    ↓
                 Phenotype
```

The phenotype is the running physical realization of the selected organism design. The Genome is durable typed design meaning; the phenotype may differ by hardware, deployment, provider and environment while preserving required identity/capability/lifecycle invariants.

## Brain is an organ

The brain may contain perception, representation, world model, planner, policy, training/inference graphs and one or many model artifacts.

Weights are versioned model artifacts:

```text
ModelDefinition
  ↓
Checkpoint / Weights
  ↓
Evaluation
  ↓
Admission
  ↓
ActiveModelBinding
```

Weights never directly become canonical organism identity, authority or world truth.

## Body

Body is the organism's bounded interaction surface with an environment: filesystem, browser, APIs, database, network, sensors, robot, PLC, camera, microphone, GPU or another runtime substrate.

```text
Perception
  ↓
Internal State
  ↓
World Model
  ↓
Intent
  ↓
Authority / Policy Gate
  ↓
Action
  ↓
Environment
  ↓
Observation
  ↓
Memory / Evidence
```

Body adapters expose capabilities. They do not grant authority.

## Persistent identity

Process lifetime and organism lifetime are distinct.

```text
OrganismId
GenomeId
SpeciesId
Generation
BirthEvent
CurrentState
MemoryLineage
ModelLineage
CapabilityLineage
LearningLineage
```

Restarting a process may create a new RuntimeInstanceId while preserving the same OrganismId and durable lineage.

## Independence

No generated organism requires Chronica or any particular external runtime. Chronica may be one environment/capability substrate among many. Atlas must support standalone, cloud, local, browser, embedded, robot and other target environments according to explicit adapters and profiles.
