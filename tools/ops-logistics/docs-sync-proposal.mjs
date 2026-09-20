#!/usr/bin/env node
import { writeFileSync } from 'node:fs'
import { resolve } from 'node:path'
import { buildAbsorptionPlan, buildDocsSyncProposal, collectOpsReality, loadOpsManifest, parseCli, validateSemanticConvergence } from './lib.mjs'
import { validateChronicaEvidence } from './chronica-evidence.mjs'
import { compileAtlas } from '../docs-atlas/lib.mjs'
import { compileReality } from '../reality-atlas/lib.mjs'

const args = parseCli(process.argv.slice(2))
const opsRoot = resolve(String(args.ops ?? process.cwd()))
const chronicaRoot = resolve(String(args.chronica ?? process.cwd()))
const { manifest } = loadOpsManifest(opsRoot)
const reality = collectOpsReality(opsRoot)
const semantic = validateSemanticConvergence(manifest, reality)
const chronicaEvidence = validateChronicaEvidence(chronicaRoot, manifest)
const combined = {
  ...semantic,
  errors: [...semantic.errors, ...chronicaEvidence.errors],
  warnings: [...semantic.warnings, ...chronicaEvidence.warnings],
}
const plan = buildAbsorptionPlan(manifest, reality, combined)
let chronicaReality = null
try {
  const atlas = compileAtlas(chronicaRoot)
  chronicaReality = compileReality(chronicaRoot, atlas)
} catch (error) {
  chronicaReality = { drift: [{ kind: 'CHRONICA_REALITY_UNAVAILABLE', message: error instanceof Error ? error.message : String(error) }] }
}
const proposal = buildDocsSyncProposal(manifest, reality, plan, chronicaReality)
const payload = `${JSON.stringify(proposal, null, 2)}\n`
if (args.output) writeFileSync(resolve(String(args.output)), payload, 'utf8')
else process.stdout.write(payload)

if (args['require-clean'] && proposal.docsSyncRequired) process.exit(2)
