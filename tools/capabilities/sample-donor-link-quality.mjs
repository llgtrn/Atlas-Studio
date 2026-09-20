#!/usr/bin/env node
// sample-donor-link-quality.mjs - cloud-readable stratified random sampler over ALL donor-source ->
// canonical-capability provenance links (not just a money-flag slice), for manual functional-
// coherence review.
//
// Context: coverage-cycle-3 (docs/_machine/reconcile-reports/cloud/coverage-cycle-3-money-
// underclaimed-20260728.json) found a 94% (46/49) false-positive rate among source_money_
// underclaimed provenance links -- links where the canonical capability's real function has no
// discernible relationship to the donor feature it is mapped to -- once judged by functional
// coherence rather than donor/keyword-name proximity. This tool draws a stratified random sample
// across domains from the FULL source_capability<->canonical_capability link population (the same
// git-tracked docs/capabilities-cloud/{cap-core.db,cap-provenance.db} snapshot
// tools/capabilities/audit-truth-cloud-snapshot.mjs reads), excluding a caller-supplied set of
// already-reviewed canonical_key values, so a human/agent reviewer can check whether the 94% figure
// is specific to the money-flag slice or reflects donor-link provenance quality more generally.
//
// This is read-only, diagnostic/reporting only. It does not write to any DB and it does not judge
// functional coherence itself -- per docs/doctrines/094-operating-donor-grounding-protocol.md's rule that
// classification must come from reading the real described behavior, not a distilled/keyword
// summary, coherence judgment on the sampled rows remains manual review work.
//
// Usage:
//   node tools/capabilities/sample-donor-link-quality.mjs --per-domain 5 --seed 20260728
//   node tools/capabilities/sample-donor-link-quality.mjs --per-domain 5 --exclude-file <path.json>
import { join, resolve } from 'node:path'
import { fileURLToPath } from 'node:url'
import { readFileSync } from 'node:fs'
import { DOCS } from '../_paths.mjs'
import { openReadOnlyDatabase } from '../db/sqlite-open.mjs'

// Deterministic PRNG (mulberry32) so a given --seed reproduces the same sample across runs. This is
// diagnostic sampling, not a security or fairness-critical random source.
export function mulberry32(seed) {
  let a = seed >>> 0
  return function next() {
    a = (a + 0x6d2b79f5) | 0
    let t = Math.imul(a ^ (a >>> 15), 1 | a)
    t = (t + Math.imul(t ^ (t >>> 7), 61 | t)) ^ t
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296
  }
}

export function stratifiedSample(rowsByDomain, { perDomain, excludedKeys = new Set(), rng = Math.random } = {}) {
  const sample = []
  const domainReport = {}
  for (const domain of Object.keys(rowsByDomain).sort()) {
    const rows = rowsByDomain[domain]
    const pool = rows.filter((row) => !excludedKeys.has(row.canonicalKey))
    const shuffled = pool.slice()
    for (let i = shuffled.length - 1; i > 0; i -= 1) {
      const j = Math.floor(rng() * (i + 1))
      ;[shuffled[i], shuffled[j]] = [shuffled[j], shuffled[i]]
    }
    const picked = shuffled.slice(0, perDomain)
    domainReport[domain] = { pool_size: pool.length, excluded_from_pool: rows.length - pool.length, sampled: picked.length }
    sample.push(...picked)
  }
  return { sample, domainReport }
}

const LINKS_QUERY = `
  SELECT
    s.id AS sourceId, s.donor AS donor, s.canonical_name AS sourceName,
    s.source_files AS sourceFiles, s.business_behavior AS sourceBusinessBehavior,
    s.technical_behavior AS sourceTechnicalBehavior, s.target_crate AS sourceTargetCrate,
    c.key AS canonicalKey, c.canonical_name AS canonicalName, c.domain AS canonicalDomain,
    c.target_crate AS canonicalTargetCrate, c.status AS canonicalStatus,
    COALESCE(c.moves_money, 0) AS canonicalMovesMoney
  FROM source_capability s
  JOIN core.canonical_capability c ON c.id = s.canonical_id
`

export function loadLinksByDomain({ dir = join(DOCS, 'capabilities-cloud') } = {}) {
  const provenanceDb = join(dir, 'cap-provenance.db')
  const coreDb = join(dir, 'cap-core.db')
  const db = openReadOnlyDatabase(provenanceDb)
  db.exec(`ATTACH DATABASE '${coreDb.replace(/'/g, "''")}' AS core`)

  let rows
  try {
    rows = db.prepare(LINKS_QUERY).all()
  } finally {
    db.close()
  }

  const byDomain = {}
  for (const row of rows) {
    const domain = row.canonicalDomain || 'unknown'
    byDomain[domain] = byDomain[domain] || []
    byDomain[domain].push(row)
  }
  return byDomain
}

function parseArgs(argv) {
  const out = { dir: join(DOCS, 'capabilities-cloud'), perDomain: 5, seed: 1, excludeFile: null, help: false }
  for (let i = 0; i < argv.length; i += 1) {
    const arg = argv[i]
    if (arg === '--help' || arg === '-h') {
      out.help = true
      continue
    }
    if (arg === '--dir') {
      out.dir = argv[i + 1]
      i += 1
      continue
    }
    if (arg === '--per-domain') {
      out.perDomain = Number(argv[i + 1])
      i += 1
      continue
    }
    if (arg === '--seed') {
      out.seed = Number(argv[i + 1])
      i += 1
      continue
    }
    if (arg === '--exclude-file') {
      out.excludeFile = argv[i + 1]
      i += 1
      continue
    }
    throw new Error(`unknown argument: ${arg}`)
  }
  return out
}

function usage() {
  return 'Usage: node tools/capabilities/sample-donor-link-quality.mjs [--dir <docs/capabilities-cloud>] [--per-domain 5] [--seed 1] [--exclude-file <keys.json>]'
}

function main() {
  let args
  try {
    args = parseArgs(process.argv.slice(2))
  } catch (error) {
    console.error(`${error.message}\n${usage()}`)
    process.exit(2)
  }
  if (args.help) {
    console.log(usage())
    return
  }
  try {
    const excludedKeys = new Set(args.excludeFile ? JSON.parse(readFileSync(args.excludeFile, 'utf8')) : [])
    const byDomain = loadLinksByDomain({ dir: args.dir })
    const { sample, domainReport } = stratifiedSample(byDomain, {
      perDomain: args.perDomain,
      excludedKeys,
      rng: mulberry32(args.seed),
    })
    console.log(JSON.stringify({ seed: args.seed, per_domain: args.perDomain, excluded_count: excludedKeys.size, domain_report: domainReport, sample }, null, 2))
  } catch (error) {
    console.error(`sample-donor-link-quality failed: ${error.message}`)
    process.exit(2)
  }
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  main()
}
