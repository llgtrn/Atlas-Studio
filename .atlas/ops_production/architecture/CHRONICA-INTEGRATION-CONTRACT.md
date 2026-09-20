# Chronica Integration Contract

Status: **ACTIVE OPS INTEGRATION CONTRACT**

This document defines what an Ops must preserve so Chronica can integrate it as a first-class production block during incubation and later absorb mature source without losing semantic meaning.

Connected Ops attach through the shared cross-boundary composition described by `docs/architecture/operations/fabric.md`. Fabric is not a second runtime, authority plane or truth store. The bridge maps meaning, not just transport.

## One-world semantic law

Every connected Ops must preserve the same semantic separations as Chronica:

```text
DESCRIPTIVE   what is / was / was observed
NORMATIVE     what may / must / must not / is empowered to happen
COGNITIVE     what is inferred / interpreted / predicted / recommended
ACTIVE        what is requested / execution-admitted / executed / reconciled
```

These are aspects of one world, not separate databases or engines.

Core distinctions:

```text
Can(...)                  technical / semantic applicability
Authorized(...)           operational authority
May/Must/...               normative relation
ExecutionAdmission         shared composed pre-effect decision
CandidateBinding           eligible provider/implementation path
SelectedBinding            concrete path chosen for an ExecutionPlan/WorkRun
Canonicalization           candidate observation/effect -> durable canonical history
ContextCompiled            scoped context prepared
Disclosed                  data actually transmitted across a boundary
```

Therefore:

```text
Can != Authorized != May/Must
ExecutionAdmission != Canonicalization
CandidateBinding != SelectedBinding
Binding != Authority
ContextCompiled != Disclosed
ProviderSuccess != ReconciledEffect
ReconciledEffect != CanonicalizedEvent
UNKNOWN != SUCCESS
```

## Fabric attachment contract

A Chronica bridge is a Fabric attachment port, not a second domain/runtime owner.

For every meaningful shared workflow, preserve the relevant:

```text
stable principal/resource identity
semantic command/query meaning
state/lifecycle meaning
epistemic / normative / cognitive / active classification
Capability Resolution inputs / Can(...) where relevant
operational authority / deny / scope / expiry
normative/legal/contractual meaning where real
privacy / purpose / sovereignty constraints
runtime safety / limits where relevant
candidate vs selected binding/provider boundary
correlation / idempotency
WorkRequest / ExecutionAdmission / WorkRun lineage
event / evidence / Canonicalization meaning
failure / unknown-outcome / reconciliation
offline / reconnect / recovery
context compilation vs external disclosure
contract/version compatibility
```

A bridge that can call endpoints but cannot preserve these meanings is transport-connected, not semantically integrated.

## Required semantic mapping

Every connected Ops defines mappings for its real public/shared behavior:

```text
principal / identity
resource types and stable resource identifiers
canonical vs observed/derived state
read/query semantics
effectful semantic actions
technical applicability / candidate bindings
operational authority requirements
normative requirements where applicable
privacy / purpose / Holding scope
runtime safety where applicable
approvals / evidence requirements
events / receipts / evidence
idempotency / correlation
sync / reconciliation / recovery
version compatibility
```

Do not mechanically map provider labels such as `approved`, `allowed`, `active`, `capability` or `permission` into Chronica authority. Determine what the source concept actually means.

For normatively governed domains, additionally preserve as relevant:

```text
normative source / issuer / provenance
source authenticity
effective interval / revision lineage
jurisdiction / organizational scope / purpose
subject / holder / duty-bearer / target
permission / prohibition / obligation / power
conditions / exceptions / defenses
required procedure / approvals / segregation of duties
precedence / override / conflict / supersession
controlling interpretation / decision where applicable
conformance / violation / unresolved result
explanation / evidence path
```

Do not invent normativity where none exists, and do not collapse normativity into operational authorization where it does exist.

## Shared execution path

Every consequential connected ACT converges on the shared effect spine:

```text
Caller / UI / Agent / Ops workflow
-> Intent / Proposal
-> WorkRequest
-> Capability Resolution / Can(...) + candidate binding availability
-> Authorized(...) + Normative + Privacy + Safety + approvals/evidence as applicable
-> ExecutionAdmission
-> ExecutionPlan
-> SelectedBinding
-> Ops execution adapter / provider call
-> WorkRun / Effect
-> Evidence
-> Reconciliation when needed
-> Candidate Effect Event
-> Canonicalization
-> canonical history / replay-derived state
```

Chronica does not need to execute donor/provider mechanics itself during incubation. The Ops may remain the selected execution provider for its domain until native absorption, but it does not get a separate authority/admission universe.

Operational authority does not prove normative permission. `Can=true` does not prove either. A reachable provider path proves none of them.

At minimum, consequential `ExecutionAdmission` distinguishes:

```text
ALLOW
DENY
REQUIRE_APPROVAL
REQUIRE_EVIDENCE
UNRESOLVED
```

`UNRESOLVED` never silently degrades to `ALLOW`.

## Resource identity

Never create disconnected duplicate identity merely because the donor did.

Examples:

```text
Ops customer       -> Chronica Person/Organization relation
Ops property       -> Chronica Resource
Ops reservation    -> Chronica Resource/Event pattern
Ops ticket         -> Chronica issue/conversation/work resource pattern
Ops account        -> mapped financial/resource identity
Ops robot/device   -> Chronica machine/resource identity
```

The exact semantic pattern may evolve, but mappings must be stable/versioned and cannot depend on mutable UI labels alone.

