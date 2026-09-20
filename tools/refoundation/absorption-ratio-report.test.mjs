import { test } from 'node:test'
import assert from 'node:assert/strict'
import { execFileSync } from 'node:child_process'
import { buildRatioReport } from './absorption-ratio-report.mjs'

test('buildRatioReport produces a report against the real repo history with a small real range', () => {
  // Use the two most recent real commits as a cheap, always-valid range rather than a synthetic
  // fixture -- this module reads real Git history/tree state by design, so its own test proves
  // it runs end-to-end without throwing, not that every ratio has a specific value (those move
  // every time the corpus/tree changes).
  const log = execFileSync('git', ['log', '--format=%H', '-n', '2'], { encoding: 'utf8' }).trim().split('\n')
  const [finalSha, baseSha] = log
  const lines = buildRatioReport({
    baseSha,
    finalSha,
    live: {
      candidatesPlanned: 1,
      buildersCompleted: 1,
      accepted: 1,
      rejected: 0,
      conflicted: 0,
      verificationSubmitted: 1,
      verificationPassed: 1,
      acceptedWithRealNativeCode: 1,
      safeDrainClassification: { DRAINED: 0, RETAINED_COUPLED: 1, REFERENCE_ONLY: 0 },
    },
  })
  assert.ok(Array.isArray(lines))
  assert.ok(lines.some((l) => l.startsWith('CHRONICA ABSORPTION LOOP RATIO REPORT')))
  assert.ok(lines.some((l) => l.includes('ACCEPTANCE_RATIO')))
  assert.ok(lines.some((l) => l.startsWith('VERDICT:')))
})

test('buildRatioReport renders DONOR_EXTINCTION_RATIO as a clean percentage, never NaN, including when zero donors are currently extinct', () => {
  const log = execFileSync('git', ['log', '--format=%H', '-n', '2'], { encoding: 'utf8' }).trim().split('\n')
  const [finalSha, baseSha] = log
  const lines = buildRatioReport({
    baseSha,
    finalSha,
    live: {
      candidatesPlanned: 1, buildersCompleted: 1, accepted: 1, rejected: 0, conflicted: 0,
      verificationSubmitted: 1, verificationPassed: 1, acceptedWithRealNativeCode: 1,
      safeDrainClassification: { DRAINED: 0, RETAINED_COUPLED: 1, REFERENCE_ONLY: 0 },
    },
  })
  const extinctionLine = lines.find((l) => l.startsWith('DONOR_EXTINCTION_RATIO:'))
  assert.ok(extinctionLine, 'expected a DONOR_EXTINCTION_RATIO line')
  assert.doesNotMatch(extinctionLine, /NaN/)
  assert.match(extinctionLine, /=\s+\d+(\.\d+)?%$/)
  const ingestionLine = lines.find((l) => l.startsWith('DONOR_INGESTION_RATIO:'))
  const functionalCoverageLine = lines.find((l) => l.startsWith('FUNCTIONAL_DONOR_COVERAGE:'))
  const sourceDrainRatioLine = lines.find((l) => l.startsWith('SOURCE_DRAIN_DONOR_RATIO:'))
  assert.ok(functionalCoverageLine, 'expected a FUNCTIONAL_DONOR_COVERAGE line, distinct from source drain')
  assert.ok(sourceDrainRatioLine, 'expected a SOURCE_DRAIN_DONOR_RATIO line, distinct from functional absorption')
  assert.doesNotMatch(ingestionLine, /NaN/)
  assert.doesNotMatch(functionalCoverageLine, /NaN/)
  assert.doesNotMatch(sourceDrainRatioLine, /NaN/)
})

test('buildRatioReport never conflates FUNCTIONAL_DONOR_COVERAGE with SOURCE_DRAIN_DONOR_RATIO: postgres/postgres is real, live evidence they can legitimately differ (functional=yes, source-drain=no)', () => {
  const log = execFileSync('git', ['log', '--format=%H', '-n', '2'], { encoding: 'utf8' }).trim().split('\n')
  const [finalSha, baseSha] = log
  const lines = buildRatioReport({
    baseSha,
    finalSha,
    live: {
      candidatesPlanned: 1, buildersCompleted: 1, accepted: 1, rejected: 0, conflicted: 0,
      verificationSubmitted: 1, verificationPassed: 1, acceptedWithRealNativeCode: 1,
      safeDrainClassification: { DRAINED: 0, RETAINED_COUPLED: 1, REFERENCE_ONLY: 0 },
    },
  })
  const functionalLine = lines.find((l) => l.startsWith('  Functional absorption started:'))
  const drainLine = lines.find((l) => l.startsWith('  Source drain started:'))
  assert.ok(functionalLine && drainLine)
  const functionalCount = Number(functionalLine.split(':')[1].trim())
  const drainCount = Number(drainLine.split(':')[1].trim())
  // postgres/postgres alone proves functionalAbsorptionStartedDonors can legitimately exceed
  // sourceDrainStartedDonors in this real corpus today -- if the two counts were still secretly
  // the same underlying number (the original defect), this would be impossible.
  assert.ok(functionalCount > drainCount, `expected functional (${functionalCount}) > source-drain (${drainCount}) given postgres/postgres's real current state`)
})

test('buildRatioReport reports ROOT_COUNT_DELTA of 0 over a range with no workspace member changes', () => {
  const log = execFileSync('git', ['log', '--format=%H', '-n', '2'], { encoding: 'utf8' }).trim().split('\n')
  const [finalSha, baseSha] = log
  const lines = buildRatioReport({
    baseSha,
    finalSha,
    live: {
      candidatesPlanned: 1, buildersCompleted: 1, accepted: 1, rejected: 0, conflicted: 0,
      verificationSubmitted: 1, verificationPassed: 1, acceptedWithRealNativeCode: 1,
      safeDrainClassification: { DRAINED: 1, RETAINED_COUPLED: 0, REFERENCE_ONLY: 0 },
    },
  })
  // Over a real, tiny two-commit range with no workspace member changes, root delta must be 0.
  assert.ok(lines.some((l) => l.startsWith('ROOT_COUNT_DELTA:           0')))
})
