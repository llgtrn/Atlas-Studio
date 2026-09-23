---
id: atlas.contract.multi-ai-construction-fabric
type: contract
status: active
canonical: true
---
# Multi-AI Construction Fabric Contract

## Purpose

Canonical `*.atlas` construction may use many AI systems, many concurrent agents/subagents, Jev-class decision providers, deterministic tools, remote services and Atlas-managed sandboxes.

Atlas MUST therefore treat multi-AI construction as an orchestrated engineering fabric, not as one privileged model session and not as a free-form group chat.

The fundamental separation is:

~~~text
AI systems reason / research / synthesize / critique
Atlas owns durable semantic state, task authority, evidence lineage and admission
Sandboxes execute candidate work
Jev-class decision logic proposes routing/ranking/next actions
Policy/Human/Hybrid authority selects where required
AdmissionTransaction commits canonical self-modification
Seal publishes canonical *.atlas
~~~

No provider, host-native subagent, remote model, judge model or swarm coordinator becomes canonical truth merely because Atlas asked it to work.

This contract is normative with:

- `AGENT-HOST-EMBEDDED-RUNTIME.md` for where the fabric executes;
- `EXTERNAL-PROVIDER-TRUST.md` for trust/capability limits;
- `ATLAS-CREATION-PIPELINE.md` for construction lifecycle;
- `ASIR-CONSTRUCTION-MODEL.md` for typed construction operations;
- `VERIFICATION-METRICS-PERFORMANCE.md` for evidence and verification;
- `SELECTED-DESIGN.md` and `ADMISSION-TRANSACTION.md` for authority and mutation.

## Five-plane architecture

Atlas construction separates five logical planes even when they are physically co-located:

~~~text
1. SEMANTIC PLANE
   Census / normalized semantic world / closure / unknowns / conflicts / provenance

2. INTELLIGENCE PLANE
   Claude / GPT / Gemini / Jev / local models / future providers / host-native subagents

3. CONSTRUCTION PLANE
   ConstructionTaskGraph / CandidateAtlas / CandidateChangeSet / repair branches / ASIR transactions

4. EXECUTION PLANE
   CandidateWorkspace / SandboxBackend / build / test / fuzz / benchmark / failure workers

5. AUTHORITY PLANE
   policy / Genome / obligations / selection authority / AdmissionTransaction / seal
~~~

The Intelligence Plane MUST NOT directly mutate the Authority Plane.

Physical process hierarchy is not authority hierarchy.

A model running "above" Atlas in a host UI is not semantically above Atlas.

## Worker classes

Atlas distinguishes at least these execution classes.

### HOST_NATIVE_AI_WORKER

An AI/subagent supplied by the current AgentHost and able to operate inside the host sandbox.

Example: a coding-agent-native subagent inside an embedded cloud coding session.

The worker MAY be efficient because repository/tool access is local.

Host-native status grants no extra semantic authority.

### REMOTE_AI_PROVIDER_WORKER

A model/service reached through an admitted provider adapter or gateway.

Examples may include Claude, GPT, Gemini, Jev-class services, specialist models or future providers.

The worker receives only the context/data/capabilities authorized for the task.

A remote provider response is EXTERNAL_PROVIDER_OUTPUT until typed and admitted through the normal pipeline.

### REMOTE_ATLAS_EXECUTION_WORKER

An Atlas worker outside the current AgentHost used for work requiring different compute, isolation or locality.

Examples:

- large donor/dependency census;
- GPU workloads;
- deterministic benchmark fleets;
- multi-node failure scenarios;
- stronger microVM/container isolation;
- organization-local/private-data execution.

A remote Atlas worker is execution infrastructure. If it invokes AI, those invocations still receive ProviderReceipts and capability policy.

### DETERMINISTIC_EXECUTION_WORKER

Compiler, test runner, verifier, solver, model checker, benchmark harness, content-addresser or other non-AI worker.

Deterministic/non-AI output is not automatically canonical evidence either; artifact identity, environment, inputs and policy still determine evidence status.

## Provider role is not provider vendor

Atlas routes logical roles, not brand names.

Canonical roles remain:

~~~text
RESEARCH_PROVIDER
DECISION_PROVIDER
SYNTHESIS_PROVIDER
VERIFICATION_PROVIDER
~~~

A vendor/model MAY serve multiple roles across separate invocations.

Each invocation retains its own role, capability envelope, inputs, outputs, ProviderReceipt and task identity.

The following are invalid shortcuts:

