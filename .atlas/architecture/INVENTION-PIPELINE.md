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
compare against current canonical blueprint
  ↓
BlueprintRevisionDecision when architecture would change
  ↓
selected design / selected blueprint revision
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

Observed source reality is never rewritten by inference. Census uses the canonical EpistemicStatus vocabulary: OBSERVED, DECLARED, DERIVED, INFERRED, HYPOTHESIS, CONFLICT, UNKNOWN, UNSUPPORTED and IGNORED. Fact kind, evidence kind and epistemic status remain distinct.

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

## Blueprint evolution from invention/census

A validated candidate may reveal that the current canonical blueprint is no longer the best design.

Atlas is allowed to change the blueprint.

The change is governed by `../contracts/BLUEPRINT-EVOLUTION.md` and MUST be explicit.

~~~text
observed mechanism / validated candidate
→ current blueprint comparison
→ alternatives comparison
→ invariant + migration + performance analysis
→ BlueprintRevisionDecision
→ SELECTED
→ update canonical blueprint/roadmap/contracts if required
→ implementation
→ recensus / verification
~~~

This distinction matters:

~~~text
new implementation under unchanged design
= ordinary implementation/optimization

better architecture or semantic boundary
= blueprint revision

change to higher-level normative invariant
= explicit contract/Genome revision
~~~

Do not force a superior discovered mechanism into an obsolete blueprint merely to keep old docs unchanged.

Do not silently change the blueprint in code either.

## Absorption and extinction

```text
bulk stage donors
→ pin SHA/license/provenance
→ coarse census all + transitive dependency closure
→ attribute mechanisms to actual providers
→ explicit discovery disposition
→ deep census selected provider scope
→ extract principles/invariants
→ synthesize/invent Atlas-native design
→ validate
→ commit knowledge into *.atlas
→ materialize via ATLAS-TO-ATLASX contract
→ compile through explicit IR pipeline
→ benchmark / verify / recensus
→ extinction gate
→ delete absorbed donor checkout
```

Donor runtime ownership is forbidden for technology declared native.
