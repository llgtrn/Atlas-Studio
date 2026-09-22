---
id: atlas.contract.creation-pipeline
type: contract
status: active
canonical: true
---
# ATLAS Creation Pipeline Contract

## Authority

Artifact roles are fixed by `ARTIFACT-LAYERING.md`: ADL is the Intent Artifact, `*.atlas` is the Canonical Semantic Artifact, and `*.atlasx` is the downstream Closed-World System Capsule.

This contract ends at canonical `*.atlas` publication. AtlasX closure/packaging is a separate post-Atlas stage governed by `ATLAS-TO-ATLASX.md` and MUST NOT reopen semantic interpretation.

## Purpose

A canonical `*.atlas` file is NOT produced by serializing a design sketch.

It is produced by a constrained engineering loop that may search, compare, synthesize, generate code, census that generated code, validate it, select one design, then seal and mechanically compact the resulting semantic world.

The normative path is:

~~~text
Human/Agent Intent
        ↓
Constraint Envelope
        ↓
Research / OSS Discovery
        ↓
Candidate Mechanism Set
        ↓
Typed Decision Fabric
        ↓
External Synthesis / Code Generation
        ↓
CandidateChangeSet
        ↓
Untrusted Ingestion + Census
        ↓
Semantic / Security / Dependency / License Validation
        ↓
Tests / Benchmarks / Proof / Differential Checks
        ↓
Candidate Revision Loop
        ↓
SelectedDesign
        ↓
SEALED Logical Atlas
        ↓
Deterministic Mechanical Compaction
        ↓
*.atlas
~~~

Only after a canonical Atlas exists may the downstream system-closure path begin:

~~~text
*.atlas
  ↓
selected-system + transitive dependency/runtime/resource/build closure
  ↓
*.atlasx binary capsule
~~~

That downstream step is NOT an opportunity to reinterpret ADL, call a model for missing meaning, or choose a new design.

The AI-assisted stages occur BEFORE semantic seal.

The compaction stage occurs AFTER semantic seal.

No AI/model decision is permitted inside canonical compaction.

## Machine contracts

The following machine schemas are normative for the first implementation profile:

- `../schemas/constraint-envelope.schema.json`;
- `../schemas/provider-receipt.schema.json`;
- `../schemas/decision-proposal.schema.json`;
- `../schemas/candidate-change-set.schema.json`.

A later implementation may add richer typed Rust/API forms, but those forms MUST preserve these semantic obligations or explicitly version/migrate them.

## Stage C0 — Intent

Inputs may come from:

- human conversation;
- ADL;
- Studio edits;
- machine-generated repair requests;
- capability-gap queries;
- donor-census discoveries;
- blueprint revision work;
- compiler/runtime evidence.

Intent is typed into explicit goals and constraints before a material design decision is admitted.

Unstructured intent may start the process. Natural-language-first Human+AI ADL is explicitly allowed and expected. It may not bypass typing/validation, and raw prose is never canonical execution semantics.

## Stage C1 — Constraint Envelope

Before research or synthesis, Atlas constructs a constraint envelope from all applicable canonical policy.

The v1 machine envelope is `../schemas/constraint-envelope.schema.json`.

The envelope MUST include where applicable:

- Genome hash/version;
- security policy;
- authority/capability constraints;
- target/deployment/hardware/workload profile;
- determinism requirements;
- performance objectives;
- memory/latency/size/energy limits;
- supported language/backend constraints;
- dependency policy;
- donor-runtime policy;
- license/provenance policy;
- network/tool execution policy;
- persistence/recovery requirements;
- ownership/resource requirements;
- concurrency/order requirements;
- verification requirements;
- completion/unknown policy.

External providers MUST receive the relevant constraint envelope before proposing implementation.

A provider MUST NOT be allowed to propose a candidate under a weaker hidden policy than Atlas will later enforce.

## Stage C2 — Research and OSS discovery

Research proceeds in this order unless policy justifies otherwise:

~~~text
current canonical Atlas knowledge
→ existing censused donors
→ admitted dependency graph
→ already-known Technology Genomes
→ local reference corpora
→ external OSS search
→ specifications / papers / documentation
→ other admitted research providers
~~~

The purpose is to avoid reinventing known mechanisms while still allowing new discovery.

A Perplexity-class research provider is an example adapter for this stage; no named service is mandatory.

Research output is typed as ResearchClaim/CandidateMechanism evidence.

Research output MUST record:

- query identity;
- provider identity;
- source locator;
- retrieval time;
- source revision/version when available;
- provenance/license cues when relevant;
- confidence/quality metadata;
- claim text or structured claim;
- exact evidence links;
- whether the mechanism has been census-verified.

Search ranking is not engineering truth.

