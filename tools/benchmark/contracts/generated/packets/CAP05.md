# CAP05 CRM — acceptance packet

GENERATED. Source of truth is Contract IR (chronica.benchmark.contract.v1). Do not hand-edit.

- BENCHMARK_RELEASE: `BR-2026.08.21.1`
- BENCHMARK_SHA: `2f0a95472e72842e7ec70cdac6eeef8297df2b55`
- SOURCE_HASH: `bea492307c825d9bd8fc9b677e0b6028b79cbee119bfb216d7c25d976d82c8f1`
- PACKET_HASH: `d8ef734cbf098284e00a9aec275867737668aa9634eef27a0201976654ef95ee`
- STANDARDS: {"BLACKBOX":"1.0.0"}

## Owns
- CRM accounts/contacts/leads/opportunities

## Does not own
- Helpdesk kernel
- Store
- employment
- black-box views

## Hard gates

## Owned flows
- `FLOW-CAP05-LEAD-001` SPEC_ONLY missing=PO-STATE,PO-TENANCY,PO-PERSISTENCE
- `FLOW-CAP05-CUSTOMER-001` SPEC_ONLY missing=PO-TENANCY,PO-CONSUMER
- `FLOW-CROSS-STORE-CRM-001` SPEC_ONLY missing=PO-CONSUMER,PO-TENANCY

## Next actionable
- `FLOW-CAP05-LEAD-001` (SPEC_ONLY)
- `FLOW-CAP05-CUSTOMER-001` (SPEC_ONLY)
- `FLOW-CROSS-STORE-CRM-001` (SPEC_ONLY)

If the standard appears wrong: emit BENCHMARK_CHANGE_REQUEST. Do not weaken fixtures.
