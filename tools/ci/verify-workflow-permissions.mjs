#!/usr/bin/env node
// verify-workflow-permissions.mjs — CI gate for issue #1548 (CI workflow permission
// hardening). Fails non-zero if any GitHub Actions workflow file lacks an explicit
// `permissions:` block, either:
//   (a) a top-level `permissions:` key (applies to every job in the file), or
//   (b) a `permissions:` key on EVERY job under `jobs:` (no job silently inherits
//       the GITHUB_TOKEN's org/repo default, which is often broader than the job
//       actually needs).
//
// This is deliberately a plain line-based scanner, not a full YAML parser: it must
// run in the `policy` job of pr.yml before `pnpm install` has necessarily completed
// (see pr.yml's policy job), so it has no dependency on the `yaml` npm package (or
// any other dependency) being installed yet. GitHub Actions workflow YAML is a
// narrow, predictable dialect (2-space indent, no flow-style job/permissions
// blocks in this repo), so this is a safe simplification for that dialect.
//
// Usage:
//   node tools/ci/verify-workflow-permissions.mjs                # scan .github/workflows/*.yml
//   node tools/ci/verify-workflow-permissions.mjs <file> [file2 ...]   # scan specific files (tests / ad-hoc)
//
// Exit code: 0 if every scanned file has an explicit permissions block, 1 otherwise.
import { existsSync, readdirSync, readFileSync } from 'node:fs'
import { dirname, join } from 'node:path'
import { fileURLToPath } from 'node:url'

const SCRIPT_DIR = dirname(fileURLToPath(import.meta.url))
const REPO_ROOT = join(SCRIPT_DIR, '..', '..')
const WORKFLOWS_DIR = join(REPO_ROOT, '.github', 'workflows')

const TOP_LEVEL_PERMISSIONS_RE = /^permissions:\s*(\{.*\}|\S.*)?\s*$/
const JOB_HEADER_RE = /^ {2}([A-Za-z0-9_.-]+):\s*$/
const JOB_PERMISSIONS_RE = /^ {4}permissions:\s*(\{.*\}|\S.*)?\s*$/

/**
 * @param {string} content raw file text
 * @returns {{ ok: boolean, errors: string[] }}
 */
export function checkPermissions(content) {
  const lines = content.split(/\r?\n/)
  const errors = []

  const hasTopLevelPermissions = lines.some((line) => TOP_LEVEL_PERMISSIONS_RE.test(line))
  if (hasTopLevelPermissions) {
    return { ok: true, errors: [] }
  }

  const jobsIndex = lines.findIndex((line) => line === 'jobs:')
  if (jobsIndex === -1) {
    return {
      ok: false,
      errors: [
        'missing a top-level `permissions:` block, and no `jobs:` key was found to check for job-level permissions instead',
      ],
    }
  }

  // Collect job header line indices (2-space indent, directly under `jobs:`),
  // bounded by the next top-level (0-indent, non-blank) key or EOF.
  const jobHeaderIndices = []
  for (let i = jobsIndex + 1; i < lines.length; i += 1) {
    const line = lines[i]
    if (line.length > 0 && !/^\s/.test(line)) break // dedent back to a new top-level key
    const match = line.match(JOB_HEADER_RE)
    if (match) jobHeaderIndices.push({ index: i, name: match[1] })
  }

  if (jobHeaderIndices.length === 0) {
    return {
      ok: false,
      errors: ['missing a top-level `permissions:` block, and `jobs:` has no jobs to check for job-level permissions instead'],
    }
  }

  const jobsMissingPermissions = []
  for (let j = 0; j < jobHeaderIndices.length; j += 1) {
    const { index: start, name } = jobHeaderIndices[j]
    const end = j + 1 < jobHeaderIndices.length ? jobHeaderIndices[j + 1].index : lines.length
    const block = lines.slice(start + 1, end)
    const hasJobPermissions = block.some((line) => JOB_PERMISSIONS_RE.test(line))
    if (!hasJobPermissions) jobsMissingPermissions.push(name)
  }

  if (jobsMissingPermissions.length > 0) {
    return {
      ok: false,
      errors: [
        `missing a top-level \`permissions:\` block, and the following job(s) also lack their own job-level \`permissions:\` key: ${jobsMissingPermissions.join(', ')}`,
      ],
    }
  }

  return { ok: true, errors: [] }
}

function defaultWorkflowFiles() {
  if (!existsSync(WORKFLOWS_DIR)) return []
  return readdirSync(WORKFLOWS_DIR)
    .filter((f) => f.endsWith('.yml') || f.endsWith('.yaml'))
    .map((f) => join(WORKFLOWS_DIR, f))
    .sort()
}

function main() {
  const argFiles = process.argv.slice(2)
  const files = argFiles.length > 0 ? argFiles : defaultWorkflowFiles()

  if (files.length === 0) {
    console.error('verify-workflow-permissions: no workflow files found to check')
    process.exit(1)
  }

  let anyFailed = false
  for (const file of files) {
    const content = readFileSync(file, 'utf8')
    const { ok, errors } = checkPermissions(content)
    if (ok) {
      console.log(`OK    ${file}`)
    } else {
      anyFailed = true
      for (const err of errors) {
        console.error(`FAIL  ${file}: ${err}`)
      }
    }
  }

  process.exit(anyFailed ? 1 : 0)
}

// Only run as a CLI when executed directly (not when imported by tests).
if (import.meta.url === `file://${process.argv[1]}`) {
  main()
}
