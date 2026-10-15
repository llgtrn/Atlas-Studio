# Local Verification Ledger

Chronica can accumulate execution evidence without GitHub-hosted CI.

Chronica canonical World truth is governed by its admitted history and reconciliation rules; repository state, command success and local ledger entries are distinct evidence classes. This source tool's ignored `.chronica/verification-evidence.json` is a local verification projection, not another canonical history or authority owner.

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
