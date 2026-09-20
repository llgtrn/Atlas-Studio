#!/usr/bin/env node

import { loadArchitectureContracts, validateContractSet } from './architecture-contract-lib.mjs'

const root = process.cwd()
const errors = []
let docs = []
let contracts = []

try {
  ;({ docs, contracts } = loadArchitectureContracts(root))
  errors.push(...validateContractSet(contracts))
} catch (error) {
  errors.push(error.message)
}

// Deliberately no minimum equation/invariant/performance counts and no required
// prose phrases. The engineering contract is human-first: this verifier checks
// only machine contracts authors explicitly chose to declare, their ownership,
// syntax and cross-references. It does not manufacture architecture content.
const counts = {
  equation: contracts.filter((row) => row.kind === 'equation').length,
  invariant: contracts.filter((row) => row.kind === 'invariant').length,
  performance: contracts.filter((row) => row.kind === 'performance').length,
}

if (errors.length) {
  console.error('Chronica architecture engineering contract FAILED.')
  for (const error of errors) console.error(`\n- ${error}`)
  process.exit(1)
}

const measured = contracts.filter((row) => row.kind === 'performance' && row.gate !== 'STRUCTURAL').length
console.log('Chronica architecture engineering contract OK.')
console.log(`Canonical architecture owners scanned: ${docs.length}`)
console.log(`Declared contracts: ${contracts.length} (equations=${counts.equation}, invariants=${counts.invariant}, performance=${counts.performance})`)
console.log(`Measured/production performance contracts: ${measured}`)
