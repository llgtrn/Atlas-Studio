import assert from 'node:assert/strict'
import test from 'node:test'
import {
  buildReviewQueueMarkdown,
  buildReviewQueueRecords,
  isCategoryBFinding,
  reasonLabel,
  suggestedNextStep,
} from './subdb-conflict-writer-lib.mjs'
import { redactValue } from './sync-anchor-v2-lib.mjs'

function agree(overrides = {}) {
  return { verdict: 'AGREE', capability_key: 'x.agree', crate: 'chronica-x', evidence: {}, ...overrides }
}

function disagree(reason, overrides = {}) {
  return {
    verdict: 'DISAGREE',
    capability_key: overrides.capability_key ?? `x.${reason.toLowerCase()}`,
    crate: overrides.crate ?? 'chronica-x',
    target_module: overrides.target_module ?? 'some_mod',
    shard_status: overrides.shard_status ?? 'unimplemented',
    doc_status: overrides.doc_status ?? null,
    reason,
    evidence: { match_confidence: 'exact', matched_modules: ['src/some_mod.rs'], code_state: 'real', reachable: true, ...overrides.evidence },
  }
}

test('isCategoryBFinding: excludes AGREE and EXACT-confidence auto-correctable findings', () => {
  assert.equal(isCategoryBFinding(agree()), false)
  assert.equal(
    isCategoryBFinding(disagree('SHARD_CLAIMS_UNIMPLEMENTED_BUT_CODE_IS_REAL', { evidence: { match_confidence: 'exact' } })),
    false,
  )
})

test('isCategoryBFinding: includes PREFIX-confidence deferred findings, ISLAND, overclaim, doc mismatch', () => {
  assert.equal(
    isCategoryBFinding(disagree('SHARD_CLAIMS_UNIMPLEMENTED_BUT_CODE_IS_REAL', { evidence: { match_confidence: 'prefix' } })),
    true,
  )
  assert.equal(isCategoryBFinding(disagree('CODE_EXISTS_BUT_UNREACHABLE_FROM_ANY_ENTRY_POINT')), true)
  assert.equal(isCategoryBFinding(disagree('SHARD_CLAIMS_IMPLEMENTED_BUT_NO_REAL_CODE_FOUND')), true)
  assert.equal(isCategoryBFinding(disagree('DOC_STATUS_CONTRADICTS_SHARD_STATUS')), true)
})

test('suggestedNextStep: ISLAND mentions wiring in vs deleting without leaking the matched file', () => {
  const step = suggestedNextStep(disagree('CODE_EXISTS_BUT_UNREACHABLE_FROM_ANY_ENTRY_POINT', { crate: 'chronica-cli' }))
  assert.match(step, /wire it into/i)
  assert.match(step, /delete\/deprecate/i)
  assert.ok(!step.includes('src/some_mod.rs'))
})

test('suggestedNextStep: overclaim asks to confirm shard vs stale doc claim', () => {
  const step = suggestedNextStep(disagree('SHARD_CLAIMS_IMPLEMENTED_BUT_NO_REAL_CODE_FOUND', { shard_status: 'implemented' }))
  assert.match(step, /overclaim/i)
  assert.match(step, /confirm the shard status/i)
})

test('suggestedNextStep: PREFIX-confidence match needs human confirmation before promotion', () => {
  const step = suggestedNextStep(
    disagree('SHARD_CLAIMS_UNIMPLEMENTED_BUT_CODE_IS_REAL', { evidence: { match_confidence: 'prefix' } }),
  )
  assert.match(step, /PREFIX-confidence/)
  assert.match(step, /subdb-apply-agreed-corrections\.mjs/)
})

test('suggestedNextStep: doc/shard mismatch names both conflicting values', () => {
  const step = suggestedNextStep(
    disagree('DOC_STATUS_CONTRADICTS_SHARD_STATUS', { doc_status: 'unimplemented', shard_status: 'implemented' }),
  )
  assert.match(step, /"unimplemented"/)
  assert.match(step, /"implemented"/)
})

test('suggestedNextStep: unknown reason codes still produce a safe fallback', () => {
  const step = suggestedNextStep(disagree('SOME_FUTURE_REASON_CODE'))
  assert.match(step, /needs local audit review/i)
})