~~~text
"Claude is the coding provider, therefore Claude may select."
"GPT criticized the candidate, therefore the candidate is rejected."
"Jev ranked A first, therefore A is SelectedDesign."
"The host-native agent created a commit, therefore it is admitted."
~~~

Provider identity is provenance.

Role + policy determines what the invocation is allowed to attempt.

Atlas authority determines what can become canonical.

## ProviderRouter

Atlas SHOULD expose one provider-neutral routing layer to construction logic.

Conceptually:

~~~text
ConstructionTask
    ↓
ProviderRouter
    ├─ host-native agent/subagent
    ├─ remote Claude-class provider
    ├─ remote GPT-class provider
    ├─ remote Gemini-class provider
    ├─ Jev-class decision provider
    ├─ local/self-hosted model
    └─ remote Atlas worker
~~~

Routing inputs MAY include:

- required logical role;
- task type;
- semantic scope;
- context size;
- coding/tool capability;
- required locality to source/data;
- privacy/data-exposure policy;
- network availability;
- required independence from another provider;
- expected latency;
- monetary/token/compute budget;
- model/provider health;
- benchmarked provider capability;
- deterministic fallback availability.

ProviderRouter output is an execution route, not a semantic decision.

No routing heuristic may silently weaken the task's policy or evidence requirements.

Atlas MUST be able to change provider/vendor without changing the semantic identity of the task itself.

## ConstructionTaskGraph

Multi-agent work MUST be represented as a typed dependency graph rather than an implicit conversation tree.

A ConstructionTask conceptually binds:

~~~text
task_id
parent_task_ids[]
work_order / construction objective
role
semantic scope
candidate lineage
required inputs
expected output type
capability requirements
independence requirements
completion conditions
budget
retry/escalation policy
~~~

Typical graph:

~~~text
research-A ─┐
research-B ─┼→ candidate-mechanisms → Jev shortlist ─┬→ synthesize-A → verify-A
critic-C  ──┘                                        └→ synthesize-B → verify-B
                                                               ↓
                                                        compare evidence
                                                               ↓
                                                        next decision
~~~

Task completion is determined by typed output and policy, not by an agent saying "done".

A task may fan out to multiple providers for diversity or independent challenge.

A task may fan in only through explicit reconciliation/decision logic.

## AgentLease

Any worker that can access Atlas data, tools, filesystem mutation, network, secrets or external effects MUST operate under a bounded lease or an equivalent host-enforced capability envelope.

A lease SHOULD bind at least:

~~~text
lease_id
task_id
provider / model / worker identity
role
parent candidate / exact revision
allowed semantic context refs
filesystem read scope
filesystem write scope
tool capabilities
network destination classes
secret/capability references
process/resource limits
token / money / time / compute budget
expected output type
expiration
child-task policy
~~~

An AgentLease grants execution capability.

It does NOT grant:

- semantic truth;
- SelectedDesign authority;
- verification authority beyond the declared role;
- canonical repository write authority;
- seal authority.

## Subagent and recursive-spawn discipline

Unbounded recursive agent spawning is forbidden.

A provider may use opaque internal reasoning that Atlas cannot observe; that does not create additional Atlas authority.

Any child agent/subagent that can independently access Atlas-controlled tools, source, network, credentials or mutable candidate state MUST be represented by:

- a child ConstructionTask;
- an Atlas-issued or adapter-enforced child AgentLease;
- lineage to its parent task/provider receipt.

A child lease MUST NOT widen:

- filesystem scope;
- network scope;
- secret access;
- canonical mutation rights;
- budget;
- authority class;

beyond what policy explicitly grants.

A parent agent cannot delegate authority it does not possess.

## Typed semantic blackboard

Agents MUST NOT use a shared chat transcript as the durable project memory or canonical inter-agent communication substrate.

The construction fabric uses a typed blackboard over Atlas-owned state.

Relevant typed records include, where applicable:

~~~text
ConstraintEnvelope
ResearchClaim
CandidateMechanism
AtlasConstructionOperation
DecisionProposal
CandidateChangeSet
CandidateAtlas
Unknown / Conflict / Obligation
VerificationFinding / Evidence
MetricContract / Observation
SelectedDesign
AdmissionTransaction
~~~

Free-form text MAY accompany records for explanation.

Text alone MUST NOT silently become the semantic claim when a typed form exists.

An agent asking "what did the previous agent decide?" SHOULD query typed Atlas state rather than depend on preserved conversational history.

## Context Compiler

Atlas SHOULD construct bounded task-specific context from the semantic world rather than dumping the whole repository or whole `*.atlas` into every provider context window.

Conceptually:

~~~text
canonical / candidate semantic world
        ↓