Normative identities must also remain stable where applicable: contract version, policy source, approval mandate, organizational role, safety rule or standards document cannot be identified only by display text.

Cross-boundary correlation should preserve stable references across the relevant:

```text
principal_id
resource_id
intent / work_request / execution_admission / work_run
selected binding / provider request / result
event / evidence / candidate event
canonicalization result
sync checkpoint
```

Provider-local IDs may participate in mapping but must not silently become universal identity.

## Action mapping

For each public effectful Ops action define as relevant:

```text
operation id
provider-independent semantic action
principal / target resource
purpose / scope
risk class
technical preconditions / Can(...) inputs
candidate binding requirements
operational authority requirements
normative / privacy / safety constraints
limits / budget
idempotency / correlation
approval requirements
required evidence
expected effect
unknown-outcome / reconciliation behavior
candidate-event / Canonicalization behavior
```

If the action changes normative state, preserve source authority/power, required procedure, preconditions, intended normative effect, validity conditions and resulting rights/duties/permissions/prohibitions/powers.

Examples include approval, waiver, delegation, revocation, contract acceptance, appointment, suspension, policy change or safety-override request.

## Event and truth mapping

Ops/provider events must distinguish at least:

```text
raw / observed provider event
local operational observation
requested action
ExecutionAdmission decision
execution attempt
provider acknowledgement
observed effect
unknown outcome
reconciled effect
Candidate Effect Event
Canonicalized Event
```

A provider/webhook emission is a candidate observation/evidence source, not canonical truth by transport alone.

```text
ProviderEvent
-> normalize + provenance / dedupe
-> Candidate Event
-> Canonicalization
-> canonical history
```

For ACT-originated effects:

```text
WorkRun / Effect
-> Evidence + Reconciliation
-> Candidate Effect Event
-> Canonicalization
```

Do not emit canonical success merely because a request was sent or a provider returned HTTP 200.

For normative acts also preserve the distinction between source existence, effectiveness, interpretation/conformance and actual governed effect. A document existing does not prove it is effective or applicable.

## Evidence

Useful execution evidence may include:

```text
provider response / receipt
remote object/version id
timestamp
request/correlation/idempotency id
actor/principal
Can(...) decision inputs/ref
AuthorityDecision ref
normative/privacy/safety/approval result refs
ExecutionAdmission decision ref
SelectedBinding / provider version
input/output digest
before/after state refs
reconciliation result
Candidate Event / Canonicalization refs
```

Normative/conformance evidence may additionally include source/version/hash, issuer/source authority, effective interval, scope/jurisdiction/purpose, procedure/approval evidence, exception/precedence rule and explanation.

Compatibility requires enough evidence to reconstruct why an operation crossed the boundary and what happened after it did.

## Context, memory and disclosure

Chronica may compile an authorized projection of Ops state into a `ContextEnvelope`. The Ops bridge must support purpose/scope/sensitivity/provider filtering and must not expose unrestricted local databases to AI providers.

```text
ContextCompiled != Disclosed
```

Actually transmitting context to an external provider is a separate governed ACT:

```text
ContextEnvelope
-> Disclosure / ProviderInvocation WorkRequest
-> ExecutionAdmission
-> Selected provider binding
-> transmission/invocation
-> disclosure evidence
```

When context contains normative material, preserve the difference between source artifact, normative relation/state, interpretation/decision, non-authoritative commentary/AI analysis and current applicability/conformance result.

A summary must not silently turn commentary into authority or erase source/version/time provenance.

## MCP and transport

The bridge may use API, event transport, database CDC, MCP, message bus, queue, RPC or another adapter.

**Transport is not semantics and connectivity is not authority.**

```text
MCP tool visible != Can(...)
MCP tool visible != Authorized(...)
MCP tool visible != ExecutionAdmission=ALLOW
```

Changing REST to MCP, queue to stream, or provider A to provider B must not change canonical semantic action identity unless the contract is intentionally versioned.

A material provider/binding change that affects privacy, residency, safety, compatibility or other assumptions used by `ExecutionAdmission` requires revalidation before effect.

## Offline and reconciliation

Offline/standalone operation may keep bounded local operational state, but connected shared truth must converge through canonical event/reconciliation semantics rather than last-writer guesswork.

Unknown effects remain explicit. Reconnect must reconcile provider/remote state before unsafe retry or canonical success.

Standalone caches and local projections do not become permanent competing shared truth when connection resumes.

## Compatibility

Each release should publish enough version identity to reason about compatibility, for example:

```text
ops_version
semantic_contract_version
chronica_bridge_version
minimum_chronica_contract_version
schema_version
```

If a release changes identity, semantic action meaning, state classification, authority/normative conditions, binding semantics, event/evidence meaning, unknown-outcome behavior, Canonicalization behavior or disclosure rules, treat it as semantic compatibility work rather than a harmless transport change.

## Complete integration

An Ops is fully integrated only when its public/shared surface has no unexplained semantic gaps and real tests prove the required behavior.

For normatively governed workflows, source/validity/permission-duty-power/procedure/precedence/conformance semantics must be mapped/tested or explicitly classified as external/local-only with a justified ownership boundary.

For shared effectful workflows, complete integration requires stable identity, `Can`/authority/normativity separation, one `ExecutionAdmission`, selected binding semantics, correlation/idempotency, evidence, unknown-outcome/reconciliation, Canonicalization and recovery/version compatibility.

> **Map meaning, not endpoints. Attach through one Fabric. Preserve one world. Keep capability, authority, normativity, cognition, execution, evidence, disclosure and canonical truth distinguishable.**
