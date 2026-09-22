---
id: atlas.architecture.invention-pipeline
type: architecture
status: active
canonical: true
---
# Invention Pipeline

Atlas converts observed implementation and research evidence into reusable engineering knowledge, then invents and validates target-native designs. Census and invention are distinct epistemic layers.

## Full pipeline

~~~text
implementation / source reality
  ↓
exhaustive inventory + dependency closure
  ↓
function + semantic-atom census
  ↓
mechanism / algorithm / protocol extraction
  ↓
invariant / trade-off / limitation
  ↓
Human intent + Genome/security/target constraint envelope
  ↓
Atlas knowledge + donor/dependency knowledge + external research
  ↓
candidate mechanism set
  ↓
typed fast decision fabric (rank / shortlist / route)
  ↓
external synthesis / code generation
  ↓
CandidateChangeSet
  ↓
inventory + census generated implementation as untrusted source
  ↓
compare intended semantics vs observed generated semantics
  ↓
security / dependency / license / tests / benchmark / proof
  ↓
candidate repair / alternative loop
  ↓
validated candidate(s)
  ↓
compare against current canonical blueprint
  ↓
BlueprintRevisionDecision when architecture would change
  ↓
authorized SelectedDesign
  ↓
SEALED logical Atlas
  ↓
deterministic mechanical semantic compaction
  ↓
*.atlas
  ↓
closed-world *.atlasx binary capsule
  ↓
semantic/world optimization
  ↓
compiled product
  ↓
runtime evidence / PGO / recensus
~~~

The detailed authoring/provider/seal rules are canonical in:

- `../contracts/HUMAN-AI-ADL-AUTHORING.md`;
- `../contracts/ATLAS-CREATION-PIPELINE.md`;
- `../contracts/EXTERNAL-PROVIDER-TRUST.md`.

## Census before invention

Observed source reality is never rewritten by inference. Census uses the canonical EpistemicStatus vocabulary: OBSERVED, DECLARED, DERIVED, INFERRED, HYPOTHESIS, CONFLICT, UNKNOWN, UNSUPPORTED and IGNORED. Fact kind, evidence kind and epistemic status remain distinct.

Every function is accounted for. Important blocks and expressions lower to semantic atoms. Dynamic behavior, reflection, macros, FFI, generated code, feature flags, config-driven behavior and unresolved external targets must be explicit rather than silently skipped.

## Research, decision and synthesis are separate roles

Atlas treats external intelligence as replaceable role-specific providers.

~~~text
ResearchProvider
  finds and attributes possibilities

DecisionProvider
  ranks/scores/routes candidate exploration

SynthesisProvider
  writes candidate ADL/code/tests

VerificationProvider
  contributes/checks evidence
~~~

A Jev-class decision provider is suitable for low-latency typed ranking/routing.

A research/search provider may behave like a research assistant over OSS/web/papers.

A frontier synthesis provider may generate real implementation candidate code.

None of these roles owns canonical truth.

## Evidence triangulation

Atlas may correlate code, compiler/parser facts, tests, runtime traces, benchmarks, build graphs, specs/RFCs, scientific papers and explanatory documentation. Source describes current implementation; research explains mechanisms/trade-offs; model output remains candidate analysis until verified.

## Candidate code is part of invention, not a post-Atlas afterthought

Atlas SHOULD be able to synthesize and test the actual implementation before logical Atlas seal.

The preferred loop is:

~~~text
candidate architecture
→ generate implementation
→ census implementation
→ validate
→ revise
→ select
→ seal
~~~

The final logical Atlas should therefore contain/point to the selected implementation semantics and evidence, not merely an architecture description that requires a future model to invent the real logic.

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
→ generate CandidateChangeSet implementation
→ census generated implementation
→ validate
→ select
→ seal logical Atlas
→ mechanically compact into *.atlas
→ materialize via ATLAS-TO-ATLASX contract
→ compile through explicit IR pipeline
→ benchmark / verify / recensus
→ extinction gate
→ delete absorbed donor checkout
```

Donor runtime ownership is forbidden for technology declared native.
