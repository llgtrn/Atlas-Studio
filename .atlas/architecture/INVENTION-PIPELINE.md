---
id: atlas.architecture.invention-pipeline
type: architecture
status: active
canonical: true
---
# Invention Pipeline

Atlas does not census OSS to copy or mechanically translate repositories. It converts implementations into reusable engineering understanding and then synthesizes native target designs.

```text
implementation
  ↓
mechanism
  ↓
algorithm / data structure / protocol
  ↓
invariant
  ↓
trade-off / limitation
  ↓
technology primitive
  ↓
cross-donor comparison
  ↓
candidate synthesis
  ↓
target design
  ↓
*.atlas
  ↓
*.atlasx/
  ↓
compiled implementation
  ↓
verification / benchmark / recensus
```

## Evidence triangulation

A census unit may combine:

- code itself, compiler/parser facts and runtime behavior for current implementation reality;
- tests and benchmarks for observed behavior/performance;
- official specifications and RFCs for normative contracts;
- scientific papers for algorithms, theory and known trade-offs;
- DeepWiki-style explanations and repository docs for navigation and explanatory context;
- model-generated analysis only as inference/candidate until verified.

Conflicting evidence is preserved as conflict, not averaged into false certainty.

## Adaptive depth

Census starts broad and zooms deeper where importance, complexity, risk, uncertainty, state mutation, concurrency, persistence, security/authority, performance or architectural conflict requires it.

Every implementation plan must reconcile three simultaneous views:

```text
GLOBAL IMPACT
∩
LOCAL IMPLEMENTATION
∩
ATOMIC SEMANTICS
```

A plan that is correct only at one scope is incomplete.

## Absorption and extinction

For donor technology:

```text
pin revision
→ capture license/provenance
→ security admission
→ census
→ extract principles/invariants
→ synthesize target-native design
→ write *.atlas
→ materialize *.atlasx/
→ compile
→ test/benchmark
→ recensus
→ evidence
→ extinguish absorbed donor runtime dependency/scope
```

Donor source may remain as temporary evidence/reference while needed, but production-native technology may not secretly depend on it.
