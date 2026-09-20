#!/usr/bin/env node
import { resolve } from 'node:path'
import { buildAbsorptionPlan, buildDocsSyncProposal, collectOpsReality, loadOpsManifest, logisticsScorecard, parseCli, validateSemanticConvergence } from './lib.mjs'
import { validateChronicaEvidence } from './chronica-evidence.mjs'
import { detectSemanticFingerprintCollisions } from './semantic-fingerprint.mjs'

const args = parseCli(process.argv.slice(2))
const opsRoot = resolve(String(args.ops ?? process.cwd()))
const chronicaRoot = resolve(String(args.chronica ?? process.cwd()))
const { manifest } = loadOpsManifest(opsRoot)
const reality = collectOpsReality(opsRoot)
const semantic = validateSemanticConvergence(manifest, reality)
const collisions = detectSemanticFingerprintCollisions(manifest)
const chronicaEvidence = validateChronicaEvidence(chronicaRoot, manifest)
const combined = {
  ...semantic,
  errors: [...semantic.errors, ...collisions.errors, ...chronicaEvidence.errors],
  warnings: [...semantic.warnings, ...chronicaEvidence.warnings],
  fingerprintCollisions: collisions.collisions,
}
if (chronicaEvidence.stale && !args['allow-stale']) combined.errors.push(`Chronica semantic audit is stale: recorded=${chronicaEvidence.sha} current=${chronicaEvidence.currentHead}`)
const plan = buildAbsorptionPlan(manifest, reality, combined)
const proposal = buildDocsSyncProposal(manifest, reality, plan, null)
const scorecard = logisticsScorecard({ manifest, reality, semanticCheck: combined, absorptionPlan: plan, docsProposal: proposal })

const semanticCheck = scorecard.checks.find((entry) => entry.id === 'SEMANTIC_CONVERGENCE_CHECKER')
if (semanticCheck) semanticCheck.pass = combined.errors.length === 0
const deploymentCheck = scorecard.checks.find((entry) => entry.id === 'DEPLOYMENT_RUNTIME_EVIDENCE')
if (deploymentCheck) {
  deploymentCheck.pass = reality.deployment.requiredEnvironments.every((environment) => {
    const entry = reality.deployment.environments.find((candidate) => candidate.environment === environment)
    return entry?.status === 'CURRENT_HEAD' && entry?.record?.healthStatus === 'PASS'
  })
}
scorecard.score = scorecard.checks.filter((entry) => entry.pass).length
scorecard.total = scorecard.checks.length
scorecard.percent = Number(((scorecard.score / scorecard.total) * 100).toFixed(1))

console.log(`Ops Logistics Scorecard: ${scorecard.score}/${scorecard.total} (${scorecard.percent}%)`)
console.log(`semanticFingerprintCollisionGroups=${collisions.collisions.length}`)
for (const check of scorecard.checks) console.log(`${check.pass ? 'PASS' : 'FAIL'} ${check.id}`)
for (const warning of combined.warnings) console.log(`WARN ${warning}`)
for (const error of combined.errors) console.error(`ERROR ${error}`)
if (scorecard.score !== scorecard.total) process.exit(1)
