# Standalone and Connected Modes

Every Ops must support two deliberate modes without forking the product codebase.

## Standalone mode

Chronica is absent.

The Ops owns for that deployment:

```text
local identity/authentication
local durable database/event history
local authorization/policy
local background jobs
local secrets/configuration
local UI/API/CLI/MCP
local backup/restore
local audit/evidence adequate for the product
```

Standalone mode is not a demo mode. It must be production-usable within the Ops' own scope.

## Connected mode

Chronica is present as the shared world/resource/authority/execution/evidence substrate.

The Ops still runs its own core and provider-specific mechanics, but shared semantics converge:

```text
Ops local resource
-> semantic mapping
-> Chronica resource identity

Ops command
-> mapped intent/work request
-> Chronica authority when required
-> Ops execution/provider adapter
-> evidence/result
-> Chronica canonical event/reconciliation
```

## Canonicality

Standalone:

```text
Ops local durable truth
= authoritative for that standalone deployment
```

Connected:

```text
Chronica
= canonical for shared connected identity/resource/business events/effects

Ops local store
= operational state + local provider state + cache/projection + bounded local truth
```

The Ops may retain durable local state necessary to operate, but it must classify whether each class is:

```text
SHARED_CANONICAL
LOCAL_PROVIDER_STATE
LOCAL_OPERATIONAL_STATE
REBUILDABLE_PROJECTION
EPHEMERAL_CACHE
```

## Connection must not delete independence

Do not compile Chronica dependencies into the Ops domain core.

Target:

```text
Ops Core
  |
  +-- LocalPolicyAdapter
  +-- ChronicaAuthorityAdapter

Ops Core
  |
  +-- LocalEventSink
  +-- ChronicaEventBridge
```

The selected adapter changes by deployment mode; the business rule is not rewritten.

## Offline/disconnected behavior

Connected deployments must define what happens when Chronica becomes unreachable.

Possible policy classes:

```text
READ_LOCAL
QUEUE_REQUEST
ALLOW_WITHIN_PREDELEGATED_ENVELOPE
DENY_UNTIL_RECONNECTED
EMERGENCY_LOCAL_POLICY
```

Never silently fall back from Chronica-governed authority to unrestricted local authority.

## Reconnect

Reconnect requires a reconciliation journal containing enough information to determine:

```text
what was observed
what was requested
what was executed
under which authority/delegation
idempotency/correlation keys
what evidence exists
what remains unknown
```

Unknown outcome is a real state. Do not convert transport timeout into success or failure without evidence.

## Mode switch test

A release is not dual-mode proven until the same product version demonstrates:

```text
standalone boot -> workflow -> restart -> recovery
connected boot -> mapped workflow -> authority/evidence
connected -> temporary disconnect -> policy behavior -> reconnect reconciliation
```

## Rule

> **Standalone and connected are deployment modes over one Ops Core, not two products.**
