import test from 'node:test'
import assert from 'node:assert/strict'
import { spawnSync } from 'node:child_process'
import { readFileSync } from 'node:fs'
import { dirname, join } from 'node:path'
import { fileURLToPath } from 'node:url'
import { checkPermissions } from './verify-workflow-permissions.mjs'

const here = dirname(fileURLToPath(import.meta.url))
const repoRoot = join(here, '..', '..')
const scriptPath = join(here, 'verify-workflow-permissions.mjs')
const fixturesDir = join(here, 'fixtures')

function readFixture(name) {
  return readFileSync(join(fixturesDir, name), 'utf8')
}

test('checkPermissions: passes a workflow with a top-level permissions block', () => {
  const { ok, errors } = checkPermissions(readFixture('well-formed.yml'))
  assert.equal(ok, true)
  assert.deepEqual(errors, [])
})

test('checkPermissions: passes a workflow where every job has its own permissions block', () => {
  const { ok, errors } = checkPermissions(readFixture('well-formed-job-level-permissions.yml'))
  assert.equal(ok, true)
  assert.deepEqual(errors, [])
})

test('checkPermissions: fails a workflow with no top-level permissions and at least one job missing permissions', () => {
  const { ok, errors } = checkPermissions(readFixture('missing-permissions.yml'))
  assert.equal(ok, false)
  assert.equal(errors.length, 1)
  assert.match(errors[0], /job\(s\) also lack their own job-level `permissions:`/)
  assert.match(errors[0], /\bbuild\b/)
  // `other` DOES have job-level permissions and must not be flagged.
  assert.doesNotMatch(errors[0], /\bother\b/)
})

test('CLI: exits 0 on well-formed fixtures', () => {
  const result = spawnSync('node', [scriptPath, join(fixturesDir, 'well-formed.yml'), join(fixturesDir, 'well-formed-job-level-permissions.yml')], {
    cwd: repoRoot,
    encoding: 'utf8',
  })
  assert.equal(result.status, 0, result.stdout + result.stderr)
})

test('CLI: exits non-zero with a clear message on missing-permissions.yml', () => {
  const result = spawnSync('node', [scriptPath, join(fixturesDir, 'missing-permissions.yml')], {
    cwd: repoRoot,
    encoding: 'utf8',
  })
  assert.notEqual(result.status, 0)
  assert.match(result.stderr, /FAIL/)
  assert.match(result.stderr, /permissions/)
})

test('CLI: the real repo workflows all pass (regression guard for issue #1548)', () => {
  const result = spawnSync('node', [scriptPath], { cwd: repoRoot, encoding: 'utf8' })
  assert.equal(result.status, 0, result.stdout + result.stderr)
})
