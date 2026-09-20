import test from 'node:test'
import assert from 'node:assert/strict'
import { spawnSync } from 'node:child_process'
import { existsSync, mkdirSync, mkdtempSync, writeFileSync } from 'node:fs'
import { join } from 'node:path'
import { tmpdir } from 'node:os'
import { fileURLToPath } from 'node:url'
import Database from 'better-sqlite3'

const scriptPath = new URL('./audit-truth.mjs', import.meta.url)
const scriptFile = fileURLToPath(scriptPath)

function makeTruthAuditFixture() {
  const root = mkdtempSync(join(tmpdir(), 'chronica-truth-audit-'))
  mkdirSync(join(root, 'docs'), { recursive: true })
  mkdirSync(join(root, 'crates', 'chronica-erp', 'src'), { recursive: true })
  writeFileSync(join(root, 'docs', 'live-architecture.md'), '# live evidence\n')
  writeFileSync(join(root, 'crates', 'chronica-erp', 'src', 'lib.rs'), `
pub fn post_supplier_payment() {}
pub fn same_symbol() {}

#[test]
#[ignore = "tracked verification should not rely on skipped tests"]
fn ignored_money_test() {
    assert!(true);
}

#[test]
fn same_symbol() {
    assert!(true);
}
`)

  const capabilitiesDb = join(root, 'docs', 'capabilities.db')
  const caps = new Database(capabilitiesDb)
  caps.exec(`
    CREATE TABLE canonical_capability (
      id INTEGER PRIMARY KEY,
      key TEXT UNIQUE NOT NULL,
      canonical_name TEXT NOT NULL,
      moves_money INTEGER DEFAULT 0,
      requires_approval INTEGER DEFAULT 0,
      status TEXT DEFAULT 'unimplemented'
    );
    CREATE TABLE canonical_status_override (
      canonical_key TEXT PRIMARY KEY,
      status TEXT,
      moves_money INTEGER,
      requires_approval INTEGER
    );
    CREATE TABLE source_capability (
      id INTEGER PRIMARY KEY,
      donor TEXT NOT NULL,
      canonical_name TEXT NOT NULL,
      source_files TEXT NOT NULL,
      side_effect_class TEXT,
      moves_money INTEGER DEFAULT 0,
      requires_approval INTEGER DEFAULT 0,
      canonical_id INTEGER
    );
    CREATE TABLE provenance (
      source_id INTEGER NOT NULL,
      canonical_id INTEGER NOT NULL,
      PRIMARY KEY (source_id, canonical_id)
    );
    CREATE TABLE impl_evidence (
      canonical_key TEXT PRIMARY KEY,
      impl_file TEXT,
      impl_symbols TEXT,
      test_file TEXT,
      test_symbol TEXT
    );
  `)
  caps.prepare(`INSERT INTO canonical_capability
    (id, key, canonical_name, moves_money, requires_approval, status)
    VALUES (?, ?, ?, ?, ?, ?)`)
    .run(1, 'erp.pay_supplier', 'Pay supplier', 0, 0, 'verified')
  caps.prepare(`INSERT INTO canonical_capability
    (id, key, canonical_name, moves_money, requires_approval, status)
    VALUES (?, ?, ?, ?, ?, ?)`)
    .run(2, 'runtime.test_as_impl', 'Test-as-implementation sample', 0, 0, 'verified')
  caps.prepare(`INSERT INTO canonical_capability
    (id, key, canonical_name, moves_money, requires_approval, status)
    VALUES (?, ?, ?, ?, ?, ?)`)
    .run(3, 'erp.override_money', 'Already overridden money cap', 0, 0, 'unimplemented')
  caps.prepare(`INSERT INTO canonical_status_override
    (canonical_key, moves_money, requires_approval) VALUES (?, ?, ?)`)
    .run('erp.override_money', 1, 1)
  caps.prepare(`INSERT INTO source_capability
    (id, donor, canonical_name, source_files, side_effect_class, moves_money, requires_approval, canonical_id)
    VALUES (?, ?, ?, ?, ?, ?, ?, ?)`)
    .run(10, 'erpnext-main', 'submit supplier payment', 'accounts/payment_entry.py', 'money', 1, 1, 1)
  caps.prepare(`INSERT INTO provenance (source_id, canonical_id) VALUES (?, ?)`).run(10, 1)
  caps.prepare(`INSERT INTO source_capability
    (id, donor, canonical_name, source_files, side_effect_class, moves_money, requires_approval, canonical_id)
    VALUES (?, ?, ?, ?, ?, ?, ?, ?)`)
    .run(11, 'erpnext-main', 'already overridden supplier payment', 'accounts/payment_entry.py', 'money', 1, 1, 3)
  caps.prepare(`INSERT INTO provenance (source_id, canonical_id) VALUES (?, ?)`).run(11, 3)
  caps.prepare(`INSERT INTO impl_evidence
    (canonical_key, impl_file, impl_symbols, test_file, test_symbol)
    VALUES (?, ?, ?, ?, ?)`)
    .run('erp.pay_supplier', 'crates/chronica-erp/src/lib.rs', 'post_supplier_payment', 'crates/chronica-erp/src/lib.rs', 'ignored_money_test')
  caps.prepare(`INSERT INTO impl_evidence
    (canonical_key, impl_file, impl_symbols, test_file, test_symbol)
    VALUES (?, ?, ?, ?, ?)`)
    .run('runtime.test_as_impl', 'crates/chronica-erp/src/lib.rs', 'same_symbol', 'crates/chronica-erp/src/lib.rs', 'same_symbol')
  caps.close()

  const architectureDb = join(root, 'docs', 'architecture.db')
  const arch = new Database(architectureDb)
  arch.exec(`
    CREATE TABLE architecture_node (
      id TEXT PRIMARY KEY,
      kind TEXT NOT NULL,
      name TEXT NOT NULL,
      evidence_ref TEXT
    );
    CREATE TABLE architecture_edge (
      id INTEGER PRIMARY KEY AUTOINCREMENT,
      from_node TEXT NOT NULL,
      to_node TEXT NOT NULL,
      kind TEXT NOT NULL,
      evidence_ref TEXT
    );
    CREATE TABLE architecture_evidence (
      id INTEGER PRIMARY KEY AUTOINCREMENT,
      architecture_node TEXT NOT NULL,
      status TEXT NOT NULL,
      evidence_kind TEXT NOT NULL,
      evidence_ref TEXT NOT NULL
    );
  `)
  arch.prepare(`INSERT INTO architecture_node
    (id, kind, name, evidence_ref) VALUES (?, ?, ?, ?)`)
    .run('gate:money', 'gate', 'Money gate', 'docs/live-architecture.md')
  arch.prepare(`INSERT INTO architecture_edge
    (from_node, to_node, kind, evidence_ref) VALUES (?, ?, ?, ?)`)
    .run('gate:money', 'audit:chain', 'writes', 'docs/live-architecture.md')
  arch.prepare(`INSERT INTO architecture_evidence
    (architecture_node, status, evidence_kind, evidence_ref) VALUES (?, ?, ?, ?)`)
    .run('gate:money', 'verified', 'repo_file', 'docs/stale-architecture.md')
  arch.close()

  return { root, capabilitiesDb, architectureDb }
}

