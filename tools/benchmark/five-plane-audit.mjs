#!/usr/bin/env node
// five-plane-audit.mjs — validate the live five-plane architecture audit.
import { existsSync } from 'node:fs'
import path from 'node:path'
import { dirname, join } from 'node:path'
import { fileURLToPath } from 'node:url'
import { loadContracts } from './intent-contract-validate.mjs'
import { loadFivePlaneAudit, validateFivePlaneAudit } from './five-plane-audit-lib.mjs'

const ROOT = path.resolve(dirname(fileURLToPath(import.meta.url)), '..', '..')

function main() {
  const audit = loadFivePlaneAudit(ROOT)
  if (!audit) {
    console.error('missing docs/_machine/five-plane-architecture-audit.json')
    process.exitCode = 2
    return
  }
  const contractDir = join(ROOT, 'docs/_machine/intent-contracts')
  const contracts = existsSync(contractDir) ? loadContracts(contractDir).map((row) => row.contract) : []
  const result = validateFivePlaneAudit(audit, { contracts })
  const still = (audit.no_ai_matrix || []).filter((d) => d.no_ai === 'STILL_WORKS_WITHOUT_AI').length
  console.log(`five-plane-audit: verdict=${audit.verdict} ${audit.verdict_label}`)
  console.log(`  freeze=${audit.freeze?.five_plane_audit_status || 'MISSING'} authoritative=${audit.freeze?.authoritative_architecture_verdict}`)
  console.log(`  controller=${audit.freeze?.benchmark_controller_status || 'MISSING'} rerun=${audit.freeze?.rerun_trigger || 'MISSING'}`)
  console.log(`  until_then=${audit.freeze?.until_then || 'MISSING'}`)
  console.log(`  spine=${audit.spine_decision?.verdict} core_spine=${audit.freeze?.core_spine || 'MISSING'} imbalance=${audit.imbalance}`)
  console.log(`  no_ai_domains=${(audit.no_ai_matrix || []).length} still_works_without_ai=${still}`)
  if (!result.ok) {
    console.error(result.errors.join('\n'))
    process.exitCode = 1
  }
}

if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  main()
}
