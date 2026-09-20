---
id: atlas.contract.organism-homeostasis-metabolism
type: contract
status: active
canonical: true
---
# Organism Homeostasis and Metabolism Contract

Homeostasis and metabolism are typed runtime semantics, not biological decoration.

## Homeostatic state

A Genome may define a vector including:

```text
CPU
RAM
VRAM
disk
energy
latency
error rate
queue/backlog
network state
confidence
model drift
memory pressure
risk
cost/budget
```

Each metric has observation source, temporal validity, safe/target ranges and permitted responses.

## Homeostatic actions

Examples include:

- reduce context/window;
- select a smaller admitted model;
- throttle or defer work;
- consolidate memory;
- increase observation before acting;
- migrate placement;
- rollback checkpoint;
- request external capacity;
- suspend non-critical capability.

Homeostatic response never bypasses authority/safety constraints.

## Metabolism

Metabolism measures resource conversion:

```text
inputs:
  data / compute / CPU / GPU / memory / bandwidth / energy / tokens / money

outputs:
  useful work / predictions / actions / knowledge / memory / training examples
```

A Genome may define one or more efficiency objectives rather than one universal scalar.

## Compiler role

Atlas may optimize organ placement, model selection, memory layout, batching, scheduling and provider bindings for metabolic objectives under hard correctness/authority/safety constraints.
