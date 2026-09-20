import test from 'node:test'
import assert from 'node:assert/strict'
import { mkdirSync, mkdtempSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { dirname, join, resolve } from 'node:path'
import { fileURLToPath } from 'node:url'
import { spawnSync } from 'node:child_process'
import Database from 'better-sqlite3'

const __dirname = dirname(fileURLToPath(import.meta.url))
const script = resolve(__dirname, 'census-schema.mjs')

function makeRoot() {
  const root = mkdtempSync(join(tmpdir(), 'chronica-census-schema-'))
  mkdirSync(join(root, 'docs'), { recursive: true })
  return root
}

function run(root, extraArgs = []) {
  return spawnSync(process.execPath, [script, ...extraArgs], { cwd: root, encoding: 'utf8' })
}

test('a brand-new db (no prior side-table work) safely bootstraps an empty canonical_capability table', () => {
  const root = makeRoot()
  const result = run(root)
  assert.equal(result.status, 0, result.stderr)

  const db = new Database(join(root, 'docs', 'capabilities.db'), { readonly: true })
  assert.equal(db.prepare('SELECT count(*) n FROM canonical_capability').get().n, 0)
  db.close()
})

test('refuses (exit 1) when canonical_capability would be left empty while side tables carry real prior work', () => {
  const root = makeRoot()
  // simulate the issue #713 trap: a db whose canonical table is gone but whose survivable side
  // tables were populated by real prior work.
  const dbPath = join(root, 'docs', 'capabilities.db')
  const seed = new Database(dbPath)
  seed.exec(`CREATE TABLE canonical_status_override (canonical_key TEXT PRIMARY KEY, status TEXT, acceptance_test TEXT,
    financial_control_test TEXT, moves_money INTEGER, requires_approval INTEGER, blocker TEXT, set_at TEXT, set_by TEXT)`)
  seed.prepare("INSERT INTO canonical_status_override (canonical_key, status) VALUES ('demo.one', 'implemented_unverified')").run()
  seed.close()

  const result = run(root)
  assert.equal(result.status, 1)
  assert.match(result.stderr, /CANONICAL_CAPABILITY_EMPTY_WITH_SURVIVING_SIDE_TABLES/)

  // canonical_capability must exist (schema ensured) but the script must have refused to leave it
  // silently empty without loud warning — the table itself is a harmless side effect of ensuring
  // schema; the failure signal (exit 1 + stderr) is what a caller/CI must react to.
  const db = new Database(dbPath, { readonly: true })
  assert.equal(db.prepare('SELECT count(*) n FROM canonical_capability').get().n, 0)
  db.close()
})

test('--allow-empty-canonical bypasses the guard for an intentional fresh bootstrap', () => {
  const root = makeRoot()
  const dbPath = join(root, 'docs', 'capabilities.db')
  const seed = new Database(dbPath)
  seed.exec(`CREATE TABLE canonical_status_override (canonical_key TEXT PRIMARY KEY, status TEXT, acceptance_test TEXT,
    financial_control_test TEXT, moves_money INTEGER, requires_approval INTEGER, blocker TEXT, set_at TEXT, set_by TEXT)`)
  seed.prepare("INSERT INTO canonical_status_override (canonical_key, status) VALUES ('demo.one', 'implemented_unverified')").run()
  seed.close()

  const result = run(root, ['--allow-empty-canonical'])
  assert.equal(result.status, 0, result.stderr)
})

test('does not refuse when canonical_capability already carries rows (normal repeated caps:schema run)', () => {
  const root = makeRoot()
  const dbPath = join(root, 'docs', 'capabilities.db')
  const seed = new Database(dbPath)
  seed.exec(`CREATE TABLE canonical_capability (id INTEGER PRIMARY KEY, key TEXT UNIQUE NOT NULL, canonical_name TEXT NOT NULL,
    domain TEXT, target_crate TEXT, target_module TEXT, side_effect_class TEXT, moves_money INTEGER DEFAULT 0,
    requires_approval INTEGER DEFAULT 0, acceptance_criteria TEXT, required_tests TEXT, financial_control_test TEXT,
    acceptance_test TEXT, status TEXT DEFAULT 'unimplemented', exclusion_note TEXT, blocker TEXT, slice INTEGER, donor_count INTEGER DEFAULT 0)`)
  seed.prepare("INSERT INTO canonical_capability (key, canonical_name) VALUES ('already.here', 'Already Here')").run()
  seed.close()

  const result = run(root)
  assert.equal(result.status, 0, result.stderr)
})
