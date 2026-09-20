#!/usr/bin/env node

import { mkdirSync, writeFileSync } from 'node:fs'
import { dirname, resolve } from 'node:path'
import { fileURLToPath } from 'node:url'
import { compileAtlas, validateAtlas } from './lib.mjs'
import { compileReality, validateReality } from '../reality-atlas/lib.mjs'
import { compileToolingReality, validateToolingReality } from '../reality-atlas/tooling-audit.mjs'
import { compileRootTopology, validateRootTopology } from '../reality-atlas/root-topology.mjs'
import { compileUiReality, validateUiReality } from '../reality-atlas/ui-audit.mjs'
import { validateSystemCoverage } from '../reality-atlas/system-coverage.mjs'
import { compileFullSystemCoverage } from '../reality-atlas/system-coverage-pipeline.mjs'
import { validateUnifiedSystemCoverage } from '../reality-atlas/system-coverage-unified.mjs'

function outputPath(argv) {
  const index = argv.indexOf('--output')
  if (index >= 0 && argv[index + 1]) return argv[index + 1]
  return 'apps/ui/public/docs-atlas.json'
}

const toolDir = dirname(fileURLToPath(import.meta.url))
const root = resolve(toolDir, '../..')
const target = resolve(root, outputPath(process.argv.slice(2)))
const atlas = compileAtlas(root)
const atlasErrors = validateAtlas(atlas)
const reality = compileReality(root, atlas)
const tooling = compileToolingReality(root)
const rootTopology = compileRootTopology(root)
const ui = compileUiReality(root)
const systemCoverage = compileFullSystemCoverage(root, { tooling, ui })
reality.tooling = tooling
reality.rootTopology = rootTopology
reality.ui = ui
reality.systemCoverage = systemCoverage
const errors = [
  ...atlasErrors,
  ...validateReality(reality, atlas),
  ...validateToolingReality(tooling),
  ...validateRootTopology(rootTopology),
  ...validateUiReality(ui),
  ...validateSystemCoverage(systemCoverage),
  ...validateUnifiedSystemCoverage(systemCoverage),
]

if (errors.length) {
  console.error('Chronica System Atlas build FAILED.')
  for (const error of errors) console.error(`- ${error}`)
  process.exit(1)
}

atlas.reality = reality
mkdirSync(dirname(target), { recursive: true })
writeFileSync(target, `${JSON.stringify(atlas, null, 2)}\n`, 'utf8')
console.log(`Chronica System Atlas built: ${target}`)
console.log(`documents=${atlas.stats.documents} contracts=${atlas.stats.contracts} runtimeOwners=${atlas.stats.runtimeOwners} edges=${atlas.stats.edges}`)
console.log(`realitySha=${reality.sourceSha} dirty=${reality.workingTreeDirty} observedSourceFiles=${reality.stats.observedSourceFiles} legacy=${reality.stats.legacySourceFiles} drift=${reality.stats.driftFindings}`)
console.log(`toolGroups=${tooling.stats.groups} toolReviewCandidates=${tooling.stats.reviewCandidates} transitionalToolGroups=${tooling.stats.transitionalGroups}`)
console.log(`rootEntries=${rootTopology.stats.rootEntries} rootReview=${rootTopology.stats.reviewEntries} retiredRootViolations=${rootTopology.stats.retiredViolations} donorResidues=${rootTopology.stats.donorResidues}`)
console.log(`uiSource=${ui.stats.sourceFiles} uiReachable=${ui.stats.productionReachableFiles} uiOrphans=${ui.stats.orphanCandidates} uiTransitional=${ui.stats.transitionalFiles}`)
console.log(`systemGraphs=${systemCoverage.stats.graphFamilies}/${systemCoverage.stats.requiredGraphFamilies} domainCoverage=${systemCoverage.stats.domainCoveragePercent}% trackedCoverage=${systemCoverage.stats.trackedArtifactCoveragePercent}% semanticMapping=${systemCoverage.stats.semanticMappingPercent}% nodes=${systemCoverage.stats.totalNodes} edges=${systemCoverage.stats.totalEdges} gaps=${systemCoverage.stats.totalGaps}`)
console.log(`unifiedNodes=${systemCoverage.stats.unifiedNodes} unifiedEdges=${systemCoverage.stats.unifiedEdges} crossFamilyEdges=${systemCoverage.stats.crossFamilyEdges}`)
console.log(`verification source=${systemCoverage.verification?.sourceEvidence ?? 'AVAILABLE'} local=${systemCoverage.verification?.localExecution?.status ?? 'NOT_RUN'} assurance=${systemCoverage.verification?.localExecution?.assurance ?? 'NOT_OBSERVED'} cleanExactShaChecks=${systemCoverage.verification?.localExecution?.exactCleanShaChecks?.length ?? 0} hostedCi=${systemCoverage.verification?.hostedCi?.status ?? 'NOT_OBSERVED'} production=${systemCoverage.verification?.productionRuntime?.status ?? 'SEPARATE_EVIDENCE_SOURCE'}`)