test('audit-truth reports business and evidence contradictions as JSON and can fail on findings', () => {
  const { root, capabilitiesDb, architectureDb } = makeTruthAuditFixture()
  assert.equal(existsSync(scriptFile), true)

  const result = spawnSync(process.execPath, [
    scriptFile,
    '--root', root,
    '--capabilities-db', capabilitiesDb,
    '--architecture-db', architectureDb,
    '--fail-on-findings',
  ], { encoding: 'utf8' })

  assert.equal(result.status, 1, result.stderr)
  assert.equal(result.stderr, '')

  const report = JSON.parse(result.stdout)
  assert.equal(report.ok, false)
  assert.equal(report.counts.totalFindings, 4)
  assert.deepEqual(report.counts.byType, {
    architecture_missing_file_ref: 1,
    ignored_verified_test: 1,
    source_money_underclaimed: 1,
    test_as_implementation: 1,
  })
  assert.equal(report.findings.source_money_underclaimed[0].canonicalKey, 'erp.pay_supplier')
  assert.equal(report.findings.source_money_underclaimed[0].canonicalRequiresApproval, 0)
  assert.equal(report.findings.ignored_verified_test[0].testSymbol, 'ignored_money_test')
  assert.equal(report.findings.test_as_implementation[0].canonicalKey, 'runtime.test_as_impl')
  assert.equal(report.findings.architecture_missing_file_ref[0].evidenceRef, 'docs/stale-architecture.md')
})
