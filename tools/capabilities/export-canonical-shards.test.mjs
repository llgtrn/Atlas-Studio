import test from 'node:test'
import assert from 'node:assert/strict'
import Database from 'better-sqlite3'
import { mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join } from 'node:path'

import { buildCapabilityDbFromCanonicalShards, computeCanonicalInputFingerprint } from './build-db-from-canonical-shards.mjs'
import { exportCanonicalCapabilityShards, parseExpectedChanges } from './export-canonical-shards.mjs'

// CONV0AR2: write authority is key AND FIELD bounded, not whole-record. exportCanonicalCapabilityShards()
// no longer does `mergedByKey.set(key, freshRecord)` for a declared key -- it starts from the
// EXISTING canonical record and replaces only the fields named in expectedChanges[key]. This
// matters because docs/capabilities.db is known-lossy even at build time (impl_evidence keeps only
// the first code_ref/test_ref per capability, see build-db-from-canonical-shards.mjs): a tool that
// only mutated status/blocker must never have the DB's compressed code_refs/test_refs/evidence_refs
// -- or any other field it didn't touch -- leak into canonical truth for that SAME key. The
// canonical_input_sha256 baseline fingerprint (CONV0AR) still protects against canonical edits made
// AFTER the DB was built; field-bounded patching protects against DB representation loss that
// already existed AT build time. Both are required; they solve different failure classes.

function makeRoot() {
  const root = mkdtempSync(join(tmpdir(), 'chronica-export-field-bounded-'))
  mkdirSync(join(root, 'docs', 'capabilities-canonical', 'domains'), { recursive: true })
  return root
}

function record(overrides = {}) {
  return {
    schema_version: 1,
    capability_key: 'alpha.one',
    canonical_name: 'Test capability',
    domain: 'alpha',
    target_crate: 'chronica-erp',
    target_module: 'test',
    status: 'implemented_unverified',
    side_effect_class: 'internal_write',
    moves_money: false,
    requires_approval: false,
    acceptance_criteria: 'Local audit must prove this is implemented.',
    required_tests: [],
    financial_control_test: null,
    acceptance_test: null,
    docs_refs: [],
    code_refs: [],
    test_refs: [],
    architecture_refs: [],
    source_refs: [],
    evidence_refs: [],
    blocker: null,
    ...overrides,
  }
}

const shardPath = (root) => join(root, 'docs', 'capabilities-canonical', 'domains', 'alpha.jsonl')
const writeShard = (root, rows) => writeFileSync(shardPath(root), rows.map((r) => JSON.stringify(r)).join('\n') + '\n')
const readShardRecords = (root) => readFileSync(shardPath(root), 'utf8').trim().split('\n').filter(Boolean).map((l) => JSON.parse(l))
const byKey = (root) => new Map(readShardRecords(root).map((r) => [r.capability_key, r]))

function writeMeta(root, rows) {
  const meta = {
    schema_version: 1, authority: 'canonical_jsonl', domain_count: 1,
    files: ['docs/capabilities-canonical/domains/alpha.jsonl'],
    generated_by: 'tools/capabilities/export-canonical-shards.mjs',
    implemented_unverified_count: rows.filter((r) => r.status === 'implemented_unverified').length,
    money_count: rows.filter((r) => r.moves_money).length,
    record_count: rows.length,
    source_db: 'docs/capabilities.db',
    verified_count: rows.filter((r) => r.status === 'verified').length,
  }
  writeFileSync(join(root, 'docs', 'capabilities-canonical', 'meta.json'), JSON.stringify(meta, null, 2) + '\n')
}

// Seeds canonical JSONL + a freshly-built (baseline-matching) capabilities.db from `rows`.
function seed(root, rows) {
  writeShard(root, rows)
  writeMeta(root, rows)
  const dbPath = join(root, 'docs', 'capabilities.db')
  buildCapabilityDbFromCanonicalShards({ root, outPath: dbPath })
  return dbPath
}

function mutateDb(dbPath, key, fields) {
  const db = new Database(dbPath)
  const sets = Object.keys(fields).map((f) => `${f}=@${f}`).join(', ')
  db.prepare(`UPDATE canonical_capability SET ${sets} WHERE key=@key`).run({ key, ...fields })
  db.close()
}

