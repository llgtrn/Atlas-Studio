import test from 'node:test'
import assert from 'node:assert/strict'
import { spawnSync } from 'node:child_process'
import { mkdirSync, mkdtempSync, readFileSync, writeFileSync } from 'node:fs'
import { join } from 'node:path'
import { tmpdir } from 'node:os'
import { fileURLToPath } from 'node:url'
import Database from 'better-sqlite3'

const scriptFile = fileURLToPath(new URL('./gen-crate-docs.mjs', import.meta.url))

test('backfilled reconcile reports do not say all_agree when mapped capabilities are unverified', () => {
  const root = mkdtempSync(join(tmpdir(), 'chronica-crate-docs-'))
  mkdirSync(join(root, 'crates', 'chronica-demo', 'src'), { recursive: true })
  mkdirSync(join(root, 'docs', '_machine', 'reconcile-reports'), { recursive: true })
  mkdirSync(join(root, 'docs'), { recursive: true })

  writeFileSync(join(root, 'crates', 'chronica-demo', 'Cargo.toml'), '[package]\nname = "chronica-demo"\nversion = "0.1.0"\nedition = "2021"\n')
  writeFileSync(join(root, 'crates', 'chronica-demo', 'src', 'lib.rs'), 'pub fn demo() {}\n')

  const caps = new Database(join(root, 'docs', 'capabilities.db'))
  caps.exec(`
    CREATE TABLE canonical_capability (
      key TEXT PRIMARY KEY,
      canonical_name TEXT,
      domain TEXT,
      moves_money INTEGER DEFAULT 0,
      side_effect_class TEXT,
      target_crate TEXT,
      target_module TEXT,
      acceptance_test TEXT,
      financial_control_test TEXT,
      status TEXT DEFAULT 'unimplemented'
    );
    INSERT INTO canonical_capability
      (key, canonical_name, domain, moves_money, side_effect_class, target_crate, target_module, status)
    VALUES
      ('demo.unverified', 'Unverified demo capability', 'demo', 0, 'pure', 'chronica-demo', 'lib', 'unimplemented');
  `)
  caps.close()

  const arch = new Database(join(root, 'docs', 'architecture.db'))
  arch.close()

  const result = spawnSync(process.execPath, [scriptFile], {
    cwd: root,
    encoding: 'utf8',
  })

  assert.equal(result.status, 0, result.stderr || result.stdout)
  const report = JSON.parse(readFileSync(join(root, 'docs', '_machine', 'reconcile-reports', 'chronica-demo.json'), 'utf8'))
  assert.equal(report.judgment, 'mixed')
  assert.match(report.doc_action, /0 verified/)
})
