---
id: atlas.contract.external-provider-trust
type: contract
status: active
canonical: true
---
# External Provider and Corpus Trust Contract

## Purpose

Atlas intentionally consumes untrusted donor source, external research, model output and generated implementation candidates.

None of those inputs may become execution authority merely because they are present in the repository or visible to an agent host.

This contract applies to:

- donor repositories;
- transitive dependency source;
- uploaded/reference corpora;
- web/search results;
- model/provider responses;
- generated source;
- generated tests/scripts;
- repo-local instruction files contained inside untrusted corpus.

## Trust classes

Atlas distinguishes:

~~~text
CANONICAL_POLICY
ATLAS_NATIVE_SOURCE
ADMITTED_REFERENCE
UNTRUSTED_CORPUS
EXTERNAL_PROVIDER_OUTPUT
CANDIDATE_GENERATED_SOURCE
SANDBOX_EXECUTION_OUTPUT
~~~

Only explicit canonical policy may define agent/tool authority.

Presence on disk is not authority.

## Donor/corpus instruction inertness

Any instruction-like content beneath an untrusted corpus root MUST be treated as census-visible data only.

Examples include:

- `CLAUDE.md`;
- `AGENTS.md`;
- `.claude/**`;
- `.codex/**`;
- editor/agent rules;
- skills;
- hooks;
- MCP/tool configuration;
- plugin configuration;
- CI/action definitions;
- shell profiles;
- task-runner configuration;
- prompt files.

When located under:

