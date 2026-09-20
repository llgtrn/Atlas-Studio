#!/usr/bin/env node
// CLI for the benchmark truth engine. Local tooling only. Does not push, CI-skip, or mutate product crates.

import { mkdirSync, writeFileSync } from 'node:fs'
import { dirname, resolve } from 'node:path'
import { fileURLToPath } from 'node:url'
import { buildSnapshot } from './snapshot.mjs'
import { renderTruthReport, validateReportMatchesSnapshot } from './report.mjs'
import { diffSnapshots, assertUnmixed } from './diff.mjs'
import { loadJson } from '../fic-recompute-lib.mjs'

const ROOT = resolve(dirname(fileURLToPath(import.meta.url)), '..', '..', '..')

function arg(args, name) {
  const idx = args.indexOf(name)
  return idx >= 0 ? args[idx + 1] : undefined
}

function main() {
  const args = process.argv.slice(2)
  const root = arg(args, '--root') || ROOT
  const out = arg(args, '--out')
  const reportOut = arg(args, '--report')
  const diffAgainst = arg(args, '--diff')
  const generatedAt = arg(args, '--generated-at') || new Date().toISOString()
  const repositorySha = arg(args, '--repository-sha')
  const baseSha = arg(args, '--base-sha')
  const productTruth = args.includes('--product-truth-sha') ? arg(args, '--product-truth-sha') : undefined

  const built = buildSnapshot({
    root,
    generated_at: generatedAt,
    repository_sha: repositorySha,
    base_sha: baseSha,
    product_truth_sha: productTruth === '' ? null : productTruth,
  })
  const snapshot = built.snapshot
  const markdown = renderTruthReport(snapshot)
  const reportCheck = validateReportMatchesSnapshot(markdown, snapshot)
  if (!reportCheck.ok) built.errors.push(...reportCheck.errors)

  if (diffAgainst) {
    const previous = loadJson(diffAgainst)
    const diff = diffSnapshots(previous, snapshot)
    const mixed = assertUnmixed(diff)
    snapshot.diff = diff
    if (!mixed.ok) built.errors.push(...mixed.errors)
  }

  const payload = { ok: built.ok, errors: built.errors, snapshot }
  if (out) {
    mkdirSync(dirname(out), { recursive: true })
    writeFileSync(out, `${JSON.stringify(payload, null, 2)}\n`)
  }
  if (reportOut) {
    mkdirSync(dirname(reportOut), { recursive: true })
    writeFileSync(reportOut, markdown)
  }
  if (!out) {
    console.log(`benchmark-truth-engine: freshness=${snapshot.provenance.freshness} FIC=${snapshot.fic.l3_or_higher}/${snapshot.fic.denominator} verdict=${snapshot.frozen_architecture.verdict}`)
    console.log(`  sha=${snapshot.provenance.repository_sha}`)
    console.log(`  stale_reason=${snapshot.provenance.stale_reason || 'none'}`)
    console.log(`  authoritative=${snapshot.frozen_architecture.authoritative_architecture_verdict}`)
    if (built.errors.length) {
      console.error(built.errors.slice(0, 30).join('\n'))
    }
  }
  process.exitCode = 0
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  main()
}
