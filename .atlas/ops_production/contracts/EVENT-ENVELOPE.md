# Ops Event Envelope Contract

Ops events must be durable, identifiable, replayable, correlatable, versioned, and safe to reconcile across standalone and Chronica-connected operation.

This contract does not require every internal event to leave the Ops. It standardizes externally meaningful events that cross process, deployment, or Chronica integration boundaries.

## Minimum envelope

```yaml
event_id: <globally-unique-id>
event_type: reservation.updated
schema_version: 1.0.0
ops_id: bnbops
source_id: booking-adapter
source_version: 2026-09
subject:
  type: reservation
  id: R-123
principal:
  id: user-or-service-id
  kind: human|service|agent|external
scope:
  tenant_id: T-1
  holding_id: H-1
occurred_at: 2026-09-13T04:00:00Z
observed_at: 2026-09-13T04:00:02Z
correlation_id: C-123
causation_id: CMD-456
idempotency_key: optional-stable-key
payload: {}
evidence_refs: []
sensitivity: internal
sync:
  sequence: 10021
  cursor: optional-source-cursor
```

## Semantics

- `event_id`: stable identity of this emitted event. Retries reuse it; they do not mint semantic duplicates.
- `event_type`: stable semantic name, not provider endpoint name.
- `schema_version`: version of event payload/envelope interpretation.
- `occurred_at`: when the source event/effect happened when known.
- `observed_at`: when this Ops observed/recorded it.
- `correlation_id`: groups one business/execution flow.
- `causation_id`: identifies the command/event that caused this event when known.
- `idempotency_key`: used where upstream/downstream effect semantics require stable deduplication.
- `sequence/cursor`: synchronization aid, not universal chronological truth.

## Ordering

Never infer total canonical history solely from network arrival order.

Consumers must be prepared for:

```text
duplicate delivery
out-of-order delivery
delayed delivery
missing predecessor during temporary outage
replay after restart
schema evolution
```

## Evidence

Effectful events should link to receipts/evidence when available. An event saying `payment.succeeded` without corresponding provider/execution evidence must not silently become stronger truth than the source supports.

## Security

The envelope carries classification metadata but MUST NOT embed secrets. Payload projection follows purpose/scope/sensitivity rules. Connected mode does not justify copying private local fields into Chronica.

## Authority

Events report or request state transitions; they do not grant authority. Consuming an event does not authorize an effect merely because the event originated from an internal system.

## Reconciliation

Inbox processing MUST deduplicate on stable event identity and track processing outcome. Outbox publishing MUST survive process restart. See `architecture/OFFLINE-RECONCILIATION.md`.
