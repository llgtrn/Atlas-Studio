import test from 'node:test'
import assert from 'node:assert/strict'
import { execFileSync } from 'node:child_process'
import { mkdirSync, mkdtempSync, writeFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join } from 'node:path'
import Database from 'better-sqlite3'

test('numbered docs surface every canonical capability from capabilities.db', () => {
  const output = execFileSync(process.execPath, ['tools/capabilities/verify-docs.mjs', '--report'], {
    encoding: 'utf8',
  })

  const match = output.match(/surfaced in numbered docs · (\d+) not surfaced\./)
  assert.ok(match, `verify-docs output did not include the numbered-doc surfaced-count line:\n${output}`)
  assert.equal(Number(match[1]), 0, output)
})

test('uses the cloud core shard when the local DB lacks canonical_capability', () => {
  const repoRoot = process.cwd()
  const root = mkdtempSync(join(tmpdir(), 'chronica-verify-docs-'))
  mkdirSync(join(root, 'docs', 'capabilities-cloud'), { recursive: true })
  mkdirSync(join(root, 'docs', '_generated'), { recursive: true })
  writeFileSync(
    join(root, 'docs', '_generated', '001-demo.md'),
    [
      '<!-- chronica:caps 1 -->',
      '- [x] ✅ verified `demo.capability` — Demo Capability',
      '<!-- /chronica:caps -->',
      '',
    ].join('\n'),
  )

  const local = new Database(join(root, 'docs', 'capabilities.db'))
  local.exec(`CREATE TABLE canonical_status_override (
    canonical_key TEXT PRIMARY KEY,
    status TEXT,
    acceptance_test TEXT,
    financial_control_test TEXT,
    moves_money INTEGER,
    requires_approval INTEGER,
    blocker TEXT,
    set_at TEXT,
    set_by TEXT
  )`)
  local.close()

  const core = new Database(join(root, 'docs', 'capabilities-cloud', 'cap-core.db'))
  core.exec(`CREATE TABLE canonical_capability (
    id INTEGER PRIMARY KEY,
    key TEXT UNIQUE NOT NULL,
    canonical_name TEXT NOT NULL,
    domain TEXT,
    target_crate TEXT,
    target_module TEXT,
    side_effect_class TEXT,
    moves_money INTEGER DEFAULT 0,
    requires_approval INTEGER DEFAULT 0,
    acceptance_criteria TEXT,
    required_tests TEXT,
    financial_control_test TEXT,
    acceptance_test TEXT,
    status TEXT DEFAULT 'unimplemented',
    exclusion_note TEXT,
    blocker TEXT,
    slice INTEGER,
    donor_count INTEGER DEFAULT 0
  )`)
  core
    .prepare(
      `INSERT INTO canonical_capability
       (key, canonical_name, domain, moves_money, requires_approval, status)
       VALUES (?, ?, ?, ?, ?, ?)`,
    )
    .run('demo.capability', 'Demo Capability', 'demo', 0, 0, 'verified')
  core.close()

  const output = execFileSync(process.execPath, [join(repoRoot, 'tools/capabilities/verify-docs.mjs'), '--report'], {
    cwd: root,
    encoding: 'utf8',
  })

  assert.match(output, /canonical source: docs\/capabilities-cloud\/cap-core\.db fallback/)
  assert.match(output, /verify-docs: 1 canonical caps · 1 surfaced in numbered docs · 0 not surfaced\./)
})

test('counts crate-doc capability tables as surfaced canonical rows', () => {
  const repoRoot = process.cwd()
  const root = mkdtempSync(join(tmpdir(), 'chronica-verify-crate-docs-'))
  mkdirSync(join(root, 'docs', 'capabilities-cloud'), { recursive: true })
  mkdirSync(join(root, 'docs', 'crates'), { recursive: true })
  writeFileSync(
    join(root, 'docs', 'crates', '300-crate-demo.md'),
    [
      '# 300 — Crate Demo',
      '',
      '| Capability | Status | Money | Side effect | Crate | Module | Test |',
      '|---|---|---|---|---|---|---|',
      '| demo.crate_capability |verified |0 |none |chronica-demo |lib |crate_capability_works |',
      '',
    ].join('\n'),
  )

  const local = new Database(join(root, 'docs', 'capabilities.db'))
  local.exec(`CREATE TABLE canonical_status_override (
    canonical_key TEXT PRIMARY KEY,
    status TEXT,
    acceptance_test TEXT,
    financial_control_test TEXT,
    moves_money INTEGER,
    requires_approval INTEGER,
    blocker TEXT,
    set_at TEXT,
    set_by TEXT
  )`)
  local.close()

  const core = new Database(join(root, 'docs', 'capabilities-cloud', 'cap-core.db'))
  core.exec(`CREATE TABLE canonical_capability (
    id INTEGER PRIMARY KEY,
    key TEXT UNIQUE NOT NULL,
    canonical_name TEXT NOT NULL,
    domain TEXT,
    moves_money INTEGER DEFAULT 0,
    financial_control_test TEXT,
    status TEXT DEFAULT 'unimplemented'
  )`)
  core
    .prepare('INSERT INTO canonical_capability (key, canonical_name, domain, moves_money, status) VALUES (?, ?, ?, ?, ?)')
    .run('demo.crate_capability', 'Demo Crate Capability', 'demo', 0, 'verified')
  core.close()

  const output = execFileSync(process.execPath, [join(repoRoot, 'tools/capabilities/verify-docs.mjs'), '--report'], {
    cwd: root,
    encoding: 'utf8',
  })

  assert.match(output, /verify-docs: 1 canonical caps · 1 surfaced in numbered docs · 0 not surfaced\./)
})
