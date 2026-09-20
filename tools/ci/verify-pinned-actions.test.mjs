import test from 'node:test'
import assert from 'node:assert/strict'
import { spawnSync } from 'node:child_process'
import { readFileSync } from 'node:fs'
import { dirname, join } from 'node:path'
import { fileURLToPath } from 'node:url'
import { checkPinnedActions } from './verify-pinned-actions.mjs'

const here = dirname(fileURLToPath(import.meta.url))
const repoRoot = join(here, '..', '..')
const scriptPath = join(here, 'verify-pinned-actions.mjs')
const fixturesDir = join(here, 'fixtures')

function readFixture(name) {
  return readFileSync(join(fixturesDir, name), 'utf8')
}

test('checkPinnedActions: passes a workflow whose uses: lines are all SHA-pinned', () => {
  const { ok, errors } = checkPinnedActions(readFixture('well-formed.yml'))
  assert.equal(ok, true)
  assert.deepEqual(errors, [])
})

test('checkPinnedActions: fails uses: actions/checkout@v4 (mutable tag, not a SHA)', () => {
  const { ok, errors } = checkPinnedActions(readFixture('unpinned-tag.yml'))
  assert.equal(ok, false)
  assert.equal(errors.length, 1)
  assert.match(errors[0], /mutable tag/)
  assert.match(errors[0], /actions\/checkout@v4/)
})

test('checkPinnedActions: fails uses: dtolnay/rust-toolchain@stable with a distinct "floating identifier" message', () => {
  const { ok, errors } = checkPinnedActions(readFixture('floating-identifier.yml'))
  assert.equal(ok, false)
  assert.equal(errors.length, 1)
  assert.match(errors[0], /floating identifier/)
  assert.match(errors[0], /dtolnay\/rust-toolchain@stable/)
  // Must NOT be misreported as the generic "mutable tag" case -- these are
  // deliberately different failure messages so a reviewer knows @stable can't be
  // fixed by just resolving today's tag (there isn't one).
  assert.doesNotMatch(errors[0], /mutable tag/)
})

test('CLI: exits 0 on the well-formed fixture', () => {
  const result = spawnSync('node', [scriptPath, join(fixturesDir, 'well-formed.yml')], { cwd: repoRoot, encoding: 'utf8' })
  assert.equal(result.status, 0, result.stdout + result.stderr)
})

test('CLI: exits non-zero with a clear message on unpinned-tag.yml', () => {
  const result = spawnSync('node', [scriptPath, join(fixturesDir, 'unpinned-tag.yml')], { cwd: repoRoot, encoding: 'utf8' })
  assert.notEqual(result.status, 0)
  assert.match(result.stderr, /FAIL/)
  assert.match(result.stderr, /mutable tag/)
})

test('CLI: exits non-zero with the floating-identifier message on floating-identifier.yml', () => {
  const result = spawnSync('node', [scriptPath, join(fixturesDir, 'floating-identifier.yml')], { cwd: repoRoot, encoding: 'utf8' })
  assert.notEqual(result.status, 0)
  assert.match(result.stderr, /FAIL/)
  assert.match(result.stderr, /floating identifier/)
})

test('CLI: the real repo workflows all pass (regression guard for issue #1548)', () => {
  const result = spawnSync('node', [scriptPath], { cwd: repoRoot, encoding: 'utf8' })
  assert.equal(result.status, 0, result.stdout + result.stderr)
})
