import test from 'node:test'
import assert from 'node:assert/strict'
import Database from 'better-sqlite3'
import { existsSync, mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join } from 'node:path'
import {
  SUBDB_JSONL,
  SUBDB_SQLITE,
  collectCrateSubdbs,
  generateCrateSubdbs,
  verifyCrateSubdbs,
} from './subdb-lib.mjs'

function writeMiniWorkspace() {
  const root = mkdtempSync(join(tmpdir(), 'chronica-subdb-'))
  mkdirSync(join(root, 'crates', 'chronica-demo', 'src'), { recursive: true })
  mkdirSync(join(root, 'crates', 'chronica-empty', 'src'), { recursive: true })
  mkdirSync(join(root, 'docs', 'capabilities-cloud'), { recursive: true })

  writeFileSync(join(root, 'Cargo.toml'), 'members = ["crates/chronica-demo", "crates/chronica-empty"]\n')
  writeFileSync(
    join(root, 'crates', 'chronica-demo', 'Cargo.toml'),
    '[package]\nname = "chronica-demo"\nversion = "0.1.0"\nedition = "2021"\n\n[dependencies]\nchronica-empty = { path = "../chronica-empty" }\n',
  )
  writeFileSync(join(root, 'crates', 'chronica-demo', 'src', 'lib.rs'), 'pub fn demo() -> bool { true }\n')
  writeFileSync(join(root, 'crates', 'chronica-empty', 'Cargo.toml'), '[package]\nname = "chronica-empty"\nversion = "0.1.0"\nedition = "2021"\n')
  writeFileSync(join(root, 'crates', 'chronica-empty', 'src', 'lib.rs'), 'pub struct Empty;\n')

  const core = new Database(join(root, 'docs', 'capabilities-cloud', 'cap-core.db'))
  core.exec(`CREATE TABLE canonical_capability (
    key TEXT PRIMARY KEY,
    canonical_name TEXT,
    domain TEXT,
    target_crate TEXT,
    target_module TEXT,
    side_effect_class TEXT,
    moves_money INTEGER,
    requires_approval INTEGER,
    status TEXT,
    donor_count INTEGER
  )`)
  core.prepare(`INSERT INTO canonical_capability VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)`).run(
    'demo.capability',
    'Demo Capability',
    'demo',
    'chronica-demo',
    'lib',
    'money_free',
    0,
    0,
    'implemented_unverified',
    1,
  )
  core.close()

  const arch = new Database(join(root, 'docs', 'capabilities-cloud', 'cap-architecture.db'))
  arch.exec(`CREATE TABLE capability_architecture_link (
    capability_key TEXT,
    architecture_node TEXT,
    relationship TEXT,
    status TEXT,
    target_crate TEXT,
    target_module TEXT
  )`)
  arch.prepare(`INSERT INTO capability_architecture_link VALUES (?, ?, ?, ?, ?, ?)`).run(
    'demo.capability',
    'crate:chronica-demo',
    'owned_by',
    'implemented_unverified',
    'chronica-demo',
    'lib',
  )
  arch.close()

  return root
}

function writeArchitectureCanonicalEvidence(root, rows) {
  const dir = join(root, 'docs', 'architecture-canonical')
  mkdirSync(dir, { recursive: true })
  writeFileSync(join(dir, 'evidence.jsonl'), `${rows.map((row) => JSON.stringify(row)).join('\n')}\n`)
}