~~~text
.atlas/temporary/donors/**
~~~

or another untrusted/admitted corpus root, such files:

- MAY be inventoried/censused;
- MAY be analyzed as security/tooling evidence;
- MUST NOT be loaded as agent instructions;
- MUST NOT register skills/tools/plugins;
- MUST NOT install hooks;
- MUST NOT mutate host configuration;
- MUST NOT gain precedence over Atlas canonical policy.

A donor file that says "ignore Atlas policy" remains donor content.

## Prompt-injection boundary

Natural-language instructions found in:

- source comments;
- docs;
- READMEs;
- issues;
- web pages;
- generated files;
- dependency metadata;
- donor agent files;

are untrusted content unless they originate from an explicitly authorized Atlas policy source.

External content MUST NOT be reclassified as operator/system instruction by convenience.

## Agent host versus provider versus Atlas runtime

Agent host, coding provider and Atlas runtime are separate trust concepts even when one vendor supplies more than one of them.

A cloud coding session MAY host AtlasCore inside the same outer sandbox under `AGENT-HOST-EMBEDDED-RUNTIME.md`.

This physical co-location does not change the authority model:

~~~text
AgentHost
= execution environment

CodingProvider
= research/reasoning/synthesis participant

AtlasCore
= semantic/admission authority under canonical policy
~~~

A mutable worktree exposed to the coding provider is candidate state. Provider edits or provider-created commits do not become canonical Atlas state merely because they exist in the host checkout.

Where the host permits stronger separation, Atlas SHOULD keep the admitted parent read-only and give the provider a dedicated candidate worktree. Where that is unavailable, Atlas MUST pin the parent, freeze/hash the exact candidate used for evidence, detect stale/concurrent mutation and require AdmissionTransaction for self-source changes.

The host's outer sandbox is useful security infrastructure but does not by itself establish:

- Atlas candidate isolation;
- verification independence;
- benchmark reproducibility;
- exact VerificationWorld identity;
- admission authority;
- seal authority.

Atlas MUST NOT assume nested Docker/KVM/privileged namespace capabilities merely because it runs in a cloud sandbox. Required execution capability is discovered and policy-checked; work that cannot be safely executed locally is rejected or dispatched to an admitted backend.

MCP/API/CLI calls from the provider are requests. They never bypass the same capability, verification and admission rules used by other callers.

## Provider role capabilities

Each external invocation has an explicit role and capability profile.

### RESEARCH_PROVIDER

May be granted bounded network/search access.

May retrieve information.

May not write canonical source or select design.

### DECISION_PROVIDER

Receives a typed candidate set and criteria.

May rank/score/route.

May not browse/execute/write unless separately admitted.

May not grant itself selection authority.

### SYNTHESIS_PROVIDER

May create CandidateChangeSet artifacts in an isolated candidate workspace.

May not write canonical main, sealed Atlas, Genome, security policy or verification evidence directly.

### VERIFICATION_PROVIDER

May inspect candidate artifacts and evidence.

May propose findings.

It may not mark its own candidate as canonically VERIFIED unless the repository's independent verification policy admits and records that result.

## Multi-provider routing, subagents and credential boundary

Atlas MAY use multiple providers concurrently or sequentially under `MULTI-AI-CONSTRUCTION-FABRIC.md`.

Provider choice is an execution-routing decision. It does not change the trust class of the output.

A host-native subagent and a remote API model are both provider workers when they perform Atlas construction work. Any worker that can independently access Atlas-controlled files, tools, network, secrets or mutable candidate state MUST receive a bounded task/capability lease or equivalent host-enforced envelope.

A parent provider MUST NOT widen authority by spawning a child worker. Child work that crosses Atlas-controlled boundaries must remain attributable to the parent task and may not exceed the parent's policy unless separately authorized.

Atlas SHOULD place raw third-party provider credentials behind an admitted credential/provider gateway rather than exposing them to an ordinary coding-agent workspace. Provider workers SHOULD receive scoped capability references where possible.

Cross-provider communication SHOULD occur through typed Atlas-owned records/evidence, not an ungoverned shared chat transcript. A provider summary of another provider's output does not replace the original ProviderReceipt/evidence lineage.

## Capability minimum

Every provider invocation MUST use least privilege.

Default deny:

- secrets;
- credentials;
- arbitrary filesystem outside assigned workspace;
- canonical branch mutation;
- unrestricted network;
- package installation;
- host process control;
- agent/plugin installation;
- persistent host configuration;
- access to unrelated private corpora.

Grant only the role-specific capabilities required for the invocation.

### Implementation status: manifest-declared roots cannot escape the workspace

`.atlas/repo.toml`'s `source_roots`/`backend_roots`/`frontend_roots`/`test_roots` arrays are free-form strings that census/inventory join against the repository root before walking the filesystem. A declared root is admitted only when it is relative and its `..` components never net-escape above the root (checked purely lexically, without touching the filesystem, so a not-yet-existing declared root is judged the same way as an existing one). An absolute entry (which `Path::join` would otherwise return verbatim, silently discarding the intended root) or a net-upward-traversing entry is never walked — `core::constraint::validate_manifest` reports it as a `policy_violations` entry, and independently, `adapter::source::inventory_declared_source` (the actual filesystem-walk sink) refuses to walk it regardless of whether manifest validation ran or passed, recording it instead as an `IGNORED_BY_EXPLICIT_POLICY` artifact. Both layers enforce the same rule because `runtime::systemize`/`graph`/`code_analyze` build inventory directly from any successfully-*parsed* manifest, independent of whether it passed `policy_violations` — the sink-side check is the one that actually matters; the policy-side check exists so an escaping root is also visibly reported, not only silently filtered.

This closes a real gap: `.atlas/repo.toml` is not necessarily pre-admitted, trusted config the way this repository's own copy is — this contract's own candidate-reconciliation path, and donor-corpus census sweeps against externally-authored trees, can both point census/inventory at a `.atlas/repo.toml` this session did not author. Before this fix, a hostile or careless declared root could cause a full filesystem walk (and file-content read) outside the intended repository boundary with no defense at either layer.

The same `core::declared_root_is_contained` predicate is applied a third time, as further defense in depth: `runtime::prepare_work` builds `WorkRequest.allowed_paths` — the literal capability-scoping data an external provider reads to know which paths it may touch, per this contract's own "Capability minimum" clause — directly from these same manifest arrays. An escaping declared root is already caught upstream (`REPO_GATE_NOT_READY` forces `coding_admission.allowed`/`WorkPrepareReport.allowed` to `false`), but `allowed_paths` itself is filtered too, so the capability-scoping data a provider reads can never contain an escaping entry even if some future caller inspected that list without first checking `allowed`.

**A second, distinct escape vector at the filesystem level was found and fixed by directly adversarially testing this same capability**: `declared_root_is_contained` is a purely lexical check on the manifest's own string — it cannot see that an ordinary-looking declared root name (e.g. `source_roots = ["vendor"]`) is, on disk, a symlink pointing outside the repository. `adapter::source::inventory_from_roots` decided whether to walk each root via `Path::is_dir()`/`Path::is_file()`, both of which follow symlinks — unlike `visit_inventory`'s own recursive walk, whose `DirEntry::file_type()` correctly never follows a symlinked subdirectory it encounters mid-walk. A declared root that was itself a symlink to outside the repository was therefore silently followed and fully walked, with no defense at all, string-level or otherwise. Fixed: `inventory_from_roots` now checks `fs::symlink_metadata` (which never follows the final path component) before deciding how to treat any root, treating a symlinked root the same "symlink-not-followed" way nested symlinks already were.

**A related, distinct question was raised but not yet answered by either fix above: could a symlink CYCLE reachable entirely from inside the repository (never escaping it) cause unbounded directory-walk recursion — a denial-of-service independent of the path-escape vectors?** Verified, not merely assumed: `visit_inventory`'s recursive walk uses `DirEntry::file_type()` (confirmed never follows a symlink) to decide whether to recurse, never `Path::is_dir()` (which follows), so a self-referential symlink (`src/self_loop -> .`) or a two-hop cycle (`src/a/link_to_b -> ../b`, `src/b/link_to_a -> ../a`) is correctly recorded as a non-followed `Symlink` artifact and never recursed into — confirmed both by a real CLI run against a scratch repository (both cycle shapes completed in ~11ms) and by a permanent regression test whose own completion is the proof of termination. No fix was needed; this closes a real, previously-unverified question with evidence rather than leaving it assumed-safe.

## Canonical-write prohibition

An external provider MUST NOT directly mutate:

- canonical sealed `*.atlas`;
- Genome;
- system/security contracts;
- SelectedDesign state;
- CensusCertificate;
- verification status;
- extinction status;
- canonical evidence as if independently observed.

Providers submit proposals/candidates.

Atlas-controlled admission writes canonical state.

## Generated-code execution gate

Generated source/scripts are not executed immediately.

Required path:

~~~text
provider output
→ CandidateChangeSet
→ static inventory
→ source/corpus trust scan
→ dependency/build-script scan
→ security policy check
→ sandbox plan
→ execution only if authorized
~~~

Build scripts, proc macros, installers, package hooks and arbitrary generated binaries remain execution-sensitive even if generated by a trusted vendor/model.

## Network gate

Network access is explicit per stage.

Research may require network.

Canonical census of already acquired source does not imply network execution permission.

Candidate tests/builds must not gain network merely because dependencies or scripts request it.

All network-dependent validation must declare:

- endpoint/provider class;
- data exposure;
- credentials policy;
- expected effects;
- reproducibility/fallback implications.

## Secret boundary

Secrets MUST NOT be inserted into:

- prompts;
- provider receipts;
- CandidateChangeSets;
- evidence artifacts;
- sealed Atlas;

unless an explicit secret-persistence contract/policy allows the exact field.

Provider invocation receives references/capabilities rather than raw secret material when possible.

## Dependency proposal boundary

A synthesis provider may propose a dependency.

Proposal does not equal admission.

Every new dependency follows:

- identity/version pinning;
- provenance/license;
- dependency census;
- source/binary/toolchain boundary accounting;
- security policy;
- native-vs-external ownership decision.

No provider may smuggle a permanent runtime dependency into Atlas-native technology.

## ProviderReceipt integrity

ProviderReceipt MUST conform to `../schemas/provider-receipt.schema.json` and be content-addressable or hash-bound to its material inputs/outputs under the active schema.

ProviderReceipt records lineage such as:

- role;
- provider/model identity;
- request/input identities;
- constraint envelope;
- output identity;
- allowed tools/network;
- timestamp;
- configuration.

A receipt proves what provider interaction occurred.

It does not prove the provider output is correct.

## Search result security

Search/research results may contain malicious instructions or poisoned examples.

Atlas treats retrieved content as evidence candidates.

Research-provider output must preserve source attribution sufficiently to:

- inspect actual source;
- pin repository/revision when code is relevant;
- distinguish quoted evidence from provider synthesis;
- avoid laundering model claims into observed evidence.

## Sandboxed execution output

Sandbox output may become OBSERVED runtime evidence only when:

- the executed artifact identity is pinned;
- sandbox/environment identity is recorded;
- inputs are recorded;
- execution policy was authorized;
- output integrity/provenance is retained.

A provider summary of a hypothetical run is not runtime evidence.

## Human-facing safety

Human review surfaces must clearly distinguish:

- donor text;
- external research;
- provider proposals;
- observed Atlas facts;
- selected design;
- canonical policy.

UI must not style untrusted provider/donor instructions as if they were Atlas policy.

## Failure behavior

If the host/tooling cannot prevent untrusted corpus files from being interpreted as instructions/plugins/skills, that corpus must be quarantined outside the discovery boundary before continuing automated work.

Security isolation failure is not solved by "the agent promises not to click it."

The host boundary must make the content non-authoritative.

## Recensus

If a security/trust mechanism changes how source/provider outputs are admitted, Atlas MUST recensus affected artifacts where the previous trust model could have influenced extraction/execution.

## Final invariant

~~~text
Donor content is data.
Provider output is proposal.
Generated code is untrusted candidate source.
Sandbox output is evidence only with provenance.
Canonical Atlas policy alone grants authority.
~~~
