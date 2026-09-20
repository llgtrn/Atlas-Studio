import test from 'node:test'
import assert from 'node:assert/strict'
import { mkdirSync, mkdtempSync, rmSync, writeFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join } from 'node:path'
import Database from 'better-sqlite3'
import { buildCapabilitySourceDbFromGit } from './build-cloud-source-db.mjs'

function makeRoot() {
  const root = mkdtempSync(join(tmpdir(), 'chronica-cloud-source-db-'))
  mkdirSync(join(root, 'docs', 'capabilities-canonical', 'domains'), { recursive: true })
  mkdirSync(join(root, 'docs', 'capabilities-canonical', 'local-authority'), { recursive: true })
  return root
}

function writeJsonl(path, rows) {
  writeFileSync(path, rows.map((row) => JSON.stringify(row)).join('\n') + (rows.length ? '\n' : ''))
}

const RECORD_A = {
  schema_version: 1,
  capability_key: 'demo.alpha',
  canonical_name: 'Alpha',
  domain: 'demo',
  target_crate: 'chronica-demo',
  target_module: 'alpha',
  status: 'unimplemented',
  side_effect_class: 'internal_write',
  moves_money: false,
  requires_approval: false,
  acceptance_criteria: 'Local audit must prove Alpha is implemented natively with behavior and failure-path tests.',
  required_tests: [],
  financial_control_test: null,
  acceptance_test: null,
  docs_refs: [],
  code_refs: [],
  test_refs: [],
  architecture_refs: [],
  source_refs: ['source_id=1;donor=demo-donor;module=core;name=Alpha Source;files=a.js;symbols=alpha'],
  evidence_refs: [],
  blocker: null,
}

const RECORD_B = { ...RECORD_A, capability_key: 'demo.beta', canonical_name: 'Beta', source_refs: [] }

function seedCanonicalShards(root) {
  const file = join(root, 'docs', 'capabilities-canonical', 'domains', 'demo.jsonl')
  writeJsonl(file, [RECORD_A, RECORD_B])
  writeFileSync(
    join(root, 'docs', 'capabilities-canonical', 'meta.json'),
    JSON.stringify({
      schema_version: 1,
      authority: 'canonical_jsonl',
      generated_by: 'test',
      record_count: 2,
      domain_count: 1,
      verified_count: 0,
      money_count: 0,
      files: ['docs/capabilities-canonical/domains/demo.jsonl'],
    }),
  )
}

test('buildCapabilitySourceDbFromGit rebuilds canonical_capability from domain shards with fresh sequential ids', () => {
  const root = makeRoot()
  seedCanonicalShards(root)
  const result = buildCapabilitySourceDbFromGit({ root })
  const db = new Database(result.dbPath, { readonly: true })
  const rows = db.prepare('SELECT id, key FROM canonical_capability ORDER BY id').all()
  db.close()
  assert.deepEqual(rows, [
    { id: 1, key: 'demo.alpha' },
    { id: 2, key: 'demo.beta' },
  ])
  rmSync(root, { recursive: true, force: true })
  if (result.tempDir) rmSync(result.tempDir, { recursive: true, force: true })
})

test('missing local-authority tables are reported as explicit gaps, not silently emptied without a trace', () => {
  const root = makeRoot()
  seedCanonicalShards(root)
  const result = buildCapabilitySourceDbFromGit({ root })
  assert.equal(result.gaps.capability, 'docs/capabilities-canonical/local-authority/capability.jsonl not present')
  assert.equal(result.gaps.slice_canonical, 'docs/capabilities-canonical/local-authority/slice_canonical.jsonl not present')
  assert.match(result.gaps.donor_file_census, /not git-tracked/)
  assert.match(result.gaps.source_file_capability_link, /not git-tracked/)
  assert.match(result.gaps.agent_note, /not git-tracked/)

  const db = new Database(result.dbPath, { readonly: true })
  // slice_canonical must still exist (empty, valid schema) - exportProvenance()
  // indexes slice_canonical(canonical_key) unconditionally.
  const columns = db.prepare('PRAGMA table_info(slice_canonical)').all().map((c) => c.name)
  assert.ok(columns.includes('canonical_key'))
  assert.equal(db.prepare('SELECT count(*) n FROM slice_canonical').get().n, 0)
  db.close()

  rmSync(root, { recursive: true, force: true })
  if (result.tempDir) rmSync(result.tempDir, { recursive: true, force: true })
})

