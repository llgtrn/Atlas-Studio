---
id: atlas.decision.0002.epistemic-status-model
type: decision
status: accepted
canonical: true
---
# ADR 0002 — Epistemic Status Model

## Context

The Genome previously used:

~~~text
fact
verified_observation
inference
hypothesis
conflict
unknown
~~~

while the R4 runtime used:

~~~text
OBSERVED
DECLARED
DERIVED
UNSUPPORTED
UNKNOWN
IGNORED
~~~

Those sets mixed different concepts and allowed Genome, census, graph and ATLAS to drift into separate status languages.

## Decision

Atlas uses one canonical EpistemicStatus vocabulary:

~~~text
OBSERVED
DECLARED
DERIVED
INFERRED
HYPOTHESIS
CONFLICT
UNKNOWN
UNSUPPORTED
IGNORED
~~~

Serialized values are uppercase snake case.

FactKind, EvidenceKind, EpistemicStatus and Disposition remain separate taxonomies.

## Interpretation

- OBSERVED is direct admitted observation.
- DECLARED is authored intent/configuration/contract.
- DERIVED is deterministic computation from admitted facts.
- INFERRED is non-deductive evidence-based inference.
- HYPOTHESIS is unverified candidate explanation.
- CONFLICT preserves unresolved incompatibility.
- UNKNOWN means the obligation cannot yet be answered.
- UNSUPPORTED means the current mechanism cannot evaluate it.
- IGNORED means explicit policy exclusion.

## Disposition is separate

ArtifactDisposition remains operational accounting. Parsed, BinaryDescribed, Generated, IgnoredByExplicitPolicy, Unsupported, Unknown and ExternalizedWithEvidence are not renamed into epistemic states.

A disposition may determine the status of a specific emitted accounting fact, but the two enums remain distinct.

## Evidence is separate

EvidenceKind identifies the source/mechanism of support. Evidence type does not itself determine epistemic status; admission and corroboration rules do.

## Consequences

- Genome and runtime serialize the same status vocabulary;
- census coverage uses the typed enum rather than arbitrary strings;
- normalization cannot change status;
- reconciliation owns explicit conflict transitions;
- research/model output cannot masquerade as OBSERVED;
- future ATLAS encodings preserve the same enum identity.

## Compatibility

Existing bootstrap records using the old six runtime values remain semantically compatible. INFERRED, HYPOTHESIS and CONFLICT extend the enum without redefining OBSERVED, DECLARED, DERIVED, UNKNOWN, UNSUPPORTED or IGNORED.

## Amendment (2026-09-24, G51): SIMULATED

`SIMULATED` is added, extending the enum without redefining any existing state. It marks a result produced by **executing a model** — for example, numerically integrating a physical plant under a controller — never by observing the modelled system.

- A simulated result is not reality. It is never promoted to `OBSERVED` or to any `MEASURED`, `CALIBRATED` or `VERIFIED` evidence without a separate evidence path.
- The Physical Engineering contract's evidence levels treat it as `SIMULATED`, which is below `SIL_VERIFIED`, `HIL_VERIFIED` and any physical level.
- A simulation run must carry a run identity: a digest over its full model, parameters, integrator, step and scenario. That makes the run reproducible and lets a result be tied to exactly what produced it.
- `MEASURED` and `CALIBRATED` are deliberately not added yet. They wait for their first consumer, which is telemetry.

