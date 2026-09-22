---
id: atlas.contract.adl-to-atlas
type: contract
status: active
canonical: true
---
# ADL → ATLAS Semantic Resolution Contract

## Authority

This contract is governed by `ARTIFACT-LAYERING.md`.

ADL is the collaborative intent/constraint authoring layer.

`*.atlas` is the canonical binary semantic artifact.

They are not the same representation.

## Purpose

This contract defines how natural-language-first ADL, existing implementation census, and AI-generated candidate implementation converge into one exact typed semantic world before `*.atlas` publication.

The core rule is:

> ADL describes desired truth. ATLAS encodes resolved exact truth.

## Canonical convergence

~~~text
EXISTING IMPLEMENTATION       AUTHORED ADL / CONVERSATION       AI-GENERATED CANDIDATE
pinned source @ revision      natural language + typed hints    CandidateChangeSet source
        ↓                                ↓                               ↓
Inventory / Frontend            interpret / elaborate            untrusted Inventory
        ↓                                ↓                               ↓
SemanticExtractor              typed DECLARED records            SemanticExtractor
        ↓                                │                               ↓
typed OBSERVED                 │                         typed OBSERVED generated-source facts
        └──────────────────────┴───────────────┬─────────────────────────┘
                                              ↓
                                            Census
                                              ↓
                                         Normalize
                                              ↓
                                         Reconcile
                                              ↓
                              research / decisions / validation
                                              ↓
                                       SelectedDesign
                                              ↓
                              resolved typed semantic world
                                              ↓
                                        logical seal
                                              ↓
                        deterministic semantic compaction/encoding
                                              ↓
                                         *.atlas
~~~

Direct prose → canonical binary is forbidden.

Direct provider output → canonical truth is forbidden.

Direct generated code → SelectedDesign is forbidden.

## ADL is natural-language-first

ADL MAY be written primarily in natural language.

Example:

~~~text
Build a local-first note application.

User data must stay on device unless sync is explicitly enabled.

Prefer mature permissively licensed OSS.

Atlas may choose the storage architecture.
Minimize attack surface and operational complexity.
~~~

The authoring system may enrich this into structured constraints, but natural language remains a first-class input surface.

The compiler MUST NOT treat raw prose as the final semantic representation.

## ADL semantic extraction

Each meaningful ADL statement is converted into one or more typed authoring records such as:

- goal;
- requirement;
- constraint;
- preference;
- prohibition;
- non-goal;
- acceptance criterion;
- security policy;
- performance objective;
- allowed degree of freedom;
- evidence requirement;
- design authority requirement.

Where the meaning is ambiguous, the typed result MUST retain ambiguity or create a resolution obligation.

It MUST NOT silently guess a canonical answer.

## Epistemic status

Raw ADL declarations enter as `DECLARED`.

Named deterministic rules may emit `DERIVED` records with explicit lineage.

ADL compilation alone cannot create `OBSERVED` implementation evidence.

Observed truth comes from admitted census/measurement/verification paths.

When declared and observed facts agree, both lineages survive.

When they disagree, Atlas creates an explicit reconciliation obligation/conflict.

## Human + AI authoring

Human and AI may jointly author ADL.

AI may:

- reformulate natural language;
- surface ambiguity;
- search Atlas knowledge;
- search admitted/public OSS;
- propose candidate mechanisms;
- generate structured constraints;
- propose DecisionProposal records;
- generate CandidateChangeSet implementation;
- explain semantic diffs.

AI may not self-promote its proposal into canonical truth.

Authoring authority and selection authority follow `HUMAN-AI-ADL-AUTHORING.md`, `EXTERNAL-PROVIDER-TRUST.md`, and `SELECTED-DESIGN.md`.

## Research and donor census

When ADL leaves a design coordinate open, Atlas may research/census OSS mechanisms.

Promising donor claims remain research/candidate material until admitted and censused.

For selected mechanisms Atlas SHOULD census transitive dependency closure to the depth required by policy.

A README claim or model summary is not equivalent to observed semantics.

## Candidate implementation reconciliation

When an external synthesis provider generates code:

1. provider intent is recorded as candidate/declared claims;
2. generated files enter the untrusted corpus path;
3. Atlas inventories/censuses those files;
4. observed implementation facts remain separate from provider assertions;
5. declared-versus-observed discrepancies become obligations/conflicts;
6. security/dependency/license/test/benchmark/proof gates run;
7. only validated implementation candidates may enter selection.

