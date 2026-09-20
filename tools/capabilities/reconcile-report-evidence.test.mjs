import test from 'node:test'
import assert from 'node:assert/strict'
import { spawnSync } from 'node:child_process'
import { mkdirSync, mkdtempSync, writeFileSync } from 'node:fs'
import { join } from 'node:path'
import { tmpdir } from 'node:os'
import { fileURLToPath } from 'node:url'
import Database from 'better-sqlite3'

const scriptFile = fileURLToPath(new URL('./reconcile-report-evidence.mjs', import.meta.url))

function makeFixtureRoot() {
  const root = mkdtempSync(join(tmpdir(), 'chronica-report-evidence-'))
  mkdirSync(join(root, 'docs', '_machine', 'reconcile-reports'), { recursive: true })
  mkdirSync(join(root, 'crates', 'chronica-demo', 'src'), { recursive: true })

  const db = new Database(join(root, 'docs', 'capabilities.db'))
  db.exec(`
    CREATE TABLE canonical_capability (
      id INTEGER PRIMARY KEY,
      key TEXT UNIQUE NOT NULL,
      canonical_name TEXT,
      target_crate TEXT,
      target_module TEXT,
      moves_money INTEGER DEFAULT 0,
      status TEXT DEFAULT 'unimplemented',
      acceptance_test TEXT,
      financial_control_test TEXT
    );
    CREATE TABLE canonical_status_override (
      canonical_key TEXT PRIMARY KEY,
      status TEXT,
      acceptance_test TEXT,
      financial_control_test TEXT,
      moves_money INTEGER,
      set_at TEXT,
      set_by TEXT
    );
    CREATE TABLE impl_evidence (
      canonical_key TEXT PRIMARY KEY,
      impl_file TEXT,
      impl_symbols TEXT,
      test_file TEXT,
      test_symbol TEXT,
      donor_source TEXT,
      verified_at TEXT,
      verified_by TEXT,
      notes TEXT
    );
  `)
  const ins = db.prepare(`
    INSERT INTO canonical_capability
      (key, canonical_name, target_crate, target_module, moves_money, status)
    VALUES (?, ?, 'chronica-demo', 'demo', 0, 'unimplemented')
  `)
  ins.run('demo.full', 'Full demo capability')
  ins.run('demo.partial', 'Partial demo capability')
  ins.run('demo.missing', 'Missing-test demo capability')
  ins.run('demo.unstructured', 'Unstructured demo capability')
  db.close()

  writeFileSync(join(root, 'crates', 'chronica-demo', 'src', 'demo.rs'), `
pub fn full_flow() {}
pub fn unstructured_flow() {}

#[cfg(test)]
mod tests {
  #[test]
  fn full_flow_works() {}

  #[test]
  fn partial_flow_works() {}

  #[test]
  fn unstructured_flow_works() {}
}
`)

  writeFileSync(join(root, 'docs', '_machine', 'reconcile-reports', 'chronica-demo.json'), JSON.stringify({
    crate: 'chronica-demo',
    judgment: 'feature_added',
    doc_action: 'fixture',
    tracking_changes: [
      {
        db: 'capabilities',
        what: 'demo.full implemented',
        to: 'demo.full implemented with proving test full_flow_works',
        evidence: {
          impl_file: 'crates/chronica-demo/src/demo.rs',
          impl_symbols: ['full_flow'],
          test_file: 'crates/chronica-demo/src/demo.rs',
          test_symbol: 'full_flow_works',
        },
      },
      {
        db: 'capabilities',
        what: 'demo.partial partially implemented',
        to: 'demo.partial partial implementation with proving test partial_flow_works',
        evidence: {
          impl_file: 'crates/chronica-demo/src/demo.rs',
          impl_symbols: ['full_flow'],
          test_file: 'crates/chronica-demo/src/demo.rs',
          test_symbol: 'partial_flow_works',
        },
      },
      {
        db: 'capabilities',
        what: 'demo.missing implemented',
        to: 'demo.missing implemented with proving test missing_flow_works',
        evidence: {
          impl_file: 'crates/chronica-demo/src/demo.rs',
          impl_symbols: ['full_flow'],
          test_file: 'crates/chronica-demo/src/demo.rs',
          test_symbol: 'missing_flow_works',
        },
      },
      {
        db: 'capabilities',
        what: 'demo.unstructured implemented',
        to: 'demo.unstructured implemented with existing test unstructured_flow_works but no structured impl symbols',
      },
    ],
    rationale_pointer: 'docs/999-crate-chronica-demo.md#rationale',
  }, null, 2))

  return root
}

test('imports only non-partial report claims with real Rust test evidence', () => {
  const root = makeFixtureRoot()
  const result = spawnSync(process.execPath, [scriptFile], {
    cwd: root,
    encoding: 'utf8',
    env: { ...process.env, CHRONICA_RECONCILE_NOW: '2026-07-09T00:00:00.000Z' },
  })

  assert.equal(result.status, 0, result.stderr || result.stdout)
  assert.match(result.stdout, /verified 1 report-backed cap/)

  const db = new Database(join(root, 'docs', 'capabilities.db'), { readonly: true })
  assert.equal(db.prepare("SELECT status FROM canonical_capability WHERE key='demo.full'").get().status, 'verified')
  assert.equal(db.prepare("SELECT acceptance_test FROM canonical_capability WHERE key='demo.full'").get().acceptance_test, 'full_flow_works')
  assert.equal(db.prepare("SELECT status FROM canonical_capability WHERE key='demo.partial'").get().status, 'unimplemented')
  assert.equal(db.prepare("SELECT status FROM canonical_capability WHERE key='demo.missing'").get().status, 'unimplemented')
  assert.equal(db.prepare("SELECT status FROM canonical_capability WHERE key='demo.unstructured'").get().status, 'implemented_unverified')
  const evidence = db.prepare("SELECT impl_symbols, test_symbol FROM impl_evidence WHERE canonical_key='demo.full'").get()
  assert.equal(evidence.impl_symbols, 'full_flow')
  assert.equal(evidence.test_symbol, 'full_flow_works')
  assert.equal(db.prepare("SELECT COUNT(*) n FROM impl_evidence WHERE canonical_key='demo.unstructured'").get().n, 0)
  assert.equal(db.prepare('SELECT COUNT(*) n FROM canonical_status_override').get().n, 2)
  db.close()
})
