import { test } from 'node:test'
import assert from 'node:assert/strict'
import { execFileSync } from 'node:child_process'
import { mkdtempSync, mkdirSync, writeFileSync, rmSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join } from 'node:path'
import { readFileSync } from 'node:fs'
import { fileURLToPath } from 'node:url'
import { dirname } from 'node:path'
import {
  commitTouchedNativeCode,
  classifyDonorDrain,
  buildLiveCounts,
  emitRatioReportForLoop,
} from './integrate-candidates.mjs'

const HERE = dirname(fileURLToPath(import.meta.url))
const ROOT = join(HERE, '..', '..')

function realHead() {
  return execFileSync('git', ['rev-parse', 'HEAD'], { cwd: ROOT, encoding: 'utf8' }).trim()
}

function realGitStatus() {
  return execFileSync('git', ['status', '--porcelain'], { cwd: ROOT, encoding: 'utf8' })
}

function makeFixtureRepo() {
  const root = mkdtempSync(join(tmpdir(), 'integrate-candidates-'))
  const git = (args) => execFileSync('git', args, { cwd: root, encoding: 'utf8' })
  git(['init', '--quiet'])
  git(['config', 'user.email', 'test@example.com'])
  git(['config', 'user.name', 'Test'])
  return { root, git }
}

function writeAndCommit(root, git, files, message) {
  for (const [path, content] of Object.entries(files)) {
    const full = join(root, path)
    mkdirSync(join(full, '..'), { recursive: true })
    writeFileSync(full, content)
  }
  git(['add', '-A'])
  git(['commit', '--quiet', '-m', message])
  return git(['rev-parse', 'HEAD']).trim()
}

// --- commitTouchedNativeCode / classifyDonorDrain: real-repo-derived, no guessing ---

test('commitTouchedNativeCode is true for a real commit that only touched runtime+adapter', () => {
  // e72ecf1fac added runtime/src/recovery.rs + adapter/src/durable_recovery_kernel_proof.rs and
  // touched no temporary/ path at all -- a real, known-shape native-only commit.
  assert.equal(commitTouchedNativeCode('e72ecf1fac', ROOT), true)
})

test('classifyDonorDrain is REFERENCE_ONLY for a real commit that deleted no temporary/ file', () => {
  assert.equal(classifyDonorDrain('bb02a69aa5', 'e72ecf1fac', ROOT), 'REFERENCE_ONLY')
})

test('classifyDonorDrain is RETAINED_COUPLED for a real commit that drained part of a donor tree still 9000+ files deep', () => {
  // ece144bb3d deleted temporary/vault/shamir/{shamir.go,shamir_test.go} while temporary/vault/
  // still has thousands of other files -- a real, gate-checked PARTIAL drain, not an extinction.
  assert.equal(classifyDonorDrain('ece144bb3d^', 'ece144bb3d', ROOT), 'RETAINED_COUPLED')
})

test('classifyDonorDrain is DRAINED once a donor tree becomes completely empty as of that commit (synthetic fixture)', () => {
  const { root, git } = makeFixtureRepo()
  const before = writeAndCommit(root, git, {
    'temporary/tinydonor/only.go': 'package tinydonor\n',
    'core/src/lib.rs': '// native\n',
  }, 'seed tiny donor')
  git(['rm', '-q', 'temporary/tinydonor/only.go'])
  git(['commit', '--quiet', '-m', 'drain tinydonor'])
  const after = git(['rev-parse', 'HEAD']).trim()

  assert.equal(classifyDonorDrain(before, after, root), 'DRAINED')
  rmSync(root, { recursive: true, force: true })
})

test('classifyDonorDrain is RETAINED_COUPLED when a donor deletion leaves siblings behind (synthetic fixture)', () => {
  const { root, git } = makeFixtureRepo()
  const before = writeAndCommit(root, git, {
    'temporary/bigdonor/a.go': 'package bigdonor\n',
    'temporary/bigdonor/b.go': 'package bigdonor\n',
  }, 'seed donor with two files')
  git(['rm', '-q', 'temporary/bigdonor/a.go'])
  git(['commit', '--quiet', '-m', 'partially drain bigdonor'])
  const after = git(['rev-parse', 'HEAD']).trim()

  assert.equal(classifyDonorDrain(before, after, root), 'RETAINED_COUPLED')
  rmSync(root, { recursive: true, force: true })
})

test('classifyDonorDrain is REFERENCE_ONLY when a candidate touches native code but no donor file (synthetic fixture)', () => {
  const { root, git } = makeFixtureRepo()
  const before = writeAndCommit(root, git, { 'temporary/donor/a.go': 'package donor\n' }, 'seed')
  writeAndCommit(root, git, { 'runtime/src/foo.rs': '// native only\n' }, 'native-only candidate')
  const after = git(['rev-parse', 'HEAD']).trim()

  assert.equal(classifyDonorDrain(before, after, root), 'REFERENCE_ONLY')
  rmSync(root, { recursive: true, force: true })
})

// --- buildLiveCounts: pure aggregation, no guessing ---

test('buildLiveCounts reports conflict/rejection/acceptance counts exactly as the results list says', () => {
  const results = [
    { branch: 'a', before: 'x', result: 'CONFLICT', detail: 'conflict detail' },
    { branch: 'b', before: 'x', result: 'TEST_FAILURE', detail: 'test failure detail' },
    { branch: 'c', before: 'bb02a69aa5', result: 'INTEGRATED', detail: 'e72ecf1fac' },
  ]
  const live = buildLiveCounts(results, ROOT)
  assert.equal(live.candidatesPlanned, 3)
  assert.equal(live.buildersCompleted, 3)
  assert.equal(live.accepted, 1)
  assert.equal(live.rejected, 1)
  assert.equal(live.conflicted, 1)
  // Only CONFLICT never reaches verification; the other two (TEST_FAILURE, INTEGRATED) do.
  assert.equal(live.verificationSubmitted, 2)
  assert.equal(live.verificationPassed, 1)
})

