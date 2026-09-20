import test from 'node:test'
import assert from 'node:assert/strict'
import { spawnSync } from 'node:child_process'
import { existsSync, mkdirSync, mkdtempSync, readFileSync, writeFileSync } from 'node:fs'
import { dirname, join } from 'node:path'
import { tmpdir } from 'node:os'
import { fileURLToPath } from 'node:url'
import Database from 'better-sqlite3'

const scriptFile = fileURLToPath(new URL('./reconcile-helpdesk-evidence.mjs', import.meta.url))

function unique(values) {
  return [...new Set(values.filter(Boolean))]
}

function makeFixtureRoot() {
  const root = mkdtempSync(join(tmpdir(), 'chronica-helpdesk-reconcile-'))
  mkdirSync(join(root, 'docs'), { recursive: true })

  const db = new Database(join(root, 'docs', 'capabilities.db'))
  db.exec(`
    CREATE TABLE canonical_capability (
      id INTEGER PRIMARY KEY,
      key TEXT UNIQUE NOT NULL,
      canonical_name TEXT,
      moves_money INTEGER DEFAULT 0,
      side_effect_class TEXT,
      status TEXT DEFAULT 'unimplemented',
      acceptance_test TEXT,
      financial_control_test TEXT,
      blocker TEXT
    );
    CREATE TABLE canonical_status_override (
      canonical_key TEXT PRIMARY KEY,
      status TEXT,
      acceptance_test TEXT,
      financial_control_test TEXT,
      moves_money INTEGER,
      blocker TEXT,
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
  db.close()

  const script = readFileSync(scriptFile, 'utf8')
  const testEntries = [...script.matchAll(/test_file:\s*"([^"]+)",\s*test_symbol:\s*"([^"]+)"/g)]
  const implEntries = [...script.matchAll(/impl_file:\s*"([^"]+)",\s*impl_symbols:\s*"([^"]+)"/g)]

  const testsByFile = new Map()
  for (const [, file, symbol] of testEntries) {
    testsByFile.set(file, [...(testsByFile.get(file) || []), symbol])
  }
  for (const [file, symbols] of testsByFile) {
    const path = join(root, file)
    mkdirSync(dirname(path), { recursive: true })
    writeFileSync(path, unique(symbols).map((s) => `fn ${s}() {}`).join('\n'))
  }

  const implsByFile = new Map()
  for (const [, file, symbols] of implEntries) {
    implsByFile.set(file, [...(implsByFile.get(file) || []), ...symbols.split(',').map((s) => s.trim())])
  }
  for (const [file, symbols] of implsByFile) {
    const path = join(root, file)
    mkdirSync(dirname(path), { recursive: true })
    writeFileSync(path, unique(symbols).map((s) => `pub fn ${s}() {}`).join('\n'))
  }

  return root
}

test('reconcile-helpdesk-evidence treats absent legacy caps as already clean', () => {
  assert.equal(existsSync(scriptFile), true)
  const root = makeFixtureRoot()

  const result = spawnSync(process.execPath, [scriptFile], {
    cwd: root,
    encoding: 'utf8',
  })

  assert.equal(result.status, 0, result.stderr || result.stdout)
  assert.match(result.stdout, /skipped \d+ absent legacy cap/)
})