// ── fingerprint primitive ──────────────────────────────────────────────────────────────────────
test('computeCanonicalInputFingerprint: independent of input array order, sensitive to content', () => {
  const a = record({ capability_key: 'a' })
  const b = record({ capability_key: 'b', blocker: 'x' })
  assert.equal(computeCanonicalInputFingerprint([a, b]), computeCanonicalInputFingerprint([b, a]))
  assert.notEqual(computeCanonicalInputFingerprint([a, b]), computeCanonicalInputFingerprint([a, { ...b, blocker: 'y' }]))
})

test('computeCanonicalInputFingerprint: insensitive to object key insertion order, sensitive to array order', () => {
  const r1 = { capability_key: 'a', status: 'verified', blocker: null }
  const r2 = { blocker: null, status: 'verified', capability_key: 'a' }
  assert.equal(computeCanonicalInputFingerprint([r1]), computeCanonicalInputFingerprint([r2]))

  const withOrder = record({ capability_key: 'a', code_refs: ['A', 'B'] })
  const reordered = record({ capability_key: 'a', code_refs: ['B', 'A'] })
  assert.notEqual(computeCanonicalInputFingerprint([withOrder]), computeCanonicalInputFingerprint([reordered]))
})

// ── expectedChanges validation ─────────────────────────────────────────────────────────────────
test('parseExpectedChanges: rejects blank key, blank field, unknown field, nested path, wildcard field', () => {
  assert.throws(() => parseExpectedChanges({ '': ['status'] }), /blank\/non-string key/)
  assert.throws(() => parseExpectedChanges({ 'a.one': [''] }), /blank\/non-string field/)
  assert.throws(() => parseExpectedChanges({ 'a.one': ['not_a_real_field'] }), /unknown canonical field/)
  assert.throws(() => parseExpectedChanges({ 'a.one': ['evidence_refs.notes'] }), /no wildcard, no nested path/)
  assert.throws(() => parseExpectedChanges({ 'a.one': ['*'] }), /no wildcard, no nested path/)
  assert.throws(() => parseExpectedChanges({ 'a.one': [] }), /non-empty array/)
})

test('parseExpectedChanges: rejects capability_key/schema_version as patchable fields (identity, not state)', () => {
  assert.throws(() => parseExpectedChanges({ 'a.one': ['capability_key'] }), /unknown canonical field/)
  assert.throws(() => parseExpectedChanges({ 'a.one': ['schema_version'] }), /unknown canonical field/)
})

test('parseExpectedChanges: an exact duplicate field name in the same list is harmlessly de-duplicated', () => {
  const changes = parseExpectedChanges({ 'a.one': ['status', 'status', 'blocker'] })
  assert.deepEqual([...changes.get('a.one')].sort(), ['blocker', 'status'])
})

// ── baseline fence: happy path ─────────────────────────────────────────────────────────────────
test('a no-op export right after a DB build (empty expectedChanges) leaves the corpus unchanged', () => {
  const root = makeRoot()
  const rows = [record({ capability_key: 'alpha.one' }), record({ capability_key: 'alpha.two', blocker: 'x' })]
  const dbPath = seed(root, rows)
  try {
    const result = exportCanonicalCapabilityShards({ root, dbPath, expectedChanges: {} })
    assert.equal(result.record_count, 2)
    assert.deepEqual([...byKey(root).values()], rows)
  } finally {
    rmSync(root, { recursive: true, force: true })
  }
})

test('a single-field promotion succeeds and only changes its own declared field on its own declared key', () => {
  const root = makeRoot()
  const dbPath = seed(root, [record({ capability_key: 'alpha.one' }), record({ capability_key: 'alpha.two' })])
  try {
    mutateDb(dbPath, 'alpha.one', { status: 'verified' })
    const result = exportCanonicalCapabilityShards({ root, dbPath, expectedChanges: { 'alpha.one': ['status'] } })
    assert.equal(result.record_count, 2)
    assert.equal(byKey(root).get('alpha.one').status, 'verified')
    assert.equal(byKey(root).get('alpha.two').status, 'implemented_unverified')
  } finally {
    rmSync(root, { recursive: true, force: true })
  }
})

