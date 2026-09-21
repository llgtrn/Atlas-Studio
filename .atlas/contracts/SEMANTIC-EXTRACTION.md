---
id: atlas.contract.semantic-extraction
type: contract
status: active
canonical: true
---
# Semantic Extraction Contract

Semantic extraction is the boundary between structural source admission and canonical semantic observation.

SourceFrontend answers which structural source family can admit an artifact. SemanticExtractor answers which semantic obligations can be observed for that admitted source.

The two responsibilities are intentionally separate.

## Pipeline position

~~~text
Inventory
→ Structural SourceFrontend
→ SemanticExtractor
→ Raw typed observations
→ Census
→ Normalize
→ Reconcile
→ Graph projection
~~~

An extractor MUST NOT write directly to the engineering graph or sealed ATLAS storage.

## Input contract

An extraction request is scoped to a pinned semantic source unit containing at least:

- artifact identity;
- normalized path;
- language/source family;
- repository revision when available;
- content fingerprint when available;
- admitted bytes or a deterministic handle to them.

The request must also declare the semantic obligation set being attempted.

## Obligation classes

R4 obligation classes include:

~~~text
SYMBOL
TYPE
FUNCTION_IDENTITY
FUNCTION_SIGNATURE
CALL
CONTROL_FLOW
DATA_FLOW
STATE_ACCESS
EFFECT
OWNERSHIP
CONCURRENCY
PERSISTENCE
EVIDENCE_LINK
~~~

An extractor advertises which classes it can produce.

Support for a language does not imply support for every obligation class.

## Output contract

An extraction batch contains:

- extractor identity and version;
- source unit identity;
- typed semantic facts;
- per-obligation coverage;
- diagnostics;
- explicit unresolved dynamic targets;
- explicit UNKNOWN or UNSUPPORTED obligations;
- evidence/provenance links.

A successful call with zero facts is not proof of absence unless the extractor also proves closure for the relevant obligation.

## Determinism

For identical pinned input, extractor version, configuration, and obligation set, output ordering and stable fact identity MUST be deterministic.

Extractor implementation details may use compiler APIs, language servers, SCIP, tree-sitter, custom parsers, build metadata, or other engines. Those mechanisms do not own canonical truth.

## Multi-engine reconciliation

Independent engines may disagree.

Extractor disagreement is preserved as evidence and routed to reconciliation. It must never be silently flattened during extraction.

## Failure semantics

- parser failure is not verified absence;
- unsupported syntax is UNSUPPORTED;
- unresolved dynamic behavior is UNKNOWN unless stronger evidence exists;
- intentionally excluded work is IGNORED only when an explicit policy identifies the exclusion.

Silent skip is forbidden.

## AI boundary

Model-assisted extraction may produce INFERRED or HYPOTHESIS candidates.

A model response cannot directly emit OBSERVED facts without independent admitted evidence.
