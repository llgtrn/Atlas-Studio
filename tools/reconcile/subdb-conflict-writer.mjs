#!/usr/bin/env node
// subdb-conflict-writer.mjs -- CLI for the "needs human judgment" half of the conflict-audit writer
// (docs/doctrines/023-five-dimension-cloud-shard-contract.md, marked NOT_BUILT there).
//
// Runs the sync-anchor-v2 atom parser (imported, never re-implemented) over the requested crates
// and writes every remaining Category-B disagreement (ISLAND, overclaim, PREFIX-confidence deferred
// findings, doc/shard mismatches, missing-test-evidence findings) to a dated review queue:
//   docs/_machine/subdb-conflicts/<date>-review-queue.jsonl  (one record per finding)
//   docs/_machine/subdb-conflicts/<date>-review-queue.md     (grouped/sorted-by-crate summary)
//
// This never fixes any of it -- it only produces the backlog. Run
// `subdb-apply-agreed-corrections.mjs` first so EXACT-confidence findings don't clutter the queue.
//
// Usage:
//   node tools/reconcile/subdb-conflict-writer.mjs --date 2026-07-31
//   node tools/reconcile/subdb-conflict-writer.mjs --date 2026-07-31 --crate chronica-cli [--crate ...]
import { existsSync, linkSync, mkdirSync, rmSync, writeFileSync } from 'node:fs'
import { randomUUID } from 'node:crypto'
import { join, resolve, sep } from 'node:path'
import { fileURLToPath } from 'node:url'
import { buildReviewQueueMarkdown, buildReviewQueueRecords } from './subdb-conflict-writer-lib.mjs'
import { discoverWorkspaceCrates, sanitizePathField, verifyCrates } from './sync-anchor-v2-lib.mjs'

export class CliUsageError extends Error {}

function usageError(message) {
  throw new CliUsageError(`subdb-conflict-writer: ${message}`)
}

function requiredValue(argv, index, flag) {
  const value = argv[index + 1]
  if (!value || value.startsWith('--')) usageError(`${flag} requires a value`)
  return value
}

export function parseArgs(argv) {
  const opts = { crates: [], date: null, outDir: join('docs', '_machine', 'subdb-conflicts') }
  for (let i = 0; i < argv.length; i += 1) {
    const arg = argv[i]
    if (arg === '--crate') {
      opts.crates.push(requiredValue(argv, i, arg))
      i += 1
    } else if (arg === '--date') {
      opts.date = requiredValue(argv, i, arg)
      i += 1
    } else if (arg === '--out-dir') {
      opts.outDir = requiredValue(argv, i, arg)
      i += 1
    }
    else {
      usageError('unknown argument')
    }
  }
  if (!opts.date) {
    usageError('--date <YYYY-MM-DD> is required (this script never reads the system clock).')
  }
  if (!/^\d{4}-\d{2}-\d{2}$/.test(opts.date)) {
    usageError('--date must use YYYY-MM-DD format')
  }
  return opts
}

function displayPath(path) {
  return sanitizePathField(path.split(sep).join('/'))
}

function writeAtomicNoOverwrite(path, content) {
  if (existsSync(path)) throw new Error('REFUSING_TO_OVERWRITE_OUTPUT')
  const tempPath = `${path}.${process.pid}.${randomUUID()}.tmp`
  try {
    writeFileSync(tempPath, content, { flag: 'wx' })
    linkSync(tempPath, path)
  } catch (error) {
    if (error.code === 'EEXIST') throw new Error('REFUSING_TO_OVERWRITE_OUTPUT')
    throw error
  } finally {
    rmSync(tempPath, { force: true })
  }
}

export function writeReviewQueueFiles({ outDir, date, records, markdown }) {
  mkdirSync(outDir, { recursive: true })
  const jsonlPath = join(outDir, `${date}-review-queue.jsonl`)
  const mdPath = join(outDir, `${date}-review-queue.md`)
  if (existsSync(jsonlPath) || existsSync(mdPath)) throw new Error('REFUSING_TO_OVERWRITE_OUTPUT')
  writeAtomicNoOverwrite(jsonlPath, `${records.map((r) => JSON.stringify(r)).join('\n')}\n`)
  writeAtomicNoOverwrite(mdPath, markdown)
  return { jsonlPath, mdPath }
}

export function main(argv = process.argv.slice(2), { cwd = process.cwd(), log = console.log } = {}) {
  const root = cwd
  const opts = parseArgs(argv)
  const crates = discoverWorkspaceCrates(root)
  const targetCrates = opts.crates.length ? opts.crates : crates.map((c) => c.name)

  const summary = verifyCrates(root, targetCrates)
  const records = buildReviewQueueRecords(summary.disagreements)
  const markdown = buildReviewQueueMarkdown(records, { date: opts.date })

  const { jsonlPath, mdPath } = writeReviewQueueFiles({ outDir: opts.outDir, date: opts.date, records, markdown })

  log(
    `subdb-conflict-writer: ${records.length} Category-B finding(s) across ${targetCrates.length} crate(s) scoped ` +
      `-> ${displayPath(jsonlPath)}, ${displayPath(mdPath)}`,
  )
  log('  This is a backlog only -- nothing here was auto-fixed. See doc 023 sec 4/7.')
  return { records, jsonlPath, mdPath }
}

if (process.argv[1] && resolve(fileURLToPath(import.meta.url)) === resolve(process.argv[1])) {
  try {
    main()
  } catch (error) {
    console.error(error instanceof CliUsageError ? error.message : 'subdb-conflict-writer: failed to write review queue')
    process.exit(error instanceof CliUsageError ? 2 : 1)
  }
}
