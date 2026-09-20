import { test } from 'node:test'
import assert from 'node:assert/strict'
import { loadChecklist, validateChecklist, generateReport } from './donor-corpus.mjs'

test('the committed donor-corpus.yaml passes its own validator', () => {
  const doc = loadChecklist()
  const errors = validateChecklist(doc)
  assert.deepEqual(errors, [])
})

test('the committed donor-corpus.yaml has no unmarked duplicate canonical_upstream', () => {
  const doc = loadChecklist()
  const seen = new Map()
  for (const d of doc.donors) {
    const key = d.canonical_upstream.toLowerCase().replace(/\.git$/, '').replace(/\/$/, '')
    assert.equal(seen.has(key), false, `duplicate canonical_upstream: ${d.canonical_upstream} (also ${seen.get(key)})`)
    seen.set(key, d.repo)
  }
})

test('validator rejects a malformed doc (wrong schema_version)', () => {
  const errors = validateChecklist({ schema_version: 2, donors: [] })
  assert.ok(errors.some((e) => e.includes('schema_version')))
})

test('validator rejects a donor row missing required fields', () => {
  const errors = validateChecklist({ schema_version: 1, donors: [{ repo: 'foo/bar' }] })
  assert.ok(errors.length > 0)
})

test('validator rejects an invalid status value', () => {
  const errors = validateChecklist({
    schema_version: 1,
    donors: [
      {
        id: 'D001',
        repo: 'foo/bar',
        canonical_upstream: 'https://github.com/foo/bar',
        family: ['OTHER'],
        status: 'NOT_A_REAL_STATUS',
        needed: 'UNKNOWN',
        source_present: false,
        census_complete: false,
        absorption_state: 'none',
        can_delete_source: false,
      },
    ],
  })
  assert.ok(errors.some((e) => e.includes('invalid status')))
})

test('validator flags an unmarked duplicate canonical_upstream between two different rows', () => {
  const row = (id, repo) => ({
    id,
    repo,
    canonical_upstream: 'https://github.com/foo/bar',
    family: ['OTHER'],
    status: 'DISCOVERED',
    needed: 'UNKNOWN',
    source_present: false,
    census_complete: false,
    absorption_state: 'none',
    can_delete_source: false,
  })
  const errors = validateChecklist({ schema_version: 1, donors: [row('D001', 'foo/bar'), row('D002', 'foo/bar-mirror')] })
  assert.ok(errors.some((e) => e.includes('duplicate canonical_upstream')))
})

test('generateReport produces a Markdown doc with totals and a full donor table', () => {
  const doc = {
    generated_at: '2026-01-01T00:00:00.000Z',
    source_evidence: ['test fixture'],
    donors: [
      {
        id: 'D001',
        repo: 'foo/bar',
        canonical_upstream: 'https://github.com/foo/bar',
        family: ['OTHER'],
        status: 'DISCOVERED',
        needed: 'UNKNOWN',
        source_present: false,
        absorption_state: 'none',
      },
    ],
  }
  const report = generateReport(doc)
  assert.match(report, /Total unique donors:\*\* 1/)
  assert.match(report, /foo\/bar/)
  assert.match(report, /DISCOVERED/)
})


test('validator rejects terminal absorption when the technology census is incomplete', () => {
  const errors = validateChecklist({
    schema_version: 1,
    donors: [
      {
        id: 'D900',
        repo: 'example/foundation',
        canonical_upstream: 'https://github.com/example/foundation',
        family: ['DURABLE_STATE'],
        status: 'ABSORBED',
        needed: 'YES',
        source_present: false,
        census_complete: false,
        absorption_state: 'complete',
        can_delete_source: true,
      },
    ],
  })
  assert.ok(errors.some((e) => e.includes('terminal status "ABSORBED" requires census_complete: true')))
  assert.ok(errors.some((e) => e.includes('absorption_state "complete" requires census_complete: true')))
})

test('validator blocks premature extinction of promoted foundational technology', () => {
  const errors = validateChecklist({
    schema_version: 1,
    donors: [
      {
        id: 'D901',
        repo: 'example/foundation',
        canonical_upstream: 'https://github.com/example/foundation',
        family: ['DURABLE_STATE'],
        status: 'PARTIALLY_ABSORBED',
        needed: 'YES',
        source_present: true,
        census_complete: false,
        absorption_state: 'partial',
        can_delete_source: true,
        native_promotion: {
          strategic_class: 'FOUNDATIONAL',
          lifecycle_state: 'NATIVE_PROTOTYPE',
          extinction_eligible: true,
        },
      },
    ],
  })
  assert.ok(errors.some((e) => e.includes('cannot set can_delete_source: true')))
  assert.ok(errors.some((e) => e.includes('cannot be extinction_eligible')))
  assert.ok(errors.some((e) => e.includes('extinction_eligible requires census_complete: true')))
})

test('D237 stays partial and non-extinct after exact pinned source rehydration', () => {
  const doc = loadChecklist()
  const d237 = doc.donors.find((d) => d.id === 'D237')
  assert.ok(d237)
  assert.equal(d237.status, 'PARTIALLY_ABSORBED')
  assert.equal(d237.absorption_state, 'partial')
  assert.equal(d237.census_complete, false)
  assert.equal(d237.source_present, true)
  assert.equal(d237.temporary_path, 'temporary/donors/D237-postgres-postgres/source')
  assert.equal(d237.can_delete_source, false)
  assert.equal(d237.native_promotion?.technology_id, 'chronica.durable_world_engine')
  assert.equal(d237.native_promotion?.strategic_class, 'FOUNDATIONAL')
  assert.equal(d237.native_promotion?.lifecycle_state, 'NATIVE_PROTOTYPE')
  assert.equal(d237.native_promotion?.extinction_eligible, false)
  assert.equal(d237.native_promotion?.source_rehydration_required, false)
  assert.equal(d237.native_promotion?.proof_status, 'P0_CENSUS_PROOF_IN_PROGRESS')
})