// ── THE key architectural property: field-bounded patching on the SAME key ───────────────────────
test('SAME-KEY: a status/blocker-only mutation preserves multi-value code_refs/test_refs/evidence_refs', () => {
  const root = makeRoot()
  const target = record({
    capability_key: 'alpha.one',
    code_refs: ['a.rs#A', 'b.rs#B'],
    test_refs: ['a_test.rs#T1', 'b_test.rs#T2'],
    evidence_refs: ['donor_source:x', 'verified_at:2026-01-01', 'verified_by:loop', 'notes:extra'],
    status: 'implemented_unverified',
    blocker: null,
  })
  const dbPath = seed(root, [target])
  try {
    // The DB representation is naturally lossy for code_refs/test_refs/evidence_refs (impl_evidence
    // keeps only one row per key) -- that must not matter for a mutation authorized only for
    // status/blocker.
    mutateDb(dbPath, 'alpha.one', { status: 'verified' })
    const db = new Database(dbPath)
    db.prepare("UPDATE canonical_status_override SET blocker='cleared' WHERE canonical_key='alpha.one'").run()
    if (db.prepare("SELECT count(*) n FROM canonical_status_override WHERE canonical_key='alpha.one'").get().n === 0) {
      db.prepare("INSERT INTO canonical_status_override (canonical_key, blocker) VALUES ('alpha.one', 'cleared')").run()
    }
    db.close()

    exportCanonicalCapabilityShards({ root, dbPath, expectedChanges: { 'alpha.one': ['status', 'blocker'] } })

    const after = byKey(root).get('alpha.one')
    assert.equal(after.status, 'verified')
    assert.equal(after.blocker, 'cleared')
    assert.deepEqual(after.code_refs, target.code_refs)
    assert.deepEqual(after.test_refs, target.test_refs)
    assert.deepEqual(after.evidence_refs, target.evidence_refs)
  } finally {
    rmSync(root, { recursive: true, force: true })
  }
})

test('SAME-KEY: a blocker/status-only mutation preserves a stale-in-DB target_crate (the #5244 shape)', () => {
  const root = makeRoot()
  const target = record({ capability_key: 'alpha.one', target_crate: 'canonical-new-home', status: 'implemented_unverified' })
  const dbPath = seed(root, [target])
  try {
    // DB's target_crate is stale relative to canonical -- this export is authorized for
    // status/blocker only, so the stale DB value must never reach canonical truth.
    mutateDb(dbPath, 'alpha.one', { status: 'verified', target_crate: 'deleted-old-home' })
    exportCanonicalCapabilityShards({ root, dbPath, expectedChanges: { 'alpha.one': ['status'] } })
    const after = byKey(root).get('alpha.one')
    assert.equal(after.status, 'verified')
    assert.equal(after.target_crate, 'canonical-new-home')
  } finally {
    rmSync(root, { recursive: true, force: true })
  }
})

test('SAME-KEY: the mechanism is generic -- an unauthorized field survives even with no special-case name', () => {
  const root = makeRoot()
  // acceptance_criteria is not evidence, not target_crate, and has no dedicated preservation test
  // elsewhere -- proving the field-bounded model is generic, not a hardcoded "preserve evidence" rule.
  const target = record({ capability_key: 'alpha.one', acceptance_criteria: 'ORIGINAL CRITERIA TEXT' })
  const dbPath = seed(root, [target])
  try {
    mutateDb(dbPath, 'alpha.one', { status: 'verified', acceptance_criteria: 'DB-side stale rewrite' })
    exportCanonicalCapabilityShards({ root, dbPath, expectedChanges: { 'alpha.one': ['status'] } })
    const after = byKey(root).get('alpha.one')
    assert.equal(after.status, 'verified')
    assert.equal(after.acceptance_criteria, 'ORIGINAL CRITERIA TEXT')
  } finally {
    rmSync(root, { recursive: true, force: true })
  }
})

test('SAME-KEY: unauthorized DB field drift never reaches canonical output (no error, just not published)', () => {
  const root = makeRoot()
  const target = record({ capability_key: 'alpha.one', code_refs: ['a.rs#A', 'b.rs#B'], blocker: null })
  const dbPath = seed(root, [target])
  try {
    mutateDb(dbPath, 'alpha.one', { blocker: 'blocked' })
    const result = exportCanonicalCapabilityShards({ root, dbPath, expectedChanges: { 'alpha.one': ['blocker'] } })
    assert.equal(result.record_count, 1)
    const after = byKey(root).get('alpha.one')
    assert.equal(after.blocker, 'blocked')
    assert.deepEqual(after.code_refs, target.code_refs)
  } finally {
    rmSync(root, { recursive: true, force: true })
  }
})

