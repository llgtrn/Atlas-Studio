import test from 'node:test'
import assert from 'node:assert/strict'
import { mkdirSync, mkdtempSync, writeFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { dirname, join, resolve } from 'node:path'
import { fileURLToPath } from 'node:url'
import { spawnSync } from 'node:child_process'
import Database from 'better-sqlite3'

const __dirname = dirname(fileURLToPath(import.meta.url))
const queryScript = resolve(__dirname, 'query.mjs')

test('next-unverified-canonical uses cloud fallback when local canonical table is absent', () => {
  const root = mkdtempSync(join(tmpdir(), 'chronica-query-fallback-'))
  mkdirSync(join(root, 'docs', 'capabilities-cloud'), { recursive: true })

  const local = new Database(join(root, 'docs', 'capabilities.db'))
  local.exec('CREATE TABLE meta (k TEXT PRIMARY KEY, v TEXT)')
  local.close()

  const core = new Database(join(root, 'docs', 'capabilities-cloud', 'cap-core.db'))
  core.exec(`CREATE TABLE canonical_capability (
    key TEXT PRIMARY KEY,
    canonical_name TEXT,
    domain TEXT,
    target_crate TEXT,
    moves_money INTEGER,
    donor_count INTEGER,
    status TEXT
  )`)
  core
    .prepare(`INSERT INTO canonical_capability
      (key, canonical_name, domain, target_crate, moves_money, donor_count, status)
      VALUES (?, ?, ?, ?, ?, ?, ?)`)
    .run('demo.unverified', 'Demo Unverified', 'demo', 'chronica-demo', 0, 2, 'unimplemented')
  core.close()

  const result = spawnSync(process.execPath, [queryScript, 'next-unverified-canonical', '--limit', '1'], {
    cwd: root,
    encoding: 'utf8',
  })

  assert.equal(result.status, 0, result.stderr)
  assert.deepEqual(JSON.parse(result.stdout), [{
    key: 'demo.unverified',
    canonical_name: 'Demo Unverified',
    domain: 'demo',
    target_crate: 'chronica-demo',
    moves_money: 0,
    donor_count: 2,
  }])
})

test('by-crate falls back to canonical_capability (cloud snapshot) when the local capability table is absent, and stamps the source', () => {
  const root = mkdtempSync(join(tmpdir(), 'chronica-query-by-crate-fallback-'))
  mkdirSync(join(root, 'docs', 'capabilities-cloud'), { recursive: true })

  // No `capability` table at all -- matches what census-schema.mjs produces in every
  // environment that only has the cloud-tracked cap-core.db (issue #1947).
  const local = new Database(join(root, 'docs', 'capabilities.db'))
  local.exec('CREATE TABLE meta (k TEXT PRIMARY KEY, v TEXT)')
  local.close()

  const core = new Database(join(root, 'docs', 'capabilities-cloud', 'cap-core.db'))
  core.exec(`CREATE TABLE canonical_capability (
    key TEXT PRIMARY KEY,
    canonical_name TEXT,
    domain TEXT,
    target_crate TEXT,
    status TEXT,
    acceptance_test TEXT
  )`)
  core
    .prepare('INSERT INTO canonical_capability (key, canonical_name, domain, target_crate, status, acceptance_test) VALUES (?, ?, ?, ?, ?, ?)')
    .run('demo.by_crate', 'Demo By Crate', 'demo', 'chronica-demo', 'unimplemented', null)
  core
    .prepare('INSERT INTO canonical_capability (key, canonical_name, domain, target_crate, status, acceptance_test) VALUES (?, ?, ?, ?, ?, ?)')
    .run('demo.other_crate', 'Demo Other Crate', 'demo', 'chronica-other', 'unimplemented', null)
  core.close()

  const result = spawnSync(process.execPath, [queryScript, 'by-crate', 'chronica-demo'], {
    cwd: root,
    encoding: 'utf8',
  })

  assert.equal(result.status, 0, result.stderr)
  const output = JSON.parse(result.stdout)
  assert.equal(output.capability_schema, 'canonical_capability_cloud_snapshot_fallback')
  assert.equal(output.local_audit_required, true)
  assert.match(output.schema_warning, /canonical_capability/)
  assert.deepEqual(output.rows, [{
    key: 'demo.by_crate',
    canonical_name: 'Demo By Crate',
    status: 'unimplemented',
    target_crate: 'chronica-demo',
    domain: 'demo',
    acceptance_test: null,
  }])
})

test('by-crate raw crashes are gone: a genuinely populated local capability table is queried directly (no fallback stamp)', () => {
  const root = mkdtempSync(join(tmpdir(), 'chronica-query-by-crate-primary-'))
  mkdirSync(join(root, 'docs', 'capabilities-cloud'), { recursive: true })

  const local = new Database(join(root, 'docs', 'capabilities.db'))
  local.exec(`CREATE TABLE capability (
    key TEXT PRIMARY KEY, title TEXT, status TEXT, percent INTEGER, gate_class TEXT, acceptance_test TEXT, target_crate TEXT, slice INTEGER
  )`)
  local
    .prepare('INSERT INTO capability (key, title, status, percent, gate_class, acceptance_test, target_crate, slice) VALUES (?, ?, ?, ?, ?, ?, ?, ?)')
    .run('demo.primary', 'Demo Primary', 'spec', 0, 'none', null, 'chronica-demo', 1)
  local.close()

  const result = spawnSync(process.execPath, [queryScript, 'by-crate', 'chronica-demo'], {
    cwd: root,
    encoding: 'utf8',
  })

  assert.equal(result.status, 0, result.stderr)
  const output = JSON.parse(result.stdout)
  assert.deepEqual(output, [{
    key: 'demo.primary', title: 'Demo Primary', status: 'spec', percent: 0, gate_class: 'none', acceptance_test: null,
  }])
})

test('get-canonical addresses the cloud row by key and never joins synthetic IDs to local provenance', () => {
  const root = mkdtempSync(join(tmpdir(), 'chronica-query-key-safe-fallback-'))
  mkdirSync(join(root, 'docs', 'capabilities-cloud'), { recursive: true })

  const local = new Database(join(root, 'docs', 'capabilities.db'))
  local.exec(`
    CREATE TABLE meta (k TEXT PRIMARY KEY, v TEXT);
    CREATE TABLE canonical_capability (id INTEGER PRIMARY KEY, key TEXT UNIQUE, canonical_name TEXT);
    CREATE TABLE source_capability (id INTEGER PRIMARY KEY, donor TEXT, canonical_name TEXT, source_files TEXT, canonical_id INTEGER);
    CREATE TABLE provenance (source_id INTEGER, canonical_id INTEGER);
  `)
  local.prepare("INSERT INTO source_capability VALUES (1, 'wrong-donor', 'Wrong Source', 'secret-path', 42)").run()
  local.prepare('INSERT INTO provenance VALUES (1, 42)').run()
  local.close()

  const core = new Database(join(root, 'docs', 'capabilities-cloud', 'cap-core.db'))
  core.exec(`CREATE TABLE canonical_capability (
    id INTEGER PRIMARY KEY,
    key TEXT UNIQUE,
    canonical_name TEXT,
    domain TEXT,
    target_crate TEXT,
    target_module TEXT,
    moves_money INTEGER,
    requires_approval INTEGER,
    status TEXT
  )`)
  core.prepare('INSERT INTO canonical_capability VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)')
    .run(42, 'demo.safe', 'Demo Safe', 'demo', 'chronica-demo', 'safe', 0, 0, 'unimplemented')
  core.close()

  const result = spawnSync(process.execPath, [queryScript, 'get-canonical', 'demo.safe'], {
    cwd: root,
    encoding: 'utf8',
  })
  assert.equal(result.status, 0, result.stderr)
  const output = JSON.parse(result.stdout)
  assert.equal(output.key, 'demo.safe')
  assert.equal(output.id, undefined, 'synthetic cloud IDs must not be exposed as local canonical identity')
  assert.deepEqual(output.provenance, [], 'numeric-ID provenance must not be joined across an untrusted fallback boundary')
  assert.equal(output.provenance_status, 'LOCAL_AUDIT_REQUIRED_UNSAFE_SYNTHETIC_ID_JOIN')
  assert.doesNotMatch(result.stdout, /wrong-donor|secret-path/)
})

test('target-crate-census reports canonical JSONL rows pointing at crates absent from disk', () => {
  const root = mkdtempSync(join(tmpdir(), 'chronica-query-target-crate-census-'))
  mkdirSync(join(root, 'crates', 'chronica-present'), { recursive: true })
  mkdirSync(join(root, 'docs', 'capabilities-canonical', 'domains'), { recursive: true })
  writeFileSync(join(root, 'docs', 'capabilities-canonical', 'meta.json'), JSON.stringify({
    canonical_capabilities: 4,
  }))
  writeFileSync(join(root, 'docs', 'capabilities-canonical', 'domains', 'demo.jsonl'), [
    JSON.stringify({
      schema_version: 1,
      capability_key: 'demo.present',
      canonical_name: 'Present',
      domain: 'demo',
      target_crate: 'chronica-present',
      status: 'verified',
      moves_money: false,
      source_refs: [],
    }),
    JSON.stringify({
      schema_version: 1,
      capability_key: 'demo.missing_money',
      canonical_name: 'Missing Money',
      domain: 'demo',
      target_crate: 'chronica-missing',
      status: 'unimplemented',
      moves_money: true,
      source_refs: [],
    }),
    JSON.stringify({
      schema_version: 1,
      capability_key: 'demo.missing_verified',
      canonical_name: 'Missing Verified',
      domain: 'demo',
      target_crate: 'chronica-missing',
      status: 'verified',
      moves_money: false,
      source_refs: [],
    }),
    JSON.stringify({
      schema_version: 1,
      capability_key: 'demo.no_target',
      canonical_name: 'No Target',
      domain: 'demo',
      target_crate: null,
      status: 'unimplemented',
      moves_money: false,
      source_refs: [],
    }),
  ].join('\n') + '\n')

  const result = spawnSync(process.execPath, [queryScript, 'target-crate-census'], {
    cwd: root,
    encoding: 'utf8',
  })

  assert.equal(result.status, 0, result.stderr)
  const output = JSON.parse(result.stdout)
  assert.equal(output.capability_schema, 'canonical_jsonl_shards')
  assert.equal(output.known_crates_on_disk, 1)
  assert.deepEqual(output.target_crate_summary, {
    known_target_crate_rows: 1,
    nonexistent_target_crate_rows: 2,
    null_target_crate: 1,
    distinct_nonexistent_target_crates: 1,
  })
  assert.deepEqual(output.missing_target_crates, [{
    target_crate: 'chronica-missing',
    exists: false,
    rows: 2,
    verified: 1,
    implemented_unverified: 0,
    unimplemented: 1,
    blocked: 0,
    moves_money: 1,
    examples: [
      { key: 'demo.missing_money', status: 'unimplemented', domain: 'demo', moves_money: 1 },
      { key: 'demo.missing_verified', status: 'verified', domain: 'demo', moves_money: 0 },
    ],
  }])
})

test('target-crate-census filters one existing crate into an actionable queue', () => {
  const root = mkdtempSync(join(tmpdir(), 'chronica-query-target-crate-filter-'))
  mkdirSync(join(root, 'crates', 'chronica-security'), { recursive: true })
  mkdirSync(join(root, 'crates', 'chronica-browser'), { recursive: true })
  mkdirSync(join(root, 'docs', 'capabilities-canonical', 'domains'), { recursive: true })
  writeFileSync(join(root, 'docs', 'capabilities-canonical', 'meta.json'), JSON.stringify({
    canonical_capabilities: 4,
  }))
  writeFileSync(join(root, 'docs', 'capabilities-canonical', 'domains', 'demo.jsonl'), [
    JSON.stringify({
      schema_version: 1,
      capability_key: 'security.verified',
      canonical_name: 'Security Verified',
      domain: 'security',
      target_crate: 'chronica-security',
      status: 'verified',
      moves_money: false,
      source_refs: [{ id: 'donor-a' }],
    }),
    JSON.stringify({
      schema_version: 1,
      capability_key: 'security.money_unverified',
      canonical_name: 'Security Money Unverified',
      domain: 'security',
      target_crate: 'chronica-security',
      status: 'implemented_unverified',
      moves_money: true,
      source_refs: [{ id: 'donor-a' }],
    }),
    JSON.stringify({
      schema_version: 1,
      capability_key: 'security.unimplemented',
      canonical_name: 'Security Unimplemented',
      domain: 'security',
      target_crate: 'chronica-security',
      status: 'unimplemented',
      moves_money: false,
      source_refs: [{ id: 'donor-a' }, { id: 'donor-b' }],
      blocker: 'needs native caller',
    }),
    JSON.stringify({
      schema_version: 1,
      capability_key: 'browser.unimplemented',
      canonical_name: 'Browser Unimplemented',
      domain: 'browser',
      target_crate: 'chronica-browser',
      status: 'unimplemented',
      moves_money: false,
      source_refs: [{ id: 'donor-a' }],
    }),
  ].join('\n') + '\n')

  const result = spawnSync(process.execPath, [queryScript, 'target-crate-census', 'chronica-security', '--limit', '10'], {
    cwd: root,
    encoding: 'utf8',
  })

  assert.equal(result.status, 0, result.stderr)
  const output = JSON.parse(result.stdout)
  assert.equal(output.target_filter, 'chronica-security')
  assert.deepEqual(output.summary, {
    matching_rows: 3,
    matching_not_verified_rows: 2,
    matching_moves_money_rows: 1,
  })
  assert.deepEqual(output.matching_target_crates.map((row) => row.target_crate), ['chronica-security'])
  assert.deepEqual(output.actionable_rows.map((row) => row.key), [
    'security.money_unverified',
    'security.unimplemented',
  ])
  assert.equal(output.actionable_rows[1].blocker, 'needs native caller')
})

test('full-census summarizes every canonical JSONL row into build and hygiene queues', () => {
  const root = mkdtempSync(join(tmpdir(), 'chronica-query-full-census-'))
  mkdirSync(join(root, 'crates', 'chronica-present'), { recursive: true })
  mkdirSync(join(root, 'docs', 'capabilities-canonical', 'domains'), { recursive: true })
  writeFileSync(join(root, 'docs', 'capabilities-canonical', 'meta.json'), JSON.stringify({
    canonical_capabilities: 5,
  }))
  writeFileSync(join(root, 'docs', 'capabilities-canonical', 'domains', 'demo.jsonl'), [
    JSON.stringify({
      schema_version: 1,
      capability_key: 'demo.present_verified',
      canonical_name: 'Present Verified',
      domain: 'demo',
      target_crate: 'chronica-present',
      status: 'verified',
      moves_money: false,
      source_refs: [{ id: 'donor-a' }],
    }),
    JSON.stringify({
      schema_version: 1,
      capability_key: 'demo.present_money_unverified',
      canonical_name: 'Present Money Unverified',
      domain: 'demo',
      target_crate: 'chronica-present',
      status: 'implemented_unverified',
      moves_money: true,
      source_refs: [{ id: 'donor-a' }, { id: 'donor-b' }],
    }),
    JSON.stringify({
      schema_version: 1,
      capability_key: 'demo.present_unimplemented',
      canonical_name: 'Present Unimplemented',
      domain: 'demo',
      target_crate: 'chronica-present',
      status: 'unimplemented',
      moves_money: false,
      source_refs: [{ id: 'donor-a' }, { id: 'donor-b' }, { id: 'donor-c' }],
    }),
    JSON.stringify({
      schema_version: 1,
      capability_key: 'demo.missing_target',
      canonical_name: 'Missing Target',
      domain: 'other',
      target_crate: 'chronica-missing',
      status: 'unimplemented',
      moves_money: false,
      source_refs: [],
    }),
    JSON.stringify({
      schema_version: 1,
      capability_key: 'demo.null_target',
      canonical_name: 'Null Target',
      domain: 'other',
      target_crate: null,
      status: 'blocked',
      moves_money: false,
      source_refs: [],
    }),
  ].join('\n') + '\n')

  const result = spawnSync(process.execPath, [queryScript, 'full-census', '--limit', '10'], {
    cwd: root,
    encoding: 'utf8',
  })

  assert.equal(result.status, 0, result.stderr)
  const output = JSON.parse(result.stdout)
  assert.equal(output.capability_schema, 'canonical_jsonl_shards')
  assert.equal(output.canonical_rows, 5)
  assert.deepEqual(output.summary, {
    verified_rows: 1,
    not_verified_rows: 4,
    moves_money_rows: 1,
    known_target_crate_rows: 3,
    nonexistent_target_crate_rows: 1,
    null_target_crate_rows: 1,
    distinct_target_crates: 2,
    distinct_nonexistent_target_crates: 1,
  })
  assert.deepEqual(output.status_counts, [
    { key: 'unimplemented', rows: 2 },
    { key: 'blocked', rows: 1 },
    { key: 'implemented_unverified', rows: 1 },
    { key: 'verified', rows: 1 },
  ])
  assert.deepEqual(output.money_by_status, [{ key: 'implemented_unverified', rows: 1 }])
  assert.deepEqual(output.hottest_existing_crates, [{
    target_crate: 'chronica-present',
    exists: true,
    rows: 3,
    verified: 1,
    implemented_unverified: 1,
    unimplemented: 1,
    blocked: 0,
    unknown: 0,
    moves_money: 1,
  }])
  assert.deepEqual(output.actionable_existing_crate_queue.map((row) => row.key), [
    'demo.present_money_unverified',
    'demo.present_unimplemented',
  ])
  assert.deepEqual(output.missing_target_crates.map((row) => row.target_crate), ['chronica-missing'])
  assert.deepEqual(output.null_target_examples.map((row) => row.key), ['demo.null_target'])
})