task + scope + obligations + unknowns
        ↓
ContextCompiler
        ↓
minimal sufficient attributed context slice
        ↓
provider
~~~

A context slice SHOULD preserve:

- exact source/revision/candidate identity;
- relevant semantic identities and relations;
- constraints and obligations;
- relevant evidence/provenance;
- UNKNOWN/CONFLICT state;
- expected output contract;
- capability boundaries.

Context optimization MUST NOT silently omit a known blocker or convert UNKNOWN into absence.

Context is an execution cache.

Atlas semantic state is the durable memory.

## Jev-class Decision Fabric

Jev is a logical decision role/fabric, not a singular root judge.

A Jev-class invocation MAY:

- rank;
- score;
- shortlist;
- route;
- choose the next experiment;
- select which candidate to implement or benchmark next;
- recommend escalation to a stronger/different provider;
- perform multi-objective trade-off analysis;
- consume CostModel predictions and evidence.

It MUST emit or map to a typed `DecisionProposal`.

It MUST NOT directly:

- create OBSERVED facts;
- grant a candidate VERIFIED status;
- grant itself authority;
- mutate canonical state;
- produce SelectedDesign except through the separately authorized selection process;
- seal `*.atlas`.

Atlas MAY request multiple independent DecisionProposals and reconcile them.

A disagreement among judges remains disagreement/evidence, not hidden majority truth.

## Candidate branching and speculative construction

Atlas MAY construct multiple candidates concurrently.

Each candidate MUST have separate lineage:

~~~text
admitted parent P
├─ candidate A
├─ candidate B
└─ candidate C
~~~

Each candidate retains its own:

- CandidateChangeSet;
- mutable workspace;
- semantic delta;
- dependency delta;
- provider receipts;
- verification evidence;
- benchmark observations;
- failures/unknowns;
- repair descendants.

Mutable source MUST NOT be shared across candidates in a way that destroys attribution.

Candidate B passing does not make candidate A's evidence apply to B.

Repair produces a new candidate identity when material bytes/semantics change.

## Critic and adversarial workers

Atlas SHOULD be able to assign explicit critic/adversarial tasks.

Critics may search for:

- violated invariants;
- missing edge cases;
- security escalation;
- semantic gaps;
- false closure;
- performance regressions;
- dependency/provenance issues;
- mismatches between declared and observed candidate behavior.

Critic output is a typed finding/challenge/evidence candidate.

A critic does not receive canonical veto authority merely because it is labeled critic.

Policy decides which findings block progression.

## Verification independence

Multi-provider execution can improve independence but does not guarantee it automatically.

Atlas MAY require:

- a different provider from synthesis;
- a separate sandbox;
- a separate host;
- independent deterministic verification;
- multiple corroborating evidence sources.

The verification policy defines required independence.

"Different model" is not by itself proof of independent evidence if both models merely repeat the same unverified source claim.

## Credential and provider gateway boundary

Raw provider credentials SHOULD remain outside ordinary provider-visible candidate workspaces.

Preferred shape:

~~~text
Atlas task
→ ProviderRouter / admitted gateway
→ credential broker attaches scoped credential
→ provider endpoint
~~~

The worker SHOULD receive capability/reference handles rather than long-lived raw secrets where possible.

Policy SHOULD bind:

- allowed provider/vendor;
- model class/version where material;
- destination;
- data classification allowed to leave the host;
- max token/money/compute budget;
- allowed tools;
- retention/privacy constraints where known;
- expiration.

A coding agent sharing the same OuterSandbox with Atlas MUST NOT gain raw credentials merely because Atlas can call another provider.

## Budget and resource scheduling

Atlas SHOULD treat provider and compute cost as schedulable resources.

A task MAY have budgets for:

- wall-clock time;
- token count;
- provider monetary spend;
- CPU;
- memory;
- GPU;
- storage;
- network;
- benchmark fleet time.

Budget exhaustion must produce an explicit task state.

An agent MUST NOT weaken verification, broaden authority or silently drop UNKNOWN state merely to fit budget.

Policy may choose:

- retry;
- route to cheaper provider;
- route to stronger provider;
- reduce optional exploration;
- require human escalation;
- stop.

## Embedded Claude-cloud example

A valid deployment may be:

~~~text
Claude/agent cloud OuterSandbox
│
├─ host-native coding agent
├─ AtlasCore + local MCP adapter
├─ Atlas CandidateWorkspaces
│
└─ ProviderRouter
    ├─ host-native Claude/subagents
    ├─ remote GPT-class provider
    ├─ remote Gemini-class provider
    ├─ remote Jev-class provider
    └─ remote Atlas workers