## Stage C3 — Candidate Mechanism Set

Atlas forms a typed candidate set.

Candidates may originate from:

- Atlas-native mechanisms;
- donor mechanisms;
- dependency-provided mechanisms;
- published algorithms;
- synthesized combinations;
- newly invented pieces.

Each material candidate MUST identify:

- candidate identity;
- required capability;
- actual provider when known;
- mechanism/invariant summary;
- required dependencies;
- security implications;
- license/provenance state;
- expected objective impact;
- known limitations;
- unknowns;
- required validation depth;
- required implementation owner;
- whether blueprint revision is implicated.

A missing candidate is allowed: Atlas may synthesize a new mechanism.

## Stage C4 — Typed Decision Fabric

Atlas may call a fast typed decision provider to reduce search cost.

A Jev-class decision provider may:

- rank candidates;
- score candidates;
- shortlist candidates;
- route tasks;
- select the next candidate to implement/test;
- choose reuse vs combine vs invent;
- decide escalation to a stronger provider.

The output is a typed `DecisionProposal` conforming to `../schemas/decision-proposal.schema.json`.

The decision provider MUST NOT:

- create OBSERVED evidence;
- silently add a dependency;
- bypass license/security constraints;
- grant itself selection authority;
- convert UNKNOWN to a default;
- directly seal Atlas;
- directly write SelectedDesign.

Scores/confidence are decision evidence only.

## Stage C5 — External Synthesis / Coding

A synthesis/code provider may generate actual Atlas-native implementation candidate material.

Permitted outputs include:

- ADL;
- Rust/TypeScript/C boundary code where current phase permits;
- typed semantic candidate structures;
- tests;
- benchmarks;
- proofs/specification candidates;
- migration scripts;
- implementation rationale;
- dependency proposals.

All outputs MUST be packaged into a typed `CandidateChangeSet` conforming to `../schemas/candidate-change-set.schema.json`.

The provider MUST NOT modify the canonical sealed Atlas directly.

Provider-generated source is untrusted source.

## Stage C6 — CandidateChangeSet

A CandidateChangeSet is the complete proposed implementation delta.

It MUST include at least:

- change identity;
- parent Atlas/design/repository revision;
- provider receipt(s);
- declared intent;
- affected semantic scope;
- files/artifacts added/changed/deleted;
- proposed dependencies;
- proposed capability/binding changes;
- proposed state/effect/resource changes;
- proposed security/authority changes;
- tests;
- benchmarks/proofs if supplied;
- expected semantic result;
- unresolved assumptions;
- rollback information where applicable.

Candidate code without a CandidateChangeSet is not eligible for automatic admission.

## Stage C7 — Untrusted ingestion and census

Generated code is treated exactly as untrusted implementation input.

Required path:

~~~text
CandidateChangeSet source/artifacts
        ↓
Inventory
        ↓
SourceFrontend
        ↓
SemanticExtractor
        ↓
Raw typed observations
        ↓
Census
        ↓
Normalize
        ↓
Reconcile
~~~

The provider's declared intention and Atlas's observed implementation remain separate evidence paths.

Atlas MUST compare:

~~~text
claimed candidate semantics
vs
observed generated implementation semantics
~~~

Agreement does not erase lineage.

Disagreement creates a blocking obligation or explicit policy-bounded unresolved state.

## Stage C8 — Security and dependency validation

Before executing generated candidate code, Atlas MUST run the applicable static/admission gates.

At minimum consider:

- secret exposure;
- forbidden file/tool control surfaces;
- authority escalation;
- network access;
- filesystem effects;
- process execution;
- unsafe/FFI boundaries;
- dependency additions;
- build/proc-macro/codegen execution;
- dynamic loading/plugins;
- supply-chain provenance;
- license constraints;
- sandbox requirements.

Generated code MUST NOT be executed merely because the synthesis provider requested it.

Execution occurs only under the active security/sandbox policy.

## Stage C9 — Validation

A candidate becomes VALIDATED only after the required policy-specific gates pass.

Possible gates include:

- semantic verifier;
- compiler/type checks;
- unit/integration/property tests;
- differential tests;
- benchmark objectives;
- memory/latency/size limits;
- fuzzing;
- model checking;
- proof checks;
- security tests;
- dependency closure checks;
- license/provenance checks;
- Atlas recensus;
- regression comparison.

A benchmark winner is not automatically semantically valid.

A semantically correct candidate is not automatically selected if it violates another hard objective.

## Stage C10 — Candidate revision loop

Failure does not require abandoning the entire process.

Atlas may feed typed validation evidence back to:

- the decision provider;
- the synthesis provider;
- the human;
- the blueprint revision pipeline.