test('local-authority tables keyed by canonical_key/capability_key load wholesale (no numeric-id join risk)', () => {
  const root = makeRoot()
  seedCanonicalShards(root)
  writeJsonl(join(root, 'docs', 'capabilities-canonical', 'local-authority', 'slice_canonical.jsonl'), [
    { slice: 1, canonical_key: 'demo.alpha', match_method: 'manifest', confidence: 'high', reviewed: 0, note: null },
  ])
  writeJsonl(join(root, 'docs', 'capabilities-canonical', 'local-authority', 'canonical_status_override.jsonl'), [
    { canonical_key: 'demo.alpha', status: 'verified', acceptance_test: null, financial_control_test: null, moves_money: 0, requires_approval: null, blocker: null, set_at: '2026-01-01', set_by: 'test' },
  ])
  writeJsonl(join(root, 'docs', 'capabilities-canonical', 'local-authority', 'capability.jsonl'), [
    { id: 1, key: 'slice.1', title: 'Demo slice', donor: 'demo', status: 'verified' },
  ])

  const result = buildCapabilitySourceDbFromGit({ root })
  assert.equal(result.loaded.slice_canonical, 1)
  assert.equal(result.loaded.canonical_status_override, 1)
  assert.equal(result.loaded.capability, 1)

  const db = new Database(result.dbPath, { readonly: true })
  assert.deepEqual(
    db.prepare('SELECT canonical_key, status FROM canonical_status_override').all(),
    [{ canonical_key: 'demo.alpha', status: 'verified' }],
  )
  assert.deepEqual(
    db.prepare('SELECT slice, canonical_key FROM slice_canonical').all(),
    [{ slice: 1, canonical_key: 'demo.alpha' }],
  )
  db.close()

  rmSync(root, { recursive: true, force: true })
  if (result.tempDir) rmSync(result.tempDir, { recursive: true, force: true })
})

test('donor_file_census / source_file_capability_link load wholesale from their donor-sharded directories, no longer a gap once present (PR #2612)', () => {
  const root = makeRoot()
  seedCanonicalShards(root)
  const censusDir = join(root, 'docs', 'capabilities-canonical', 'local-authority', 'donor_file_census')
  const linkDir = join(root, 'docs', 'capabilities-canonical', 'local-authority', 'source_file_capability_link')
  mkdirSync(censusDir, { recursive: true })
  mkdirSync(linkDir, { recursive: true })
  writeJsonl(join(censusDir, 'demo-donor.jsonl'), [
    { donor: 'demo-donor', path: 'a.js', read_status: 'unread_pending' },
    { donor: 'demo-donor', path: 'b.js', read_status: 'reviewed' },
  ])
  writeJsonl(join(linkDir, 'demo-donor.jsonl'), [
    { source_id: 1, donor: 'demo-donor', path: 'a.js', symbol: 'alpha', evidence_note: 'demo' },
  ])

  const result = buildCapabilitySourceDbFromGit({ root })
  assert.equal(result.gaps.donor_file_census, undefined)
  assert.equal(result.gaps.source_file_capability_link, undefined)
  assert.equal(result.loaded.donor_file_census, 2)
  assert.equal(result.loaded.source_file_capability_link, 1)

  const db = new Database(result.dbPath, { readonly: true })
  assert.equal(db.prepare('SELECT count(*) n FROM donor_file_census').get().n, 2)
  assert.equal(db.prepare('SELECT count(*) n FROM source_file_capability_link').get().n, 1)
  db.close()

  rmSync(root, { recursive: true, force: true })
  if (result.tempDir) rmSync(result.tempDir, { recursive: true, force: true })
})

test('source_capability is enriched by stable source id, not by the local-authority dump\'s stale canonical_id', () => {
  const root = makeRoot()
  seedCanonicalShards(root)
  writeJsonl(join(root, 'docs', 'capabilities-canonical', 'local-authority', 'source_capability.jsonl'), [
    {
      id: 1,
      donor: 'demo-donor',
      module: 'core',
      canonical_name: 'Alpha Source',
      source_files: 'a.js',
      source_symbols: 'alpha',
      business_behavior: 'Does the alpha thing.',
      technical_behavior: 'Technical alpha detail.',
      inputs: 'x',
      outputs: 'y',
      persistence: 'none',
      surface: 'library',
      side_effect_class: 'internal_write',
      moves_money: 0,
      requires_approval: 0,
      external_services: 'none',
      target_crate: 'chronica-demo',
      target_module: 'alpha',
      // Deliberately a canonical_id that does NOT match the rebuilt canonical_capability.id
      // for demo.alpha (which is 1 in this fixture, so pick something wrong to prove it's ignored).
      canonical_id: 999,
      created_pass: 'test',
    },
  ])

  const result = buildCapabilitySourceDbFromGit({ root })
  const db = new Database(result.dbPath, { readonly: true })
  const row = db.prepare('SELECT * FROM source_capability WHERE id = 1').get()

  assert.equal(row.business_behavior, 'Does the alpha thing.')
  assert.equal(row.persistence, 'none')
  // canonical_id must stay whatever build-db-from-canonical-shards.mjs derived
  // from THIS rebuild's fresh ids (1, matching demo.alpha), never the stale 999
  // carried by the local-authority dump.
  assert.equal(row.canonical_id, 1)

  const provenance = db.prepare('SELECT * FROM provenance').all()
  assert.deepEqual(provenance, [{ source_id: 1, canonical_id: 1 }])
  db.close()

  rmSync(root, { recursive: true, force: true })
  if (result.tempDir) rmSync(result.tempDir, { recursive: true, force: true })
})
