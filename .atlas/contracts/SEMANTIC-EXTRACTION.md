---
id: atlas.contract.semantic-extraction
type: contract
status: active
canonical: true
---
# Semantic Extraction Contract

## Purpose

**SourceFrontend** owns structural source recognition. A semantic extractor owns evidence-producing analysis behind that frontend. Neither owns canonical truth.

R4 extraction converts a pinned admitted artifact into typed raw observations and explicit unresolved obligations. It does not normalize identities globally, reconcile conflicts, invent facts, or write the engineering graph directly.

## Canonical path

~~~text
Inventory
→ SourceFrontend
→ SemanticExtractor
→ Raw typed observations
→ Census
→ Normalize
→ Reconcile
→ Engineering Graph / ATLAS projection
~~~

There is one normalized semantic path. Direct extractor → graph, parser → graph or compiler-metadata → graph shortcuts are forbidden.

## Extractor identity

Every extractor MUST expose a stable identity and implementation version. Results are attributable to both.

Conceptual obligation:

~~~text
SemanticExtractor
├─ id
├─ version
├─ supported_languages
├─ semantic_dimensions
└─ extract(ExtractionInput) → ExtractionBatch
~~~

The physical Rust trait may differ, but these obligations may not disappear.

## ExtractionInput

An input MUST identify repository, exact revision, artifact identity/path, content fingerprint when available, source frontend identity, language/profile, scope policy, requested semantic dimensions and relevant build/config profile.

Extractors MUST NOT read undeclared mutable global state as semantic input.

## ExtractionBatch

~~~text
ExtractionBatch
├─ extractor_id/version
├─ artifact/revision identity
├─ typed observations[]
├─ evidence[]
├─ obligation results[]
├─ dynamic/unresolved records[]
├─ diagnostics[]
└─ input fingerprint
~~~

Every requested dimension receives an obligation result. Silent omission is forbidden.

## Semantic dimensions

Minimum R4 dimensions:

~~~text
SYMBOL
TYPE
FUNCTION_IDENTITY
FUNCTION_SIGNATURE
CALL
CONTROL_FLOW
DATA_FLOW
STATE
EFFECT
OWNERSHIP
CONCURRENCY
PERSISTENCE
~~~

Build/dependency metadata may come from dedicated resolvers/extractors but enters the same census path. Dependency extraction MUST satisfy `DEPENDENCY-CENSUS.md`: root-manifest parsing alone is insufficient; resolution is contextual and transitively closed.

## Dependency extraction boundary

A package/build ecosystem adapter may discover dependency declarations and resolution evidence, but it does not own dependency truth.

Conceptually:

~~~text
DependencyResolver
├─ id/version
├─ supported ecosystems/build systems
├─ resolve(root, resolution_context)
└─ → typed dependency nodes + edges + diagnostics + evidence
~~~

Resolved dependency records enter Census before normalization/reconciliation. Independent sources such as manifests, lockfiles, package-manager metadata, compiler metadata, binary linkage and runtime traces may disagree; such disagreements remain explicit reconciliation obligations.

Source-backed dependency nodes are recursively admitted to inventory/census until dependency fixed point. Non-source nodes terminate only through explicit typed boundary disposition.

## Obligation result

For each requested dimension and applicable scope, an extractor emits one of:

- evidence-backed OBSERVED facts;
- verified absence with closure evidence;
- UNKNOWN;
- UNSUPPORTED;
- policy-backed IGNORED;
- observations that later reconcile into CONFLICT.

UNKNOWN means evidence is insufficient. UNSUPPORTED means the current extractor/runtime cannot perform the analysis. They are not interchangeable.

A dimension MAY emit evidence-backed observations while its overall obligation remains UNKNOWN when the extractor covers only part of the dimension's declared semantic surface. UNKNOWN with non-empty observation identities means "useful partial evidence exists; closure is not proven." Verified absence is permitted only when the full declared obligation scope for that extractor/profile was exhaustively checked. A partial STATE/EFFECT extractor MUST NOT turn "no fact found in the subset I understand" into a negative semantic fact about unmodeled state/effect forms.

## Determinism

For identical artifact content, repository revision, extractor version, language/build profile and scope policy, output MUST be semantically equivalent independent of traversal order, thread scheduling or filesystem enumeration order.

Generated record IDs must not depend on nondeterministic collection ordering.

## Evidence rules

