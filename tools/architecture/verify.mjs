#!/usr/bin/env node
import { join } from 'node:path'
import { verifyArchitectureDb } from './architecture-lib.mjs'
import { verifyCanonicalCrateTopology } from './crate-topology-invariant.mjs'

const root = process.cwd()
const topology = verifyCanonicalCrateTopology({ root })
if (!topology.ok) {
  console.log(`verify-crate-topology: ✗ ${topology.errors.length} issue(s)`)
  for (const err of topology.errors) console.log(`    ${err}`)
  process.exit(1)
}
console.log(`verify-crate-topology: ✓ canonical roots ${topology.roots.join(', ')}`)

const dbPath = join(root, 'docs', 'architecture.db')
const report = verifyArchitectureDb({ root, dbPath })

console.log(`verify-architecture: ${report.counts.nodes ?? 0} nodes · ${report.counts.edges ?? 0} edges · ${report.counts.capabilityLinks ?? 0} capability links`)
if (report.counts.crates != null) {
  console.log(`  crates=${report.counts.crates} modules=${report.counts.modules} invariants=${report.counts.invariants} authority_rules=${report.counts.authorityRules} data_flows=${report.counts.dataFlows}`)
  console.log(`  money_gate_evidence_files=${report.counts.moneyGateEvidenceFiles} possible_bypass_files=${report.counts.bypassEvidenceFiles}`)
}
if (report.counts.canonicalCapabilities != null) {
  console.log(`  coverage: ${report.counts.accountedCapabilities}/${report.counts.canonicalCapabilities} capabilities accounted (unaccounted=${report.counts.unaccountedCapabilities}) · linked=${report.counts.linkedDistinctCapabilities} gap=${report.counts.gapCapabilities} planned_nodes=${report.counts.plannedNodes}`)
}

if (report.errors.length) {
  console.log(`  ✗ architecture drift: ${report.errors.length} issue(s)`)
  for (const err of report.errors.slice(0, 20)) console.log(`    ${err}`)
  if (report.errors.length > 20) console.log(`    ...and ${report.errors.length - 20} more`)
  process.exit(1)
}

console.log('  ✓ architecture DB agrees with repo evidence.')