~~~text
candidate
→ validation result
→ structured failure evidence
→ re-rank / repair / synthesize alternative
→ new CandidateChangeSet
→ census / validate again
~~~

Each iteration receives a new durable identity/lineage coordinate.

## Stage C11 — Selection

Selection follows `SELECTED-DESIGN.md`.

The selection decision MUST know:

- candidate set;
- validation evidence;
- selection authority policy;
- trade-offs;
- external boundaries;
- unresolved permitted dynamics;
- provider lineage;
- blueprint revision state.

Selection may be:

- human-required;
- policy-auto;
- hybrid.

An external provider may propose. Atlas/policy owns admission.

## Stage C12 — Logical Atlas seal

Only after selection and closure may Atlas seal the logical Atlas.

The SEALED logical Atlas MUST preserve enough information to explain and reproduce the selected engineering meaning, including where policy requires:

- selected semantic records;
- observed implementation semantics;
- declarations;
- CandidateChangeSet lineage;
- SelectedDesign;
- rejected/deferred alternative identities;
- decision proposals;
- provider receipts;
- research/evidence references;
- dependencies;
- security/admission decisions;
- tests/benchmarks/proofs;
- conflicts/unknowns permitted by policy;
- Genome/schema/version pins;
- CensusCertificate.

The logical Atlas is already complete engineering meaning before physical compaction starts.

## Stage C13 — Mechanical compaction

After logical seal:

~~~text
NO RESEARCH
NO LLM CALL
NO DECISION PROVIDER
NO SYNTHESIS PROVIDER
NO SEMANTIC INVENTION
~~~

Canonical compaction is a deterministic mechanical transformation governed by `ATLAS-SEMANTIC-COMPACTION.md`.

Allowed operations include:

- exact interning;
- exact semantic deduplication;
- identity factoring;
- evidence factoring;
- DAG sharing;
- graph packing;
- columnar grouping;
- delta encoding;
- varint;
- bit packing;
- deterministic codec compression;
- content-addressed sharding.

Compaction MUST NOT decide that some semantic detail is "unimportant".

For the same sealed logical Atlas and same compaction profile:

~~~text
same semantic input
→ same canonical logical/root identity
~~~

Physical codec variation is allowed only where the wire contract explicitly permits decoded-content identity to remain stable.

## AI may improve the compactor, but not participate in a compaction run

External AI may help discover or implement a better compaction algorithm before adoption.

That change requires:

~~~text
research / donor census
→ benchmark
→ BlueprintRevisionDecision
→ contract/schema update if required
→ implementation
→ round-trip verification
~~~

Once the compaction blueprint/profile is selected, the actual seal→bytes transformation is mechanical.

## Build-in-place principle

Atlas creation is allowed to generate implementation logic before the `*.atlas` artifact exists.

This is intentional.

The desired cycle is:

~~~text
design candidate
→ code candidate
→ census code
→ verify code
→ select code/design
→ seal semantics
→ compact
~~~

NOT:

~~~text
design-only *.atlas
→ later ask another agent to invent all implementation
~~~

The final logical Atlas should know the selected implementation semantics, not only an architecture sketch.

## Provider-role separation

Provider classes are logically distinct:

- `RESEARCH_PROVIDER`;
- `DECISION_PROVIDER`;
- `SYNTHESIS_PROVIDER`;
- `VERIFICATION_PROVIDER`.

One vendor/model may implement multiple roles, but each invocation retains its declared role and policy.

Role separation is required because:

- research finds possibilities;
- decision reduces search;
- synthesis proposes implementation;
- verification produces/checks evidence.

No role owns canonical truth.

## Provider receipts

Every material external-provider invocation MUST be traceable through a ProviderReceipt conforming to `../schemas/provider-receipt.schema.json` and containing at least:

- provider identity;
- model/service identity/version where available;
- role;
- input artifact/hash set;
- constraint-envelope hash;
- tool/network permission profile;
- output artifact/hash;
- invocation timestamp;
- sampling/configuration identity when material;
- cost/latency metadata when policy tracks it;
- parent candidate/decision identity;
- provenance.

ProviderReceipt is lineage/evidence, not proof of correctness.

## External service independence

The canonical artifact MUST NOT require the continued availability of the provider that helped create it.

If a provider disappears:

- already sealed Atlas meaning remains readable;
- selected implementation remains independently verifiable;
- canonical compaction still works;
- compiler behavior does not change merely because the provider is unavailable.

## Final invariant

The Atlas creation process is an engineering admission loop:

~~~text
search widely
decide quickly
synthesize aggressively
trust nothing implicitly
census what was generated
verify what was claimed
select explicitly
seal complete meaning
compress mechanically
~~~

External intelligence accelerates invention.

Atlas owns truth.