test('shards crate-owned architecture_evidence rows from the canonical evidence.jsonl', () => {
  const root = writeMiniWorkspace()
  try {
    writeArchitectureCanonicalEvidence(root, [
      {
        schema_version: 1,
        record_type: 'architecture_evidence',
        id: 1,
        architecture_node: 'crate:chronica-demo',
        status: 'implemented',
        evidence_kind: 'repo_file',
        evidence_ref: 'crates/chronica-demo/Cargo.toml',
        test_command: null,
        test_result: 'generated_present',
        verified_at: null,
        verified_by: 'tools/architecture/build-db.mjs',
        blocker_reason: null,
      },
      {
        schema_version: 1,
        record_type: 'architecture_evidence',
        id: 2,
        architecture_node: 'crate:chronica-empty',
        status: 'implemented',
        evidence_kind: 'repo_file',
        evidence_ref: 'crates/chronica-empty/Cargo.toml',
        test_command: null,
        test_result: 'generated_present',
        verified_at: null,
        verified_by: 'tools/architecture/build-db.mjs',
        blocker_reason: null,
      },
      {
        schema_version: 1,
        record_type: 'architecture_evidence',
        id: 3,
        architecture_node: 'crate:some-other-crate-entirely',
        status: 'implemented',
        evidence_kind: 'repo_file',
        evidence_ref: 'crates/some-other-crate-entirely/Cargo.toml',
        test_command: null,
        test_result: 'generated_present',
        verified_at: null,
        verified_by: 'tools/architecture/build-db.mjs',
        blocker_reason: null,
      },
      // architecture-canonical/evidence.jsonl also carries global record types
      // (architecture_invariant, authority_rule, data_flow) that must never be
      // sharded per-crate — they legitimately stay root-only since they span crates.
      {
        schema_version: 1,
        record_type: 'authority_rule',
        id: 1,
        actor: 'CouncilPresident',
        scope: 'Group',
        may_see: 'group rollups',
        may_propose: 'strategy',
        may_approve: 'constitution',
        may_execute: 'approval decisions only',
        may_audit: 'all',
        forbidden_actions: 'bypass',
        enforcing_node: 'runtime_entity:CouncilPresident',
      },
    ])

    const generated = generateCrateSubdbs({ root, crates: ['chronica-demo', 'chronica-empty'] })
    assert.equal(generated.crates.find((c) => c.crate === 'chronica-demo').architecture_evidence, 1)
    assert.equal(generated.crates.find((c) => c.crate === 'chronica-empty').architecture_evidence, 1)

    const demoRows = readFileSync(join(root, 'crates', 'chronica-demo', '.chronica', SUBDB_JSONL), 'utf8')
      .trim().split('\n').map((line) => JSON.parse(line))
    const demoEvidence = demoRows.filter((row) => row.record_type === 'architecture_evidence')
    assert.equal(demoEvidence.length, 1)
    assert.equal(demoEvidence[0].evidence_id, 1)
    assert.equal(demoEvidence[0].architecture_node, 'crate:chronica-demo')
    assert.equal(demoEvidence[0].evidence_ref, 'crates/chronica-demo/Cargo.toml')
    assert.ok(demoRows.every((row) => row.record_type !== 'authority_rule'))

    const emptyRows = readFileSync(join(root, 'crates', 'chronica-empty', '.chronica', SUBDB_JSONL), 'utf8')
      .trim().split('\n').map((line) => JSON.parse(line))
    const emptyEvidence = emptyRows.filter((row) => row.record_type === 'architecture_evidence')
    assert.equal(emptyEvidence.length, 1)
    assert.equal(emptyEvidence[0].evidence_id, 2)

    const verified = verifyCrateSubdbs({ root })
    assert.equal(verified.ok, true, JSON.stringify(verified.errors, null, 2))
  } finally {
    rmSync(root, { recursive: true, force: true })
  }
})

test('generates and verifies crate-local graph-chain subdb shards', () => {
  const root = writeMiniWorkspace()
  try {
    const generated = generateCrateSubdbs({ root })
    assert.equal(generated.crates.length, 2)

    const jsonlPath = join(root, 'crates', 'chronica-demo', '.chronica', SUBDB_JSONL)
    const sqlitePath = join(root, 'crates', 'chronica-demo', '.chronica', SUBDB_SQLITE)
    assert.equal(existsSync(jsonlPath), true)
    assert.equal(existsSync(sqlitePath), true)

    const rows = readFileSync(jsonlPath, 'utf8').trim().split('\n').map((line) => JSON.parse(line))
    assert.equal(rows[0].record_type, 'meta')
    assert.equal(rows[0].crate, 'chronica-demo')
    assert.ok(rows.every((row) => typeof row.record_hash === 'string' && row.record_hash.startsWith('sha256:')))
    assert.ok(rows.some((row) => row.record_type === 'capability' && row.capability_key === 'demo.capability'))
    assert.ok(rows.some((row) => row.record_type === 'architecture_node' && row.node_id === 'crate:chronica-demo'))
    assert.ok(rows.some((row) => row.record_type === 'capability_architecture_link' && row.capability_key === 'demo.capability'))

    const db = new Database(sqlitePath, { readonly: true, fileMustExist: true })
    assert.equal(db.prepare("SELECT count(*) n FROM subdb_record WHERE record_type='capability'").get().n, 1)
    db.close()

    const verified = verifyCrateSubdbs({ root })
    assert.equal(verified.ok, true, JSON.stringify(verified.errors, null, 2))
    assert.equal(verified.crates.length, 2)
  } finally {
    rmSync(root, { recursive: true, force: true })
  }
})

test('detects tampered and missing crate-local subdb shards', () => {
  const root = writeMiniWorkspace()
  try {
    generateCrateSubdbs({ root, crates: ['chronica-demo'] })
    const jsonlPath = join(root, 'crates', 'chronica-demo', '.chronica', SUBDB_JSONL)
    const raw = readFileSync(jsonlPath, 'utf8')
    writeFileSync(jsonlPath, raw.replace('demo.capability', 'demo.tampered'))

    const verified = verifyCrateSubdbs({ root })
    assert.equal(verified.ok, false)
    assert.ok(verified.errors.some((error) => error.includes('hash mismatch')))
    assert.ok(verified.errors.some((error) => error.includes('missing shard for crate chronica-empty')))

    const collected = collectCrateSubdbs({ root })
    assert.equal(collected.missing_crates.includes('chronica-empty'), true)
  } finally {
    rmSync(root, { recursive: true, force: true })
  }
})
