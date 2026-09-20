#!/usr/bin/env node
// fic-domain-evidence.mjs -- CLI for the per-domain FIC evidence aggregator
// (docs/doctrines/180-doctrine-world-class-infrastructure-coverage.md section 6's 55-domain denominator).
//
// For every domain in docs/_machine/world-class-infrastructure-coverage-v1.json (or a subset named
// with --domain), prints how many capability_keys map to it, by what method, how many AGREE/
// DISAGREE in the current sync-anchor-v2 pass, how many have reachable code, how many have test
// evidence, and whether the backing crate(s) have a crate doc at all.
//
// This is an evidence packet, NOT a level assignment -- it never prints or computes an L0-L6 value.
// See tools/reconcile/fic-domain-evidence-lib.mjs's header for the full honesty note, and
// tools/reconcile/fic-evidence-report.mjs for the full-run report writer (deliverable 2).
//
// Usage:
//   node tools/reconcile/fic-domain-evidence.mjs                       # all 55 domains, human summary
//   node tools/reconcile/fic-domain-evidence.mjs --domain identity-session [--domain ...]
//   node tools/reconcile/fic-domain-evidence.mjs --json
//   node tools/reconcile/fic-domain-evidence.mjs --out path.json
import { mkdirSync, writeFileSync } from 'node:fs'
import { dirname } from 'node:path'
import { computeAllDomainEvidence } from './fic-domain-evidence-lib.mjs'

function parseArgs(argv) {
  const opts = { domains: [], json: false, out: null }
  for (let i = 0; i < argv.length; i += 1) {
    const arg = argv[i]
    if (arg === '--domain') opts.domains.push(argv[++i])
    else if (arg === '--json') opts.json = true
    else if (arg === '--out') opts.out = argv[++i]
    else {
      console.error(`fic-domain-evidence: unknown argument "${arg}"`)
      process.exit(2)
    }
  }
  return opts
}

function printHumanSummary(result) {
  console.log(
    `fic-domain-evidence: evidence packets for ${result.per_domain.length}/${result.domains_total} domain(s) ` +
      `(v1.json as_of_date=${result.v1_as_of_date}, status=${result.v1_status})`,
  )
  console.log('  This does NOT compute FIC and does NOT assign a level -- see the doc header before trusting a green run.')
  console.log('')
  for (const d of result.per_domain) {
    console.log(`── ${d.key} (hand-assigned: ${d.hand_assigned.level} / ${d.hand_assigned.state}) ──`)
    if (d.mapping.method === 'NOT_MAPPABLE') {
      console.log(`  mapping: NOT_MAPPABLE -- ${d.mapping.note}`)
      console.log('')
      continue
    }
    console.log(`  mapping: ${d.mapping.method} via crate(s) [${d.backing_crates.join(', ') || 'none in workspace'}]`)
    if (d.mapping.crate_names_missing_from_workspace.length) {
      console.log(`  note: configured crate(s) not found in workspace: ${d.mapping.crate_names_missing_from_workspace.join(', ')}`)
    }
    console.log(`  mapped capabilities: ${d.mapped_capabilities.total} (explicit_domain_field=${d.mapped_capabilities.by_method.explicit_domain_field}, crate_name_heuristic=${d.mapped_capabilities.by_method.crate_name_heuristic})`)
    if (d.mapped_capabilities.total === 0) {
      console.log('  no capability-catalog rows found for the backing crate(s) -- cannot compute sub-signals.')
      console.log('')
      continue
    }
    console.log(`  sync-anchor-v2: ${d.sync_anchor_v2.agree} AGREE / ${d.sync_anchor_v2.disagree} DISAGREE (${d.sync_anchor_v2.agree_percent}% agree)`)
    if (d.sync_anchor_v2.disagree) {
      for (const [reason, count] of Object.entries(d.sync_anchor_v2.disagree_by_reason)) console.log(`    ${reason}: ${count}`)
    }
    console.log(`  reachable code (Tier 1/2): ${d.code_reachability.reachable}/${d.mapped_capabilities.total} (${d.code_reachability.reachable_percent}%)`)
    console.log(`  test evidence (Tier 3): ${d.test_evidence.tested}/${d.mapped_capabilities.total} (${d.test_evidence.tested_percent}%)`)
    const docsPresent = Object.entries(d.crate_doc).filter(([, path]) => path).length
    console.log(`  crate doc exists: ${docsPresent}/${Object.keys(d.crate_doc).length} backing crate(s) -- NOT a full owner/contract check`)
    console.log('')
  }
}

function main() {
  const root = process.cwd()
  const opts = parseArgs(process.argv.slice(2))
  const result = computeAllDomainEvidence(root)

  if (opts.domains.length) {
    result.per_domain = result.per_domain.filter((d) => opts.domains.includes(d.key))
  }

  if (opts.json) console.log(JSON.stringify(result, null, 2))
  else printHumanSummary(result)

  if (opts.out) {
    mkdirSync(dirname(opts.out), { recursive: true })
    writeFileSync(opts.out, `${JSON.stringify(result, null, 2)}\n`)
    console.log(`fic-domain-evidence: wrote report to ${opts.out}`)
  }
}

main()
