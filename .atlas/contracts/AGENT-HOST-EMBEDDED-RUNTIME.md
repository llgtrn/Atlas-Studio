---
id: atlas.contract.agent-host-embedded-runtime
type: contract
status: active
canonical: true
---
# Embedded Agent Host and Sandbox Runtime Contract

## Purpose

Atlas is a semantic engineering runtime and tool substrate before it is an IDE.

Atlas MUST be able to run as an embedded subsystem inside an external coding-agent host, including an ephemeral cloud coding session, without making that host, model vendor, transport, or sandbox implementation part of canonical Atlas truth.

A primary target deployment is:

~~~text
Claude Code / another coding agent
        ↓
local agent transport adapter
        ↓
Atlas Core running inside the same outer host sandbox
        ↓
Atlas-managed candidate workspace / execution backend
        ↓
Census / semantic query / verification / admission / seal
~~~

Claude Code is an example host/provider combination, not an Atlas dependency.

The same Atlas Core MUST remain usable from local CLI, CI, remote services, future Atlas Studio, and other agent hosts.

## Canonical topology

~~~text
Agent Host
┌─────────────────────────────────────────────────────────────┐
│ Outer Host Sandbox                                          │
│                                                             │
│  Human ↔ Coding Agent                                       │
│            │                                                │
│            │ MCP / API / CLI adapter                        │
│            ▼                                                │
│       Atlas Adapter                                         │
│            │                                                │
│            ▼                                                │
│       Atlas Core / Control Plane                            │
│       ├─ Census                                             │
│       ├─ Semantic Store / Query                             │
│       ├─ Construction Controller                            │
│       ├─ Verification Orchestrator                          │
│       ├─ Admission                                          │
│       └─ Atlas / AtlasX engines                             │
│            │                                                │
│            ▼                                                │
│       Atlas Job Isolation                                   │
│       ├─ candidate workspace A                              │
│       ├─ candidate workspace B                              │
│       ├─ verifier workspace                                 │
│       └─ benchmark/failure workspace                        │
└─────────────────────────────────────────────────────────────┘
             │ optional scale-out
             ▼
      remote verifier / benchmark / GPU / multi-node workers
~~~

The outer host sandbox and Atlas job isolation are different boundaries.

The outer host may provide process/filesystem/network isolation from the host machine. Atlas job isolation exists to keep candidate construction, verification evidence, destructive tests, and admission lineage separated from one another even when all of them execute inside one outer host sandbox.

## Terms

### AgentHost

The environment that owns the interactive coding session and its outer execution sandbox.

Examples include a local developer shell, a cloud coding-agent session, CI, or a future hosted Atlas environment.

AgentHost is execution infrastructure, not semantic authority.

### CodingProvider

The model/agent performing reasoning, research, editing or synthesis.

The provider may be supplied by the AgentHost, but host and provider remain distinct concepts.

### AtlasCore

The provider-neutral Atlas runtime that owns Census, semantic identity, verification orchestration, admission rules, Atlas construction and materialization semantics.

### TransportAdapter

A noncanonical interface exposing AtlasCore to a caller.

Target adapters include:

- CLI;
- local stdio MCP;
- remote MCP/HTTP;
- typed API/gRPC or local IPC;
- future Atlas Studio application bindings.

Transport representation MUST NOT become canonical semantic representation.

### OuterSandbox

The security/resource boundary supplied by the AgentHost.

Atlas MUST NOT assume that OuterSandbox permits nested Docker, privileged containers, KVM, user namespaces, mount namespaces, or any other specific OS primitive.

### CandidateWorkspace

A mutable workspace containing one candidate revision or candidate construction state.

A CandidateWorkspace is not canonical repository state merely because it exists on disk or because a provider has committed it to a noncanonical branch.

### SandboxBackend

The Atlas execution backend used to isolate a candidate build/test/benchmark/failure job.

Backend strength is environment-dependent. Possible implementations include workspace isolation, restricted subprocess execution, OS sandboxing, containers, microVMs, remote workers, or stronger future mechanisms.

### DurableAtlasState

Canonical durable engineering knowledge that must survive the lifetime of one agent session.

This includes sealed Atlas artifacts and the identities/lineage/evidence required by their active contracts.

