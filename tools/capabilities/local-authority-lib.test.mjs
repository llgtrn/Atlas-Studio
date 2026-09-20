import test from 'node:test'
import assert from 'node:assert/strict'
import { mkdirSync, mkdtempSync, rmSync, writeFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join } from 'node:path'
import Database from 'better-sqlite3'
import {
  createTableFromLocalAuthorityRows,
  loadLocalAuthorityMeta,
  loadLocalAuthorityShardedTable,
  loadLocalAuthorityTable,
} from './local-authority-lib.mjs'

function makeRoot() {
  return mkdtempSync(join(tmpdir(), 'chronica-local-authority-'))
}

test('loadLocalAuthorityTable returns null (not []) when the dump file is absent, so callers can tell "no data" from "gap"', () => {
  const root = makeRoot()
  assert.equal(loadLocalAuthorityTable(root, 'provenance'), null)
  rmSync(root, { recursive: true, force: true })
})

test('loadLocalAuthorityTable parses one JSON object per line', () => {
  const root = makeRoot()
  const dir = join(root, 'docs', 'capabilities-canonical', 'local-authority')
  mkdirSync(dir, { recursive: true })
  writeFileSync(join(dir, 'provenance.jsonl'), '{"source_id":1,"canonical_id":1}\n{"source_id":2,"canonical_id":1}\n')
  assert.deepEqual(loadLocalAuthorityTable(root, 'provenance'), [
    { source_id: 1, canonical_id: 1 },
    { source_id: 2, canonical_id: 1 },
  ])
  rmSync(root, { recursive: true, force: true })
})

test('loadLocalAuthorityTable throws a located error on invalid JSONL rather than silently skipping the row', () => {
  const root = makeRoot()
  const dir = join(root, 'docs', 'capabilities-canonical', 'local-authority')
  mkdirSync(dir, { recursive: true })
  writeFileSync(join(dir, 'provenance.jsonl'), '{"source_id":1,"canonical_id":1}\nnot json\n')
  assert.throws(() => loadLocalAuthorityTable(root, 'provenance'), /invalid JSONL/)
  rmSync(root, { recursive: true, force: true })
})

test('loadLocalAuthorityMeta returns null when meta.json is absent, and parses it when present', () => {
  const root = makeRoot()
  assert.equal(loadLocalAuthorityMeta(root), null)
  const dir = join(root, 'docs', 'capabilities-canonical', 'local-authority')
  mkdirSync(dir, { recursive: true })
  writeFileSync(join(dir, 'meta.json'), JSON.stringify({ not_yet_exported: { donor_file_census: { rows: 1 } } }))
  assert.deepEqual(loadLocalAuthorityMeta(root), { not_yet_exported: { donor_file_census: { rows: 1 } } })
  rmSync(root, { recursive: true, force: true })
})

test('createTableFromLocalAuthorityRows infers columns from the union of row keys and normalizes booleans/objects', () => {
  const db = new Database(':memory:')
  createTableFromLocalAuthorityRows(db, 'demo', [
    { id: 1, flag: true, tags: ['a', 'b'], note: null },
    { id: 2, flag: false, tags: [], extra: 'only on row 2' },
  ])
  const rows = db.prepare('SELECT * FROM demo ORDER BY id').all()
  assert.equal(rows[0].flag, 1)
  assert.equal(rows[1].flag, 0)
  assert.equal(rows[0].tags, JSON.stringify(['a', 'b']))
  assert.equal(rows[0].extra, null)
  assert.equal(rows[1].extra, 'only on row 2')
  db.close()
})

test('loadLocalAuthorityShardedTable returns null (not []) when the shard directory is absent', () => {
  const root = makeRoot()
  assert.equal(loadLocalAuthorityShardedTable(root, 'donor_file_census'), null)
  rmSync(root, { recursive: true, force: true })
})

test('loadLocalAuthorityShardedTable concatenates every *.jsonl file under <table>/, in sorted filename order', () => {
  const root = makeRoot()
  const dir = join(root, 'docs', 'capabilities-canonical', 'local-authority', 'donor_file_census')
  mkdirSync(dir, { recursive: true })
  writeFileSync(join(dir, 'zeta.jsonl'), '{"donor":"zeta","path":"z.js"}\n')
  writeFileSync(join(dir, 'alpha.jsonl'), '{"donor":"alpha","path":"a.js"}\n{"donor":"alpha","path":"b.js"}\n')
  writeFileSync(join(dir, 'README.md'), 'not jsonl, must be ignored')
  assert.deepEqual(loadLocalAuthorityShardedTable(root, 'donor_file_census'), [
    { donor: 'alpha', path: 'a.js' },
    { donor: 'alpha', path: 'b.js' },
    { donor: 'zeta', path: 'z.js' },
  ])
  rmSync(root, { recursive: true, force: true })
})

test('loadLocalAuthorityShardedTable throws a located error on invalid JSONL in any shard', () => {
  const root = makeRoot()
  const dir = join(root, 'docs', 'capabilities-canonical', 'local-authority', 'donor_file_census')
  mkdirSync(dir, { recursive: true })
  writeFileSync(join(dir, 'alpha.jsonl'), '{"donor":"alpha"}\nnot json\n')
  assert.throws(() => loadLocalAuthorityShardedTable(root, 'donor_file_census'), /invalid JSONL/)
  rmSync(root, { recursive: true, force: true })
})

test('createTableFromLocalAuthorityRows drops any pre-existing table of the same name', () => {
  const db = new Database(':memory:')
  db.exec('CREATE TABLE demo (stale_column TEXT)')
  createTableFromLocalAuthorityRows(db, 'demo', [{ id: 1 }])
  const columns = db.prepare('PRAGMA table_info(demo)').all().map((c) => c.name)
  assert.deepEqual(columns, ['id'])
  db.close()
})
