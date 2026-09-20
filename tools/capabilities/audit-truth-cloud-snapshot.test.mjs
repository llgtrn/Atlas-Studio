import test from 'node:test'
import assert from 'node:assert/strict'
import { mkdirSync, mkdtempSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join } from 'node:path'
import Database from 'better-sqlite3'
import { auditSourceMoneyUnderclaimsCloudSnapshot } from './audit-truth-cloud-snapshot.mjs'

function makeSnapshotFixture() {
  const dir = mkdtempSync(join(tmpdir(), 'chronica-audit-truth-cloud-'))

  const core = new Database(join(dir, 'cap-core.db'))
  core.exec(`
    CREATE TABLE canonical_capability (
      id INTEGER PRIMARY KEY, key TEXT UNIQUE NOT NULL, canonical_name TEXT,
      domain TEXT, target_crate TEXT, moves_money INTEGER, requires_approval INTEGER, status TEXT
    );
    CREATE TABLE canonical_status_override (
      canonical_key TEXT PRIMARY KEY, moves_money INTEGER, requires_approval INTEGER
    );
  `)
  core
    .prepare('INSERT INTO canonical_capability (id, key, canonical_name, domain, target_crate, moves_money, requires_approval, status) VALUES (?,?,?,?,?,?,?,?)')
    .run(1, 'workflow-runtime.durable_submissions', 'Agent Submissions & Durable Execution Store', 'workflow-runtime', 'chronica-workflow-runtime', 0, 0, 'unimplemented')
  core
    .prepare('INSERT INTO canonical_capability (id, key, canonical_name, domain, target_crate, moves_money, requires_approval, status) VALUES (?,?,?,?,?,?,?,?)')
    .run(2, 'already.correct', 'Already Correctly Classified', 'infra', 'chronica-infra', 1, 1, 'verified')
  core
    .prepare('INSERT INTO canonical_capability (id, key, canonical_name, domain, target_crate, moves_money, requires_approval, status) VALUES (?,?,?,?,?,?,?,?)')
    .run(3, 'override.corrected', 'Corrected Via Override', 'infra', 'chronica-infra', 0, 0, 'unimplemented')
  core.prepare('INSERT INTO canonical_status_override (canonical_key, moves_money, requires_approval) VALUES (?,?,?)').run('override.corrected', 1, 1)
  core.close()

  const provenance = new Database(join(dir, 'cap-provenance.db'))
  provenance.exec(`
    CREATE TABLE source_capability (
      id INTEGER PRIMARY KEY, donor TEXT, canonical_name TEXT, source_files TEXT,
      moves_money INTEGER, requires_approval INTEGER, side_effect_class TEXT, canonical_id INTEGER
    );
    CREATE TABLE provenance (source_id INTEGER, canonical_id INTEGER);
  `)
  // Direct join path: source_capability.canonical_id -> underclaimed (finding expected)
  provenance
    .prepare('INSERT INTO source_capability (id, donor, canonical_name, source_files, moves_money, requires_approval, side_effect_class, canonical_id) VALUES (?,?,?,?,?,?,?,?)')
    .run(101, 'ComfyUI-master', 'External Video-Gen API Nodes', 'comfy_api_nodes/nodes_kling.py', 1, 1, 'money', 1)
  // Already correctly classified canonical row -> no finding
  provenance
    .prepare('INSERT INTO source_capability (id, donor, canonical_name, source_files, moves_money, requires_approval, side_effect_class, canonical_id) VALUES (?,?,?,?,?,?,?,?)')
    .run(102, 'donor-x', 'Already Money Source', 'src/pay.ts', 1, 1, 'money', 2)
  // Override raises canonical moves_money to 1 -> no finding despite base row being 0
  provenance
    .prepare('INSERT INTO source_capability (id, donor, canonical_name, source_files, moves_money, requires_approval, side_effect_class, canonical_id) VALUES (?,?,?,?,?,?,?,?)')
    .run(103, 'donor-y', 'Overridden Source', 'src/billing.ts', 1, 1, 'money', 3)
  // Provenance-only join path (no canonical_id on the source row itself) -> finding expected
  provenance
    .prepare('INSERT INTO source_capability (id, donor, canonical_name, source_files, moves_money, requires_approval, side_effect_class, canonical_id) VALUES (?,?,?,?,?,?,?,?)')
    .run(104, 'donor-z', 'Provenance-Linked Source', 'src/checkout.ts', 1, 1, 'money', null)
  provenance.prepare('INSERT INTO provenance (source_id, canonical_id) VALUES (?,?)').run(104, 1)
  provenance.close()

  return dir
}

