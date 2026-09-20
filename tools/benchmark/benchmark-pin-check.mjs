#!/usr/bin/env node
import { fileURLToPath } from 'node:url'
import path from 'node:path'
import { readFileSync } from 'node:fs'
import { checkPortfolioAndLedger, portfolioMinimumForFamily } from './benchmark-pin-check-lib.mjs'
import { matchPortfolioEntries, loadRegistry } from './query-benchmark-ratio.mjs'

const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..', '..')

function main() {
  const result = checkPortfolioAndLedger(ROOT, { requireLedger094: true })
  console.log(`benchmark-pin-check: portfolio=${result.portfolio_count} ledger=${result.ledger_count}`)
  if (!result.ok) {
    console.error(`BENCHMARK_BLOCKED_WITH_EVIDENCE (${result.errors.length} errors)`)
    for (const err of result.errors) console.error(`  ${err}`)
  }

  const registry = loadRegistry()
  let missing = 0
  for (const family of registry.families) {
    const matched = matchPortfolioEntries(family, result.portfolio)
    const min = portfolioMinimumForFamily(family, matched)
    const label = min.meets_minimum ? 'MEETS' : 'MISSING'
    if (!min.meets_minimum) missing += 1
    console.log(`  family ${family.family_key}: ${label} ${JSON.stringify(min.counts)}${min.gaps.length ? ` gaps=${min.gaps.join(';')}` : ''}`)
  }
  console.log(`portfolio_minimum_missing: ${missing}/${registry.families.length}`)
  process.exitCode = result.ok ? 0 : 1
}

if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  main()
}