// ── baseline fence: refuses ANY post-build canonical edit ─────────────────────────────────────
for (const [field, before, after] of [
  ['code_refs', ['a.rs#foo'], ['a.rs#foo', 'b.rs#bar']],
  ['test_refs', ['a.rs#t1'], ['a.rs#t1', 'a.rs#t2']],
  ['evidence_refs', ['donor_source:x'], ['donor_source:x', 'notes:y']],
]) {
  test(`an out-of-band ${field} edit on an UNDECLARED key after DB build is rejected before any write`, () => {
    const root = makeRoot()
    const rows = [record({ capability_key: 'alpha.one' }), record({ capability_key: 'alpha.two', [field]: before })]
    const dbPath = seed(root, rows)
    try {
      writeShard(root, rows.map((r) => (r.capability_key === 'alpha.two' ? { ...r, [field]: after } : r)))
      mutateDb(dbPath, 'alpha.one', { status: 'verified' })
      assert.throws(
        () => exportCanonicalCapabilityShards({ root, dbPath, expectedChanges: { 'alpha.one': ['status'] } }),
        /STALE_CAPABILITIES_DB_BASELINE_REFUSED/,
      )
      assert.deepEqual(byKey(root).get('alpha.two')[field], after, 'the out-of-band edit itself must survive the refusal')
      assert.equal(byKey(root).get('alpha.one').status, 'implemented_unverified', 'no write happened at all')
    } finally {
      rmSync(root, { recursive: true, force: true })
    }
  })
}

test('expectedChanges is not permission to overwrite a NEWER canonical edit on the SAME key', () => {
  const root = makeRoot()
  const dbPath = seed(root, [record({ capability_key: 'alpha.one' })])
  try {
    writeShard(root, [record({ capability_key: 'alpha.one', blocker: 'hand-edited after DB build' })])
    mutateDb(dbPath, 'alpha.one', { status: 'verified' })
    assert.throws(
      () => exportCanonicalCapabilityShards({ root, dbPath, expectedChanges: { 'alpha.one': ['status'] } }),
      /STALE_CAPABILITIES_DB_BASELINE_REFUSED/,
    )
    assert.equal(byKey(root).get('alpha.one').blocker, 'hand-edited after DB build')
  } finally {
    rmSync(root, { recursive: true, force: true })
  }
})

// ── rollback lifecycle across baseline advancement, WITHOUT broadening field authority ───────────
test('promotion + rollback round-trips canonical A back to its original state using the SAME field authority; B never changes', () => {
  const root = makeRoot()
  const original = record({ capability_key: 'alpha.one' })
  const unrelated = record({ capability_key: 'alpha.two', blocker: 'must survive' })
  const dbPath = seed(root, [original, unrelated])
  const forwardFields = { 'alpha.one': ['status'] }
  try {
    mutateDb(dbPath, 'alpha.one', { status: 'verified' })
    exportCanonicalCapabilityShards({ root, dbPath, expectedChanges: forwardFields }) // baseline -> H1
    assert.equal(byKey(root).get('alpha.one').status, 'verified')

    // downstream verification fails -> roll the DB row back; rollback declares the IDENTICAL field
    // set as the forward call, never a broader one.
    mutateDb(dbPath, 'alpha.one', { status: original.status })
    exportCanonicalCapabilityShards({ root, dbPath, expectedChanges: forwardFields }) // baseline -> H0-equivalent

    assert.deepEqual(byKey(root).get('alpha.one'), original)
    assert.deepEqual(byKey(root).get('alpha.two'), unrelated)
  } finally {
    rmSync(root, { recursive: true, force: true })
  }
})

// ── write-authorization boundaries ────────────────────────────────────────────────────────────
test('a declared key that does not exist in the current canonical corpus is refused (no capability creation)', () => {
  const root = makeRoot()
  const dbPath = seed(root, [record({ capability_key: 'alpha.one' })])
  try {
    const db = new Database(dbPath)
    db.prepare(`INSERT INTO canonical_capability (id, key, canonical_name, domain, side_effect_class, moves_money, requires_approval, acceptance_criteria, status)
      VALUES (2, 'alpha.brand-new', 'Brand new', 'alpha', 'internal_write', 0, 0, 'x', 'unimplemented')`).run()
    db.close()
    assert.throws(
      () => exportCanonicalCapabilityShards({ root, dbPath, expectedChanges: { 'alpha.brand-new': ['status'] } }),
      /capability creation is not authorized/,
    )
  } finally {
    rmSync(root, { recursive: true, force: true })
  }
})

test('a declared key that does not exist in the DB is refused', () => {
  const root = makeRoot()
  const dbPath = seed(root, [record({ capability_key: 'alpha.one' })])
  try {
    assert.throws(
      () => exportCanonicalCapabilityShards({ root, dbPath, expectedChanges: { 'alpha.nonexistent': ['status'] } }),
      /does not exist in the current canonical corpus/,
    )
  } finally {
    rmSync(root, { recursive: true, force: true })
  }
})

