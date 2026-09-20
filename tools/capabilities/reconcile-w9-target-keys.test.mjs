import test from 'node:test'
import assert from 'node:assert/strict'
import { mkdtempSync, readFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { dirname, join, resolve } from 'node:path'
import { fileURLToPath } from 'node:url'
import { spawnSync } from 'node:child_process'
import Database from 'better-sqlite3'

const __dirname = dirname(fileURLToPath(import.meta.url))
const script = resolve(__dirname, 'reconcile-w9-target-keys.mjs')
const seed = JSON.parse(readFileSync(resolve(__dirname, 'w9-target-reconciliation.json'), 'utf8'))

function makeDb() {
  const root = mkdtempSync(join(tmpdir(), 'chronica-w9-reconcile-'))
  const dbPath = join(root, 'capabilities.db')
  const db = new Database(dbPath)
  db.exec(`
    CREATE TABLE canonical_capability (id INTEGER PRIMARY KEY, key TEXT UNIQUE NOT NULL, canonical_name TEXT NOT NULL,
      domain TEXT, target_crate TEXT, target_module TEXT, side_effect_class TEXT, moves_money INTEGER DEFAULT 0,
      requires_approval INTEGER DEFAULT 0, acceptance_criteria TEXT, required_tests TEXT, financial_control_test TEXT,
      acceptance_test TEXT, status TEXT DEFAULT 'unimplemented', exclusion_note TEXT, blocker TEXT, slice INTEGER, donor_count INTEGER DEFAULT 0);
    CREATE TABLE agent_note (id INTEGER PRIMARY KEY AUTOINCREMENT, author TEXT, kind TEXT, status TEXT,
      capability_key TEXT, donor TEXT, body TEXT, created_at TEXT, resolved_by TEXT, resolved_at TEXT);
  `)
  // seed the two pre-existing keys decision-feed.actionability is expected to map onto.
  const ins = db.prepare(`INSERT INTO canonical_capability (key, canonical_name, domain, target_crate, status)
    VALUES (@key, @name, 'policy', 'chronica-approvals', 'unimplemented')`)
  ins.run({ key: 'policy.access_approval_workflow', name: 'Access approval policies & requests (JIT access)' })
  ins.run({ key: 'policy.secret_approval_workflow', name: 'Secret approval policies & requests' })
  db.close()
  return dbPath
}

function run(dbPath) {
  return spawnSync(process.execPath, [script, '--db', dbPath], { encoding: 'utf8' })
}

test('inserts the four new W9 keys as implemented_unverified and maps decision-feed.actionability without minting a new row', () => {
  const dbPath = makeDb()
  const result = run(dbPath)
  assert.equal(result.status, 0, result.stderr)
  const report = JSON.parse(result.stdout)

  assert.deepEqual(
    report.inserted_unverified.sort(),
    ['business-event.dead_letter_redrive', 'business-map.store', 'commerce.product_catalog_persistence', 'helpdesk.sla_escalation_sweep'].sort(),
  )
  assert.equal(report.mapped_to_existing.length, 1)
  assert.equal(report.mapped_to_existing[0].issue_key, 'decision-feed.actionability')
  assert.equal(report.agent_notes_posted, 1)

  const db = new Database(dbPath, { readonly: true })
  const total = db.prepare('SELECT count(*) n FROM canonical_capability').get().n
  assert.equal(total, 6, '2 pre-seeded + 4 newly inserted; decision-feed.actionability mints nothing')

  for (const key of ['commerce.product_catalog_persistence', 'business-event.dead_letter_redrive', 'business-map.store', 'helpdesk.sla_escalation_sweep']) {
    const row = db.prepare('SELECT status, moves_money, acceptance_criteria FROM canonical_capability WHERE key=?').get(key)
    assert.ok(row, `${key} must be inserted`)
    assert.equal(row.status, 'implemented_unverified')
    assert.notEqual(row.status, 'verified')
    assert.equal(row.moves_money, 0)
    assert.ok(row.acceptance_criteria && row.acceptance_criteria.length > 20)
  }

  // decision-feed.actionability must not have minted a canonical row of its own.
  assert.equal(db.prepare("SELECT count(*) n FROM canonical_capability WHERE key='decision-feed.actionability'").get().n, 0)
  // the two mapped-onto rows must be left exactly as they were (still unimplemented).
  assert.equal(db.prepare("SELECT status FROM canonical_capability WHERE key='policy.access_approval_workflow'").get().status, 'unimplemented')
  assert.equal(db.prepare("SELECT status FROM canonical_capability WHERE key='policy.secret_approval_workflow'").get().status, 'unimplemented')

  const note = db.prepare("SELECT body FROM agent_note WHERE kind='handoff'").get()
  assert.ok(note.body.includes('decision-feed.actionability'))
  assert.ok(note.body.includes('policy.access_approval_workflow'))
  db.close()
})

test('never writes status=verified for any of the five W9 keys, by construction', () => {
  const seedStatuses = seed.targets
    .filter((t) => t.action === 'insert_unverified')
    .map((t) => t.status)
  // the seed file itself carries no status field (the script hardcodes implemented_unverified);
  // this asserts the seed never smuggles one in.
  assert.ok(seedStatuses.every((s) => s === undefined))
})

test('idempotent rerun inserts nothing new and does not duplicate the agent_note', () => {
  const dbPath = makeDb()
  const first = run(dbPath)
  assert.equal(first.status, 0, first.stderr)

  const second = run(dbPath)
  assert.equal(second.status, 0, second.stderr)
  const report = JSON.parse(second.stdout)
  assert.deepEqual(report.inserted_unverified, [])
  assert.equal(report.already_present.length, 4)
  assert.equal(report.agent_notes_posted, 0, 'the mapping note must not be re-posted on rerun')

  const db = new Database(dbPath, { readonly: true })
  assert.equal(db.prepare('SELECT count(*) n FROM canonical_capability').get().n, 6)
  assert.equal(db.prepare('SELECT count(*) n FROM agent_note').get().n, 1)
  db.close()
})

test('refuses (exit 1) instead of inserting when canonical_capability is empty-but-present while side tables carry real prior work', () => {
  // reproduces the exact issue #713 trap: canonical_capability exists (an empty shell, e.g. from
  // census-schema.mjs's `CREATE TABLE IF NOT EXISTS` under its --allow-empty-canonical escape
  // hatch) but canonical_status_override/impl_evidence/slice_canonical still carry real prior work.
  // Inserting here would push the row count above zero and permanently trip recover-canonical.mjs's
  // ALREADY_RECOVERED_NOOP guard, orphaning the surviving side-table data.
  const root = mkdtempSync(join(tmpdir(), 'chronica-w9-reconcile-empty-trap-'))
  const dbPath = join(root, 'capabilities.db')
  const db = new Database(dbPath)
  db.exec(`
    CREATE TABLE canonical_capability (id INTEGER PRIMARY KEY, key TEXT UNIQUE NOT NULL, canonical_name TEXT NOT NULL,
      domain TEXT, target_crate TEXT, target_module TEXT, side_effect_class TEXT, moves_money INTEGER DEFAULT 0,
      requires_approval INTEGER DEFAULT 0, acceptance_criteria TEXT, required_tests TEXT, financial_control_test TEXT,
      acceptance_test TEXT, status TEXT DEFAULT 'unimplemented', exclusion_note TEXT, blocker TEXT, slice INTEGER, donor_count INTEGER DEFAULT 0);
    CREATE TABLE canonical_status_override (canonical_key TEXT PRIMARY KEY, status TEXT);
    CREATE TABLE agent_note (id INTEGER PRIMARY KEY AUTOINCREMENT, author TEXT, kind TEXT, status TEXT,
      capability_key TEXT, donor TEXT, body TEXT, created_at TEXT, resolved_by TEXT, resolved_at TEXT);
  `)
  db.prepare("INSERT INTO canonical_status_override (canonical_key, status) VALUES ('demo.one', 'implemented_unverified')").run()
  db.close()

  const result = run(dbPath)
  assert.equal(result.status, 1)
  assert.match(result.stderr, /CANONICAL_CAPABILITY_EMPTY_WITH_SURVIVING_SIDE_TABLES/)

  const after = new Database(dbPath, { readonly: true })
  assert.equal(after.prepare('SELECT count(*) n FROM canonical_capability').get().n, 0, 'the guard must refuse before inserting any W9 row')
  assert.equal(after.prepare('SELECT count(*) n FROM agent_note').get().n, 0, 'no mapping note may be posted either, when the guard refuses')
  after.close()
})

test('proceeds normally on a genuinely fresh bootstrap: canonical_capability empty AND no side-table evidence', () => {
  // the flip side of the trap test above: an empty canonical_capability with no surviving
  // side-table rows is a real fresh bootstrap (e.g. CI's docs-tracking.yml from-scratch run), and
  // must be unaffected by the new guard.
  const root = mkdtempSync(join(tmpdir(), 'chronica-w9-reconcile-fresh-bootstrap-'))
  const dbPath = join(root, 'capabilities.db')
  const db = new Database(dbPath)
  db.exec(`
    CREATE TABLE canonical_capability (id INTEGER PRIMARY KEY, key TEXT UNIQUE NOT NULL, canonical_name TEXT NOT NULL,
      domain TEXT, target_crate TEXT, target_module TEXT, side_effect_class TEXT, moves_money INTEGER DEFAULT 0,
      requires_approval INTEGER DEFAULT 0, acceptance_criteria TEXT, required_tests TEXT, financial_control_test TEXT,
      acceptance_test TEXT, status TEXT DEFAULT 'unimplemented', exclusion_note TEXT, blocker TEXT, slice INTEGER, donor_count INTEGER DEFAULT 0);
    CREATE TABLE agent_note (id INTEGER PRIMARY KEY AUTOINCREMENT, author TEXT, kind TEXT, status TEXT,
      capability_key TEXT, donor TEXT, body TEXT, created_at TEXT, resolved_by TEXT, resolved_at TEXT);
  `)
  db.close() // no canonical_status_override/impl_evidence/slice_canonical tables at all

  const result = run(dbPath)
  assert.equal(result.status, 0, result.stderr)
  const report = JSON.parse(result.stdout)
  assert.equal(report.inserted_unverified.length, 4)
})

test('reports an unresolved mapping instead of failing when a map_existing target key is absent', () => {
  const root = mkdtempSync(join(tmpdir(), 'chronica-w9-reconcile-unresolved-'))
  const dbPath = join(root, 'capabilities.db')
  const db = new Database(dbPath)
  db.exec(`CREATE TABLE canonical_capability (id INTEGER PRIMARY KEY, key TEXT UNIQUE NOT NULL, canonical_name TEXT NOT NULL,
    domain TEXT, target_crate TEXT, target_module TEXT, side_effect_class TEXT, moves_money INTEGER DEFAULT 0,
    requires_approval INTEGER DEFAULT 0, acceptance_criteria TEXT, required_tests TEXT, financial_control_test TEXT,
    acceptance_test TEXT, status TEXT DEFAULT 'unimplemented', exclusion_note TEXT, blocker TEXT, slice INTEGER, donor_count INTEGER DEFAULT 0);
    CREATE TABLE agent_note (id INTEGER PRIMARY KEY AUTOINCREMENT, author TEXT, kind TEXT, status TEXT, capability_key TEXT, donor TEXT, body TEXT, created_at TEXT, resolved_by TEXT, resolved_at TEXT);`)
  db.close() // no policy.* rows seeded — decision-feed.actionability's mapping cannot resolve

  const result = run(dbPath)
  assert.equal(result.status, 0, result.stderr)
  const report = JSON.parse(result.stdout)
  assert.equal(report.unresolved_mappings.length, 1)
  assert.equal(report.unresolved_mappings[0].issue_key, 'decision-feed.actionability')
  assert.equal(report.mapped_to_existing.length, 0)
  // the other 4 keys still get inserted independently of the unresolved mapping.
  assert.equal(report.inserted_unverified.length, 4)
})