## Atlas as subsystem, not agent memory

An embedded Atlas deployment exists specifically so that the coding agent does not have to recreate the engineering world from conversational context on every session.

The intended relationship is:

~~~text
Coding agent
= temporary reasoning + implementation intelligence

Atlas Census
= durable perception

Atlas semantic world
= durable machine-readable engineering memory

Obligations / evidence / closure
= durable expectations and uncertainty
~~~

A provider session ending MUST NOT erase the semantic meaning of an already sealed Atlas artifact.

## Deployment profiles

Atlas MUST preserve the same semantic contracts across deployment profiles.

### EMBEDDED_LOCAL

~~~text
local coding agent / human
→ local Atlas adapter
→ AtlasCore on the same machine
~~~

Useful for development, offline work and local CI.

### EMBEDDED_AGENT_CLOUD

~~~text
cloud coding-agent session
→ local adapter inside that session
→ AtlasCore inside the same outer cloud sandbox
~~~

This is a first-class target profile.

The profile may use the cloud host's existing compute, checkout, build cache and outer sandbox while Atlas supplies its own candidate/admission semantics.

### REMOTE_ATLAS

~~~text
agent/human client
→ authenticated remote transport
→ Atlas service / workers
~~~

Useful when the host cannot execute Atlas locally or when organization policy centralizes semantic/evidence state.

### HYBRID

~~~text
embedded Atlas control/query path
→ local fast census/query
→ selected heavy work dispatched to remote Atlas workers
~~~

Useful for large benchmarks, multi-node failure testing, GPU work, very large donor census or workloads exceeding the host session quota.

Changing deployment profile MUST NOT silently change semantic identity or admission meaning.

## Embedded cloud startup contract

An embedded cloud session should conceptually perform:

~~~text
checkout repository / establish exact parent revision
→ locate or install compatible Atlas runtime
→ start local Atlas adapter
→ load durable Atlas state when available
→ validate schema/Genome/runtime compatibility
→ incremental census of the current revision
→ expose typed Atlas tools to the coding agent
→ perform candidate work
→ verify / admit / seal according to policy
~~~

Bootstrap convenience does not grant the agent authority to weaken policy.

If no durable Atlas artifact exists yet, Atlas may construct the initial semantic world from source and evidence under the normal creation contracts.

## Mutable worktree is candidate state

Embedded agent hosts commonly give the coding agent write access to the checked-out repository.

Atlas therefore MUST distinguish physical mutability from canonical authority.

Canonical truth is pinned to admitted identities and revisions, not to whatever bytes currently exist in the mutable worktree.

The required rule is:

~~~text
admitted parent revision
≠ mutable host working tree
≠ provider-created commit
≠ admitted resulting revision
~~~

Edits made by the coding agent are CandidateChangeSet material until the applicable admission path accepts them.

A provider-created Git commit MAY improve reproducibility of a candidate, but commit existence alone does not make that revision canonical Atlas state.

Atlas self-modification still requires AdmissionTransaction.

When stronger filesystem separation is available, Atlas SHOULD place synthesis work in a dedicated worktree/snapshot and keep the admitted parent read-only.

When the host cannot enforce that layout, Atlas MUST at minimum pin the admitted parent, hash/freeze the exact candidate used for verification, detect concurrent mutation, and reject stale or mismatched admission.

## MCP boundary

MCP is a preferred agent-facing adapter for embedded agent-host operation.

MCP is NOT:

- Atlas's canonical semantic model;
- Atlas's internal storage format;
- a requirement of sealed *.atlas artifacts;
- selection authority;
- verification authority;
- admission authority.

A target local MCP surface may expose capabilities conceptually equivalent to:

~~~text
atlas.project_status
atlas.census
atlas.query_semantics
atlas.query_unknowns
atlas.impact
atlas.construct_candidate
atlas.verify_candidate
atlas.explain_failure
atlas.compare_candidates
atlas.request_admission
atlas.materialize
~~~

Exact method names are API design, not canonical semantics.

Every materially mutating call MUST still pass the same authority, CandidateChangeSet, verification and admission rules as CLI/API/Studio callers.

An MCP call from a model is a request to Atlas, not proof that the requested action is permitted.

