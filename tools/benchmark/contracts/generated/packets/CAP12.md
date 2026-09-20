# CAP12 Identity/sessions/secrets — acceptance packet

GENERATED. Source of truth is Contract IR (chronica.benchmark.contract.v1). Do not hand-edit.

- BENCHMARK_RELEASE: `BR-2026.08.21.1`
- BENCHMARK_SHA: `2f0a95472e72842e7ec70cdac6eeef8297df2b55`
- SOURCE_HASH: `bea492307c825d9bd8fc9b677e0b6028b79cbee119bfb216d7c25d976d82c8f1`
- PACKET_HASH: `efcc044b5a6ba056fe575adffc6a84c1cd70e169f10cd63399a74aeeca201847`
- STANDARDS: {"BLACKBOX":"1.0.0","EMPLOYMENT":"1.0.0"}

## Owns
- credentials
- sessions
- secret lifecycle

## Does not own
- least-disclosure consumption (CAP18)
- employment grants
- WEB2APP catalog

## Hard gates
- SECRET_IN_AGENT_CONTEXT
- SECRET_IN_EMPLOYMENT_CONTRACT

## Owned flows
- `FLOW-CAP12-VERIFY-001` RUNTIME_CONNECTED missing=PO-TENANCY,PO-NO_AI
- `FLOW-CAP12-SESSION-001` SPEC_ONLY missing=PO-REVOCATION,PO-AUTHORITY

## Next actionable
- `FLOW-CAP12-SESSION-001` (SPEC_ONLY)
- `FLOW-CAP12-VERIFY-001` (RUNTIME_CONNECTED)

If the standard appears wrong: emit BENCHMARK_CHANGE_REQUEST. Do not weaken fixtures.
