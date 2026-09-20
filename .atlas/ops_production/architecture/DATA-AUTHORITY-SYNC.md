# Data, Authority, and Sync

This document defines how an Ops owns data in standalone mode and converges with Chronica in connected mode.

## Classify every durable data class

Each Ops schema/domain area must classify durable state as one of:

```text
SHARED_CANONICAL
LOCAL_BUSINESS_PRIVATE
LOCAL_PROVIDER_STATE
LOCAL_OPERATIONAL_STATE
REBUILDABLE_PROJECTION
EPHEMERAL_CACHE
SECRET
```

Do not treat every table as a Chronica node, and do not hide shared business truth inside an Ops-only table after connection.

The storage class above is about ownership/durability. Separately classify semantic role where relevant:

```text
DESCRIPTIVE_SOURCE_OR_STATE
NORMATIVE_SOURCE_OR_STATE
COGNITIVE / DERIVED ANALYSIS
ACTIVE / EXECUTION STATE
EVIDENCE / PROVENANCE
```

A policy/contract/approval/safety rule can be stored as `SHARED_CANONICAL` while still having a normative semantic role. Do not create a second storage taxonomy merely for normativity.

## Standalone ownership

In standalone mode, the Ops database/event history is authoritative for the product deployment.

The Ops must provide:

```text
schema migrations
transactionality
concurrency rules
backup/restore
recovery
idempotency where needed
audit/evidence appropriate to effects
```

If the Ops contains normative behavior, standalone mode must also preserve the source/version/effective-time/procedure/precedence information required to reproduce real decisions. A boolean such as `approved=true` is insufficient when the underlying meaning depends on who approved what, under which source, scope, conditions, and version.

## Connected ownership

When connected:

- Chronica owns the shared semantic identity/world/execution/evidence history;
- shared normative sources/state that are admitted to Chronica converge on the same world graph rather than a second policy universe;
- Ops owns provider-specific and local operational mechanics;
- Ops may retain a durable local copy/projection necessary for performance and independence;
- synchronization state must be explicit and reconcilable.

Where a normative source intentionally remains external/provider-owned, Chronica stores the authorized reference/provenance/observed version needed for the declared integration semantics rather than pretending to own the source.

## Existence, effectiveness, validity, authority, execution

Never collapse these states:

```text
EXISTS
EFFECTIVE
VALID / APPLICABLE
AUTHORIZED
EXECUTED
EVIDENCED / RECONCILED
```

Examples:

```text
a contract row exists but has not become effective
a policy version is effective but not applicable to this tenant/scope
a provider approval event exists but procedure/authority is invalid
a command is normatively permitted but no Chronica execution authority exists
a command was sent but real-world outcome is unknown
```

Sync code must preserve these distinctions rather than reduce them to one status field unless the source domain genuinely defines one combined state.

## No dual-write fantasy

Avoid uncontrolled:

```text
write Ops DB
AND write Chronica
```

inside one request with no durable coordination.

Prefer patterns such as:

```text
local transaction + durable outbox
-> integration worker
-> Chronica admission/execution/event
-> acknowledgement/checkpoint
```

or an explicit Chronica-authorized work request followed by Ops execution and durable evidence.

The exact pattern depends on who initiates the effect, but partial failure must be representable.

Normative transitions must follow the same rule. Do not update local `approved/revoked/effective` state and Chronica normative state independently with no durable correlation.

## Correlation

Cross-boundary activity needs stable identifiers:

```text
ops_resource_id
chronica_resource_ref
command/request_id
idempotency_key
work_run_ref
provider_request_id
provider_result_id
evidence_ref
sync_checkpoint/version
```

For normative sources/effects also preserve stable identifiers as relevant:

```text
norm_source_id
norm_source_version
issuer/authority_ref
procedure/approval_ref
interpretation/decision_ref
conformance_result_ref
```

## Authority and normativity

Standalone authority may be implemented locally.

Connected authority may be delegated/federated through Chronica for configured scopes. The Ops must not silently weaken policy when connected.

```text
LocalPolicyAdapter
ChronicaAuthorityAdapter
```

should satisfy a common application-facing decision contract where feasible.

But operational authority and normative status are distinct:

```text
Can(...) != May(...) != Must(...)
```

A local or Chronica authority adapter answers whether execution is authorized through that system. A normative resolver answers whether the act is permitted/required/forbidden/empowered under applicable sources. Some workflows need both; some need only operational authority.

## Inbound state

When Chronica updates shared state relevant to the Ops, the Ops must define whether it:

```text
APPLIES locally
PROJECTS only
REQUIRES provider synchronization
REQUIRES human approval
CONFLICTS with local state
```

For inbound normative changes additionally define whether the change:

```text
BECOMES EFFECTIVE locally now
IS STORED but future-effective
SUPERSEDES / AMENDS / REVOKES an older source
REQUIRES procedure/provider acknowledgement
IS NON-AUTHORITATIVE interpretation only
IS UNRESOLVED because precedence/applicability is unknown
```

## Conflict policy

Do not use last-write-wins by accident for business-critical state.

Define conflict handling for:

```text
version mismatch
concurrent edits
duplicate provider webhook
reordered event
stale cache
offline action
partial provider execution
unknown outcome
```

For normative material also handle:

```text
conflicting source versions
source hierarchy / precedence
amendment vs stale rule
revoke/suspend vs cached permission
scope/jurisdiction mismatch
exception/override collision
controlling interpretation change
```

Never let an LLM guess precedence or silently pick the newest text when the domain's conflict rules say otherwise.

## Offline behavior

A disconnection must not silently transform unresolved normative state into permission.

If a required authoritative source/version/conformance result cannot be verified while offline, the contract must specify one of:

```text
use a proven cached version within its valid offline envelope
require human/local authority explicitly allowed for that scope
return UNKNOWN / UNRESOLVED and fail closed
queue the action until resolution is available
```

Do not convert `unknown` into `allow` merely to keep the product responsive.

## Reconciliation

A reconciliation worker should be able to compare:

```text
intended state
local durable state
provider-observed state
Chronica shared state
```

and, where normative behavior is present:

```text
source/version expected
source/version observed
current effective/valid interval
procedure/authority evidence
precedence/applicability result
resulting normative state
```

Then produce evidence-backed corrections or unresolved exceptions.

## Deletion and retention

Deletion, retention, legal hold, and privacy behavior must be mapped across Ops and Chronica. A local delete cannot silently leave an unauthorized shared projection, and Chronica retention cannot force deletion of provider records where another legal/contractual obligation applies without policy resolution.

Historical normative source versions, revocations, decisions, and evidence may need retention even after they are no longer effective because replay/explanation depends on them. Retention policy, not convenience, decides what can be removed.

## Rule

> **Connected systems may duplicate bytes for operation; they must not duplicate authority over meaning. Preserve source, validity, precedence, procedure, execution, and evidence as distinct semantics whenever the real domain requires them.**
