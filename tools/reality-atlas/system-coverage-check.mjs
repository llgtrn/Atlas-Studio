#!/usr/bin/env node
import { dirname, resolve } from 'node:path'
import { fileURLToPath } from 'node:url'
import { compileToolingReality } from './tooling-audit.mjs'
import { compileUiReality } from './ui-audit.mjs'
import { validateSystemCoverage } from './system-coverage.mjs'
import { compileFullSystemCoverage } from './system-coverage-pipeline.mjs'
import { validateDisplayLanguage } from './system-coverage-display-language.mjs'
import { validateNamingTopology } from './system-coverage-naming.mjs'
import { validateUnifiedSystemCoverage } from './system-coverage-unified.mjs'

const toolDir = dirname(fileURLToPath(import.meta.url))
const root = resolve(toolDir, '../..')
const tooling = compileToolingReality(root)
const ui = compileUiReality(root)
const coverage = compileFullSystemCoverage(root, { tooling, ui })
const errors = [
  ...validateSystemCoverage(coverage),
  ...validateDisplayLanguage(coverage),
  ...validateNamingTopology(coverage),
  ...validateUnifiedSystemCoverage(coverage),
]

if (errors.length) {
  console.error('Chronica full System Atlas coverage FAILED.')
  for (const error of errors) console.error(`- ${error}`)
  process.exit(1)
}

console.log('Chronica full System Atlas coverage OK.')
console.log(`sourceSha=${coverage.sourceSha}`)
console.log(`domainCoverage=${coverage.stats.domainCoveragePercent}% (${coverage.stats.graphFamilies}/${coverage.stats.requiredGraphFamilies})`)
console.log(`trackedArtifactCoverage=${coverage.stats.trackedArtifactCoveragePercent}% (${coverage.stats.classifiedTrackedArtifacts}/${coverage.stats.trackedArtifacts})`)
console.log(`semanticMapping=${coverage.stats.semanticMappingPercent}% (${coverage.stats.semanticMappedSourceFiles}/${coverage.stats.sourceFiles} source files)`)
console.log(`nodes=${coverage.stats.totalNodes} familyEdges=${coverage.stats.totalEdges} gaps=${coverage.stats.totalGaps}`)
console.log(`unifiedNodes=${coverage.stats.unifiedNodes} unifiedEdges=${coverage.stats.unifiedEdges} crossFamilyEdges=${coverage.stats.crossFamilyEdges}`)
const display = coverage.displayLanguage?.stats ?? {}
console.log(`displayLanguage=activityProjectionFiles:${display.activityProjectionFiles ?? 0} translationAware:${display.translationAwareFiles ?? 0} hardcodedCopy:${display.hardcodedCopyFiles ?? 0} staleCanonicalTerms:${display.staleCanonicalTermFiles ?? 0} locales:${display.localeCatalogs ?? 0} primaryLocaleFamilies:${display.primaryLocaleFamiliesPresent ?? 0}/5`)
const naming = coverage.namingTopology?.states ?? {}
console.log(`namingTopology=canonical:${naming.CANONICAL_PATH ?? 0} legacyFlat:${naming.LEGACY_FLAT_PATH ?? 0} tierMismatch:${naming.TIER_MISMATCH ?? 0} capLegacy:${naming.CAP_LEGACY_UNRESOLVED ?? 0} namespaceMismatch:${naming.PACKAGE_NAMESPACE_MISMATCH ?? 0} legacyTier:${naming.LEGACY_TIER ?? 0}`)
console.log(`sourceEvidence=${coverage.verification?.sourceEvidence ?? 'AVAILABLE'}`)
console.log(`localExecution=${coverage.verification?.localExecution?.status ?? 'NOT_RUN'} assurance=${coverage.verification?.localExecution?.assurance ?? 'NOT_OBSERVED'} checks=${coverage.verification?.localExecution?.checks?.length ?? 0} cleanExactShaChecks=${coverage.verification?.localExecution?.exactCleanShaChecks?.length ?? 0}`)
console.log(`hostedCi=${coverage.verification?.hostedCi?.status ?? 'NOT_OBSERVED'}`)
console.log(`productionRuntime=${coverage.verification?.productionRuntime?.status ?? 'SEPARATE_EVIDENCE_SOURCE'}`)
for (const family of coverage.graphFamilies) {
  const stats = coverage.stats.familyStats[family]
  console.log(`${family}: nodes=${stats.nodes} edges=${stats.edges} artifacts=${stats.artifacts} gaps=${stats.gaps}`)
}