test('auditSourceMoneyUnderclaimsCloudSnapshot flags an underclaimed canonical row via the direct join path', () => {
  const dir = makeSnapshotFixture()
  const report = auditSourceMoneyUnderclaimsCloudSnapshot({ dir })
  assert.equal(report.truth_label, 'LOCAL_AUDIT_REQUIRED')
  const keys = report.findings.map((f) => f.canonicalKey)
  assert.ok(keys.includes('workflow-runtime.durable_submissions'))
})

test('auditSourceMoneyUnderclaimsCloudSnapshot dedupes the direct-join and provenance-join paths for the same source/canonical pair', () => {
  const dir = makeSnapshotFixture()
  const report = auditSourceMoneyUnderclaimsCloudSnapshot({ dir })
  const matches = report.findings.filter((f) => f.canonicalKey === 'workflow-runtime.durable_submissions')
  // source 101 (direct) and source 104 (provenance) are two DISTINCT sources pointing at the same
  // canonical row -- both must appear (dedup is per source+canonical pair, not per canonical key).
  assert.equal(matches.length, 2)
  assert.deepEqual(matches.map((f) => f.sourceId).sort(), [101, 104])
})

test('auditSourceMoneyUnderclaimsCloudSnapshot does not flag a canonical row that is already correctly classified', () => {
  const dir = makeSnapshotFixture()
  const report = auditSourceMoneyUnderclaimsCloudSnapshot({ dir })
  const keys = report.findings.map((f) => f.canonicalKey)
  assert.ok(!keys.includes('already.correct'))
})

test('auditSourceMoneyUnderclaimsCloudSnapshot honors canonical_status_override and does not flag a corrected row', () => {
  const dir = makeSnapshotFixture()
  const report = auditSourceMoneyUnderclaimsCloudSnapshot({ dir })
  const keys = report.findings.map((f) => f.canonicalKey)
  assert.ok(!keys.includes('override.corrected'))
})

test('auditSourceMoneyUnderclaimsCloudSnapshot reports ok:true and count:0 for a clean snapshot', () => {
  const dir = mkdtempSync(join(tmpdir(), 'chronica-audit-truth-cloud-clean-'))
  const core = new Database(join(dir, 'cap-core.db'))
  core.exec(`
    CREATE TABLE canonical_capability (id INTEGER PRIMARY KEY, key TEXT UNIQUE, canonical_name TEXT, domain TEXT, target_crate TEXT, moves_money INTEGER, requires_approval INTEGER, status TEXT);
    CREATE TABLE canonical_status_override (canonical_key TEXT PRIMARY KEY, moves_money INTEGER, requires_approval INTEGER);
  `)
  core.close()
  const provenance = new Database(join(dir, 'cap-provenance.db'))
  provenance.exec(`
    CREATE TABLE source_capability (id INTEGER PRIMARY KEY, donor TEXT, canonical_name TEXT, source_files TEXT, moves_money INTEGER, requires_approval INTEGER, side_effect_class TEXT, canonical_id INTEGER);
    CREATE TABLE provenance (source_id INTEGER, canonical_id INTEGER);
  `)
  provenance.close()

  const report = auditSourceMoneyUnderclaimsCloudSnapshot({ dir })
  assert.equal(report.ok, true)
  assert.equal(report.count, 0)
  assert.deepEqual(report.findings, [])
})
