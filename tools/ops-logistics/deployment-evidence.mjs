#!/usr/bin/env node
import { resolve } from 'node:path'
import { loadOpsManifest, parseCli, repoSha, deploymentReality, validateDeploymentRecord, writeDeploymentEvidence } from './lib.mjs'

const args = parseCli(process.argv.slice(2))
const command = String(args._[0] ?? 'check')
const opsRoot = resolve(String(args.ops ?? process.cwd()))
const { manifest } = loadOpsManifest(opsRoot)

if (command === 'record') {
  const record = {
    environment: String(args.environment ?? ''),
    gitSha: String(args.sha ?? repoSha(opsRoot)),
    deploymentId: String(args['deployment-id'] ?? ''),
    buildId: args['build-id'] ? String(args['build-id']) : null,
    runtimeVersion: args['runtime-version'] ? String(args['runtime-version']) : null,
    artifactDigest: String(args['artifact-digest'] ?? ''),
    healthStatus: String(args.health ?? 'UNKNOWN').toUpperCase(),
    evidenceRefs: args.evidence ? String(args.evidence).split(',').map((value) => value.trim()).filter(Boolean) : [],
    observedAt: args['observed-at'] ? String(args['observed-at']) : new Date().toISOString(),
  }
  const errors = validateDeploymentRecord(record)
  if (errors.length) {
    for (const error of errors) console.error(`ERROR: ${error}`)
    process.exit(1)
  }
  const { file } = writeDeploymentEvidence(opsRoot, manifest, record)
  console.log(`Recorded deployment/runtime evidence: ${file}`)
  console.log(`${record.environment} ${record.gitSha} ${record.healthStatus} ${record.deploymentId}`)
  process.exit(0)
}

if (command !== 'check') {
  console.error(`Unknown command: ${command}; expected record or check`)
  process.exit(1)
}

const reality = deploymentReality(opsRoot, manifest)
let failed = Boolean(reality.parseError)
console.log(`Deployment evidence: ${reality.file}`)
if (reality.parseError) console.error(`ERROR: ${reality.parseError}`)
for (const env of reality.environments) {
  console.log(`${env.environment}: ${env.status}${env.record ? ` sha=${env.record.gitSha} health=${env.record.healthStatus}` : ''}`)
  if (env.required && env.status === 'MISSING') failed = true
  if (env.required && args['require-current'] && env.status !== 'CURRENT_HEAD') failed = true
  if (env.record && env.record.healthStatus === 'FAIL') failed = true
}
if (failed) process.exit(1)
