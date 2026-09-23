---
id: atlas.contract.human-ai-adl-authoring
type: contract
status: active
canonical: true
---
# Human–AI ADL Authoring Contract

## Purpose

Atlas Development Language is a collaborative authoring surface for **Human + AI + Atlas semantic validation**.

It is explicitly NOT designed as:

- a human-only text language with AI autocomplete bolted on later;
- an AI-only prompt language where humans merely approve opaque output;
- a chatbot beside a conventional source editor.

The intended authoring loop is:

~~~text
Human intent / goals / preferences
        ↕
AI research / exploration / synthesis
        ↕
ADL / graph / constraint / evidence projections
        ↓
Atlas semantic validation
        ↓
CandidateDesign / CandidateChangeSet
        ↓
tests / census / proof / benchmark / security
        ↓
SelectedDesign
~~~

Human and AI are collaborators. Atlas remains the semantic/admission authority.

## Roles

### Human

Humans primarily provide:

- intent;
- goals;
- hard constraints;
- trade-off preferences;
- acceptance/rejection;
- steering;
- semantic review;
- authority decisions required by policy.

Human input is not automatically correct implementation truth.

### External AI

External AI providers may:

- search/research;
- explain;
- compare;
- propose mechanisms;
- compose existing semantic pieces;
- invent missing pieces;
- generate ADL;
- generate Atlas-native implementation candidates;
- generate tests/proofs/benchmarks;
- transform candidates after evidence.

External AI output is candidate material, not canonical truth.

### Atlas

Atlas owns:

- identity;
- Genome constraints;
- security policy;
- evidence/provenance;
- census;
- normalization/reconciliation;
- semantic consistency;
- dependency accounting;
- validation;
- admission;
- SelectedDesign;
- sealing;
- deterministic compaction;
- compiler lineage.

Neither human confidence nor model confidence can bypass Atlas gates.

## ADL is one projection, not the semantic authority

Canonical meaning lives in the typed Atlas semantic world.

ADL text is a human/AI-friendly authoring projection.

Other projections may include:

- Studio graph editing;
- conversational editing;
- constraint editors;
- semantic diffs;
- evidence views;
- benchmark views;
- agent-native structured edits.

All projections converge to the same typed semantic world.

~~~text
conversation ─┐
ADL text      ├─→ typed candidate semantics → canonical Atlas path
Studio graph  ┤
constraints   ┤
agent edits   ┘
~~~

No projection may create a second truth system.

## Agent-host authoring surfaces

Atlas does not require its own IDE in order to provide the canonical semantic/admission substrate.

A coding-agent host may be the active authoring cockpit while Atlas runs beside it as an embedded subsystem governed by `AGENT-HOST-EMBEDDED-RUNTIME.md`.

~~~text
Human ↔ Claude Code / another coding agent
             ↓ local MCP/API adapter
          AtlasCore
             ↓
typed candidate semantics / Census / verification / admission
~~~

The agent host may own the editor, terminal, cloud compute and outer sandbox. It does not own Atlas semantic truth.

Agent-facing MCP tools are another projection of the same AtlasCore, alongside ADL text, conversation, CLI, CI and future Studio views.

Atlas Studio may later become the native visual cockpit over this same semantic world. Studio MUST NOT fork the semantic model or require a second project memory system.

## Conversation-native authoring

A conversation may request:

- a new capability;
- a structural change;
- a performance objective;
- a security property;
- a dependency removal;
- an architecture comparison;
- an ADL rewrite;
- an implementation change.

Conversation text itself is never canonical engineering truth.

A conversational action MUST materialize into typed candidate records such as:

- ResearchQuery;
- CandidateMechanism;
- CandidateDesign;
- CandidateChangeSet;
- DecisionProposal;
- ValidationObligation;
- SelectedDesign decision.

If a conversation cannot be converted into typed candidate semantics without ambiguity, the ambiguity remains explicit rather than being silently guessed into canonical design.

## Search/research-assisted authoring

ADL authoring may invoke a Perplexity-class research provider (provider category, not a mandatory vendor) to search:

- the current Atlas knowledge base;
- censused donors;
- admitted dependency closures;
- new OSS candidates;
- specifications;
- papers;
- public documentation;
- benchmark evidence.

The research experience may resemble a search/research assistant, but search results remain ResearchClaims/evidence candidates.

A URL, README, paper, model summary or search ranking does not become OBSERVED implementation truth.

Promising OSS mechanisms MUST enter donor/dependency admission and census before implementation claims are treated as observed.

## Multiple alternatives are preferred over one opaque answer

For material design decisions, AI SHOULD produce multiple typed alternatives when feasible.

Each alternative SHOULD expose:

- mechanism;
- actual provider when known;
- invariants;
- dependencies;
- expected benefits;
- known costs;
- security implications;
- licensing/provenance implications;
- unresolved facts;
- validation plan.

The goal is not verbose prose. The goal is a typed decision set that Atlas can compare.

