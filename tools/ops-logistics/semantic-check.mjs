#!/usr/bin/env node
import { resolve } from 'node:path'
import { collectOpsReality, loadOpsManifest, parseCli, validateSemanticConvergence } from './lib.mjs'
import { validateChronicaEvidence } from './chronica-evidence.mjs'
import { detectSemanticFingerprintCollisions } from './semantic-fingerprint.mjs'

const args = parseCli(process.argv.slice(2))
const opsRoot = resolve(String(args.ops ?? process.cwd()))
const chronicaRoot = resolve(String(args.chronica ?? process.cwd()))
const { manifest } = loadOpsManifest(opsRoot)
const reality = collectOpsReality(opsRoot)
const semantic = validateSemanticConvergence(manifest, reality)
const collisions = detectSemanticFingerprintCollisions(manifest)
const chronica = validateChronicaEvidence(chronicaRoot, manifest)
const errors = [...semantic.errors, ...collisions.errors, ...chronica.errors]
const warnings = [...semantic.warnings, ...chronica.warnings]
if (chronica.stale && !args['allow-stale']) errors.push(`Chronica semantic audit is stale: recorded=${chronica.sha} current=${chronica.currentHead}; refresh mappings before substantive refactor/absorption or pass --allow-stale for historical inspection only`)

console.log(`Ops Semantic Convergence Checker: ${errors.length ? 'FAILED' : 'OK'}`)
console.log(`mappings=${semantic.mappings} fingerprintCollisionGroups=${collisions.collisions.length} chronicaSha=${chronica.sha ?? 'unknown'} currentChronicaHead=${chronica.currentHead ?? 'unknown'} checkedChronicaPaths=${chronica.checkedPaths}`)
for (const warning of warnings) console.log(`WARN: ${warning}`)
for (const error of errors) console.error(`ERROR: ${error}`)
if (errors.length) process.exit(1)
