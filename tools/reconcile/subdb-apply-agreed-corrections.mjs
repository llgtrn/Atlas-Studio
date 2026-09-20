#!/usr/bin/env node
// subdb-apply-agreed-corrections.mjs -- CLI for the mechanically-safe half of the conflict-audit
// writer (docs/doctrines/023-five-dimension-cloud-shard-contract.md).
//
// Runs the sync-anchor-v2 atom parser (imported, never re-implemented) over the requested crates,
// then appends a hash-chained `capability_status_correction` ledger record for every EXACT-confidence
// SHARD_CLAIMS_UNIMPLEMENTED_BUT_CODE_IS_REAL finding. PREFIX-confidence findings, and every other
// disagreement reason, are left untouched -- those need `subdb-conflict-writer.mjs`'s human review
// queue, not an automatic fix.
//
// Usage:
//   node tools/reconcile/subdb-apply-agreed-corrections.mjs                 # all workspace crates
//   node tools/reconcile/subdb-apply-agreed-corrections.mjs --crate chronica-cli [--crate ...]
//   node tools/reconcile/subdb-apply-agreed-corrections.mjs --json          # machine-readable stdout
//   node tools/reconcile/subdb-apply-agreed-corrections.mjs --out path.json # also write a report to disk
//
// This never touches docs/capabilities.db or docs/architecture.db -- it is crate-local shard
// accuracy only (doc 023 sec 4). The root "Verified" ratio still requires the separate local-audit
// promotion process.
import { mkdirSync, writeFileSync } from 'node:fs'
import { dirname } from 'node:path'
import { applyAgreedCorrections } from './subdb-apply-agreed-corrections-lib.mjs'
import { discoverWorkspaceCrates, verifyCrates } from './sync-anchor-v2-lib.mjs'

function parseArgs(argv) {
  const opts = { crates: [], json: false, out: null }
  for (let i = 0; i < argv.length; i += 1) {
    const arg = argv[i]
    if (arg === '--crate') opts.crates.push(argv[++i])
    else if (arg === '--json') opts.json = true
    else if (arg === '--out') opts.out = argv[++i]
    else {
      console.error(`subdb-apply-agreed-corrections: unknown argument "${arg}"`)
      process.exit(2)
    }
  }
  return opts
}

function printHumanSummary(result) {
  console.log(
    'subdb-apply-agreed-corrections: crate-local shard fixes only ' +
      '(does not touch docs/capabilities.db\'s root Verified ratio -- see doc 023 sec 4).',
  )
  console.log(
    `  ${result.exact_confidence_findings} EXACT-confidence SHARD_CLAIMS_UNIMPLEMENTED_BUT_CODE_IS_REAL finding(s) ` +
      `-> ${result.corrected_count} corrected, ${result.skipped_count} skipped, ` +
      `${result.deferred_to_prefix_count} deferred to PREFIX-confidence review queue.`,
  )
  console.log(`  ${result.crates_touched} crate(s) touched.`)
  if (result.skipped_count) {
    console.log('  skipped findings:')
    for (const crate of result.per_crate) {
      for (const skip of crate.skipped) console.log(`    ${crate.crate}/${skip.capability_key}: ${skip.reason}`)
    }
  }
  if (result.verify_failures.length) {
    console.log('  !! subdb:verify FAILED after applying corrections for:')
    for (const failure of result.verify_failures) {
      console.log(`    ${failure.crate}: ${failure.errors.join('; ')}`)
    }
  } else if (result.crates_touched) {
    console.log('  subdb:verify passed for every touched crate.')
  }
}

function main() {
  const root = process.cwd()
  const opts = parseArgs(process.argv.slice(2))
  const crates = discoverWorkspaceCrates(root)
  const targetCrates = opts.crates.length ? opts.crates : crates.map((c) => c.name)

  const summary = verifyCrates(root, targetCrates)
  const result = applyAgreedCorrections({ root, findings: summary.disagreements })

  if (opts.json) console.log(JSON.stringify(result, null, 2))
  else printHumanSummary(result)

  if (opts.out) {
    mkdirSync(dirname(opts.out), { recursive: true })
    writeFileSync(opts.out, `${JSON.stringify(result, null, 2)}\n`)
    console.log(`subdb-apply-agreed-corrections: wrote report to ${opts.out}`)
  }

  process.exitCode = result.verify_failures.length ? 1 : 0
}

main()
