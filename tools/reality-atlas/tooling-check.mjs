#!/usr/bin/env node
import { dirname, resolve } from 'node:path'
import { fileURLToPath } from 'node:url'
import { compileToolingReality, validateToolingReality } from './tooling-audit.mjs'

const toolDir = dirname(fileURLToPath(import.meta.url))
const root = resolve(toolDir, '../..')
const tooling = compileToolingReality(root)
const errors = validateToolingReality(tooling)

if (errors.length) {
  console.error('Chronica tooling lifecycle audit FAILED.')
  for (const error of errors) console.error(`- ${error}`)
  process.exit(1)
}

console.log('Chronica tooling lifecycle audit OK.')
console.log(`groups=${tooling.stats.groups} trackedToolFiles=${tooling.stats.trackedToolFiles} executableToolFiles=${tooling.stats.executableToolFiles}`)
console.log(`reviewCandidates=${tooling.stats.reviewCandidates} transitionalGroups=${tooling.stats.transitionalGroups} rootLegacySurfaces=${tooling.stats.rootLegacySurfaces} rootLegacyFiles=${tooling.stats.rootLegacyFiles}`)
console.log(`statusCounts=${JSON.stringify(tooling.stats.statusCounts)}`)
for (const group of tooling.reviewCandidates) {
  console.log(`REVIEW ${group.status} ${group.path} refs=${group.referenceCount} ageDays=${group.ageDays ?? 'unknown'} executables=${group.executableFiles}`)
}
for (const group of tooling.transitional) {
  console.log(`TRANSITIONAL ${group.path} refs=${group.referenceCount} ageDays=${group.ageDays ?? 'unknown'} executables=${group.executableFiles}`)
}