## Typed fast decision provider

Atlas may use a fast typed decision provider — a **Jev-class decision fabric** — to:

- rank;
- score;
- route;
- shortlist;
- choose which candidate to test next;
- decide whether to reuse/combine/invent a semantic piece;
- decide when to escalate to a stronger synthesis provider.

A decision provider is an optimization/decision mechanism, not a truth oracle.

Its output MUST be represented as a typed `DecisionProposal` conforming to `../schemas/decision-proposal.schema.json` and retain:

- provider identity/version;
- exact candidate set;
- criteria;
- scores/ranking;
- confidence;
- evidence references supplied to the decision;
- unresolved/low-confidence reasons;
- policy/authority required for action.

Decision-provider output alone MUST NOT create OBSERVED facts or a SelectedDesign.

Policy MAY permit automatic selection for bounded low-risk coordinates after all mandatory Atlas gates pass.

## LEGO-style semantic composition

The unit of composition is NOT a donor file, package or module.

The unit is a typed semantic mechanism.

Conceptually:

~~~text
MechanismPiece
├─ capability provided
├─ required interfaces
├─ invariants
├─ state model
├─ effects
├─ ownership/resource rules
├─ concurrency rules
├─ persistence/recovery rules
├─ dependencies
├─ constraints
├─ cost model
├─ evidence/provenance
├─ implementation candidates
└─ verification obligations
~~~

Atlas may:

- reuse a piece;
- specialize a piece;
- combine pieces;
- reject a piece;
- synthesize a missing connector;
- invent a new piece.

Composition MUST preserve typed interface/binding/constraint semantics.

Copying a donor module because it is convenient is not semantic composition.

## External synthesis and coding during Atlas creation

External synthesis providers are allowed — and expected — to generate real implementation candidates **during the creation of the logical Atlas**, before sealing.

The intended loop is:

~~~text
intent
→ research
→ candidate mechanism set
→ typed decision
→ external synthesis/code generation
→ CandidateChangeSet
→ ingest generated source as untrusted input
→ census generated implementation
→ compare declared intent vs observed implementation
→ security / dependency / license / test / benchmark / proof gates
→ revise or accept
→ SelectedDesign
→ SEALED logical Atlas
~~~

This is mandatory separation:

~~~text
AI-generated implementation
≠ canonical implementation truth

AI-generated implementation
→ census / verification
→ observed implementation evidence
~~~

The provider may write candidate code. It may not write directly into sealed canonical truth.

## Human/AI selection policy

Selection authority is explicit.

A design coordinate MUST declare one of:

- `HUMAN_REQUIRED`;
- `POLICY_AUTO`;
- `HYBRID`.

`HUMAN_REQUIRED` requires an authorized human selection event.

`POLICY_AUTO` permits Atlas to select after all declared validation gates pass and the decision is inside a bounded policy envelope.

`HYBRID` permits automatic exploration/ranking but requires human confirmation for the final material design decision.

No provider may silently decide its own authority class.

## Autonomous self-build is policy-bounded

When Atlas initiates its own next engineering task, SELF-BUILD-CONTROLLER.md governs capability-gap selection and emits a SelfBuildWorkOrder. External AI still participates only through typed provider roles. A successful SelectedDesign becomes canonical source only through ADMISSION-TRANSACTION.md.

Metrics, VerificationWorld evidence, FailureScenario results and CostModel predictions may guide candidate search under VERIFICATION-METRICS-PERFORMANCE.md. Prediction never substitutes for required empirical evidence.

## Semantic diffs over text diffs

Human/AI review SHOULD prioritize semantic diffs:

- capability changes;
- binding changes;
- state/effect changes;
- dependency changes;
- security changes;
- invariant changes;
- performance-objective changes;
- evidence changes.

Text diffs remain useful but are not sufficient evidence that meaning stayed unchanged.

## AI-generated ADL status

AI-generated ADL enters as candidate/authored declarations.

Its semantic claims are `DECLARED` unless a deterministic rule derives them.

The fact that an LLM generated syntactically valid ADL does not make it validated, selected or observed.

## Explainability requirement

For a material proposal, Atlas must be able to show:

- what changed;
- why it was proposed;
- which evidence/mechanisms informed it;
- which external providers participated;
- which alternatives were rejected/deferred;
- which validation gates remain;
- what would falsify the candidate.

A hidden chain-of-thought is not required or authoritative. Durable typed rationale/evidence is.

## Provider independence

ADL semantics MUST NOT depend on one named provider.

Jev-class decision providers, research providers, and synthesis/code providers are replaceable adapters.

A provider can be removed/replaced without changing the canonical semantic meaning of already sealed Atlas artifacts.

## Final invariant

ADL is the collaborative language/interface where:

~~~text
Human supplies intent and steering
AI supplies search, synthesis and implementation proposals
Atlas supplies semantic truth, evidence, constraints and admission
~~~

The language must be comfortable for both humans and AI while remaining independent of either one's unverified assertions.
