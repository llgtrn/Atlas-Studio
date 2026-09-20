#!/usr/bin/env node

import { dirname, resolve } from 'node:path'
import { fileURLToPath } from 'node:url'
import { compileAtlas, validateAtlas } from '../docs-atlas/lib.mjs'
import { compileReality, validateReality } from './lib.mjs'
import { compileToolingReality, validateToolingReality } from './tooling-audit.mjs'
import { compileRootTopology, validateRootTopology } from './root-topology.mjs'
import { compileUiReality, validateUiReality } from './ui-audit.mjs'

const toolDir = dirname(fileURLToPath(import.meta.url))
const root = resolve(toolDir, '../..')
const atlas = compileAtlas(root)
const reality = compileReality(root, atlas)
const tooling = compileToolingReality(root)
const rootTopology = compileRootTopology(root)
const ui = compileUiReality(root)
reality.tooling = tooling
reality.rootTopology = rootTopology
reality.ui = ui
const errors = [
  ...validateAtlas(atlas),
  ...validateReality(reality, atlas),
  ...validateToolingReality(tooling),
  ...validateRootTopology(rootTopology),
  ...validateUiReality(ui),
]

if (errors.length) {
  console.error('Chronica Reality Mirror contract FAILED.')
  for (const error of errors) console.error(`- ${error}`)
  process.exit(1)
}

console.log('Chronica Reality Mirror contract OK.')
console.log(`sourceSha=${reality.sourceSha} dirty=${reality.workingTreeDirty}`)
console.log(`observedSourceFiles=${reality.stats.observedSourceFiles} architectureOwners=${reality.stats.architectureOwners} runtimeOwners=${reality.stats.runtimeOwners}`)
console.log(`contractRefs=${reality.stats.contractReferencesInCode} testFilesWithRefs=${reality.stats.testFilesWithArchitectureRefs}`)
console.log(`legacySourceFiles=${reality.stats.legacySourceFiles} orphanPackages=${reality.stats.orphanPackages} driftFindings=${reality.stats.driftFindings}`)
console.log(`ownerStates=${JSON.stringify(reality.stats.ownerStates)}`)
console.log(`toolReviewCandidates=${tooling.stats.reviewCandidates} rootViolations=${rootTopology.stats.retiredViolations + rootTopology.stats.donorResidues + rootTopology.stats.stalePathReferences}`)
console.log(`uiSource=${ui.stats.sourceFiles} uiReachable=${ui.stats.productionReachableFiles} uiOrphans=${ui.stats.orphanCandidates} uiTransitional=${ui.stats.transitionalFiles}`)
