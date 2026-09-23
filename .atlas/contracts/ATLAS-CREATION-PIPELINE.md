---
id: atlas.contract.creation-pipeline
type: contract
status: active
canonical: true
---
# ATLAS Creation Pipeline Contract

## Purpose

A canonical `*.atlas` file is NOT produced by serializing a design sketch.

It is produced by a constrained engineering loop that may search, compare, synthesize, generate code, census that generated code, validate it, select one design, then seal and mechanically compact the resulting semantic world.

The normative path is:

~~~text
Human/Agent Intent
        ↓
SelfBuildWorkOrder (when Atlas is improving itself)
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
CandidateAtlas semantic world
        ↓
Analytic CostModel / constraint pruning
        ↓
VerificationWorld materialization
        ↓
VERIFY / BENCH / PROVE required obligations
        ↓
Candidate Revision / Repair Loop ↺
        ↓
VALIDATED CandidateAtlas
        ↓
SelectedDesign
        ↓
AdmissionTransaction when Atlas self-source changes
        ↓
Final exact-candidate seal gate
        ↓
SEALED Logical Atlas
        ↓
Deterministic Mechanical Compaction
        ↓
canonical *.atlas
~~~

The AI-assisted stages occur BEFORE semantic seal.

Verification/benchmark/obligation evaluation are construction stages, not post-publication cleanup.

The compaction stage occurs AFTER semantic seal.

No canonical `*.atlas` publication exists before the seal gate passes.

No AI/model decision is permitted inside canonical compaction.

## Machine contracts

The following machine schemas are normative for the first implementation profile:

- `../schemas/constraint-envelope.schema.json`;
- `../schemas/provider-receipt.schema.json`;
- `../schemas/decision-proposal.schema.json`;
- `../schemas/candidate-change-set.schema.json`;
- `../schemas/self-build-work-order.schema.json`;
- `../schemas/admission-transaction.schema.json`.

A later implementation may add richer typed Rust/API forms, but those forms MUST preserve these semantic obligations or explicitly version/migrate them.

## Execution host and sandbox topology

Construction may run inside an embedded coding-agent host under `AGENT-HOST-EMBEDDED-RUNTIME.md`.

The preferred early deployment shape is:

~~~text
coding agent
→ local MCP/API adapter
→ AtlasCore in the same outer host sandbox
→ Atlas-managed CandidateWorkspace
→ census / verify / admission / seal
~~~

This permits Atlas to reuse the host's compute and checkout without making the host or provider canonical authority.

The following remain distinct even when physically co-located:

- AgentHost outer sandbox;
- Atlas control plane;
- CandidateWorkspace;
- execution sandbox/backend;
- VerificationWorld;
- admission/seal authority.

A mutable host working tree is candidate material until admitted. Canonical parent/revision identity is pinned independently of the worktree.

Atlas MUST NOT assume nested container/VM capabilities. The available SandboxBackend is capability-detected; work requiring stronger isolation, determinism or scale is rejected, explicitly downgraded by policy, or dispatched to an appropriate remote backend.

MCP is a transport adapter. It MUST NOT become the canonical semantic representation or grant a model direct authority to select, verify, admit or seal its own output.

## Construction intelligence fabric

Creation MAY use many providers/subagents under `MULTI-AI-CONSTRUCTION-FABRIC.md`.

~~~text
ConstraintEnvelope / SelfBuildWorkOrder
        ↓
ConstructionTaskGraph
        ↓
ProviderRouter + bounded AgentLease
        ↓
research / architecture / synthesis / critic / verification workers
        ↓
typed Atlas blackboard records
        ↓
CandidateAtlas branches + evidence
        ↓
DecisionProposal(s) / repair routing
        ↓
normal selection / admission / seal path
~~~

Atlas MUST NOT use free-form agent conversation as the durable coordination state of this loop.

The provider/model may differ per task. A cloud coding host may supply a native Claude-class worker while Atlas simultaneously routes other tasks to GPT-class, Gemini-class, Jev-class, local/self-hosted, or remote Atlas workers. Named vendors are examples only.

A task-specific ContextCompiler SHOULD expose the smallest sufficient attributed semantic slice, including relevant constraints, obligations, UNKNOWN/CONFLICT state and evidence. Context truncation or provider-window optimization MUST NOT convert a known uncertainty into absence.

Candidate branches retain separate identities/evidence. Cross-provider agreement is evidence, not authority.

## Autonomous self-build entrypoint

When the construction target is Atlas itself, autonomous or semi-autonomous work begins with a typed SelfBuildWorkOrder governed by SELF-BUILD-CONTROLLER.md. The controller derives bounded work from the capability-gap graph, roadmap, Genome, evidence and policy; it does not directly generate or admit code.

A conversational/human task may enter directly at C0. A POLICY_AUTO self-build task must carry a valid SelfBuildWorkOrder before research/synthesis begins.

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

Unstructured intent may start the process. It may not bypass typing/validation.

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
- semantic MetricContract/performance objectives, Workload and VerificationWorld references where applicable;
- allowed CostModel/optimization policy and empirical-evidence requirements where applicable;
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

Jev is a decision role/fabric, not a singular root judge. Atlas MAY request multiple independent DecisionProposals, use Jev to route the next experiment/provider, or reconcile disagreement. A ranking never becomes SelectedDesign merely through consensus or score.

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

In embedded-agent-host mode, the coding provider MAY edit the host checkout or an Atlas-created worktree directly. Those bytes remain candidate material. Atlas MUST derive or validate the CandidateChangeSet against the pinned parent and MUST NOT treat ordinary filesystem mutation or a provider-created commit as canonical admission.

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