test('buildReviewQueueRecords: only Category-B findings survive, with the required fields', () => {
  const findings = [
    agree(),
    disagree('SHARD_CLAIMS_UNIMPLEMENTED_BUT_CODE_IS_REAL', { evidence: { match_confidence: 'exact' } }),
    disagree('CODE_EXISTS_BUT_UNREACHABLE_FROM_ANY_ENTRY_POINT', { capability_key: 'x.island', crate: 'chronica-cli' }),
  ]
  const records = buildReviewQueueRecords(findings)
  assert.equal(records.length, 1)
  const [record] = records
  assert.equal(record.capability_key, 'x.island')
  assert.equal(record.crate, 'chronica-cli')
  assert.equal(record.reason_code, 'CODE_EXISTS_BUT_UNREACHABLE_FROM_ANY_ENTRY_POINT')
  assert.equal(record.match_confidence, 'exact')
  assert.equal(record.file, redactValue())
  assert.ok(typeof record.suggested_next_step === 'string' && record.suggested_next_step.length > 0)
})

test('buildReviewQueueRecords: untrusted fields are sanitized before JSONL/Markdown output', () => {
  const secret = 'sk_live_super_secret_that_must_not_render'
  const [record] = buildReviewQueueRecords([
    disagree('SHARD_CLAIMS_IMPLEMENTED_BUT_NO_REAL_CODE_FOUND', {
      capability_key: secret,
      crate: 'chronica secret',
      target_module: secret,
      evidence: { match_confidence: secret, matched_modules: [`src/${secret}.rs`] },
    }),
  ])
  assert.equal(record.capability_key, redactValue())
  assert.equal(record.crate, redactValue())
  assert.equal(record.match_confidence, redactValue())
  assert.equal(record.target_module, redactValue())
  assert.equal(record.file, redactValue())
  assert.ok(!JSON.stringify(record).includes(secret), 'queue output must not echo raw finding content')
})

test('buildReviewQueueRecords: unknown reason codes are bucketed without echoing the raw code', () => {
  const secretReason = 'SECRET_REASON_sk_live_value'
  const [record] = buildReviewQueueRecords([disagree(secretReason, { capability_key: 'safe.key' })])
  assert.equal(record.reason_code, 'UNKNOWN_REASON_CODE')
  assert.ok(!JSON.stringify(record).includes(secretReason))
})

test('buildReviewQueueRecords: findings with no matched module still get a file: null, not a crash', () => {
  const finding = disagree('SHARD_CLAIMS_IMPLEMENTED_BUT_NO_REAL_CODE_FOUND', { evidence: { matched_modules: [] } })
  const [record] = buildReviewQueueRecords([finding])
  assert.equal(record.file, null)
})

test('buildReviewQueueMarkdown: groups by crate, sorted by finding count descending', () => {
  const records = buildReviewQueueRecords([
    disagree('CODE_EXISTS_BUT_UNREACHABLE_FROM_ANY_ENTRY_POINT', { capability_key: 'a.1', crate: 'chronica-small' }),
    disagree('CODE_EXISTS_BUT_UNREACHABLE_FROM_ANY_ENTRY_POINT', { capability_key: 'b.1', crate: 'chronica-big' }),
    disagree('SHARD_CLAIMS_IMPLEMENTED_BUT_NO_REAL_CODE_FOUND', { capability_key: 'b.2', crate: 'chronica-big' }),
  ])
  const markdown = buildReviewQueueMarkdown(records, { date: '2026-07-31' })
  assert.match(markdown, /Subdb Conflict Review Queue -- 2026-07-31/)
  assert.match(markdown, /\*\*3 finding\(s\) across 2 crate\(s\)\.\*\*/)
  const bigIndex = markdown.indexOf('### chronica-big (2)')
  const smallIndex = markdown.indexOf('### chronica-small (1)')
  assert.ok(bigIndex >= 0 && smallIndex >= 0 && bigIndex < smallIndex, 'crate with more findings must be listed first')
})

test('buildReviewQueueMarkdown: reason-code breakdown table sums to the total finding count', () => {
  const records = buildReviewQueueRecords([
    disagree('CODE_EXISTS_BUT_UNREACHABLE_FROM_ANY_ENTRY_POINT', { capability_key: 'a.1' }),
    disagree('CODE_EXISTS_BUT_UNREACHABLE_FROM_ANY_ENTRY_POINT', { capability_key: 'a.2' }),
    disagree('SHARD_CLAIMS_IMPLEMENTED_BUT_NO_REAL_CODE_FOUND', { capability_key: 'a.3' }),
  ])
  const markdown = buildReviewQueueMarkdown(records, { date: '2026-07-31' })
  assert.match(markdown, /\| `CODE_EXISTS_BUT_UNREACHABLE_FROM_ANY_ENTRY_POINT` \| ISLAND \| 2 \|/)
  assert.match(markdown, /\| `SHARD_CLAIMS_IMPLEMENTED_BUT_NO_REAL_CODE_FOUND` \| overclaim \| 1 \|/)
})

test('reasonLabel: falls back to the raw code for unknown reasons', () => {
  assert.equal(reasonLabel('SOMETHING_NEW'), 'SOMETHING_NEW')
})
