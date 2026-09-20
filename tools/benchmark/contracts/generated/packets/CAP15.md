# CAP15 WEB2APP — acceptance packet

GENERATED. Source of truth is Contract IR (chronica.benchmark.contract.v1). Do not hand-edit.

- BENCHMARK_RELEASE: `BR-2026.08.21.1`
- BENCHMARK_SHA: `2f0a95472e72842e7ec70cdac6eeef8297df2b55`
- SOURCE_HASH: `bea492307c825d9bd8fc9b677e0b6028b79cbee119bfb216d7c25d976d82c8f1`
- PACKET_HASH: `80285647f00f5472d64599bd9f42292d00fb0517f509f667ca9d497783e9034d`
- STANDARDS: {"WEB2APP":"1.1.0"}

## Owns
- typed external app operations
- observation methods
- WEB2APP compiler contract

## Does not own
- inbound ingestion V2
- credential storage
- employment
- Store estate

## Hard gates
- UNTYPED_DO_WHATEVER
- CREDENTIAL_IN_AGENT_CONTEXT

## Owned flows
- `FLOW-CAP15-NEWS-001` SPEC_ONLY missing=PO-AUTHORITY,PO-DISCLOSURE,PO-ENTRYPOINT
- `FLOW-CAP15-TYPED-001` SPEC_ONLY missing=PO-AUTHORITY,PO-NO_AI

## Next actionable
- `FLOW-CAP15-NEWS-001` (SPEC_ONLY)
- `FLOW-CAP15-TYPED-001` (SPEC_ONLY)

If the standard appears wrong: emit BENCHMARK_CHANGE_REQUEST. Do not weaken fixtures.
