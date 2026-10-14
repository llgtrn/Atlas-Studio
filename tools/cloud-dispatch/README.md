# Cloud Dispatch


**Current checkout tooling, not Ynventa architecture authority (2026-10-14).** These cloud-dispatch helpers are historical/source-local implementation utilities. The one approved development-control and permission system is the central .Ynventa- Work/Lease/Result protocol, with a pinned read/write scope and independent verification. A source helper may prepare a redacted candidate payload only after a valid Work and explicit approved disclosure scope; it cannot create another permanent coordinator, choose its own source-of-truth or apply commits outside the common main-only gates. The canonical 21-name registry is in Chronica; agent utilities are not Norl, Codara or Ontogen by their names. The procedures below describe surviving source commands, not an alternate development workflow.



Two bounded, reusable helpers for delegating fast bounded Chronica tasks to Codex
Cloud. Do not send the full local Claude corpus to a cloud task as one prompt.
Do not export private conversations or a local agent transcript collection by default. Under an explicitly approved Work lease and disclosure scope, build a minimal redacted candidate workbank; otherwise do not dispatch it. Source accessibility does not grant publication permission.

**Obsolete campaign scripts and their prompt files are not in this tracked checkout.** Do not reference an untracked `docs/_archive/cloud-delegation-2026-08-06/` tree as an available local source. The only supported commands below are those backed by existing files at the pinned checkout revision.

## Build the Claude workbank

```powershell
node tools/cloud-dispatch/build-claude-workbank.mjs `
  --source "<OWNER_APPROVED_LOCAL_SOURCE_DIRECTORY>" `
  --out tools/cloud-dispatch/claude-workbank `
  --max-records 3000 `
  --max-chars 6000
```

Generated files:

- `tools/cloud-dispatch/claude-workbank/manifest.jsonl` - file identity, size, hash, and timestamps.
- `tools/cloud-dispatch/claude-workbank/records.jsonl` - redacted/truncated candidate work records.
- `tools/cloud-dispatch/claude-workbank/source-summary.json` - corpus stats and redaction counters.

## Build the capability cloud snapshot

The former `docs/capabilities.db` is **not present in the tracked checkout**; treat it as optional local state, never a canonical authority. These source scripts exist under `tools/capabilities/` and may produce a bounded snapshot only if their actual runtime prerequisites and disclosure rules are satisfied:

```powershell
node tools/capabilities/export-cloud-snapshot.mjs
node tools/capabilities/verify-cloud-snapshot.mjs
node tools/capabilities/query-cloud-snapshot.mjs summary
```

When successfully generated in a local workspace, output may reside under `docs/capabilities-cloud/`; it is not pre-existing tracked evidence and must not be presumed available on a clean checkout.

## Submit one cloud task

Author a bounded prompt file under the current shared Work/Lease/Result authority, after verifying the source, destination, permission scope and disclosure policy. **No archived prompt directory exists in this tracked checkout.** The command below is a current source-local helper only; it cannot authorize external dispatch on its own:

```powershell
.\tools\cloud-dispatch\submit-codex-cloud.ps1 `
  -EnvId "env_xxxxx" `
  -Branch "main" `
  -PromptFile ".\path\to\your-prompt.md"
```

Follow up:

```powershell
codex cloud list --env env_xxxxx
codex cloud status <TASK_ID>
codex cloud diff <TASK_ID>
codex cloud apply <TASK_ID>
```

Cloud agents must use `tools/cloud-dispatch/claude-workbank/records.jsonl` as candidate leads only. In cloud,
the readable audit inputs are repo code, tests, tracked docs, generated docs,
*actually materialized and checked* local/generated architecture and capability snapshots, and verifier output. DB files and shards are
build-time/audit-time tracking surfaces only: use them to choose targets and check claims, never to
implement runtime behavior, authorization, routing, tenancy, or money decisions. Any local capability database not explicitly and securely packaged is **unavailable to a cloud task**. Treat all such claims as `LOCAL_AUDIT_REQUIRED`; never infer a canonical capability, authority grant or confidential fact from a missing artifact.
