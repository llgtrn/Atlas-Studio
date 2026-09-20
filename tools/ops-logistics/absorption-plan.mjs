#!/usr/bin/env node
import { writeFileSync } from 'node:fs'
import { resolve } from 'node:path'
import { buildAbsorptionPlan, collectOpsReality, loadOpsManifest, parseCli, validateSemanticConvergence } from './lib.mjs'
import { validateChronicaEvidence } from './chronica-evidence.mjs'
import { detectSemanticFingerprintCollisions } from './semantic-fingerprint.mjs'
import { compileAtlas } from '../docs-atlas/lib.mjs'
import { compileReality } from '../reality-atlas/lib.mjs'

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
  chronicaEvidence,
}
if (chronicaEvidence.stale && !args['allow-stale']) combined.errors.push(`Chronica semantic audit is stale: recorded=${chronicaEvidence.sha} current=${chronicaEvidence.currentHead}`)

const plan = buildAbsorptionPlan(manifest, reality, combined)
plan.fingerprintCollisions = collisions.collisions
let currentChronicaReality = null
try {
  const atlas = compileAtlas(chronicaRoot)
  currentChronicaReality = compileReality(chronicaRoot, atlas)
  const ownerByPath = new Map(currentChronicaReality.owners.map((owner) => [owner.path, owner]))
  plan.actions = plan.actions.map((action) => {
    const owner = action.chronicaOwner ? ownerByPath.get(action.chronicaOwner) : null
    return owner ? {
      ...action,
      currentChronicaReality: {
        implementationState: owner.implementationState,
        runtimeSourceFiles: owner.runtimeSourceFiles,
        directCodeEvidenceFiles: owner.directCodeEvidenceFiles.length,
        directTestEvidenceFiles: owner.directTestEvidenceFiles.length,
        uncitedContracts: owner.uncitedContracts.length,
      },
    } : action
  })
  plan.currentChronicaRealitySha = currentChronicaReality.sourceSha
} catch (error) {
  plan.blockers.push({ kind: 'CHRONICA_REALITY_UNAVAILABLE', message: error instanceof Error ? error.message : String(error) })
  plan.readiness = 'BLOCKED'
}

const payload = `${JSON.stringify(plan, null, 2)}\n`
if (args.output) writeFileSync(resolve(String(args.output)), payload, 'utf8')
else process.stdout.write(payload)
if (args.strict && plan.readiness === 'BLOCKED') process.exit(1)