Local stdio MCP is a preferred embedded shape because it permits the agent and Atlas to communicate without requiring an independent Atlas cloud service. Remote transports remain valid.

## Sandbox hierarchy

Atlas MUST model at least the following logical layers even if some layers share one physical machine:

~~~text
L0  AgentHost / OuterSandbox
    security boundary supplied by host

L1  AtlasCore / control plane
    policy, semantic identity, orchestration

L2  CandidateWorkspace / JobWorkspace
    isolated candidate mutation and build state

L3  ExecutionSandbox
    build/test/fuzz/failure/benchmark execution boundary

L4  VerificationWorld
    recorded environment/workload/failure/evidence semantics
~~~

Physical co-location does not merge these semantic roles.

## Sandbox capability detection

Atlas MUST NOT hard-code Docker, Kubernetes, bubblewrap, KVM, Firecracker, a specific cloud, or another single sandbox implementation as a prerequisite for AtlasCore.

A runtime implementation SHOULD detect the capabilities actually available from the current AgentHost and choose a compatible SandboxBackend.

Conceptually relevant capabilities include:

- writable workspace scope;
- read-only snapshot/worktree support;
- subprocess restriction;
- filesystem namespace/isolation;
- network egress control;
- CPU/memory/time quotas;
- container support;
- VM/microVM support;
- deterministic/pinned environment support;
- destructive-failure-test support;
- credential brokering;
- remote-worker dispatch.

Work requiring a stronger isolation or reproducibility class than the host can provide MUST fail closed, downgrade only under explicit policy, or dispatch to a suitable remote backend.

Atlas MUST NOT claim a stronger sandbox than was actually used.

## Baseline embedded isolation

Atlas V1 does not require nested virtualization inside an agent-host cloud sandbox.

A valid baseline may use:

~~~text
OuterSandbox supplied by host
+ pinned parent revision
+ Atlas job directories/worktrees
+ candidate freeze/hash
+ explicit network/process policy where host permits
+ exact verification evidence
+ AdmissionTransaction
~~~

This provides engineering isolation and transactional lineage even when a second kernel/hypervisor boundary is unavailable.

It is weaker than a hostile-code microVM boundary and MUST be described honestly as such.

## Synthesis and verification separation

A synthesis provider may edit a CandidateWorkspace.

Verification MUST bind to an exact frozen candidate identity.

After a verification run starts, the provider MUST NOT be able to silently change the verified candidate while retaining the old evidence identity.

A failed candidate returns structured evidence to a repair loop.

A repaired candidate receives a new candidate/revision identity and is verified again.

Where policy requires independence, verification MAY run in a separate process, sandbox backend, host, provider or environment.

Independent verification is a policy property; sharing the same outer cloud VM does not by itself prove or disprove independence.

## Final seal inside an agent host

Canonical *.atlas creation MAY complete entirely inside an embedded agent-host cloud session.

However the authority path remains:

~~~text
provider edits / synthesis
→ CandidateChangeSet
→ Atlas census
→ CandidateAtlas
→ VERIFY / BENCH / PROVE
→ selection
→ AdmissionTransaction when applicable
→ exact-candidate seal gate
→ SEALED logical Atlas
→ mechanical compaction
→ canonical *.atlas
~~~

The final seal/compaction process MAY be physically co-resident with the coding agent inside the same OuterSandbox, but it MUST be logically Atlas-controlled.

Provider output cannot mark itself verified, selected, admitted or sealed.

Canonical post-seal compaction MUST remain deterministic/provider-independent and MUST NOT call the coding model merely because that model process is nearby.

If the host cannot protect seal input from concurrent mutation, Atlas MUST freeze/snapshot/hash the exact seal input or dispatch the seal to a stronger backend.

## Durable versus ephemeral state

An agent-host cloud sandbox may be short-lived.

Atlas MUST classify state accordingly.

Typical ephemeral/regenerable state includes:

- compiler/build caches;
- temporary candidate worktrees;
- derived search indexes that can be rebuilt;
- scratch benchmark outputs not admitted as evidence;
- temporary donor staging after its durable knowledge/provenance requirements are satisfied.

Durable state includes where applicable:

