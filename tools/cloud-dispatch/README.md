# Cloud Dispatch

Two bounded, reusable helpers for delegating fast bounded Chronica tasks to Codex
Cloud. Do not send the full local Claude corpus to a cloud task as one prompt.
Build a redacted workbank first, then delegate one domain slice per task.

This directory used to be `tools/cloud-delegation/`; the 7 wave-9/wave-10
campaign-specific generator/validator scripts that lived alongside these two
were archived under `docs/_archive/cloud-delegation-2026-08-06/` (they hardcode
PR numbers and schema versions for now-closed campaigns and are not reusable).
The 146 prompt files those campaigns submitted are archived in the same place,
under `prompts/`.

## Build the Claude workbank

```powershell
node tools/cloud-dispatch/build-claude-workbank.mjs `
  --source "C:\Users\trngh\.claude\projects\c--Users-trngh-Documents-GitHub-Chronica" `
  --out tools/cloud-dispatch/claude-workbank `
  --max-records 3000 `
  --max-chars 6000
```

Generated files:

- `tools/cloud-dispatch/claude-workbank/manifest.jsonl` - file identity, size, hash, and timestamps.
- `tools/cloud-dispatch/claude-workbank/records.jsonl` - redacted/truncated candidate work records.
- `tools/cloud-dispatch/claude-workbank/source-summary.json` - corpus stats and redaction counters.

## Build the capability cloud snapshot

The full `docs/capabilities.db` stays local-only. Build small tracked shards for cloud agents:

```powershell
node tools/capabilities/export-cloud-snapshot.mjs
node tools/capabilities/verify-cloud-snapshot.mjs
node tools/capabilities/query-cloud-snapshot.mjs summary
```

Generated files live in `docs/capabilities-cloud/`.

## Submit one cloud task

Author a bounded prompt file (see `docs/_archive/cloud-delegation-2026-08-06/prompts/`
for examples of the shape past prompts took), then:

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
`docs/architecture.db`, `docs/capabilities-cloud/*.db`, and verifier output. DB files and shards are
build-time/audit-time tracking surfaces only: use them to choose targets and check claims, never to
implement runtime behavior, authorization, routing, tenancy, or money decisions. `docs/capabilities.db`
is local-only unless explicitly packaged; cloud tasks must mark capability-DB claims as
`LOCAL_AUDIT_REQUIRED` for the local auditor.
