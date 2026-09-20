import assert from 'node:assert/strict'
import { execFileSync } from 'node:child_process'
import { mkdtempSync, readFileSync, writeFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join } from 'node:path'
import test from 'node:test'
import { LOCAL_VERIFICATION_LEDGER, readLocalVerificationEvidence, recordVerification } from './local-verification.mjs'

function fixture() {
  const root = mkdtempSync(join(tmpdir(), 'chronica-local-verification-'))
  execFileSync('git', ['init', '-q'], { cwd: root })
  execFileSync('git', ['config', 'user.email', 'verification@test.invalid'], { cwd: root })
  execFileSync('git', ['config', 'user.name', 'Verification Test'], { cwd: root })
  writeFileSync(join(root, '.gitignore'), '.chronica/\n')
  execFileSync('git', ['add', '.gitignore'], { cwd: root })
  execFileSync('git', ['commit', '-qm', 'fixture'], { cwd: root })
  return root
}

test('records clean local execution evidence against the exact current SHA', () => {
  const root = fixture()
  const sha = execFileSync('git', ['rev-parse', 'HEAD'], { cwd: root, encoding: 'utf8' }).trim()
  recordVerification(root, { check: 'atlas:check', command: 'pnpm atlas:check', status: 'PASS', durationMs: 12, exitCode: 0 })
  const evidence = readLocalVerificationEvidence(root)
  assert.equal(evidence.currentSha, sha)
  assert.equal(evidence.currentWorkingTreeDirty, false)
  assert.equal(evidence.stats.currentPass, 1)
  assert.equal(evidence.stats.currentFail, 0)
  assert.equal(evidence.stats.cleanCurrentChecks, 1)
  assert.equal(evidence.stats.dirtyCurrentChecks, 0)
  assert.equal(evidence.latestCurrentByCheck[0].workingTreeDirty, false)
  assert.equal(evidence.cleanCurrentByCheck[0].check, 'atlas:check')
  assert.ok(JSON.parse(readFileSync(join(root, LOCAL_VERIFICATION_LEDGER), 'utf8')).records.length === 1)
})

test('keeps stale SHA evidence historical instead of treating it as current proof', () => {
  const root = fixture()
  recordVerification(root, { check: 'atlas:check', command: 'pnpm atlas:check', status: 'PASS' })
  execFileSync('git', ['commit', '--allow-empty', '-qm', 'next'], { cwd: root })
  const evidence = readLocalVerificationEvidence(root)
  assert.equal(evidence.stats.currentShaRecords, 0)
  assert.equal(evidence.stats.staleRecords, 1)
})

test('latest result for the same check controls current evidence status', () => {
  const root = fixture()
  recordVerification(root, { check: 'ui:typecheck', command: 'pnpm ui:typecheck', status: 'FAIL', executedAt: '2026-09-15T00:00:00Z' })
  recordVerification(root, { check: 'ui:typecheck', command: 'pnpm ui:typecheck', status: 'PASS', executedAt: '2026-09-15T00:01:00Z' })
  const evidence = readLocalVerificationEvidence(root)
  assert.equal(evidence.latestCurrentByCheck.length, 1)
  assert.equal(evidence.latestCurrentByCheck[0].status, 'PASS')
})

test('records dirty-worktree execution as observation, not exact clean-SHA assurance', () => {
  const root = fixture()
  writeFileSync(join(root, 'uncommitted.txt'), 'dirty\n')
  recordVerification(root, { check: 'atlas:system:test', command: 'pnpm atlas:system:test', status: 'PASS' })
  const evidence = readLocalVerificationEvidence(root)
  assert.equal(evidence.currentWorkingTreeDirty, true)
  assert.equal(evidence.stats.currentPass, 1)
  assert.equal(evidence.stats.cleanCurrentChecks, 0)
  assert.equal(evidence.stats.dirtyCurrentChecks, 1)
  assert.equal(evidence.latestCurrentByCheck[0].workingTreeDirty, true)
  assert.equal(evidence.cleanCurrentByCheck.length, 0)
})
