// sync-anchor-v2-workflow.test.mjs — static assertions on the sync-anchor-v2-atoms CI job in
// .github/workflows/rust.yml. This is a line-scan over the real, tracked workflow YAML (no `yaml`
// package dependency, matching tools/build/rust-workflow-trigger.test.mjs's own no-install-required
// convention) so a future edit that silently reintroduces `pull-requests: write`, a PR-comment API
// call, or `persist-credentials: true` on this specific job fails here before it ever reaches CI.
import assert from 'node:assert/strict'
import { readdirSync, readFileSync } from 'node:fs'
import { dirname, join } from 'node:path'
import { fileURLToPath } from 'node:url'
import test from 'node:test'

const here = dirname(fileURLToPath(import.meta.url))
const repoRoot = join(here, '..', '..')
const workflowPath = join(repoRoot, '.github', 'workflows', 'rust.yml')
const rawYaml = readFileSync(workflowPath, 'utf8')

/** Minimal single-`*`-wildcard glob matcher, sufficient for the simple filename globs this
 * workflow uses (mirrors tools/build/rust-workflow-trigger.test.mjs's own no-dependency
 * approach). */
function globMatches(pattern, name) {
  const re = new RegExp(`^${pattern.split('*').map((s) => s.replace(/[.+^${}()|[\]\\]/g, '\\$&')).join('.*')}$`)
  return re.test(name)
}

/** Extracts the raw text block for one top-level job (from its `  <name>:` line up to the next
 * top-level `  <name>:` line or end of file), by indentation, not by YAML semantics -- sufficient
 * for line-scan assertions and immune to unrelated formatting changes elsewhere in the file. */
function jobBlock(jobName) {
  const lines = rawYaml.split(/\r?\n/)
  const startRe = new RegExp(`^  ${jobName}:\\s*$`)
  const start = lines.findIndex((l) => startRe.test(l))
  assert.ok(start >= 0, `job "${jobName}" must exist in ${workflowPath}`)
  let end = lines.length
  for (let i = start + 1; i < lines.length; i++) {
    if (/^  [a-zA-Z0-9_-]+:\s*$/.test(lines[i])) {
      end = i
      break
    }
  }
  return lines.slice(start, end).join('\n')
}

const block = jobBlock('sync-anchor-v2-atoms')
// Comment lines (this job's own doc comment deliberately explains and names the PRE-repair
// pitfall in prose) are stripped before pattern assertions, so those assertions check live YAML
// directives only -- never a false failure/pass driven by the explanatory comment text itself.
const liveYaml = block
  .split('\n')
  .filter((l) => !/^\s*#/.test(l))
  .join('\n')

test('sync-anchor-v2-atoms job exists and is scoped under .github/workflows/rust.yml', () => {
  assert.match(block, /sync-anchor-v2-atoms:/)
})

test('sync-anchor-v2-atoms job never requests pull-requests: write (trust-boundary repair)', () => {
  assert.doesNotMatch(liveYaml, /pull-requests:\s*write/)
})

test('sync-anchor-v2-atoms job declares permissions: contents: read and nothing broader', () => {
  const permissionsMatch = /permissions:\n((?:\s{6}.+\n)+)/.exec(block + '\n')
  assert.ok(permissionsMatch, 'job must declare an explicit permissions: block')
  const permLines = permissionsMatch[1]
    .split('\n')
    .map((l) => l.trim())
    .filter(Boolean)
  assert.deepEqual(permLines, ['contents: read'])
})

test('sync-anchor-v2-atoms job never calls the GitHub comment API (createComment/updateComment/github-script)', () => {
  assert.doesNotMatch(liveYaml, /createComment|updateComment|github-script/)
})

test('sync-anchor-v2-atoms job checkout step sets persist-credentials: false', () => {
  assert.match(block, /persist-credentials:\s*false/)
})

test('sync-anchor-v2-atoms job publishes via $GITHUB_STEP_SUMMARY, not an API write', () => {
  assert.match(block, /GITHUB_STEP_SUMMARY/)
})

test('sync-anchor-v2-atoms job has no job-level `if:` (only step-level, per the fail-closed no-job-level-if rule)', () => {
  const firstStepIdx = block.indexOf('steps:')
  const beforeSteps = firstStepIdx >= 0 ? block.slice(0, firstStepIdx) : block
  assert.doesNotMatch(beforeSteps, /^\s{4}if:/m)
})

test('sync-anchor-v2-atoms job runs every sync-anchor-v2 test file on disk, including THIS test file itself', () => {
  // A hand-typed file list previously omitted this very test file: the test asserting the job's
  // own trust-boundary properties was never actually run in CI and could have silently regressed
  // with no red build. This assertion is self-updating instead of hand-maintained: it discovers
  // every real sync-anchor-v2-*.test.mjs file on disk (via readdirSync, not a hardcoded list) and
  // requires each one be either named explicitly in the job block OR covered by a glob pattern
  // the job block contains -- so a FUTURE new test file that isn't wired in fails HERE, and this
  // file's own inclusion is checked the exact same way as every other test file, not specially.
  const testFiles = readdirSync(here).filter((f) => f.startsWith('sync-anchor-v2-') && f.endsWith('.test.mjs'))
  assert.ok(testFiles.length >= 5, 'sanity: expected at least the 5 known sync-anchor-v2 test files on disk')
  assert.ok(testFiles.includes('sync-anchor-v2-workflow.test.mjs'), 'sanity: this test file must be part of its own discovered set')

  // Extract every bash-glob-looking token and every literal filename token from the job's `run:`
  // steps, then require each real file to match at least one of them.
  const globCandidates = [...liveYaml.matchAll(/tools\/reconcile\/[A-Za-z0-9_.*-]+\.mjs/g)].map((m) => m[0].split('/').pop())
  for (const file of testFiles) {
    const covered = globCandidates.some((candidate) => (candidate.includes('*') ? globMatches(candidate, file) : candidate === file))
    assert.ok(covered, `job must run ${file} (explicitly or via a glob) -- found candidates: ${globCandidates.join(', ') || '(none)'}`)
  }
})

test('sync-anchor-v2-atoms job passes --summary-out to the parser (bounded Markdown summary, not raw stdout dumped to the PR)', () => {
  assert.match(block, /--summary-out/)
})