A provider cannot self-certify generated code.

## Semantic resolution

Before logical seal, publication-critical questions must have exact typed answers.

Resolution includes, where applicable:

- identity;
- scope;
- type;
- symbol;
- function/signature/body;
- call/control/data flow;
- state/effect;
- ownership/resource;
- concurrency;
- persistence/recovery;
- capability/interface;
- binding;
- security/authority;
- dependency;
- target/profile;
- invariants;
- obligations;
- permitted runtime dynamics/external boundaries;
- SelectedDesign.

A record that cannot be represented losslessly must remain UNKNOWN/UNSUPPORTED or block seal.

Flattening meaning into display text is not semantic resolution.

## Identity and names

ADL names are authoring handles.

They are not automatically canonical global identities.

Canonical identity resolution MUST account for sufficient design/repository/revision/module/scope/type/function coordinates to prevent accidental equivalence.

Name equality alone is never sufficient for merge.

## Typed normalization

ADL-derived typed records participate in the same normalization/reconciliation world as census-derived records.

Normalization may canonicalize representation under explicit rules.

It may not:

- erase authoring/source span lineage;
- turn spelling into compiler-resolved identity without proof;
- choose a winner between conflicting facts;
- collapse distinct scopes because names match;
- upgrade DECLARED to OBSERVED.

## Selection boundary

ADL may express preferences and authority policy.

ADL does not itself become SelectedDesign.

Selection occurs only after:

- required semantic closure;
- candidate validation;
- dependency/license/security gates;
- required tests/benchmarks/proofs;
- explicit HUMAN_REQUIRED / POLICY_AUTO / HYBRID authority handling.

The selected result is represented by `SelectedDesign`.

## Seal boundary

The logical seal is the hard boundary after which no provider may decide what the system means.

Before seal:

~~~text
human/AI authoring
research
census
decision proposals
candidate synthesis
generated-code census
validation
selection
resolution
~~~

After seal:

~~~text
deterministic normalization
canonicalization
semantic compaction
binary encoding
integrity publication
~~~

If post-seal processing discovers missing meaning, publication fails and returns to the pre-seal world.

## ATLAS publication

The SEALED logical Atlas MUST preserve all information required to recover the selected exact semantics, including:

- typed declarations/observations/derived records;
- constraints/invariants;
- graph/binding relations;
- state/effect/resource semantics;
- security/authority semantics;
- dependency semantics;
- evidence/provenance;
- selected design;
- unresolved non-blocking facts where policy retains them;
- compiler/language/Genome/schema pins.

Canonical output is binary `*.atlas`.

ADL text is not the canonical payload.

## Source retention

The resulting Atlas artifact MAY omit original ADL text when policy allows.

FAT mode MAY embed authenticated ADL/source blobs for reconstruction/audit.

Embedded ADL remains evidence/source material, not semantic authority.

A valid Atlas artifact must remain semantically meaningful when the original ADL is unavailable.

## Round-trip semantics

~~~text
*.atlas
→ atlas disasm / explain / export
→ human-readable projection
~~~

This is a semantic projection.

It is not required to reproduce original wording, formatting, comments, or conversation unless exact source blobs were deliberately embedded.

Semantic equivalence is required.

Textual byte-for-byte equivalence is not.

## Relationship to AtlasX

ADL does not directly produce `*.atlasx`.

The mandatory order is:

~~~text
ADL
→ resolved semantic world
→ SEALED *.atlas
→ selected-system/dependency/runtime/resource closure
→ *.atlasx
~~~

AtlasX packaging MUST NOT reopen ADL interpretation.

## Forbidden paths

Forbidden:

- ADL prose → opaque `*.atlas` bytes without typed resolution;
- ADL prose → `*.atlasx` directly;
- provider answer → canonical semantic graph directly;
- generated source → SelectedDesign without census/validation;
- model interpretation after seal;
- compiler inferring what ADL "probably meant";
- canonical semantic meaning represented only by strings.

## Versioning

An ADL language-version change or Atlas semantic-schema change that alters meaning requires explicit version/identity/compatibility handling.

Existing sealed Atlas artifacts are never silently reinterpreted under newer ADL semantics.

## Final invariant

ADL is where humans and AI describe, negotiate, research, and refine desired truth.

`*.atlas` is where that process ends as exact provider-independent typed semantic truth.

The transition succeeds only when Atlas can understand the resulting semantic artifact without reading the original natural-language ADL again.
