import test from 'node:test'
import assert from 'node:assert/strict'
import { spawnSync } from 'node:child_process'
import { mkdirSync, mkdtempSync } from 'node:fs'
import { join } from 'node:path'
import { tmpdir } from 'node:os'
import { fileURLToPath } from 'node:url'
import Database from 'better-sqlite3'

const scriptFile = fileURLToPath(new URL('./fix-money-flags.mjs', import.meta.url))

test('fix-money-flags propagates source-provenance money and approval evidence', () => {
  const root = mkdtempSync(join(tmpdir(), 'chronica-money-flags-'))
  mkdirSync(join(root, 'docs'), { recursive: true })

  const db = new Database(join(root, 'docs', 'capabilities.db'))
  db.exec(`
    CREATE TABLE canonical_capability (
      id INTEGER PRIMARY KEY,
      key TEXT UNIQUE NOT NULL,
      canonical_name TEXT NOT NULL,
      moves_money INTEGER DEFAULT 0,
      requires_approval INTEGER DEFAULT 0,
      side_effect_class TEXT,
      financial_control_test TEXT,
      acceptance_criteria TEXT,
      donor_count INTEGER DEFAULT 0
    );
    CREATE TABLE source_capability (
      id INTEGER PRIMARY KEY,
      canonical_id INTEGER,
      donor TEXT,
      canonical_name TEXT,
      source_files TEXT,
      moves_money INTEGER DEFAULT 0,
      requires_approval INTEGER DEFAULT 0,
      side_effect_class TEXT
    );
    CREATE TABLE provenance (
      source_id INTEGER NOT NULL,
      canonical_id INTEGER NOT NULL,
      PRIMARY KEY (source_id, canonical_id)
    );
  `)
  db.prepare(`INSERT INTO canonical_capability
    (id, key, canonical_name, moves_money, requires_approval, side_effect_class)
    VALUES (?, ?, ?, ?, ?, ?)`)
    .run(1, 'commerce.cloud_usage_metering', 'Cloud Usage Metering', 0, 0, 'internal_write')
  db.prepare(`INSERT INTO source_capability
    (id, canonical_id, donor, canonical_name, source_files, moves_money, requires_approval, side_effect_class)
    VALUES (?, ?, ?, ?, ?, ?, ?, ?)`)
    .run(10, 1, 'langfuse-main', 'Cloud Usage Metering', 'worker.ts', 1, 1, 'money')
  db.prepare('INSERT INTO provenance (source_id, canonical_id) VALUES (?, ?)').run(10, 1)
  db.close()

  const result = spawnSync(process.execPath, [scriptFile], {
    cwd: root,
    encoding: 'utf8',
  })

  assert.equal(result.status, 0, result.stderr || result.stdout)
  const out = new Database(join(root, 'docs', 'capabilities.db'), { readonly: true })
  const row = out.prepare(`SELECT moves_money, requires_approval, side_effect_class, financial_control_test
    FROM canonical_capability WHERE key='commerce.cloud_usage_metering'`).get()
  out.close()

  assert.deepEqual(row, {
    moves_money: 1,
    requires_approval: 1,
    side_effect_class: 'money',
    financial_control_test: 'REQUIRED (not yet written)',
  })
})
