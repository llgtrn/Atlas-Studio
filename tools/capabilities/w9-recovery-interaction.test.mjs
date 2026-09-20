// w9-recovery-interaction.test.mjs — end-to-end regression for the exact ordering trap the
// independent reviewer found in PR #714 (issue #713): an operator who bypasses
// census-schema.mjs's guard with `--allow-empty-canonical` and then runs the standard
// `pnpm caps:rebuild` walks straight into reconcile-w9-target-keys.mjs, which (before this fix)
// was not row-count-aware and would insert its four rows into the empty shell — permanently
// tripping recover-canonical.mjs's `ALREADY_RECOVERED_NOOP` guard and orphaning the surviving
// side-table data with no tool left able to finish the recovery.
//
// This test drives the REAL scripts (not mocks) through that exact sequence and asserts the fix
// holds at every step:
//   1. allow-empty bootstrap over a db whose canonical_capability is lost but side tables survived
//   2. reconcile-w9-target-keys.mjs must REFUSE (fail closed), leaving canonical_capability empty
//   3. recover-canonical.mjs --apply must still be able to RECOVER (not poisoned)
//   4. reconcile-w9-target-keys.mjs must now succeed against the recovered, non-empty table
import test from 'node:test'
import assert from 'node:assert/strict'
import { mkdirSync, mkdtempSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { dirname, join, resolve } from 'node:path'
import { fileURLToPath } from 'node:url'
import { spawnSync } from 'node:child_process'
import Database from 'better-sqlite3'

const __dirname = dirname(fileURLToPath(import.meta.url))
const censusSchemaScript = resolve(__dirname, 'census-schema.mjs')
const reconcileScript = resolve(__dirname, 'reconcile-w9-target-keys.mjs')
const recoverScript = resolve(__dirname, 'recover-canonical.mjs')

function makeRoot() {
  const root = mkdtempSync(join(tmpdir(), 'chronica-w9-recovery-interaction-'))
  mkdirSync(join(root, 'docs', 'capabilities-cloud'), { recursive: true })
  return root
}

// Simulates the real issue #713 loss: canonical_capability is gone entirely, but the survivable
// side tables from prior real census/verification work are still present in the same db file.
function seedLostCanonicalDb(dbPath) {
  const db = new Database(dbPath)
  db.exec(`
    CREATE TABLE canonical_status_override (canonical_key TEXT PRIMARY KEY, status TEXT, acceptance_test TEXT,
      financial_control_test TEXT, moves_money INTEGER, requires_approval INTEGER, blocker TEXT, set_at TEXT, set_by TEXT);
    CREATE TABLE impl_evidence (canonical_key TEXT PRIMARY KEY, impl_file TEXT, impl_symbols TEXT, test_file TEXT, test_symbol TEXT, donor_source TEXT, verified_at TEXT, verified_by TEXT, notes TEXT);
    CREATE TABLE slice_canonical (slice INTEGER, canonical_key TEXT, match_method TEXT, confidence TEXT, reviewed INTEGER DEFAULT 0, note TEXT, PRIMARY KEY (slice, canonical_key));
    CREATE TABLE agent_note (id INTEGER PRIMARY KEY AUTOINCREMENT, author TEXT, kind TEXT, status TEXT, capability_key TEXT, donor TEXT, body TEXT, created_at TEXT, resolved_by TEXT, resolved_at TEXT);
  `)
  db.prepare(`INSERT INTO canonical_status_override (canonical_key, status, set_at, set_by) VALUES ('demo.one', 'implemented_unverified', '2026-01-01', 'docs:sync')`).run()
  db.prepare(`INSERT INTO impl_evidence (canonical_key, impl_file, test_file, test_symbol) VALUES ('demo.one', 'crates/demo/src/lib.rs', 'crates/demo/src/lib.rs', 'demo_test')`).run()
  db.prepare(`INSERT INTO slice_canonical (slice, canonical_key, match_method, confidence) VALUES (7, 'demo.one', 'manifest', 'high')`).run()
  db.close()
}

function makeCloudCore(root) {
  const path = join(root, 'docs', 'capabilities-cloud', 'cap-core.db')
  const db = new Database(path)
  db.exec(`CREATE TABLE canonical_capability (
    id INTEGER PRIMARY KEY, key TEXT UNIQUE NOT NULL, canonical_name TEXT NOT NULL, domain TEXT,
    target_crate TEXT, target_module TEXT, side_effect_class TEXT, moves_money INTEGER DEFAULT 0,
    requires_approval INTEGER DEFAULT 0, acceptance_criteria TEXT, required_tests TEXT,
    financial_control_test TEXT, acceptance_test TEXT, status TEXT DEFAULT 'unimplemented',
    exclusion_note TEXT, blocker TEXT, slice INTEGER, donor_count INTEGER DEFAULT 0)`)
  const ins = db.prepare(`INSERT INTO canonical_capability
    (key, canonical_name, domain, target_crate, target_module, moves_money, requires_approval, status)
    VALUES (@key, @canonical_name, @domain, @target_crate, @target_module, @moves_money, @requires_approval, @status)`)
  ins.run({ key: 'demo.one', canonical_name: 'Demo One', domain: 'demo', target_crate: 'chronica-demo', target_module: null, moves_money: 0, requires_approval: 0, status: 'unimplemented' })
  // seed the two pre-existing keys decision-feed.actionability (map_existing) resolves onto, so the
  // post-recovery reconcile run exercises the full success path, not just the insert_unverified rows.
  ins.run({ key: 'policy.access_approval_workflow', canonical_name: 'Access approval policies & requests (JIT access)', domain: 'policy', target_crate: 'chronica-approvals', target_module: null, moves_money: 0, requires_approval: 1, status: 'unimplemented' })
  ins.run({ key: 'policy.secret_approval_workflow', canonical_name: 'Secret approval policies & requests', domain: 'policy', target_crate: 'chronica-approvals', target_module: null, moves_money: 0, requires_approval: 1, status: 'unimplemented' })
  db.close()
  return path
}

test('allow-empty bootstrap -> rebuild-order reconcile -> recovery: reconcile refuses on the empty shell, recovery still succeeds afterward, then reconcile succeeds', () => {
  const root = makeRoot()
  const dbPath = join(root, 'docs', 'capabilities.db')
  seedLostCanonicalDb(dbPath)
  makeCloudCore(root)

  // step 1: an operator bypasses census-schema.mjs's guard (the sanctioned, documented escape
  // hatch) against a db that is, unbeknownst to them, in the issue #713 trap state.
  const schemaResult = spawnSync(process.execPath, [censusSchemaScript, '--allow-empty-canonical'], { cwd: root, encoding: 'utf8' })
  assert.equal(schemaResult.status, 0, schemaResult.stderr)
  const afterSchema = new Database(dbPath, { readonly: true })
  assert.equal(afterSchema.prepare('SELECT count(*) n FROM canonical_capability').get().n, 0, 'census-schema.mjs must leave canonical_capability empty (not fabricate rows)')
  assert.equal(afterSchema.prepare('SELECT count(*) n FROM canonical_status_override').get().n, 1, 'side-table evidence must still be present going into the trap')
  afterSchema.close()

  // step 2: standard rebuild ordering runs reconcile-w9-target-keys.mjs next (as rebuild-pipeline.mjs
  // wires it, immediately after reapply-overrides.mjs). It must now REFUSE instead of inserting into
  // the empty-but-present canonical_capability table.
  const reconcileBlocked = spawnSync(process.execPath, [reconcileScript, '--db', dbPath], { encoding: 'utf8' })
  assert.equal(reconcileBlocked.status, 1, reconcileBlocked.stdout)
  assert.match(reconcileBlocked.stderr, /CANONICAL_CAPABILITY_EMPTY_WITH_SURVIVING_SIDE_TABLES/)
  const afterBlockedReconcile = new Database(dbPath, { readonly: true })
  assert.equal(afterBlockedReconcile.prepare('SELECT count(*) n FROM canonical_capability').get().n, 0, 'a refused reconcile must not have inserted any W9 rows')
  afterBlockedReconcile.close()

  // step 3: recover-canonical.mjs must NOT have been poisoned by step 2 — it can still recover.
  const recovered = spawnSync(process.execPath, [recoverScript, '--db', dbPath, '--apply'], { encoding: 'utf8' })
  const recoveredReport = JSON.parse(recovered.stdout)
  assert.equal(recoveredReport.status, 'RECOVERED', recovered.stderr)
  assert.equal(recoveredReport.inserted, 3)
  const afterRecovery = new Database(dbPath, { readonly: true })
  assert.equal(afterRecovery.prepare('SELECT count(*) n FROM canonical_capability').get().n, 3)
  // the side-table evidence that motivated the guard in step 2 must have survived untouched.
  assert.equal(afterRecovery.prepare('SELECT count(*) n FROM impl_evidence').get().n, 1)
  assert.equal(afterRecovery.prepare('SELECT count(*) n FROM slice_canonical').get().n, 1)
  afterRecovery.close()

  // step 4: re-running reconcile-w9-target-keys.mjs now that canonical_capability is genuinely
  // non-empty must succeed and land the four W9 rows plus the decision-feed.actionability mapping.
  const reconcileAfterRecovery = spawnSync(process.execPath, [reconcileScript, '--db', dbPath], { encoding: 'utf8' })
  assert.equal(reconcileAfterRecovery.status, 0, reconcileAfterRecovery.stderr)
  const finalReport = JSON.parse(reconcileAfterRecovery.stdout)
  assert.equal(finalReport.inserted_unverified.length, 4)
  assert.equal(finalReport.mapped_to_existing.length, 1)
  assert.equal(finalReport.mapped_to_existing[0].issue_key, 'decision-feed.actionability')

  const final = new Database(dbPath, { readonly: true })
  assert.equal(final.prepare('SELECT count(*) n FROM canonical_capability').get().n, 7, '3 recovered + 4 newly reconciled W9 rows')
  assert.equal(final.prepare("SELECT count(*) n FROM canonical_capability WHERE status='verified'").get().n, 0, 'no row in this whole sequence may ever be verified')
  assert.equal(final.prepare("SELECT status FROM canonical_capability WHERE key='demo.one'").get().status, 'implemented_unverified', 'the re-projected override from the original side-table data must survive the whole sequence')
  final.close()
})
