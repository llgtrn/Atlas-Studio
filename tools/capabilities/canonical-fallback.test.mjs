import test from 'node:test'
import assert from 'node:assert/strict'
import { mkdirSync, mkdtempSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join } from 'node:path'
import Database from 'better-sqlite3'
import { attachCanonicalCapabilityFallback, isUsableCanonicalCapabilityTable } from './canonical-fallback.mjs'

test('attaches cloud core canonical rows as a temp view when local canonical table is absent', () => {
  const root = mkdtempSync(join(tmpdir(), 'chronica-canonical-fallback-'))
  mkdirSync(join(root, 'docs', 'capabilities-cloud'), { recursive: true })

  const core = new Database(join(root, 'docs', 'capabilities-cloud', 'cap-core.db'))
  core.exec(`CREATE TABLE canonical_capability (
    key TEXT PRIMARY KEY,
    canonical_name TEXT,
    status TEXT,
    moves_money INTEGER,
    financial_control_test TEXT
  )`)
  core
    .prepare('INSERT INTO canonical_capability (key, canonical_name, status, moves_money, financial_control_test) VALUES (?, ?, ?, ?, ?)')
    .run('demo.capability', 'Demo Capability', 'unimplemented', 0, null)
  core.close()

  const local = new Database(join(root, 'docs', 'capabilities.db'))
  const result = attachCanonicalCapabilityFallback(local, { root })

  assert.equal(result.source, 'docs/capabilities-cloud/cap-core.db fallback (LOCAL_AUDIT_REQUIRED)')
  assert.deepEqual(local.prepare('SELECT key FROM canonical_capability').all(), [{ key: 'demo.capability' }])
  local.close()
})

test('an EMPTY (but present) local canonical_capability table is treated as unusable and falls back to the cloud core shard (issue #713 trap)', () => {
  const root = mkdtempSync(join(tmpdir(), 'chronica-canonical-fallback-empty-'))
  mkdirSync(join(root, 'docs', 'capabilities-cloud'), { recursive: true })

  const core = new Database(join(root, 'docs', 'capabilities-cloud', 'cap-core.db'))
  core.exec(`CREATE TABLE canonical_capability (
    key TEXT PRIMARY KEY,
    canonical_name TEXT,
    status TEXT,
    moves_money INTEGER,
    financial_control_test TEXT
  )`)
  core
    .prepare('INSERT INTO canonical_capability (key, canonical_name, status, moves_money, financial_control_test) VALUES (?, ?, ?, ?, ?)')
    .run('demo.capability', 'Demo Capability', 'unimplemented', 0, null)
  core.close()

  // caps:schema's `CREATE TABLE IF NOT EXISTS` left a real, but EMPTY, main-schema table.
  const local = new Database(join(root, 'docs', 'capabilities.db'))
  local.exec(`CREATE TABLE canonical_capability (
    key TEXT PRIMARY KEY, canonical_name TEXT, status TEXT, moves_money INTEGER, financial_control_test TEXT
  )`)

  assert.equal(isUsableCanonicalCapabilityTable(local, 'canonical_capability'), false)

  const result = attachCanonicalCapabilityFallback(local, { root })
  assert.equal(result.source, 'docs/capabilities-cloud/cap-core.db fallback (LOCAL_AUDIT_REQUIRED)')
  assert.deepEqual(local.prepare('SELECT key FROM canonical_capability').all(), [{ key: 'demo.capability' }])
  local.close()
})

test('a genuinely populated local canonical_capability table is used as-is (no fallback attach)', () => {
  const root = mkdtempSync(join(tmpdir(), 'chronica-canonical-fallback-populated-'))
  mkdirSync(join(root, 'docs', 'capabilities-cloud'), { recursive: true })

  const local = new Database(join(root, 'docs', 'capabilities.db'))
  local.exec('CREATE TABLE canonical_capability (key TEXT PRIMARY KEY, canonical_name TEXT)')
  local.prepare('INSERT INTO canonical_capability (key, canonical_name) VALUES (?, ?)').run('real.capability', 'Real Capability')

  assert.equal(isUsableCanonicalCapabilityTable(local, 'canonical_capability'), true)
  const result = attachCanonicalCapabilityFallback(local, { root })
  assert.equal(result.attached, false)
  assert.equal(result.source, 'docs/capabilities.db')
  local.close()
})
