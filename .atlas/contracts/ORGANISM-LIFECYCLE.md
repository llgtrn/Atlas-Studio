---
id: atlas.contract.organism-lifecycle
type: contract
status: active
canonical: true
---
# Digital Organism Lifecycle Contract

A Digital Organism has a durable lifecycle independent of process uptime.

## Required identities

- OrganismId
- OrganismGenomeId
- SpeciesId
- Generation
- RuntimeInstanceId
- MemoryRoot
- ModelSetId / ActiveCheckpoint bindings
- CapabilitySetId
- LearningLineageId

## Lifecycle states

A target may extend but not erase:

```text
DESIGNED
→ BUILT
→ BORN
→ ACTIVE
↔ SUSPENDED
→ RETIRED
→ ARCHIVED
```

Failure/recovery transitions are explicit and evidence-linked.

## Birth

Birth creates the durable OrganismId and binds a Genome, initial state, memory root, capability set and admitted model set. Build completion alone is not birth.

## Restart / wake

Runtime restart creates a new RuntimeInstanceId, not a new organism identity.

## Memory lineage

Working context may disappear. Durable memory lineage must remain attributable to the organism and may be compacted/consolidated without silently rewriting history.

## Learning lifecycle

```text
Experience
  ↓
Durable Observation / Event
  ↓
Memory
  ↓
Dataset Compiler
  ↓
Training / Adaptation
  ↓
Candidate Model / Candidate Rule
  ↓
Evaluation
  ↓
Simulation / Regression / Safety Gates
  ↓
Admission
  ↓
Activation
  ↓
Evidence + rollback point
```

Candidate learning output is never automatically active.

## Self-modification

Changes to weights, policies, memory algorithms, capability implementations or Genome-derived structure are typed changes with scope, evidence, compatibility and rollback requirements.

## Death / retirement

Retirement revokes active execution/capability bindings but preserves lineage/evidence according to retention policy.
