#!/usr/bin/env node

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

const toolDir = dirname(fileURLToPath(import.meta.url))
const root = resolve(toolDir, '../..')
const atlas = compileAtlas(root)
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
  ...validateAtlas(atlas),
  ...validateReality(reality, atlas),
  ...validateToolingReality(tooling),
  ...validateRootTopology(rootTopology),
  ...validateUiReality(ui),
  ...validateSystemCoverage(systemCoverage),
  ...validateUnifiedSystemCoverage(systemCoverage),
]

if (errors.length) {
  console.error('Chronica System Atlas contract FAILED.')
  for (const error of errors) console.error(`- ${error}`)
  process.exit(1)
}

const docNodes = atlas.nodes.filter((node) => node.id.startsWith('doc:'))
const orphanDocs = docNodes.filter((node) => !atlas.edges.some((edge) => edge.source === node.id || edge.target === node.id))

console.log('Chronica System Atlas contract OK.')
console.log(`documents=${atlas.stats.documents} architectureOwners=${atlas.stats.architectureOwners} contracts=${atlas.stats.contracts} runtimeOwners=${atlas.stats.runtimeOwners} presentationAnnotatedDocs=${atlas.stats.presentationAnnotatedDocs ?? 0} totalEdges=${atlas.stats.edges}`)
console.log(`semanticEdges=${atlas.stats.semanticEdges} ownersWithSemanticEdges=${atlas.stats.ownersWithSemanticEdges}/${atlas.stats.architectureOwners} semanticOwnerCoverage=${atlas.stats.semanticOwnerCoverage}%`)
console.log(`realitySha=${reality.sourceSha} dirty=${reality.workingTreeDirty} observedSourceFiles=${reality.stats.observedSourceFiles} legacySourceFiles=${reality.stats.legacySourceFiles} orphanPackages=${reality.stats.orphanPackages} driftFindings=${reality.stats.driftFindings}`)
console.log(`ownerStates=${JSON.stringify(reality.stats.ownerStates)}`)
console.log(`toolGroups=${tooling.stats.groups} toolReviewCandidates=${tooling.stats.reviewCandidates} transitionalToolGroups=${tooling.stats.transitionalGroups} toolStatusCounts=${JSON.stringify(tooling.stats.statusCounts)}`)
console.log(`rootEntries=${rootTopology.stats.rootEntries} rootReview=${rootTopology.stats.reviewEntries} retiredRootViolations=${rootTopology.stats.retiredViolations} donorResidues=${rootTopology.stats.donorResidues}`)
console.log(`uiSource=${ui.stats.sourceFiles} uiReachable=${ui.stats.productionReachableFiles} uiOrphans=${ui.stats.orphanCandidates} uiTests=${ui.stats.testOnlyFiles} uiStories=${ui.stats.storyOnlyFiles} uiTransitional=${ui.stats.transitionalFiles}`)
console.log(`systemGraphs=${systemCoverage.stats.graphFamilies}/${systemCoverage.stats.requiredGraphFamilies} domainCoverage=${systemCoverage.stats.domainCoveragePercent}% trackedArtifactCoverage=${systemCoverage.stats.trackedArtifactCoveragePercent}% semanticMapping=${systemCoverage.stats.semanticMappingPercent}%`)
console.log(`systemNodes=${systemCoverage.stats.totalNodes} systemEdges=${systemCoverage.stats.totalEdges} systemGaps=${systemCoverage.stats.totalGaps}`)
console.log(`unifiedNodes=${systemCoverage.stats.unifiedNodes} unifiedEdges=${systemCoverage.stats.unifiedEdges} crossFamilyEdges=${systemCoverage.stats.crossFamilyEdges}`)
for (const family of systemCoverage.graphFamilies) {
  const stats = systemCoverage.stats.familyStats[family]
  console.log(`systemFamily=${family}:nodes=${stats.nodes}:edges=${stats.edges}:artifacts=${stats.artifacts}:gaps=${stats.gaps}`)
}
if (tooling.reviewCandidates.length) {
  for (const group of tooling.reviewCandidates.slice(0, 20)) console.log(`toolReview=${group.status}:${group.path}:refs=${group.referenceCount}:ageDays=${group.ageDays ?? 'unknown'}`)
}
for (const entry of rootTopology.reviewEntries) console.log(`rootReview=${entry.status}:${entry.path}`)
for (const file of ui.orphanCandidates.slice(0, 30)) console.log(`uiReview=ORPHAN_CANDIDATE:${file.path}:refs=${file.inboundRefs}`)
for (const entry of ui.transitional) console.log(`uiTransitional=${entry.id}:files=${entry.files.length}`)
if (orphanDocs.length) console.log(`orphanDocs=${orphanDocs.length} (advisory; shown in Atlas for cleanup)`)