- sealed *.atlas roots/shards;
- Genome/schema/version coordinates;
- admitted repository revision identity;
- SelectedDesign lineage;
- CandidateChangeSet / AdmissionTransaction lineage required by policy;
- CensusCertificate and closure state;
- evidence/attestation roots required by a seal;
- canonical contracts and blueprint identity.

Session restart should be able to perform:

~~~text
new agent-host sandbox
→ restore/checkout admitted repository revision
→ load compatible durable Atlas state
→ incremental recensus
→ resume engineering without depending on prior model context
~~~

Provider session memory is never the only durable location of project truth.

## Network and credentials

Embedded execution does not weaken EXTERNAL-PROVIDER-TRUST.md.

Network access remains stage/capability scoped.

Secrets SHOULD be brokered by the host or an Atlas credential boundary rather than copied into provider prompts, CandidateChangeSets or sealed artifacts.

Atlas MCP/API responses MUST NOT expose credentials merely because the caller runs in the same outer sandbox.

Git push, package publication, production deployment and other external mutations remain separately authorized effects.

## Donor and repository ambient-tooling isolation

Embedding Atlas beside a coding agent makes donor instruction inertness even more important.

Files such as CLAUDE.md, AGENTS.md, agent skills/hooks, MCP configuration and related ambient instruction formats found inside untrusted donor/corpus roots remain data.

They MUST NOT auto-register against the live host merely because Atlas and the coding agent share a filesystem.

DONOR-WORKBENCH-ISOLATION.md and EXTERNAL-PROVIDER-TRUST.md remain normative.

## Scale-out without semantic drift

Atlas SHOULD use the host's local compute for low-latency operations when adequate:

- semantic query;
- incremental census;
- small builds/tests;
- candidate diffing;
- local verification.

Heavy work may move to remote workers:

- large clean builds;
- fuzzing;
- very large donor/dependency census;
- deterministic benchmark fleets;
- multi-node distributed tests;
- failure injection;
- GPU/accelerator workloads.

Dispatch location is execution metadata.

It MUST NOT become semantic identity unless the environment itself is materially part of a VerificationWorld or performance claim.

## Atlas Studio

Atlas Studio MAY later become the native visual/interactive frontend over the same AtlasCore.

Studio MUST NOT create a parallel semantic truth system.

The intended product layering is:

~~~text
Atlas Core
    ↑
CLI   MCP/API   CI   Atlas Studio
      ↑
Claude / other coding agents
~~~

Source code is one projection of the Atlas semantic world. Studio may visualize semantic identities, graph relations, state/effect/ownership/concurrency/persistence facts, obligations, evidence, unknowns, conflicts, candidate deltas and verification results without redefining them.

## Provider and host independence

A sealed Atlas artifact MUST remain meaningful if:

- the coding-provider vendor changes;
- the AgentHost changes;
- the MCP adapter changes;
- the original cloud session is destroyed;
- local execution moves to a remote worker;
- Atlas Studio replaces the original coding UI.

Host/provider-specific receipts may remain as provenance.

Host/provider availability is not required to decode or validate already sealed semantic meaning.

## Implementation status discipline

This contract defines the canonical target boundary.

It does NOT by itself claim that Atlas already implements:

- a production MCP server;
- every SandboxBackend listed above;
- remote worker scheduling;
- a hosted Atlas control plane;
- Atlas Studio;
- full credential brokering.

Until implementation and verification evidence exist, those capabilities remain TARGET.

The architectural invariants in this contract apply immediately to implementations as they are added.

## Final invariants

~~~text
AgentHost provides execution.
Agent/provider provides temporary intelligence.
Atlas owns semantic truth and admission.

OuterSandbox
≠ Atlas CandidateWorkspace
≠ VerificationWorld

mutable worktree
≠ admitted canonical revision

MCP
= adapter
≠ semantic authority

provider request
≠ permission
≠ verification
≠ admission
≠ seal

ephemeral session
≠ durable project memory

same AtlasCore
→ CLI / MCP / API / CI / Studio

big available compute
≠ big authority
~~~

Atlas may borrow an agent cloud's compute and sandbox.

It MUST NOT borrow the agent's uncertainty, memory lifetime, or authority model as canonical truth.
