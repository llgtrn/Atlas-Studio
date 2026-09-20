---
id: atlas.contract.organism-genome.v1
type: contract
status: active
canonical: true
---
# Atlas Organism Genome v1

Organism Genome is the typed executable design contract for a Digital Organism target. It is distinct from Atlas Genome: Atlas Genome governs Atlas itself; Organism Genome defines the organism Atlas is compiling.

## Required semantic classes

```text
Genome
├─ Identity
├─ Species / Traits
├─ Morphology / Organs
├─ Circuits / Flows
├─ Cognition
├─ Perception
├─ WorldModel
├─ Memory
├─ Learning
├─ Adaptation
├─ Body
├─ Capabilities
├─ AuthorityConstraints
├─ Homeostasis
├─ Metabolism
├─ Lifecycle
├─ ModelBindings
├─ EvidenceRequirements
└─ Evolution / Derivation Policy
```

The universal Atlas graph spine remains authoritative underneath these organism-specific extensions.

## Core invariants

- PERSISTENT_ORGANISM_IDENTITY_REQUIRED.
- PROCESS_RESTART_MUST_NOT_IMPLY_NEW_ORGANISM.
- WEIGHTS_ARE_ORGANS_NOT_IDENTITY.
- MODEL_PROVIDER_IS_CAPABILITY_PROVIDER_NOT_TRUTH_OWNER.
- MEMORY_IS_DURABLE_LINEAGE_NOT_PROMPT_CONTEXT.
- LEARNING_OUTPUT_IS_CANDIDATE_UNTIL_ADMITTED.
- ORGANISM_CANNOT_SELF_GRANT_AUTHORITY.
- BODY_ACTIONS_REQUIRE_EXPLICIT_CAPABILITY_AND_AUTHORITY_BINDINGS.
- HOMEOSTASIS_IS_TYPED_STATE_AND_POLICY_NOT_METAPHOR.
- METABOLISM_HAS_MEASURABLE_RESOURCE_INPUTS_AND_CAPABILITY_OUTPUTS.
- LIFECYCLE_TRANSITIONS_ARE_DURABLE_EVENTS.
- SELF_MODIFICATION_REQUIRES_VALIDATION_AND_ADMISSION.
- GENOME_DERIVATION/REPRODUCTION_IS_POLICY_GOVERNED, NOT UNBOUNDED AUTONOMOUS REPLICATION.

## Model-provider modes

An organism declares one or more model bindings:

```text
external_api
self_hosted
local_embedded
hybrid
none
```

Provider replacement MUST be possible when the Genome declares provider-agnostic capability semantics. Provider-specific constraints remain adapter-level evidence.

## Organ and circuit model

An Organ is a bounded semantic subsystem. A Circuit is a typed flow/binding connecting organs, state, capabilities, observations or actions.

Example:

```text
PerceptionOrgan
  → RepresentationCircuit
  → WorldModelOrgan
  → PlannerOrgan
  → PolicyOrgan
  → AuthorityGate
  → BodyCapability
```

Compiler optimizations may fuse physical organs/circuits only when observable Genome semantics remain equivalent.

## Trait model

Traits are inherited/selected design properties, not hidden prompts. Traits have identity, provenance, constraints and temporal validity.

## Reproduction / derivation

A child Genome is an explicit derived artifact with parent Genome lineage, policy approval, diff/evidence and new immutable GenomeId. Runtime organisms do not silently clone themselves.
