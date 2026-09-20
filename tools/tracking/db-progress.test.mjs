import test from 'node:test'
import assert from 'node:assert/strict'
import Database from 'better-sqlite3'
import { execFileSync } from 'node:child_process'
import { mkdirSync, mkdtempSync, rmSync, writeFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { dirname, join } from 'node:path'
import { fileURLToPath } from 'node:url'

const here = dirname(fileURLToPath(import.meta.url))
const script = join(here, 'db-progress.mjs')

test('db-progress reports canonical progress from current canonical schema', () => {
  const root = mkdtempSync(join(tmpdir(), 'chronica-db-progress-test-'))
  try {
    mkdirSync(join(root, 'docs'), { recursive: true })
    const caps = new Database(join(root, 'docs', 'capabilities.db'))
    caps.exec(`
      CREATE TABLE canonical_capability (key TEXT PRIMARY KEY, status TEXT, moves_money INTEGER);
      CREATE TABLE source_capability (id INTEGER PRIMARY KEY, moves_money INTEGER, requires_approval INTEGER);
      CREATE TABLE impl_evidence (canonical_key TEXT PRIMARY KEY);
      CREATE TABLE donor_file_census (donor TEXT, read_status TEXT, classification TEXT);
      INSERT INTO canonical_capability VALUES ('a', 'verified', 0), ('b', 'unimplemented', 1);
      INSERT INTO source_capability VALUES (1, 1, 1), (2, 0, 0);
      INSERT INTO impl_evidence VALUES ('a');
      INSERT INTO donor_file_census VALUES ('d', 'reviewed', 'mapped'), ('d', 'unread_pending', 'capability_review_pending');
    `)
    caps.close()

    const arch = new Database(join(root, 'docs', 'architecture.db'))
    arch.exec(`
      CREATE TABLE meta (k TEXT, v TEXT);
      CREATE TABLE capability_architecture_link (capability_key TEXT, relationship TEXT, status TEXT);
      CREATE TABLE architecture_target_gap (gap_kind TEXT);
      INSERT INTO meta VALUES ('canonical_capability_count', '2'), ('coverage_percent', '100.00'), ('gap_capabilities', '1');
      INSERT INTO capability_architecture_link VALUES ('a', 'implemented_by', 'verified');
      INSERT INTO architecture_target_gap VALUES ('planned_logical_domain');
    `)
    arch.close()

    const out = JSON.parse(execFileSync(process.execPath, [script, '--root', root], { encoding: 'utf8' }))
    assert.equal(out.headline.canonical_denominator, 2)
    assert.equal(out.headline.implementation_evidence_percent, 50)
    assert.equal(out.headline.donor_file_review_percent, 50)
    assert.equal(out.capability.write_health, 'canonical write surface present')
  } finally {
    rmSync(root, { recursive: true, force: true })
  }
})

test('db-progress treats an empty-but-present canonical_capability the same as missing (issue #713 empty-shell trap)', () => {
  const root = mkdtempSync(join(tmpdir(), 'chronica-db-progress-empty-shell-test-'))
  try {
    mkdirSync(join(root, 'docs', 'capabilities-cloud'), { recursive: true })
    const caps = new Database(join(root, 'docs', 'capabilities.db'))
    caps.exec(`
      CREATE TABLE canonical_capability (key TEXT PRIMARY KEY, status TEXT, moves_money INTEGER);
      CREATE TABLE canonical_status_override (canonical_key TEXT PRIMARY KEY, status TEXT);
      CREATE TABLE impl_evidence (canonical_key TEXT PRIMARY KEY);
      CREATE TABLE source_capability (id INTEGER PRIMARY KEY, moves_money INTEGER, requires_approval INTEGER);
      INSERT INTO canonical_status_override VALUES ('a', 'verified');
      INSERT INTO impl_evidence VALUES ('a');
      INSERT INTO source_capability VALUES (1, 1, 1);
    `) // canonical_capability exists but has 0 rows, while side tables carry real prior work
    caps.close()
    const arch = new Database(join(root, 'docs', 'architecture.db'))
    arch.exec(`CREATE TABLE meta (k TEXT, v TEXT); INSERT INTO meta VALUES ('canonical_capability_count', '10'), ('coverage_percent', '100.00'), ('gap_capabilities', '3');`)
    arch.close()
    writeFileSync(join(root, 'docs', 'capabilities-cloud', 'meta.json'), JSON.stringify({ source: { canonical_capabilities: 10, money_canonical: 2 } }))

    const out = JSON.parse(execFileSync(process.execPath, [script, '--root', root], { encoding: 'utf8' }))
    assert.equal(out.capability.local_audit_required, true, 'an empty-but-present table must not be reported as a usable canonical write surface')
    assert.equal(out.capability.canonical.total, 10, 'must fall back to the cloud snapshot count instead of reporting the empty shell\'s 0 rows')
    assert.match(out.capability.write_health, /schema drift/)
  } finally {
    rmSync(root, { recursive: true, force: true })
  }
})

test('db-progress stays useful when canonical_capability is absent', () => {
  const root = mkdtempSync(join(tmpdir(), 'chronica-db-progress-legacy-test-'))
  try {
    mkdirSync(join(root, 'docs', 'capabilities-cloud'), { recursive: true })
    const caps = new Database(join(root, 'docs', 'capabilities.db'))
    caps.exec(`
      CREATE TABLE canonical_status_override (canonical_key TEXT PRIMARY KEY, status TEXT);
      CREATE TABLE impl_evidence (canonical_key TEXT PRIMARY KEY);
      CREATE TABLE source_capability (id INTEGER PRIMARY KEY, moves_money INTEGER, requires_approval INTEGER);
      INSERT INTO canonical_status_override VALUES ('a', 'verified');
      INSERT INTO impl_evidence VALUES ('a');
      INSERT INTO source_capability VALUES (1, 1, 1);
    `)
    caps.close()
    const arch = new Database(join(root, 'docs', 'architecture.db'))
    arch.exec(`CREATE TABLE meta (k TEXT, v TEXT); INSERT INTO meta VALUES ('canonical_capability_count', '10'), ('coverage_percent', '100.00'), ('gap_capabilities', '3');`)
    arch.close()
    writeFileSync(join(root, 'docs', 'capabilities-cloud', 'meta.json'), JSON.stringify({ source: { canonical_capabilities: 10, money_canonical: 2 } }))

    const out = JSON.parse(execFileSync(process.execPath, [script, '--root', root], { encoding: 'utf8' }))
    assert.equal(out.capability.local_audit_required, true)
    assert.equal(out.capability.canonical.total, 10)
    assert.match(out.capability.write_health, /schema drift/)
  } finally {
    rmSync(root, { recursive: true, force: true })
  }
})
