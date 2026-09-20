import test from 'node:test'
import assert from 'node:assert/strict'
import { mkdtempSync, mkdirSync, writeFileSync, readFileSync, rmSync } from 'node:fs'
import { join } from 'node:path'
import { tmpdir } from 'node:os'
import Database from 'better-sqlite3'
import { exportCanonicalArchitectureShards } from './export-canonical-shards.mjs'

function makeMiniRepo() {
  const root = mkdtempSync(join(tmpdir(), 'chronica-arch-export-'))
  mkdirSync(join(root, 'docs', 'capabilities-canonical', 'domains'), { recursive: true })
  writeFileSync(
    join(root, 'docs', 'capabilities-canonical', 'domains', 'observability-analytics.jsonl'),
    [
      '{"schema_version":1,"capability_key":"obs.other_thing","domain":"observability-analytics"}',
      '{"schema_version":1,"capability_key":"obs.vectorized_compute","domain":"observability-analytics"}',
    ].join('\n') + '\n',
  )

  const dbPath = join(root, 'docs', 'architecture.db')
  const db = new Database(dbPath)
  db.exec(`
    CREATE TABLE architecture_evidence (
      id INTEGER PRIMARY KEY,
      architecture_node TEXT,
      status TEXT,
      evidence_kind TEXT,
      evidence_ref TEXT,
      test_command TEXT,
      test_result TEXT,
      verified_at TEXT,
      verified_by TEXT,
      blocker_reason TEXT
    );
  `)
  db.close()
  return { root, dbPath }
}

function insertEvidenceRow(dbPath, row) {
  const db = new Database(dbPath)
  db.prepare(
    `INSERT INTO architecture_evidence
      (id, architecture_node, status, evidence_kind, evidence_ref, test_command, test_result, verified_at, verified_by, blocker_reason)
      VALUES (@id, @architecture_node, @status, @evidence_kind, @evidence_ref, @test_command, @test_result, @verified_at, @verified_by, @blocker_reason)`,
  ).run(row)
  db.close()
}

test('capability_link evidence_ref resolves to the real shard file+line, not a literal glob', () => {
  const { root, dbPath } = makeMiniRepo()
  insertEvidenceRow(dbPath, {
    id: 1,
    architecture_node: 'crate:chronica-olap',
    status: 'verified',
    evidence_kind: 'capability_link',
    evidence_ref: 'capabilities.db:obs.vectorized_compute',
    test_command: 'pnpm caps:verify-impl',
    test_result: 'imported_verified_capability',
    verified_at: '2026-08-02T15:31:18.505Z',
    verified_by: 'tools/architecture/build-db.mjs',
    blocker_reason: null,
  })

  const outDir = join(root, 'docs', 'architecture-canonical')
  exportCanonicalArchitectureShards({ root, dbPath, outDir })

  const evidence = readFileSync(join(outDir, 'evidence.jsonl'), 'utf8')
    .trim()
    .split('\n')
    .map((line) => JSON.parse(line))
  assert.equal(evidence.length, 1)
  assert.equal(
    evidence[0].evidence_ref,
    'docs/capabilities-canonical/domains/observability-analytics.jsonl:2#obs.vectorized_compute',
  )
  assert.ok(!evidence[0].evidence_ref.includes('**'), 'must not contain an unexpanded glob')

  rmSync(root, { recursive: true, force: true })
})

test('capability_link evidence_ref for an unknown capability_key fails loudly instead of writing an unresolvable glob', () => {
  const { root, dbPath } = makeMiniRepo()
  insertEvidenceRow(dbPath, {
    id: 1,
    architecture_node: 'crate:chronica-olap',
    status: 'verified',
    evidence_kind: 'capability_link',
    evidence_ref: 'capabilities.db:obs.does_not_exist',
    test_command: null,
    test_result: null,
    verified_at: null,
    verified_by: null,
    blocker_reason: null,
  })

  const outDir = join(root, 'docs', 'architecture-canonical')
  assert.throws(
    () => exportCanonicalArchitectureShards({ root, dbPath, outDir }),
    /cannot resolve evidence_ref.*obs\.does_not_exist/,
  )

  rmSync(root, { recursive: true, force: true })
})
