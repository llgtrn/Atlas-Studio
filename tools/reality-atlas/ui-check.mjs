#!/usr/bin/env node
import { dirname, resolve } from 'node:path'
import { fileURLToPath } from 'node:url'
import { compileUiReality, validateUiReality } from './ui-audit.mjs'

const toolDir = dirname(fileURLToPath(import.meta.url))
const root = resolve(toolDir, '../..')
const ui = compileUiReality(root)
const errors = validateUiReality(ui)

if (errors.length) {
  console.error('Chronica UI lifecycle audit FAILED.')
  for (const error of errors) console.error(`- ${error}`)
  process.exit(1)
}

console.log('Chronica UI lifecycle audit OK.')
console.log(`tracked=${ui.stats.trackedUiFiles} source=${ui.stats.sourceFiles} productionReachable=${ui.stats.productionReachableFiles}`)
console.log(`orphans=${ui.stats.orphanCandidates} testOnly=${ui.stats.testOnlyFiles} storyOnly=${ui.stats.storyOnlyFiles} supportOnly=${ui.stats.supportOnlyFiles}`)
console.log(`transitional=${ui.stats.transitionalFiles} publicAssets=${ui.stats.publicAssets} unreferencedAssets=${ui.stats.unreferencedPublicAssets} workspacePackages=${ui.stats.workspacePackages} packageReview=${ui.stats.packageReviewCandidates}`)
console.log(`previewArtifacts=${ui.stats.previewArtifacts} retiredPublicArtifacts=${ui.stats.retiredPublicArtifacts} donorIdentity=${ui.stats.donorIdentityResidues}`)
for (const file of ui.orphanCandidates.slice(0, 100)) console.log(`REVIEW ORPHAN_CANDIDATE ${file.path} inbound=${file.inboundRefs}`)
for (const asset of ui.unreferencedPublicAssets.slice(0, 100)) console.log(`REVIEW UNREFERENCED_ASSET ${asset.path}`)
for (const pkg of ui.packageReviewCandidates.slice(0, 100)) console.log(`REVIEW ${pkg.status} ${pkg.path} name=${pkg.name ?? 'unknown'} rootDeclared=${pkg.rootDeclared}`)
for (const entry of ui.transitional) console.log(`TRANSITIONAL ${entry.id} files=${entry.files.length}`)
