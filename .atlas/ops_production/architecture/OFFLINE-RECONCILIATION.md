---
id: ops-production-architecture-offline-reconciliation
type: reference
status: active
canonical: true
---
# Offline, Reconnect, and Reconciliation

Standalone capability and Chronica connectivity must survive network failure, process restarts, duplicate provider delivery, and partial execution. Steady-state connectivity is not enough for production.

## Connection states

An Ops should expose an explicit state such as:

```text
STANDALONE
CONNECTED
DEGRADED
OFFLINE
RECONCILING
CONFLICTED
```

Do not hide a disconnected Chronica bridge behind a generic green health indicator.

## Durable pattern

```text
local transaction
-> durable local state
-> outbox event/evidence
-> publish/synchronize
-> remote acknowledgement/cursor
```

Inbound integration uses an inbox/deduplication ledger before applying a shared semantic event.

## Required metadata

Use the event/command contracts to preserve:

```text
event_id
source_id/source_version
occurred_at
observed_at
correlation_id
causation_id
idempotency_key
local sequence
remote acknowledgement/sync cursor
processing result
```

## Disconnected behavior

Classify actions before implementation:

### Local-safe standalone action

May continue while Chronica is absent if local policy/authority permits and the action is genuinely local to the independent product deployment.

### Chronica-governed connected action

MUST NOT silently downgrade to unrestricted local authority when Chronica authority is unavailable.

The Ops may:

```text
queue a request
return authority_unavailable
remain read-only
enter degraded mode
require explicit operator policy for a pre-authorized bounded envelope
```

but it must not pretend authorization occurred.

## Provider events during outage

External provider truth may continue arriving while Chronica is disconnected. The Ops records it durably, emits local event/evidence, and later reconciles it.

Example:

```text
Booking provider cancellation
-> Ops durable event
-> local UI reflects provider fact with sync status
-> reconnect
-> deduplicate/map
-> Chronica admission/reconciliation
-> acknowledgement cursor advances
```

## Reconciliation algorithm

At reconnect:

```text
1. establish compatible integration version
2. load last acknowledged cursor/checkpoint
3. replay unsynchronized outbox in stable order where ordering is meaningful
4. deduplicate by stable event/effect identity
5. compare expected/shared versions
6. classify conflicts
7. reconcile or require human/policy resolution
8. persist evidence/result
9. advance acknowledgement only after durable success
```

## Conflict classes

At minimum distinguish:

```text
DUPLICATE
STALE_VERSION
CONCURRENT_UPDATE
REMOTE_ALREADY_APPLIED
UNKNOWN_OUTCOME
AUTHORITY_EXPIRED
SEMANTIC_MAPPING_CHANGED
MANUAL_REVIEW_REQUIRED
```

## Unknown outcome

A timeout after sending an effect is not failure. Before retry, query/reconcile provider/execution state using correlation/idempotency identifiers. Never double-charge, double-cancel, or duplicate a machine command because transport acknowledgement was lost.

## Proof

Production acceptance includes forced disconnect/restart/reconnect tests with duplicate and out-of-order events, partial execution, and unknown outcomes. See `100-PERCENT-INTEGRATION-CHECKLIST.md`.
