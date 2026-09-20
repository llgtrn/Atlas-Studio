import test from 'node:test'
import assert from 'node:assert/strict'
import { mkdtempSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join } from 'node:path'
import Database from 'better-sqlite3'
import { mulberry32, stratifiedSample, loadLinksByDomain } from './sample-donor-link-quality.mjs'

test('mulberry32 is deterministic for a fixed seed', () => {
  const a = mulberry32(42)
  const b = mulberry32(42)
  const seqA = Array.from({ length: 5 }, () => a())
  const seqB = Array.from({ length: 5 }, () => b())
  assert.deepEqual(seqA, seqB)
  for (const v of seqA) {
    assert.ok(v >= 0 && v < 1)
  }
})

test('stratifiedSample excludes already-reviewed canonical keys from the pool', () => {
  const rowsByDomain = {
    'infra-execution': [
      { canonicalKey: 'infra.a', sourceId: 1 },
      { canonicalKey: 'infra.b', sourceId: 2 },
      { canonicalKey: 'infra.c', sourceId: 3 },
    ],
  }
  const { sample, domainReport } = stratifiedSample(rowsByDomain, {
    perDomain: 10,
    excludedKeys: new Set(['infra.b']),
    rng: mulberry32(1),
  })
  assert.equal(sample.length, 2)
  assert.ok(!sample.some((r) => r.canonicalKey === 'infra.b'))
  assert.equal(domainReport['infra-execution'].pool_size, 2)
  assert.equal(domainReport['infra-execution'].excluded_from_pool, 1)
})

test('stratifiedSample caps sampled rows at perDomain even when pool is larger', () => {
  const rowsByDomain = {
    osint: Array.from({ length: 20 }, (_, i) => ({ canonicalKey: `osint.${i}`, sourceId: i })),
  }
  const { sample, domainReport } = stratifiedSample(rowsByDomain, { perDomain: 5, rng: mulberry32(7) })
  assert.equal(sample.length, 5)
  assert.equal(domainReport.osint.sampled, 5)
  assert.equal(domainReport.osint.pool_size, 20)
  const unique = new Set(sample.map((r) => r.sourceId))
  assert.equal(unique.size, 5, 'sample must not contain duplicate rows')
})

test('stratifiedSample is deterministic given the same seed and stable given a different one', () => {
  const rowsByDomain = {
    commerce: Array.from({ length: 12 }, (_, i) => ({ canonicalKey: `commerce.${i}`, sourceId: i })),
  }
  const runA = stratifiedSample(rowsByDomain, { perDomain: 4, rng: mulberry32(99) }).sample.map((r) => r.sourceId)
  const runB = stratifiedSample(rowsByDomain, { perDomain: 4, rng: mulberry32(99) }).sample.map((r) => r.sourceId)
  assert.deepEqual(runA, runB)
})

test('stratifiedSample samples independently per domain (no cross-domain starvation)', () => {
  const rowsByDomain = {
    a: [{ canonicalKey: 'a.1', sourceId: 1 }],
    b: [{ canonicalKey: 'b.1', sourceId: 2 }, { canonicalKey: 'b.2', sourceId: 3 }],
  }
  const { sample, domainReport } = stratifiedSample(rowsByDomain, { perDomain: 1, rng: mulberry32(3) })
  assert.equal(sample.length, 2, 'one pick from each of the two domains')
  assert.equal(domainReport.a.sampled, 1)
  assert.equal(domainReport.b.sampled, 1)
})

function makeSnapshotFixture() {
  const dir = mkdtempSync(join(tmpdir(), 'chronica-sample-donor-link-'))

  const core = new Database(join(dir, 'cap-core.db'))
  core.exec(`
    CREATE TABLE canonical_capability (
      id INTEGER PRIMARY KEY, key TEXT UNIQUE NOT NULL, canonical_name TEXT,
      domain TEXT, target_crate TEXT, moves_money INTEGER, status TEXT
    );
  `)
  core
    .prepare('INSERT INTO canonical_capability (id, key, canonical_name, domain, target_crate, moves_money, status) VALUES (?,?,?,?,?,?,?)')
    .run(1, 'infra.a', 'Infra Capability A', 'infra-execution', 'chronica-infra', 0, 'unimplemented')
  core
    .prepare('INSERT INTO canonical_capability (id, key, canonical_name, domain, target_crate, moves_money, status) VALUES (?,?,?,?,?,?,?)')
    .run(2, 'osint.a', 'OSINT Capability A', 'osint', 'chronica-osint', 0, 'unimplemented')
  core.close()

  const provenance = new Database(join(dir, 'cap-provenance.db'))
  provenance.exec(`
    CREATE TABLE source_capability (
      id INTEGER PRIMARY KEY, donor TEXT, canonical_name TEXT, source_files TEXT,
      business_behavior TEXT, technical_behavior TEXT, target_crate TEXT, canonical_id INTEGER
    );
  `)
  provenance
    .prepare('INSERT INTO source_capability (id, donor, canonical_name, source_files, business_behavior, technical_behavior, target_crate, canonical_id) VALUES (?,?,?,?,?,?,?,?)')
    .run(101, 'donor-x', 'Donor Feature X', 'src/x.rs', 'does x', 'implements x', 'chronica-x', 1)
  provenance
    .prepare('INSERT INTO source_capability (id, donor, canonical_name, source_files, business_behavior, technical_behavior, target_crate, canonical_id) VALUES (?,?,?,?,?,?,?,?)')
    .run(102, 'donor-y', 'Donor Feature Y', 'src/y.rs', 'does y', 'implements y', 'chronica-y', 2)
  provenance.close()

  return dir
}

test('loadLinksByDomain groups source_capability<->canonical_capability links by canonical domain', () => {
  const dir = makeSnapshotFixture()
  const byDomain = loadLinksByDomain({ dir })
  assert.deepEqual(Object.keys(byDomain).sort(), ['infra-execution', 'osint'])
  assert.equal(byDomain['infra-execution'].length, 1)
  assert.equal(byDomain['infra-execution'][0].canonicalKey, 'infra.a')
  assert.equal(byDomain['infra-execution'][0].donor, 'donor-x')
  assert.equal(byDomain.osint[0].canonicalKey, 'osint.a')
})
