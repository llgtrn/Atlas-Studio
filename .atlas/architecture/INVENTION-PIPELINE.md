---
id: atlas.architecture.invention-pipeline
type: architecture
status: active
canonical: true
---
# Invention Pipeline

Atlas converts observed implementation and research evidence into reusable engineering knowledge, then invents and validates target-native designs. Census and invention are distinct epistemic layers.

## Full pipeline

```text
implementation / source reality
  ↓
exhaustive inventory
  ↓
function + semantic-atom census
  ↓
mechanism / algorithm / protocol extraction
  ↓
invariant / trade-off / limitation
  ↓
cross-donor + research comparison
  ↓
gap / conflict / unknown graph
  ↓
candidate invention
  ↓
experiment / simulation / benchmark / proof
  ↓
validated candidate
  ↓
selected design
  ↓
SEALED logical *.atlas
  ↓
deterministic *.atlasx/
  ↓
semantic/world optimization
  ↓
compiled product
  ↓
runtime evidence / PGO / recensus
```

## Census before invention

Observed source reality is never rewritten by inference. Census records FACT, VERIFIED_OBSERVATION, INFERENCE, HYPOTHESIS, CONFLICT and UNKNOWN distinctly.

Every function is accounted for. Important blocks and expressions lower to semantic atoms. Dynamic behavior, reflection, macros, FFI, generated code, feature flags, config-driven behavior and unresolved external targets must be explicit rather than silently skipped.

## Evidence triangulation

Atlas may correlate code, compiler/parser facts, tests, runtime traces, benchmarks, build graphs, specs/RFCs, scientific papers and explanatory documentation. Source describes current implementation; research explains mechanisms/trade-offs; model output remains candidate analysis until verified.

## Invention at every scope

Every scope may be queried for material improvement opportunities:

- simpler or stronger invariant;
- dependency extinction;
- static binding replacing dynamic indirection;
- state elimination or safer placement;
- lower allocation/copy/serialization cost;
- stronger recovery/concurrency behavior;
- reusable cross-repository capability;
- better algorithm/data structure;
- hardware/deployment specialization.

No material opportunity is itself a valid result. Invention is not mandatory churn.

## Absorption and extinction

```text
bulk stage donors
→ pin SHA/license/provenance
→ coarse census all
→ deep census selected lane
→ extract principles/invariants
→ synthesize/invent Atlas-native design
→ validate
→ commit knowledge into *.atlas
→ materialize *.atlasx/
→ compile/benchmark/recensus
→ extinction gate
→ delete absorbed donor checkout
```

Donor runtime ownership is forbidden for technology declared native.
