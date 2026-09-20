import test from 'node:test'
import assert from 'node:assert/strict'
import { existsSync, mkdirSync, mkdtempSync, readFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { dirname, join, resolve } from 'node:path'
import { fileURLToPath } from 'node:url'
import { spawnSync } from 'node:child_process'
import Database from 'better-sqlite3'

const __dirname = dirname(fileURLToPath(import.meta.url))
const script = resolve(__dirname, 'recover-canonical.mjs')

function makeRoot() {
  const root = mkdtempSync(join(tmpdir(), 'chronica-recover-canonical-'))
  mkdirSync(join(root, 'docs', 'capabilities-cloud'), { recursive: true })
  return root
}

function makeCloudCore(root, rows) {
  const path = join(root, 'docs', 'capabilities-cloud', 'cap-core.db')
  const db = new Database(path)
  db.exec(`CREATE TABLE cloud_snapshot_info (key TEXT PRIMARY KEY, value TEXT);
  INSERT INTO cloud_snapshot_info VALUES ('generated_at', 'test-generation');
  CREATE TABLE canonical_capability (
    id INTEGER PRIMARY KEY, key TEXT UNIQUE NOT NULL, canonical_name TEXT NOT NULL, domain TEXT,
    target_crate TEXT, target_module TEXT, side_effect_class TEXT, moves_money INTEGER DEFAULT 0,
    requires_approval INTEGER DEFAULT 0, acceptance_criteria TEXT, required_tests TEXT,
    financial_control_test TEXT, acceptance_test TEXT, status TEXT DEFAULT 'unimplemented',
    exclusion_note TEXT, blocker TEXT, slice INTEGER, donor_count INTEGER DEFAULT 0)`)
  const ins = db.prepare(`INSERT INTO canonical_capability
    (id, key, canonical_name, domain, target_crate, target_module, moves_money, requires_approval, status)
    VALUES (@id, @key, @canonical_name, @domain, @target_crate, @target_module, @moves_money, @requires_approval, @status)`)
  for (const r of rows) ins.run({ ...r, id: r.id ?? null })
  db.close()
  return path
}

function makeCloudProvenance(root, rows) {
  const path = join(root, 'docs', 'capabilities-cloud', 'cap-provenance.db')
  const db = new Database(path)
  db.exec(`
    CREATE TABLE cloud_snapshot_info (key TEXT PRIMARY KEY, value TEXT);
    INSERT INTO cloud_snapshot_info VALUES ('generated_at', 'test-generation');
    CREATE TABLE source_capability (id INTEGER PRIMARY KEY, canonical_id INTEGER);
    CREATE TABLE provenance (source_id INTEGER, canonical_id INTEGER, PRIMARY KEY (source_id, canonical_id));
  `)
  const sourceInsert = db.prepare('INSERT INTO source_capability (id, canonical_id) VALUES (?, ?)')
  const provenanceInsert = db.prepare('INSERT INTO provenance (source_id, canonical_id) VALUES (?, ?)')
  for (const row of rows) {
    sourceInsert.run(row.source_id, row.canonical_id)
    provenanceInsert.run(row.source_id, row.canonical_id)
  }
  db.close()
  return path
}

function makeLiveDb(root, { withSideTables = true } = {}) {
  const path = join(root, 'docs', 'capabilities.db')
  const db = new Database(path)
  db.exec(`
    CREATE TABLE meta (k TEXT PRIMARY KEY, v TEXT);
    CREATE TABLE canonical_status_override (canonical_key TEXT PRIMARY KEY, status TEXT, acceptance_test TEXT,
      financial_control_test TEXT, moves_money INTEGER, requires_approval INTEGER, blocker TEXT, set_at TEXT, set_by TEXT);
    CREATE TABLE impl_evidence (canonical_key TEXT PRIMARY KEY, impl_file TEXT, impl_symbols TEXT, test_file TEXT, test_symbol TEXT, donor_source TEXT, verified_at TEXT, verified_by TEXT, notes TEXT);
    CREATE TABLE slice_canonical (slice INTEGER, canonical_key TEXT, match_method TEXT, confidence TEXT, reviewed INTEGER DEFAULT 0, note TEXT, PRIMARY KEY (slice, canonical_key));
    CREATE TABLE agent_note (id INTEGER PRIMARY KEY AUTOINCREMENT, author TEXT, kind TEXT, status TEXT, capability_key TEXT, donor TEXT, body TEXT, created_at TEXT, resolved_by TEXT, resolved_at TEXT);
    CREATE TABLE donor_file_census (donor TEXT, path TEXT, is_directory INTEGER, kind TEXT, classification TEXT, read_status TEXT, mapped_source_ids TEXT, exclusion_reason TEXT, size_bytes INTEGER, mtime_ms INTEGER, sha256 TEXT, reviewed_by TEXT, reviewed_at TEXT, PRIMARY KEY (donor, path));
    CREATE TABLE source_capability (id INTEGER PRIMARY KEY, donor TEXT, canonical_name TEXT, source_files TEXT, canonical_id INTEGER);
    CREATE TABLE provenance (source_id INTEGER, canonical_id INTEGER, PRIMARY KEY (source_id, canonical_id));
    CREATE TABLE donor_absorption (donor TEXT PRIMARY KEY, disposition TEXT, status TEXT);
  `)
  if (withSideTables) {
    db.prepare(`INSERT INTO canonical_status_override (canonical_key, status, set_at, set_by) VALUES ('demo.one', 'implemented_unverified', '2026-01-01', 'docs:sync')`).run()
    db.prepare(`INSERT INTO impl_evidence (canonical_key, impl_file, test_file, test_symbol) VALUES ('demo.one', 'crates/demo/src/lib.rs', 'crates/demo/src/lib.rs', 'demo_test')`).run()
    db.prepare(`INSERT INTO slice_canonical (slice, canonical_key, match_method, confidence) VALUES (7, 'demo.one', 'manifest', 'high')`).run()
    db.prepare(`INSERT INTO agent_note (author, kind, status, body, created_at) VALUES ('claude', 'handoff', 'resolved', 'pre-existing note', '2026-01-01')`).run()
    db.prepare(`INSERT INTO donor_file_census (donor, path, kind, classification, read_status) VALUES ('demo-donor', 'src/x.py', 'source', 'mapped', 'reviewed')`).run()
  }
  db.close()
  return path
}

function run(args) {
  return spawnSync(process.execPath, [script, ...args], { encoding: 'utf8' })
}

test('dry-run reports the recovery plan and does not mutate the live db', () => {
  const root = makeRoot()
  makeCloudCore(root, [{ key: 'demo.one', canonical_name: 'Demo One', domain: 'demo', target_crate: 'chronica-demo', target_module: null, moves_money: 0, requires_approval: 0, status: 'unimplemented' }])
  const dbPath = makeLiveDb(root)
  const before = readFileSync(dbPath)

  const result = run(['--db', dbPath])
  assert.equal(result.status, 0, result.stderr)
  const report = JSON.parse(result.stdout)
  assert.equal(report.status, 'DRY_RUN')
  assert.equal(report.candidate_insert_count, 1)
  assert.equal(report.destructive_changes.length, 0)

  const after = readFileSync(dbPath)
  assert.deepEqual(before, after, 'dry-run must not touch the live db file bytes')
})

test('recovers canonical_capability from cap-core.db and preserves every survivable side table', () => {
  const root = makeRoot()
  makeCloudCore(root, [
    { key: 'demo.one', canonical_name: 'Demo One', domain: 'demo', target_crate: 'chronica-demo', target_module: null, moves_money: 0, requires_approval: 0, status: 'unimplemented' },
    { key: 'demo.two', canonical_name: 'Demo Two', domain: 'demo', target_crate: 'chronica-demo', target_module: null, moves_money: 1, requires_approval: 1, status: 'unimplemented' },
  ])
  const dbPath = makeLiveDb(root)

  const result = run(['--db', dbPath, '--apply'])
  assert.equal(result.status, 0, result.stderr)
  const report = JSON.parse(result.stdout)
  assert.equal(report.status, 'RECOVERED')
  assert.equal(report.inserted, 2)
  assert.equal(report.post_row_count, 2)

  const db = new Database(dbPath, { readonly: true })
  const rows = db.prepare('SELECT key, status, moves_money FROM canonical_capability ORDER BY key').all()
  assert.deepEqual(rows.map((r) => r.key), ['demo.one', 'demo.two'])
  // the survivable override for demo.one must be re-projected onto the newly recovered row.
  assert.equal(db.prepare("SELECT status FROM canonical_capability WHERE key='demo.one'").get().status, 'implemented_unverified')
  assert.equal(db.prepare("SELECT slice FROM canonical_capability WHERE key='demo.one'").get().slice, 7)

  // every OTHER side table must be byte-for-byte untouched in content.
  assert.equal(db.prepare('SELECT count(*) n FROM impl_evidence').get().n, 1)
  assert.equal(db.prepare('SELECT count(*) n FROM slice_canonical').get().n, 1)
  assert.equal(db.prepare("SELECT count(*) n FROM agent_note WHERE body='pre-existing note'").get().n, 1)
  assert.equal(db.prepare('SELECT count(*) n FROM donor_file_census').get().n, 1)
  db.close()
})

test('preserves non-contiguous canonical IDs so source and provenance mappings retain their key identity', () => {
  const root = makeRoot()
  makeCloudCore(root, [
    { id: 10, key: 'demo.one', canonical_name: 'Demo One', domain: 'demo', target_crate: 'chronica-demo', target_module: null, moves_money: 0, requires_approval: 0, status: 'unimplemented' },
    { id: 42, key: 'demo.two', canonical_name: 'Demo Two', domain: 'demo', target_crate: 'chronica-demo', target_module: null, moves_money: 0, requires_approval: 0, status: 'unimplemented' },
  ])
  makeCloudProvenance(root, [{ source_id: 7, canonical_id: 42 }])
  const dbPath = makeLiveDb(root)
  const before = new Database(dbPath)
  before.prepare("INSERT INTO source_capability (id, donor, canonical_name, source_files, canonical_id) VALUES (7, 'donor', 'Source Two', 'x.rs', 42)").run()
  before.prepare('INSERT INTO provenance (source_id, canonical_id) VALUES (7, 42)').run()
  before.close()

  const result = run(['--db', dbPath, '--apply'])
  assert.equal(result.status, 0, result.stderr)
  assert.equal(JSON.parse(result.stdout).status, 'RECOVERED')

  const after = new Database(dbPath, { readonly: true })
  assert.deepEqual(after.prepare('SELECT id, key FROM canonical_capability ORDER BY id').all(), [
    { id: 10, key: 'demo.one' },
    { id: 42, key: 'demo.two' },
  ])
  assert.equal(after.prepare(`
    SELECT c.key FROM source_capability s JOIN canonical_capability c ON c.id=s.canonical_id WHERE s.id=7
  `).get().key, 'demo.two')
  after.close()
})

test('fails closed when a surviving source/provenance reference is absent from the core snapshot', () => {
  const root = makeRoot()
  makeCloudCore(root, [{ id: 10, key: 'demo.one', canonical_name: 'Demo One', domain: 'demo', target_crate: 'chronica-demo', target_module: null, moves_money: 0, requires_approval: 0, status: 'unimplemented' }])
  makeCloudProvenance(root, [{ source_id: 7, canonical_id: 99 }])
  const dbPath = makeLiveDb(root)
  const db = new Database(dbPath)
  db.prepare("INSERT INTO source_capability (id, donor, canonical_name, source_files, canonical_id) VALUES (7, 'donor', 'Source', 'x.rs', 99)").run()
  db.prepare('INSERT INTO provenance (source_id, canonical_id) VALUES (7, 99)').run()
  db.close()
  const before = readFileSync(dbPath)

  const result = run(['--db', dbPath, '--apply'])
  const report = JSON.parse(result.stdout)
  assert.equal(report.status, 'BLOCKED_WITH_EVIDENCE')
  assert.equal(report.reference_issues[0].type, 'DANGLING_CANONICAL_REFERENCE')
  assert.deepEqual(readFileSync(dbPath), before)
})

test('rejects mixed-generation core/provenance shards before recovery', () => {
  const root = makeRoot()
  makeCloudCore(root, [{ id: 10, key: 'demo.one', canonical_name: 'Demo One', domain: 'demo', target_crate: 'chronica-demo', target_module: null, moves_money: 0, requires_approval: 0, status: 'unimplemented' }])
  const provenancePath = makeCloudProvenance(root, [{ source_id: 7, canonical_id: 10 }])
  const provenance = new Database(provenancePath)
  provenance.prepare("UPDATE cloud_snapshot_info SET value='different-generation' WHERE key='generated_at'").run()
  provenance.close()
  const dbPath = makeLiveDb(root)
  const local = new Database(dbPath)
  local.prepare("INSERT INTO source_capability (id, donor, canonical_name, source_files, canonical_id) VALUES (7, 'donor', 'Source', 'x.rs', 10)").run()
  local.prepare('INSERT INTO provenance (source_id, canonical_id) VALUES (7, 10)').run()
  local.close()

  const result = run(['--db', dbPath, '--apply'])
  const report = JSON.parse(result.stdout)
  assert.equal(report.status, 'BLOCKED_WITH_EVIDENCE')
  assert.equal(report.source_issues[0].type, 'MIXED_OR_UNKNOWN_SNAPSHOT_GENERATION')
})

test('rejects a core shard whose canonical IDs were synthesized from generated docs', () => {
  const root = makeRoot()
  const corePath = makeCloudCore(root, [{ id: 10, key: 'demo.one', canonical_name: 'Demo One', domain: 'demo', target_crate: 'chronica-demo', target_module: null, moves_money: 0, requires_approval: 0, status: 'unimplemented' }])
  const core = new Database(corePath)
  core.exec('CREATE TABLE meta (k TEXT PRIMARY KEY, v TEXT)')
  core.prepare("INSERT INTO meta (k, v) VALUES ('canonical_recovery', 'recovered from generated docs')").run()
  core.close()
  const dbPath = makeLiveDb(root)
  const before = readFileSync(dbPath)

  const result = run(['--db', dbPath, '--apply'])
  const report = JSON.parse(result.stdout)
  assert.equal(report.status, 'BLOCKED_WITH_EVIDENCE')
  assert.equal(report.source_issues[0].type, 'UNTRUSTED_RECOVERED_CORE_IDS')
  assert.deepEqual(readFileSync(dbPath), before)
})

test('idempotent rerun: a second --apply against an already-recovered db is a safe no-op', () => {
  const root = makeRoot()
  makeCloudCore(root, [{ key: 'demo.one', canonical_name: 'Demo One', domain: 'demo', target_crate: 'chronica-demo', target_module: null, moves_money: 0, requires_approval: 0, status: 'unimplemented' }])
  const dbPath = makeLiveDb(root)

  const first = run(['--db', dbPath, '--apply'])
  assert.equal(first.status, 0, first.stderr)
  const afterFirst = readFileSync(dbPath)

  const second = run(['--db', dbPath, '--apply'])
  assert.equal(second.status, 0, second.stderr)
  const secondReport = JSON.parse(second.stdout)
  assert.equal(secondReport.status, 'ALREADY_RECOVERED_NOOP')

  const afterSecond = readFileSync(dbPath)
  assert.deepEqual(afterFirst, afterSecond, 'a no-op rerun must not change the live db file at all')

  // dry-run against an already-populated db is the same safe no-op.
  const dryRerun = run(['--db', dbPath])
  assert.equal(JSON.parse(dryRerun.stdout).status, 'ALREADY_RECOVERED_NOOP')
})

test('refuses and leaves the db untouched when canonical_capability already has rows (not a recovery scenario)', () => {
  const root = makeRoot()
  const dbPath = join(root, 'docs', 'capabilities.db')
  const db = new Database(dbPath)
  db.exec(`CREATE TABLE canonical_capability (key TEXT UNIQUE NOT NULL, canonical_name TEXT, status TEXT)`)
  db.prepare("INSERT INTO canonical_capability (key, canonical_name, status) VALUES ('already.here', 'Already Here', 'unimplemented')").run()
  db.close()
  const before = readFileSync(dbPath)

  const result = run(['--db', dbPath, '--apply'])
  assert.equal(JSON.parse(result.stdout).status, 'ALREADY_RECOVERED_NOOP')
  assert.deepEqual(readFileSync(dbPath), before)
})

test('fails closed on a malformed cloud-core snapshot (missing required columns) and never touches the live db', () => {
  const root = makeRoot()
  const corePath = join(root, 'docs', 'capabilities-cloud', 'cap-core.db')
  const core = new Database(corePath)
  core.exec('CREATE TABLE canonical_capability (key TEXT UNIQUE NOT NULL, canonical_name TEXT)') // missing status/moves_money/etc.
  core.prepare("INSERT INTO canonical_capability (key, canonical_name) VALUES ('x.y', 'X Y')").run()
  core.close()
  const dbPath = makeLiveDb(root)
  const before = readFileSync(dbPath)

  const result = run(['--db', dbPath, '--apply'])
  const report = JSON.parse(result.stdout)
  assert.equal(report.status, 'BLOCKED_WITH_EVIDENCE')
  assert.equal(report.source_issues[0].type, 'MALFORMED_SNAPSHOT_MISSING_COLUMNS')
  assert.deepEqual(readFileSync(dbPath), before, 'the live db must be byte-identical after a blocked recovery')
})

test('fails closed on duplicate keys in the cloud-core snapshot and never touches the live db', () => {
  const root = makeRoot()
  const corePath = join(root, 'docs', 'capabilities-cloud', 'cap-core.db')
  const core = new Database(corePath)
  // deliberately no UNIQUE constraint, to simulate a corrupted/hand-crafted snapshot with duplicate keys.
  core.exec(`CREATE TABLE canonical_capability (id INTEGER, key TEXT NOT NULL, canonical_name TEXT, domain TEXT,
    target_crate TEXT, target_module TEXT, moves_money INTEGER DEFAULT 0, requires_approval INTEGER DEFAULT 0, status TEXT)`)
  core.prepare("INSERT INTO canonical_capability (key, canonical_name, status) VALUES ('dup.key', 'First', 'unimplemented')").run()
  core.prepare("INSERT INTO canonical_capability (key, canonical_name, status) VALUES ('dup.key', 'Second (corrupt)', 'unimplemented')").run()
  core.close()
  const dbPath = makeLiveDb(root)
  const before = readFileSync(dbPath)

  const result = run(['--db', dbPath, '--apply'])
  const report = JSON.parse(result.stdout)
  assert.equal(report.status, 'BLOCKED_WITH_EVIDENCE')
  assert.equal(report.source_issues[0].type, 'MALFORMED_SNAPSHOT_DUPLICATE_KEY')
  assert.deepEqual(readFileSync(dbPath), before)
})

test('an interrupted rebuild (injected mid-transaction failure) restores the original db byte-for-byte', () => {
  const root = makeRoot()
  makeCloudCore(root, [{ key: 'demo.one', canonical_name: 'Demo One', domain: 'demo', target_crate: 'chronica-demo', target_module: null, moves_money: 0, requires_approval: 0, status: 'unimplemented' }])
  const dbPath = makeLiveDb(root)
  const before = readFileSync(dbPath)

  const result = spawnSync(process.execPath, [script, '--db', dbPath, '--apply'], {
    encoding: 'utf8',
    env: { ...process.env, RECOVER_CANONICAL_INJECT_FAILURE_AFTER_INSERT: '1' },
  })
  const report = JSON.parse(result.stdout)
  assert.equal(report.status, 'FAILED_ROLLED_BACK')
  assert.equal(report.original_db_restored, true)

  const after = readFileSync(dbPath)
  assert.deepEqual(before, after, 'an interrupted rebuild must leave the db byte-identical to its pre-apply state')

  // and canonical_capability must still be absent/empty, not half-written.
  const db = new Database(dbPath, { readonly: true })
  const hasTable = db.prepare("SELECT count(*) n FROM sqlite_master WHERE type='table' AND name='canonical_capability'").get().n > 0
  assert.equal(hasTable, false)
  db.close()
})

test('recovers when canonical_capability exists but is empty (0 rows), the exact issue #713 trap', () => {
  const root = makeRoot()
  makeCloudCore(root, [{ key: 'demo.one', canonical_name: 'Demo One', domain: 'demo', target_crate: 'chronica-demo', target_module: null, moves_money: 0, requires_approval: 0, status: 'unimplemented' }])
  const dbPath = makeLiveDb(root)
  const db = new Database(dbPath)
  db.exec(`CREATE TABLE canonical_capability (id INTEGER PRIMARY KEY, key TEXT UNIQUE NOT NULL, canonical_name TEXT NOT NULL, domain TEXT,
    target_crate TEXT, target_module TEXT, side_effect_class TEXT, moves_money INTEGER DEFAULT 0, requires_approval INTEGER DEFAULT 0,
    acceptance_criteria TEXT, required_tests TEXT, financial_control_test TEXT, acceptance_test TEXT, status TEXT DEFAULT 'unimplemented',
    exclusion_note TEXT, blocker TEXT, slice INTEGER, donor_count INTEGER DEFAULT 0)`) // present, but 0 rows
  db.close()

  const dry = run(['--db', dbPath])
  const dryReport = JSON.parse(dry.stdout)
  assert.equal(dryReport.status, 'DRY_RUN')
  assert.equal(dryReport.pre_state.canonical_capability_table_exists, true)
  assert.equal(dryReport.pre_state.canonical_capability_rows, 0)

  const applied = run(['--db', dbPath, '--apply'])
  assert.equal(JSON.parse(applied.stdout).status, 'RECOVERED')
})

test('refuses when the target db file does not exist (never creates a db from nothing)', () => {
  const root = makeRoot()
  const dbPath = join(root, 'docs', 'capabilities.db')
  const result = run(['--db', dbPath, '--apply'])
  assert.equal(JSON.parse(result.stdout).status, 'BLOCKED_WITH_EVIDENCE')
  assert.equal(existsSync(dbPath), false)
})
