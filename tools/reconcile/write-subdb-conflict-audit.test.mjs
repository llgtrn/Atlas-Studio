import test from 'node:test'
import assert from 'node:assert/strict'
import { mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join } from 'node:path'
import { MAX_CONFLICTS, normalizeConflictAudit, writeConflictAudit } from './write-subdb-conflict-audit.mjs'

function input() {
  return {
    source: 'docs/_machine/subdb-collection.json',
    conflicts: [{ id: 'cap.demo.money', class: 'MONEY_CLASSIFICATION_MISMATCH', severity: 'MONEY_BLOCKED', summary: 'Shard classifications disagree.', evidence: ['crate-a row', 'crate-b row'] }],
    agreed_corrections: [{ conflict_id: 'cap.demo.money', correction: 'Keep blocked pending local DB review.', agreed_by: 'brain-and-human-review', evidence: ['review record 17'] }],
  }
}

test('writes paired bounded audit files without applying agreed corrections', () => {
  const root = mkdtempSync(join(tmpdir(), 'chronica-conflict-audit-'))
  try {
    const inputPath = join(root, 'input.json')
    writeFileSync(inputPath, JSON.stringify(input()))
    const result = writeConflictAudit({ inputPath, outDir: join(root, 'out'), timestamp: '2026-08-02T12:34:56.000Z' })
    const json = JSON.parse(readFileSync(result.json_path, 'utf8'))
    assert.equal(json.local_aggregate_status, 'LOCAL_AUDIT_REQUIRED')
    assert.equal(json.agreed_corrections[0].application_status, 'NOT_APPLIED_LOCAL_AUDIT_REQUIRED')
    assert.match(readFileSync(result.markdown_path, 'utf8'), /Agreed corrections \(not applied\): 1/)
    assert.throws(() => writeConflictAudit({ inputPath, outDir: join(root, 'out'), timestamp: '2026-08-02T12:34:56.000Z' }), /refusing to overwrite/)
  } finally {
    rmSync(root, { recursive: true, force: true })
  }
})

test('fails closed for severity, unknown correction references, and excessive entries', () => {
  const wrongSeverity = input()
  wrongSeverity.conflicts[0].severity = 'WARNING'
  assert.throws(() => normalizeConflictAudit(wrongSeverity), /must be MONEY_BLOCKED/)

  const unknown = input()
  unknown.agreed_corrections[0].conflict_id = 'missing'
  assert.throws(() => normalizeConflictAudit(unknown), /references unknown conflict/)

  const excessive = input()
  excessive.conflicts = Array.from({ length: MAX_CONFLICTS + 1 }, (_, index) => ({ ...input().conflicts[0], id: `conflict-${index}` }))
  assert.throws(() => normalizeConflictAudit(excessive), /exceeds the limit/)
})

test('does not permit duplicate conflict or correction identities', () => {
  const duplicateConflict = input()
  duplicateConflict.conflicts.push({ ...duplicateConflict.conflicts[0] })
  assert.throws(() => normalizeConflictAudit(duplicateConflict), /duplicate conflict id/)

  const duplicateCorrection = input()
  duplicateCorrection.agreed_corrections.push({ ...duplicateCorrection.agreed_corrections[0] })
  assert.throws(() => normalizeConflictAudit(duplicateCorrection), /duplicate agreed correction/)
})