test('buildLiveCounts derives acceptedWithRealNativeCode and safeDrainClassification from the real accepted commit, never a guess', () => {
  const results = [
    { branch: 'native-only', before: 'bb02a69aa5', result: 'INTEGRATED', detail: 'e72ecf1fac' },
  ]
  const live = buildLiveCounts(results, ROOT)
  assert.equal(live.acceptedWithRealNativeCode, 1)
  assert.deepEqual(live.safeDrainClassification, { DRAINED: 0, RETAINED_COUPLED: 0, REFERENCE_ONLY: 1 })
})

test('buildLiveCounts on an all-conflicted wave reports zero accepted and never touches git for native-code/drain classification', () => {
  const results = [
    { branch: 'a', before: 'x', result: 'CONFLICT', detail: 'd1' },
    { branch: 'b', before: 'x', result: 'CONFLICT', detail: 'd2' },
  ]
  // Uses obviously-invalid "before"/"detail" shas -- if buildLiveCounts tried to classify a
  // CONFLICT/TEST_FAILURE entry it would throw against these; it must not even try.
  const live = buildLiveCounts(results, ROOT)
  assert.equal(live.accepted, 0)
  assert.deepEqual(live.safeDrainClassification, { DRAINED: 0, RETAINED_COUPLED: 0, REFERENCE_ONLY: 0 })
})

// --- emitRatioReportForLoop: the mandatory wiring point ---

test('emitRatioReportForLoop reuses the real absorption-ratio-report.mjs reporter and returns its lines', () => {
  const result = emitRatioReportForLoop({
    baseSha: 'bb02a69aa5',
    finalSha: 'e72ecf1fac',
    results: [{ branch: 'native-only', before: 'bb02a69aa5', result: 'INTEGRATED', detail: 'e72ecf1fac' }],
  })
  assert.equal(result.ok, true)
  assert.ok(result.lines.some((l) => l.startsWith('CHRONICA ABSORPTION LOOP RATIO REPORT')))
  assert.ok(result.lines.some((l) => l.startsWith('DONOR CORPUS')))
  assert.ok(result.lines.some((l) => l.includes('DONOR_EXTINCTION_RATIO')))
  assert.ok(result.lines.some((l) => l.startsWith('VERDICT:')))
})

test('emitRatioReportForLoop writes no file to disk (no dynamic progress database, ephemeral only)', () => {
  const before = realGitStatus()
  emitRatioReportForLoop({
    baseSha: 'bb02a69aa5',
    finalSha: 'e72ecf1fac',
    results: [{ branch: 'native-only', before: 'bb02a69aa5', result: 'INTEGRATED', detail: 'e72ecf1fac' }],
  })
  assert.equal(realGitStatus(), before, 'emitting a ratio report must never change the working tree')
})

test('emitRatioReportForLoop fails closed (ok: false) rather than throwing when the range is nonsense, so the caller can detect RATIO_REPORT_FAILURE', () => {
  const result = emitRatioReportForLoop({
    baseSha: 'not-a-real-sha-at-all',
    finalSha: 'also-not-real',
    results: [],
  })
  assert.equal(result.ok, false)
  assert.ok(result.error)
})

// --- wiring/meta checks on the source itself ---

const SOURCE = readFileSync(join(HERE, 'integrate-candidates.mjs'), 'utf8')

test('main calls emitRatioReportForLoop exactly once (one ratio report per loop, not per candidate)', () => {
  const callSites = SOURCE.match(/=\s*emitRatioReportForLoop\(/g) ?? []
  assert.equal(callSites.length, 1, `expected exactly one emitRatioReportForLoop call site, found ${callSites.length}`)
})

test('a failed ratio report keeps main from returning success (RATIO_REPORT_FAILURE is visible and non-zero)', () => {
  assert.match(SOURCE, /RATIO_REPORT_FAILURE/)
  assert.match(SOURCE, /if \(!report\.ok\)\s*\{[\s\S]*?return 1/)
})

test('a failed ratio report never rolls back already-integrated candidates (no git reset in the report-failure branch)', () => {
  const failureBranch = SOURCE.slice(SOURCE.indexOf('if (!report.ok)'), SOURCE.indexOf('return 1', SOURCE.indexOf('if (!report.ok)')))
  assert.doesNotMatch(failureBranch, /reset/)
})

test('integrate-candidates.mjs never persists a progress database -- no writeFileSync/appendFileSync call anywhere in its source', () => {
  assert.doesNotMatch(SOURCE, /writeFileSync|appendFileSync/)
})

test('the four canonical native-code prefixes checked here are exactly core/runtime/adapter/organism -- root count stays 4', () => {
  const match = SOURCE.match(/const NATIVE_PREFIXES = \[([^\]]+)\]/)
  assert.ok(match, 'expected a NATIVE_PREFIXES constant in integrate-candidates.mjs')
  const prefixes = match[1].split(',').map((s) => s.trim().replace(/['"]/g, '')).filter(Boolean)
  assert.deepEqual(prefixes.sort(), ['adapter/', 'core/', 'organism/', 'runtime/'])
})

test('integrateOne\'s own per-candidate checks still run the real donor safety gates -- temporary-dependency and donor-coupling -- not weakened or removed', () => {
  assert.match(SOURCE, /no-temporary-dependency-gate\.mjs/)
  assert.match(SOURCE, /donor-coupling-gate\.mjs.*--since/)
})
