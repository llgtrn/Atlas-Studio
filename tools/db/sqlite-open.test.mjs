import test from 'node:test'
import assert from 'node:assert/strict'
import Database from 'better-sqlite3'
import { mkdtempSync, rmSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join } from 'node:path'
import { openReadOnlyDatabase } from './sqlite-open.mjs'

test('openReadOnlyDatabase reads an existing SQLite file without writes', () => {
  const dir = mkdtempSync(join(tmpdir(), 'chronica-sqlite-open-test-'))
  try {
    const path = join(dir, 'sample.db')
    const writer = new Database(path)
    writer.exec('CREATE TABLE t (id INTEGER PRIMARY KEY, name TEXT); INSERT INTO t (name) VALUES (\'ok\')')
    writer.close()

    const reader = openReadOnlyDatabase(path)
    assert.equal(reader.prepare('SELECT name FROM t').get().name, 'ok')
    assert.throws(() => reader.prepare('INSERT INTO t (name) VALUES (\'nope\')').run())
    reader.close()
  } finally {
    rmSync(dir, { recursive: true, force: true })
  }
})