Execution topology and sandbox capability claims are governed by `AGENT-HOST-EMBEDDED-RUNTIME.md`. An outer cloud sandbox does not automatically satisfy candidate isolation, verification independence, benchmark reproducibility or destructive-test requirements.

## Stage C9 — Construction-time verification

C9 executes the canonical VERIFY / BENCH / PROVE semantics in `VERIFICATION-METRICS-PERFORMANCE.md`.

A CandidateAtlas becomes VALIDATED only after the exact required policy-specific gates for that candidate/revision/profile pass.

Validation is not a post-build check against an already-final artifact. It is part of deciding whether a candidate is allowed to become seal-eligible.

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
- regression comparison;
- VerificationWorld/failure-scenario obligations;
- scoped metric objectives and empirical observations;
- analytic CostModel predictions where useful for pruning, kept distinct from measured evidence.

A benchmark winner is not automatically semantically valid.

A semantically correct candidate is not automatically selected if it violates another hard objective.

A CostModel prediction may prune or prioritize candidates but cannot satisfy an empirical performance obligation that policy requires to be measured.

C9 emits durable structured evidence/diagnostics and an obligation-evaluation result. It does not itself select or seal.

## CandidateAtlas versus final *.atlas

CandidateAtlas is a logical construction state, not a canonical published `*.atlas` file.

Atlas MAY persist/debug/cache candidate state, but any such serialization must be explicitly unsealed/noncanonical.

Only the output of C12 followed by deterministic C13 compaction may be published as canonical `*.atlas`.

## Stage C10 — Candidate revision loop

Failure does not require abandoning the entire process.

Atlas may feed typed validation evidence back to:

- the decision provider;
- the synthesis provider;
- the human;
- the blueprint revision pipeline.

~~~text
candidate
→ validation result + semantic metrics
→ structured failure evidence
→ CostModel/sensitivity/calibration where applicable
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

## Stage C11A — Admission transaction for Atlas self-build

When the selected candidate changes Atlas's own canonical repository/native source, selection is not the mutation boundary. The selected CandidateChangeSet MUST pass the AdmissionTransaction contract in ADMISSION-TRANSACTION.md.

The transaction pins the exact parent revision, applies the candidate in isolation, reruns invalidated gates, recensuses the exact applied tree, compares expected versus observed semantic delta, and either commits one resulting canonical revision or rolls back with durable failure evidence.

POLICY_AUTO selection does not imply unrestricted repository writes; the SelfBuildWorkOrder and admission policy must authorize the exact mutation class.

## Stage C11B — Final exact-candidate seal eligibility

Selection chooses the candidate/design. It does not make prior evidence automatically fresh forever.

Before C12, Atlas MUST establish that the exact selected candidate/revision and all material selected bindings still satisfy the active seal policy.

For normal creation this means checking that the required C9 evidence applies exactly to the SelectedDesign.

For Atlas self-build this also includes the post-apply AdmissionTransaction recensus/verification of the exact resulting repository tree.

If selection, application, binding resolution, dependency resolution or environment/profile changes invalidate material evidence, the affected gates MUST be rerun.

The result is SealEligibleAtlas, not yet a published artifact.

## Stage C12 — Logical Atlas seal

Only after authorized selection, required closure and exact-candidate seal eligibility may Atlas seal the logical Atlas.

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
- VerificationPlan / VerificationWorld / FailureScenario definitions required by policy;
- MetricContracts and performance/resource objectives;
- tests/benchmarks/proofs and their exact evidence/attestation identities;
- obligation evaluation state for the exact seal policy;
- evidence/attestation root(s) sufficient to verify admission;
- conflicts/unknowns permitted by policy;
- Genome/schema/version pins;
- CensusCertificate;
- SelfBuildWorkOrder and committed AdmissionTransaction lineage when the logical Atlas represents an admitted self-build revision.

The logical Atlas is already complete engineering meaning before physical compaction starts.

The seal is the boundary after which VERIFY/BENCH/PROVE results used for admission are historical evidence about an immutable selected semantic world. New observations may create new attestations or a new candidate, but they do not silently rewrite the existing seal.

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

## Build-in-place and build-through-verification principle

Atlas creation is allowed to generate implementation logic before the `*.atlas` artifact exists.

It is also required to verify that implementation before final `*.atlas` publication when the active seal policy requires verification.

This is intentional.

The desired cycle is:

~~~text
design candidate
→ code candidate
→ census code
→ CandidateAtlas
→ predict / optimize
→ verify / bench / prove required obligations
→ repair until admissible
→ select code/design
→ final exact-candidate seal gate
→ seal semantics
→ compact
→ canonical *.atlas
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

## Agent-host and session independence

A canonical Atlas may be constructed entirely inside one embedded agent-host cloud session, but the result MUST NOT depend on that session continuing to exist.

Ephemeral build caches, candidate worktrees and derived indexes may disappear. The sealed semantic meaning, required evidence/attestation roots, admitted revision identity and lineage required by policy remain durable.

A later session must be able to restore/checkout the admitted revision, load compatible durable Atlas state, perform incremental recensus and continue without relying on the previous model context.

The same creation semantics apply whether the caller is CLI, MCP, remote API, CI or future Atlas Studio.

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
model cost before brute-force search
verify the exact candidate
measure declared metrics
evaluate named obligations
repair failures
select explicitly
seal only evidence-backed meaning
compress mechanically
publish *.atlas only after seal
~~~

External intelligence accelerates invention.

Atlas owns truth.