test('a blank expectedChanges key is refused up front', () => {
  const root = makeRoot()
  const dbPath = seed(root, [record({ capability_key: 'alpha.one' })])
  try {
    assert.throws(
      () => exportCanonicalCapabilityShards({ root, dbPath, expectedChanges: { 'alpha.one': ['status'], '': ['blocker'] } }),
      /blank\/non-string key/,
    )
  } finally {
    rmSync(root, { recursive: true, force: true })
  }
})

test('a literal "*" is never a wildcard key or field', () => {
  const root = makeRoot()
  const rows = [record({ capability_key: 'alpha.one' }), record({ capability_key: 'alpha.two' })]
  const dbPath = seed(root, rows)
  try {
    writeShard(root, rows.map((r) => (r.capability_key === 'alpha.two' ? { ...r, blocker: 'out of band' } : r)))
    mutateDb(dbPath, 'alpha.one', { status: 'verified' })
    // "*" as a key: does not exist in the canonical corpus -> refused, not a bypass.
    assert.throws(
      () => exportCanonicalCapabilityShards({ root, dbPath, expectedChanges: { '*': ['status'] } }),
      /STALE_CAPABILITIES_DB_BASELINE_REFUSED|does not exist in the current canonical corpus/,
    )
    // "*" as a field name: explicitly rejected by validation.
    assert.throws(
      () => exportCanonicalCapabilityShards({ root, dbPath, expectedChanges: { 'alpha.one': ['*'] } }),
      /no wildcard, no nested path/,
    )
  } finally {
    rmSync(root, { recursive: true, force: true })
  }
})

test('trustFullDb and expectedChangedKeys are structurally gone, not just unused (source check)', () => {
  const src = readFileSync(new URL('./export-canonical-shards.mjs', import.meta.url), 'utf8')
  assert.ok(!/trustFullDb|trust-full-db/.test(src), 'trustFullDb escape hatch must not be reintroduced')
  assert.ok(!/expectedChangedKeys/.test(src), 'whole-record expectedChangedKeys authority must not be reintroduced')
})

// ── #5244-shaped simulation: one capability changes many fields at once, field-bounded ───────────
test('a #5244-shaped promotion (explicit field list) touches only its declared fields on its declared key', () => {
  const root = makeRoot()
  const target = record({ capability_key: 'alpha.target', status: 'implemented_unverified', target_crate: 'chronica-old', blocker: 'STALE_BLOCKER' })
  const siblings = Array.from({ length: 4 }, (_, i) => record({ capability_key: `alpha.sibling${i}`, blocker: `keep-${i}` }))
  const dbPath = seed(root, [...siblings, target].sort((a, b) => a.capability_key.localeCompare(b.capability_key)))
  try {
    const db = new Database(dbPath)
    db.prepare("UPDATE canonical_capability SET status='verified', target_crate='chronica-new', blocker=NULL WHERE key='alpha.target'").run()
    db.prepare(`INSERT INTO impl_evidence (canonical_key, impl_file, impl_symbols, test_file, test_symbol, donor_source, verified_at, verified_by, notes)
      VALUES ('alpha.target', 'crates/core/chronica-new/src/lib.rs', 'find_documents', 'crates/core/chronica-new/src/lib.rs', 'find_documents_test', 'documenso/documenso', '2026-01-01T00:00:00Z', 'loop', 'repaired')`).run()
    db.close()

    // Explicit field list -- exactly what a #5244-shaped promotion intends to change, target_crate
    // included ONLY because it is declared here (never implied merely because the DB has a value).
    exportCanonicalCapabilityShards({
      root, dbPath,
      expectedChanges: { 'alpha.target': ['status', 'target_crate', 'blocker', 'code_refs', 'test_refs', 'evidence_refs'] },
    })

    const after = byKey(root)
    assert.equal(after.get('alpha.target').status, 'verified')
    assert.equal(after.get('alpha.target').target_crate, 'chronica-new')
    assert.equal(after.get('alpha.target').blocker, null)
    assert.deepEqual(after.get('alpha.target').code_refs, ['crates/core/chronica-new/src/lib.rs#find_documents'])
    for (const sibling of siblings) assert.deepEqual(after.get(sibling.capability_key), sibling)
  } finally {
    rmSync(root, { recursive: true, force: true })
  }
})