~~~

The named vendors are examples only.

Claude Cloud MAY provide convenient compute, checkout and native Claude workers while Atlas calls other admitted providers through network/gateway routes.

If host egress, credentials or policy do not permit a remote provider call, Atlas MUST NOT pretend the provider was available.

It may instead:

- use another admitted local/remote route;
- dispatch the task to a remote Atlas worker that has the required capability;
- keep the task blocked;
- request authorization.

Atlas architecture MUST NOT become Claude-only merely because its first embedded host is Claude-oriented.

## Restricted-egress fallback

If the AgentHost cannot reach arbitrary external providers:

~~~text
embedded Atlas
→ durable/queued ConstructionTask
→ admitted remote Atlas execution worker/provider gateway
→ typed result + ProviderReceipt
→ embedded Atlas resumes task graph
~~~

The remote worker MUST receive only the authorized context slice.

Queueing/distribution must preserve task identity, parent candidate identity, policy and provenance.

## ASIR / ACP relationship

The multi-AI fabric determines:

- which task exists;
- which provider/worker receives it;
- what context/capabilities it receives;
- what typed output is expected.

ACP/ASIR determines how a provider's proposed semantic construction operations cross into Atlas construction state.

Therefore:

~~~text
Multi-AI fabric
= orchestration

ACP
= provider construction transport

ASIR
= typed pre-seal construction semantic state
~~~

They MUST NOT be collapsed into one provider-specific protocol.

## Evidence and provenance

Every material provider invocation remains attributable through ProviderReceipt or the repository's successor typed mechanism.

Atlas SHOULD preserve enough lineage to answer:

- which task caused this invocation;
- which context/evidence refs were exposed;
- which provider/model/worker produced the output;
- which capability/budget profile applied;
- which candidate incorporated it;
- which verification later accepted/rejected it.

Provider transcript retention is optional unless policy requires it.

Typed provenance required for admission is not optional.

## Failure and liveness

The construction fabric MUST avoid pathological swarm behavior.

It MUST NOT:

- recursively spawn without policy/budget;
- retry the identical failed action forever;
- let agents vote UNKNOWN into certainty;
- treat provider timeout as semantic rejection;
- silently substitute another provider under materially different privacy/capability policy;
- merge conflicting candidate workspaces without an explicit reconciliation path.

A blocked task remains durable with its blocker.

A retry/escalation creates attributable execution lineage.

## Authority and final seal

No amount of provider agreement bypasses Atlas authority.

The terminal path remains:

~~~text
multi-AI exploration / synthesis / critique
→ typed candidate universe
→ verification evidence
→ DecisionProposal(s)
→ authorized SelectedDesign
→ AdmissionTransaction when applicable
→ exact resulting-tree recensus / verification
→ seal eligibility
→ SEALED logical Atlas
→ deterministic mechanical compaction
→ canonical *.atlas
~~~

"All agents agree" is not a seal condition unless policy explicitly defines the exact agreement as one evidence input, and even then the normal admission/seal gates remain.

## Implementation status discipline

This contract defines the canonical target architecture.

It does NOT by itself claim that Atlas currently implements:

- ProviderRouter;
- ConstructionTaskGraph runtime;
- AgentLease machine schema/enforcement;
- ContextCompiler;
- multi-provider concurrency;
- remote AI provider gateway;
- credential broker;
- budget scheduler;
- remote task queue;
- production Jev integration.

These remain TARGET until code and verification evidence exist.

The first implementation SHOULD avoid premature provider-specific abstractions and SHOULD reuse existing ProviderReceipt, DecisionProposal, CandidateChangeSet, SelfBuildWorkOrder, ASIR/ACP and sandbox contracts rather than creating parallel truth models.

## Final invariants

~~~text
many AIs
≠ many truths

agent conversation
≠ durable engineering memory

provider router
≠ selection authority

Jev proposal
≠ SelectedDesign

subagent spawn
≠ authority expansion

host-native provider
≠ trusted provider

remote provider
≠ remote semantic authority

context window
= bounded execution cache
≠ project memory

candidate branches
remain identity-separated

verification binds exact frozen candidate

AI thinks.
Atlas remembers.
Sandbox executes.
Evidence supports claims.
Jev proposes.
Policy/Human/Hybrid selects.
Admission commits.
Seal publishes.
~~~

Atlas MAY use whichever combination of Claude, GPT, Gemini, Jev, local models, future models and deterministic workers best satisfies the active task and policy.

Canonical Atlas meaning MUST remain provider-independent.
