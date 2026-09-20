# CAP13 Inbound webhooks — acceptance packet

GENERATED. Source of truth is Contract IR (chronica.benchmark.contract.v1). Do not hand-edit.

- BENCHMARK_RELEASE: `BR-2026.08.21.1`
- BENCHMARK_SHA: `2f0a95472e72842e7ec70cdac6eeef8297df2b55`
- SOURCE_HASH: `bea492307c825d9bd8fc9b677e0b6028b79cbee119bfb216d7c25d976d82c8f1`
- PACKET_HASH: `975b944ad8ebd311b8e370771306b6bb2171a38948aeb06e209362c8e1f6a722`
- STANDARDS: {"WEB2APP":"1.1.0"}

## Owns
- inbound authenticate
- dedupe
- canonical inbound event

## Does not own
- WEB2APP method catalog
- generic scheduler

## Hard gates
- WEBHOOK_WITHOUT_AUTH

## Owned flows
- `FLOW-CAP13-INGEST-001` RUNTIME_CONNECTED missing=PO-IDEMPOTENCY,PO-FAILURE,PO-EVIDENCE
- `FLOW-CROSS-INBOUND-NOTIFY-001` SPEC_ONLY missing=PO-CONSUMER,PO-IDEMPOTENCY

## Next actionable
- `FLOW-CROSS-INBOUND-NOTIFY-001` (SPEC_ONLY)
- `FLOW-CAP13-INGEST-001` (RUNTIME_CONNECTED)

If the standard appears wrong: emit BENCHMARK_CHANGE_REQUEST. Do not weaken fixtures.
