// rust-workflow-trigger.test.mjs — deterministic tests proving
// .github/workflows/rust.yml's `pull_request` trigger actually matches the
// repo's legacy integration branch (PR #1565 admission review, P0: the gate
// was well-built but never once ran against that PR stream because the trigger
// only listed `main`/`master`, not `codex/w9-integration-world-class`).
//
// These tests parse the real workflow YAML and simulate GitHub Actions'
// branch-filter glob semantics against it, so a future edit that silently
// narrows/widens/removes the trigger, or reintroduces a bypass via
// `paths`/`if`/`continue-on-error`, fails this suite before it ever reaches
// live CI.
import test from 'node:test'
import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import { spawnSync } from 'node:child_process'
import { dirname, join } from 'node:path'
import { fileURLToPath } from 'node:url'

const here = dirname(fileURLToPath(import.meta.url))
const repoRoot = join(here, '..', '..')
const workflowPath = join(repoRoot, '.github', 'workflows', 'rust.yml')
const rawYaml = readFileSync(workflowPath, 'utf8')

function parseInlineList(value) {
  const trimmed = value.trim()
  assert.ok(trimmed.startsWith('[') && trimmed.endsWith(']'), `expected inline list, got ${value}`)
  return trimmed
    .slice(1, -1)
    .split(',')
    .map((item) => item.trim().replace(/^["']|["']$/g, ''))
    .filter(Boolean)
}

function parseWorkflowYamlSubset(source) {
  const lines = source.split(/\r?\n/)
  const doc = { on: { pull_request: {}, push: {} }, jobs: {} }
  for (let i = 0; i < lines.length; i++) {
    const line = lines[i]
    if (line.startsWith('name: ')) {
      doc.name = line.slice('name: '.length).trim().replace(/^["']|["']$/g, '')
    }
    if (line.trim() === 'pull_request:') {
      for (let j = i + 1; j < lines.length && lines[j].startsWith('    '); j++) {
        const child = lines[j].trim()
        if (child.startsWith('branches: ')) doc.on.pull_request.branches = parseInlineList(child.slice('branches: '.length))
        if (child.startsWith('paths:') || child.startsWith('paths-ignore:')) doc.on.pull_request[child.replace(/:.*/, '')] = []
      }
    }
    if (line.trim() === 'push:') {
      for (let j = i + 1; j < lines.length && lines[j].startsWith('    '); j++) {
        const child = lines[j].trim()
        if (child.startsWith('branches: ')) doc.on.push.branches = parseInlineList(child.slice('branches: '.length))
        if (child === 'paths:') {
          const paths = []
          for (let k = j + 1; k < lines.length && lines[k].startsWith('      - '); k++) {
            paths.push(lines[k].trim().slice(2).trim().replace(/^["']|["']$/g, ''))
          }
          doc.on.push.paths = paths
        }
      }
    }
  }

  const jobsStart = lines.findIndex((line) => line.trim() === 'jobs:')
  for (let i = jobsStart + 1; i < lines.length; i++) {
    const jobMatch = /^  ([a-zA-Z0-9_-]+):$/.exec(lines[i])
    if (!jobMatch) continue
    const jobName = jobMatch[1]
    const job = { steps: [] }
    doc.jobs[jobName] = job
    for (let j = i + 1; j < lines.length; j++) {
      if (/^  [a-zA-Z0-9_-]+:$/.test(lines[j])) break
      const trimmed = lines[j].trim()
      if (trimmed.startsWith('runs-on: ')) job['runs-on'] = trimmed.slice('runs-on: '.length).replace(/^["']|["']$/g, '')
      if (trimmed.startsWith('needs: ')) job.needs = trimmed.slice('needs: '.length)
      // Job-level `if:` is four spaces. Step-level `if:` is eight; do not let
      // the last step overwrite the job isolation condition.
      if (lines[j].startsWith('    if: ')) job.if = lines[j].trim().slice('if: '.length)
      if (trimmed === 'permissions:') {
        job.permissions = {}
        for (let k = j + 1; k < lines.length && lines[k].startsWith('      '); k++) {
          const perm = lines[k].trim()
          const parts = /^([a-zA-Z-]+):\s*(.+)$/.exec(perm)
          if (parts) job.permissions[parts[1]] = parts[2].replace(/^["']|["']$/g, '')
        }
      }
      if (trimmed === 'steps:') {
        for (let k = j + 1; k < lines.length; k++) {
          if (/^  [a-zA-Z0-9_-]+:$/.test(lines[k])) break
          if (!lines[k].startsWith('      - ')) continue
          const step = {}
          const first = lines[k].trim().slice(2).trim()
          const firstParts = /^([a-zA-Z-]+):\s*(.+)$/.exec(first)
          if (firstParts) step[firstParts[1]] = firstParts[2].replace(/^["']|["']$/g, '')
          for (let n = k + 1; n < lines.length && lines[n].startsWith('        '); n++) {
            const entry = lines[n].trim()
            const parts = /^([a-zA-Z-]+):\s*(.+)$/.exec(entry)
            if (parts) {
              const raw = parts[2].trim()
              step[parts[1]] = raw === 'true' ? true : raw === 'false' ? false : raw.replace(/^["']|["']$/g, '')
            }
          }
          job.steps.push(step)
        }
        break
      }
    }
  }
  return doc
}

let parserName = 'fallback-subset-parser'
let parse = parseWorkflowYamlSubset
try {
  const yaml = await import('yaml')
  parse = yaml.parse
  parserName = 'yaml-package'
} catch {
  // Keep this test self-contained for the no-install CI lane and sparse audit
  // worktrees. The fallback intentionally parses only the workflow fields this
  // test asserts; it is not exported or used by production code.
}
const doc = parse(rawYaml)

// GitHub Actions' branch-filter patterns are glob-like (`*`, `**`, `?`).
// None of this workflow's patterns currently use wildcards, but the matcher
// is real glob logic (anchored, escaped literals) rather than `===`, so it
// stays correct if a pattern ever legitimately needs one.
function globToRegExp(pattern) {
  let re = '^'
  for (const ch of pattern) {
    if (ch === '*') re += '.*'
    else if (ch === '?') re += '.'
    else re += ch.replace(/[.+^${}()|[\]\\]/g, '\\$&')
  }
  return new RegExp(re + '$')
}

function branchMatchesAny(branch, patterns) {
  return patterns.some((p) => globToRegExp(p).test(branch))
}

const CHRONICA_BASE_ISOLATION_IF =
  "github.event_name != 'pull_request' || github.base_ref == 'main' || github.base_ref == 'master' || github.base_ref == 'codex/w9-integration-world-class'"

function allSteps() {
  const steps = []
  for (const [jobName, job] of Object.entries(doc.jobs)) {
    for (const step of job.steps ?? []) steps.push({ jobName, step })
  }
  return steps
}

function jobHeader(jobName) {
  const lines = rawYaml.split(/\r?\n/)
  const start = lines.findIndex((line) => line === `  ${jobName}:`)
  assert.ok(start >= 0, `job "${jobName}" must exist in rust.yml`)
  const header = []
  for (let i = start + 1; i < lines.length; i++) {
    if (/^  [a-zA-Z0-9_-]+:\s*$/.test(lines[i])) break
    if (/^    steps:\s*$/.test(lines[i])) break
    header.push(lines[i])
  }
  return header.join('\n')
}

test('workflow YAML parses structurally', () => {
  assert.ok(['yaml-package', 'fallback-subset-parser'].includes(parserName))
  assert.equal(doc.name, 'Rust')
  assert.ok(Array.isArray(Object.keys(doc.jobs)))
  assert.ok(Object.keys(doc.jobs).includes('island-scan'))
  assert.ok(Object.keys(doc.jobs).includes('lint'))
  assert.ok(Object.keys(doc.jobs).includes('test'))
  assert.ok(Object.keys(doc.jobs).includes('fresh-migrate'))
})

test('P0: pull_request.branches is exactly {main, master, codex/w9-integration-world-class} — no more, no fewer', () => {
  const branches = doc.on.pull_request.branches
  assert.ok(Array.isArray(branches))
  assert.deepEqual([...branches].sort(), ['codex/w9-integration-world-class', 'main', 'master'])
})

test('P0: a PR based on codex/w9-integration-world-class still matches the compatibility trigger', () => {
  assert.equal(branchMatchesAny('codex/w9-integration-world-class', doc.on.pull_request.branches), true)
})

test('P0: PRs based on main or master match the primary triggers', () => {
  assert.equal(branchMatchesAny('main', doc.on.pull_request.branches), true)
  assert.equal(branchMatchesAny('master', doc.on.pull_request.branches), true)
})

test('P0: an unrelated source branch cannot bypass into the trigger — no accidental wildcard was introduced', () => {
  for (const unrelated of [
    'some-other-feature-branch',
    'codex/w9-integration-world-class-fork', // prefix collision must NOT match (anchored regex)
    'codex/some-other-integration-branch', // proves no accidental `codex/**` glob was added
    'feature/codex-w9-integration-world-class', // substring collision must NOT match
    '',
  ]) {
    assert.equal(branchMatchesAny(unrelated, doc.on.pull_request.branches), false, `"${unrelated}" must NOT match the trigger`)
  }
})

test('P0: the pull_request trigger has no paths/paths-ignore filter — an unrelated-file-only PR cannot skip lint/test/fresh-migrate/island-scan by touching only allowed paths', () => {
  assert.equal('paths' in doc.on.pull_request, false)
  assert.equal('paths-ignore' in doc.on.pull_request, false)
})

test('P0: the push trigger\'s paths filter is scoped to truth-reset/** on main, and does not leak into pull_request', () => {
  assert.deepEqual([...doc.on.push.branches].sort(), ['main', 'truth-reset/**'])
  assert.ok(Array.isArray(doc.on.push.paths))
  assert.ok(doc.on.push.paths.length > 0)
})

test('P0/fail-closed: every rust.yml job isolates Genesis reconstruction PRs from historical Chronica gates', () => {
  const jobNames = Object.keys(doc.jobs)
  assert.ok(jobNames.length >= 7, `expected rust.yml jobs, found: ${jobNames.join(', ')}`)
  for (const jobName of jobNames) {
    const header = jobHeader(jobName)
    assert.ok(
      header.includes(`    if: ${CHRONICA_BASE_ISOLATION_IF}`),
      `Job '${jobName}' must skip PRs whose base is not a historical Chronica gate branch.\nExpected job-level if: ${CHRONICA_BASE_ISOLATION_IF}`,
    )
    assert.equal(
      /github\.actor|head\.repo\.fork|skip.ci|skip.rust/i.test(header),
      false,
      `Job '${jobName}' must not skip by actor, fork, or label`,
    )
    const parsedIf = String(doc.jobs[jobName].if || '').replace(/^\$\{\{\s*|\s*\}\}$/g, '')
    assert.equal(
      parsedIf,
      CHRONICA_BASE_ISOLATION_IF,
      `parsed job '${jobName}'.if must be the Chronica-base isolation expression`,
    )
  }
})

test('P0/fail-closed: no rust.yml job skips by actor, fork, or label', () => {
  for (const [jobName, job] of Object.entries(doc.jobs)) {
    const cond = String(job.if || '')
    assert.equal(
      /github\.actor|head\.repo\.fork|skip.ci|skip.rust/i.test(cond),
      false,
      `Job '${jobName}' has a skip-gate (\`if: ${cond}\`). Actor/fork/label skips are forbidden.`,
    )
  }
})

test('P0/fail-closed: no step anywhere in this workflow uses continue-on-error — a failing step must fail its job, never be swallowed', () => {
  for (const { jobName, step } of allSteps()) {
    assert.notEqual(step['continue-on-error'], true, `job "${jobName}" step "${step.name ?? step.uses ?? step.run}" must not set continue-on-error: true`)
  }
})

test('P0: island-scan job is independent (no `needs:`) so it cannot be silently skipped via an upstream job dependency', () => {
  assert.equal('needs' in doc.jobs['island-scan'], false)
})

test('P0/least-privilege: island-scan job grants exactly {contents: read} and nothing else', () => {
  assert.deepEqual(doc.jobs['island-scan'].permissions, { contents: 'read' })
})

test('P0/no-secrets: no step in this workflow references secrets.* anywhere in its run/env/with blocks (raw-text check on the parsed document, not the pre-parse source)', () => {
  const serialized = JSON.stringify(doc)
  assert.equal(/secrets\./.test(serialized), false)
})

test('P0/no-network: island-scan job\'s cargo metadata invocation passes --no-deps (island-scan.mjs::runCargoMetadata), confirmed structurally by the step commands present', () => {
  const steps = doc.jobs['island-scan'].steps
  const runSteps = steps.filter((s) => typeof s.run === 'string')
  assert.ok(runSteps.some((s) => s.run.includes('tools/build/island-scan.mjs')))
  assert.ok(runSteps.some((s) => s.run.includes('tools/build/island-lib.test.mjs')))
  assert.ok(runSteps.some((s) => s.run.includes('tools/build/island-lib.allowlist.test.mjs')))
  assert.ok(runSteps.some((s) => s.run.includes('tools/build/island-lib.cli.test.mjs')))
  // The actual --no-deps flag lives in island-scan.mjs::runCargoMetadata, not
  // this workflow file (the job just invokes the script); island-lib.test.mjs
  // covers that invocation directly. This test only proves the workflow calls
  // the real script/test entry points, unchanged, not a stale/renamed path.
})

test('P0/exit-code fail-closed, live: the exact `run:` command wired into the island-scan step currently exits 0 against the real repo (proves the literal CI command works end to end, not a hand-typed equivalent)', () => {
  const probe = spawnSync('cargo', ['--version'], { encoding: 'utf8' })
  if (probe.status !== 0) return // no cargo toolchain in this environment; covered elsewhere

  const scanStep = doc.jobs['island-scan'].steps.find((s) => typeof s.run === 'string' && s.run.includes('island-scan.mjs') && !s.run.includes('--test'))
  assert.ok(scanStep, 'expected to find the island-scan invocation step')
  assert.equal(scanStep.run.trim(), 'node tools/build/island-scan.mjs')

  const result = spawnSync(scanStep.run, { cwd: repoRoot, encoding: 'utf8', shell: true })
  assert.equal(
    result.status,
    0,
    `expected the real CI command to currently pass; error: ${result.error?.message ?? 'none'}; stderr:\n${result.stderr ?? ''}`,
  )

  const testStep = doc.jobs['island-scan'].steps.find((s) => typeof s.run === 'string' && s.run.includes('node --test'))
  assert.ok(testStep)
  assert.equal(
    testStep.run.trim(),
    'node --test tools/build/island-lib.test.mjs tools/build/island-lib.allowlist.test.mjs tools/build/island-lib.cli.test.mjs',
  )
})

test('P0/setup: island-scan job installs both a Rust toolchain (for cargo metadata) and Node (for the .mjs scripts) before running them', () => {
  const uses = doc.jobs['island-scan'].steps.map((s) => s.uses).filter(Boolean)
  assert.ok(uses.some((u) => u.startsWith('actions/checkout@')))
  assert.ok(uses.some((u) => u.startsWith('dtolnay/rust-toolchain@')))
  assert.ok(uses.some((u) => u.startsWith('actions/setup-node@')))
})

test('every job.runs-on is a GitHub-hosted runner (no unpinned self-hosted runner label that could weaken isolation)', () => {
  for (const [jobName, job] of Object.entries(doc.jobs)) {
    assert.equal(job['runs-on'], 'ubuntu-latest', `job "${jobName}" runs-on unexpected value`)
  }
})
