# Local Verification Ledger

Chronica can accumulate execution evidence without GitHub-hosted CI.

Canonical truth remains the repository/runtime state. The ledger is an ignored local evidence projection at `.chronica/verification-evidence.json`.

Evidence classes remain distinct:

- source evidence: test/check implementation exists;
- local execution evidence: a command actually ran against an exact Git SHA;
- hosted CI evidence: an independent hosted runner executed checks;
- production runtime evidence: deployed runtime health/evidence.

Local execution never masquerades as hosted CI or production proof. Evidence recorded for an older SHA remains historical and does not prove the current HEAD.

Commands:

```bash
node tools/reality-atlas/local-verification.mjs run --check atlas:system:test node --test tools/reality-atlas/system-coverage.test.mjs
node tools/reality-atlas/local-verification.mjs record --check cargo:check --command "cargo check" --status PASS
node tools/reality-atlas/local-verification.mjs show
```

The System Atlas reads this ledger when present and projects its current-SHA records into the Test/Evidence Graph.
