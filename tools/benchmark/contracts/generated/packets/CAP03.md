# CAP03 Backup/restore — acceptance packet

GENERATED. Source of truth is Contract IR (chronica.benchmark.contract.v1). Do not hand-edit.

- BENCHMARK_RELEASE: `BR-2026.08.21.1`
- BENCHMARK_SHA: `2f0a95472e72842e7ec70cdac6eeef8297df2b55`
- SOURCE_HASH: `bea492307c825d9bd8fc9b677e0b6028b79cbee119bfb216d7c25d976d82c8f1`
- PACKET_HASH: `5232800c55d9ec718aedf889ddaea285c2eb072a17cea93f7bdc2ad122147da4`
- STANDARDS: {"BLACKBOX":"1.0.0"}

## Owns
- backup execution
- restore orchestration

## Does not own
- backup confidentiality policy (CAP18)
- Vault keys

## Hard gates
- PLAINTEXT_BACKUP_BYPASS

## Owned flows
- `FLOW-CAP03-BACKUP-001` SPEC_ONLY missing=PO-DURABILITY,PO-ROLLBACK,PO-DISCLOSURE

## Next actionable
- `FLOW-CAP03-BACKUP-001` (SPEC_ONLY)

If the standard appears wrong: emit BENCHMARK_CHANGE_REQUEST. Do not weaken fixtures.