Observed facts identify the admitted source/revision and relevant span/metadata record when available.

Parser output, compiler metadata and runtime traces are evidence sources, not truth owners.

AI/model analysis MUST NOT emit OBSERVED source facts. Model-assisted analysis enters as INFERRED or HYPOTHESIS until corroborated by an allowed observation path.

## Dynamic behavior

Reflection, dynamic dispatch, macros/proc-macros, generated code, FFI, SQL/shell strings, plugin loading, feature flags and environment-driven behavior require explicit records.

Allowed outcomes include resolved target sets, partial sets, dynamic placeholders and unknowns. “Could not resolve” never means “does not exist”.

## Multi-engine extraction

Independent extractors MAY analyze the same dimension. Their identities/evidence remain separate. Disagreement creates a reconciliation obligation; one extractor may not overwrite another.

## Failure semantics

Extractor crashes, parser errors, resource limits and unsupported syntax become diagnostics plus UNKNOWN or UNSUPPORTED according to cause. A failure must not remove the artifact from census accounting.

**Implementation note (R4.3.5, `adapter::semantic::rust`)**: a resource-limit failure mode is now real, not only declared vocabulary. `syn` is a recursive-descent parser; nothing previously bounded the structural nesting depth of an admitted artifact's source, only its byte size (`MAX_SEMANTIC_BYTES`, a separate, size-only gate). A small file with extreme bracket nesting reliably overflowed the stack and aborted the whole extraction process -- for every artifact in the run, not just the pathological one -- rather than producing a diagnostic. `RustSemanticExtractor::extract` now pre-scans raw source text for bracket-nesting depth (`max_bracket_nesting_depth`) before ever invoking `syn`, and refuses to parse (every supported dimension `UNKNOWN`, `DiagnosticCode::ResourceLimit`) above `MAX_BRACKET_NESTING_DEPTH = 64` -- far below the observed crash floor (a 300-level nesting reliably overflowed even a reduced 2MB test-thread stack) and far above any real nesting depth this repository's own source corpus has ever reached (13).

## Security boundary

Source is untrusted input. Extraction is analysis, not execution.

Executing build scripts, tests, proc-macros, binaries or project code requires a separately authorized/sandboxed capability.

## R4 acceptance matrix

| Requirement | Current bootstrap | R4 completion condition |
| --- | --- | --- |
| Semantic ontology | Contract locked | Runtime records map losslessly to typed FactKind families |
| Epistemic taxonomy | Six-state bootstrap | Canonical nine-state enum used end-to-end |
| Structural frontend | Materialized | Remains adapter recognition boundary; no graph authority |
| Semantic extractor interface | Missing | Native versioned extractor registry/output contract exists |
| Function identity/signature | Missing | Every discovered function has stable identity/signature or explicit unresolved status |
| Symbol/type | UNSUPPORTED | Evidence-backed facts or explicit scoped unresolved results |
| Call graph | UNSUPPORTED | Static/dynamic/unresolved calls represented without omission |
| CFG/dataflow | UNSUPPORTED | Applicable functions have typed facts or explicit obligation status |
| State/effect | UNSUPPORTED | Reads/writes/transitions/effects represented with evidence |
| Ownership/concurrency/persistence | Philosophy only | Applicable profiles emit typed facts or explicit scoped status |
| Revision/content evidence | Partial | Facts tied to pinned revision and input fingerprint |
| Normalization | Predicate bootstrap | Identity/equivalence/exact-dedup rules implemented |
| Conflict handling | Missing | Conflicting raw facts survive into reconciliation input |
| Graph path | Normalized facts feed graph | No parser/extractor bypass creates canonical semantics |
| CensusCertificate | Contract/schema only | Runtime issuance remains R6; R4 supplies required inputs |

### R4 Definition of Done

R4 closes only when:

1. all mandatory R4 dimensions for the declared reference language/profile are evidence-producing or explicitly accounted;
2. every discovered function has a stable identity record;
3. no mandatory dimension is silently absent;
4. normalization is deterministic and provenance-preserving;
5. exact semantic duplicates follow NORMALIZATION.md;
6. conflict candidates survive into reconciliation input;
7. graph construction consumes only the normalized path;
8. tests prove deterministic output and accounting closure;
9. SemanticFact remains a compatibility envelope, not the only semantic type system.

Any project-wide claim that R4 is complete MUST name the language/profile/reference corpus against which these gates were proven.
