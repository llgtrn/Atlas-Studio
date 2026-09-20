import { existsSync } from 'node:fs'
import { resolve } from 'node:path'
import { git } from './lib.mjs'

function nonEmpty(value) {
  return typeof value === 'string' && value.trim().length > 0
}

function exactSha(value) {
  return typeof value === 'string' && /^[0-9a-f]{40}$/i.test(value)
}

function objectExists(root, spec) {
  if (!existsSync(root)) return false
  return Boolean(git(root, ['rev-parse', '--verify', spec], { allowFailure: true }))
}

function pathExistsAtSha(root, sha, path) {
  if (!exactSha(sha) || !nonEmpty(path)) return false
  return Boolean(git(root, ['rev-parse', '--verify', `${sha}:${path}`], { allowFailure: true }))
}

export function validateChronicaEvidence(chronicaRoot, manifest) {
  const root = resolve(chronicaRoot)
  const errors = []
  const warnings = []
  const semanticSha = manifest?.semantic_convergence?.chronica_sha
  const referenceSha = manifest?.chronica_reference?.sha
  const sha = semanticSha ?? referenceSha
  const currentHead = existsSync(root) ? git(root, ['rev-parse', 'HEAD'], { allowFailure: true }) || null : null

  if (!exactSha(sha)) {
    errors.push('Chronica evidence verification requires an exact 40-character SHA')
    return { errors, warnings, sha: sha ?? null, currentHead, stale: null, checkedPaths: 0 }
  }
  if (semanticSha && referenceSha && semanticSha !== referenceSha) errors.push(`semantic_convergence.chronica_sha (${semanticSha}) must match chronica_reference.sha (${referenceSha}) unless a separately audited reference is explicitly modeled`)
  if (!objectExists(root, `${sha}^{commit}`)) {
    errors.push(`Chronica commit ${sha} is not present in the supplied Chronica repository`)
    return { errors, warnings, sha, currentHead, stale: currentHead ? currentHead !== sha : null, checkedPaths: 0 }
  }

  let checkedPaths = 0
  const mappings = Array.isArray(manifest?.semantic_convergence?.mappings) ? manifest.semantic_convergence.mappings : []
  for (const mapping of mappings) {
    const label = mapping?.semantic_id ?? '<unknown-semantic>'
    const owner = mapping?.chronica_owner
    if (nonEmpty(owner)) {
      checkedPaths += 1
      if (!pathExistsAtSha(root, sha, owner)) errors.push(`${label}: Chronica owner does not exist at ${sha}: ${owner}`)
    }
    for (const path of mapping?.chronica_code_refs ?? []) {
      checkedPaths += 1
      if (!pathExistsAtSha(root, sha, path)) errors.push(`${label}: Chronica code ref does not exist at ${sha}: ${path}`)
    }
    for (const path of mapping?.chronica_test_refs ?? []) {
      checkedPaths += 1
      if (!pathExistsAtSha(root, sha, path)) errors.push(`${label}: Chronica test ref does not exist at ${sha}: ${path}`)
    }
    for (const path of mapping?.search_evidence?.owners ?? []) {
      checkedPaths += 1
      if (!pathExistsAtSha(root, sha, path)) errors.push(`${label}: searched Chronica owner does not exist at ${sha}: ${path}`)
    }
    for (const path of mapping?.search_evidence?.code_paths ?? []) {
      checkedPaths += 1
      if (!pathExistsAtSha(root, sha, path)) errors.push(`${label}: searched Chronica code path does not exist at ${sha}: ${path}`)
    }
    for (const path of mapping?.search_evidence?.test_paths ?? []) {
      checkedPaths += 1
      if (!pathExistsAtSha(root, sha, path)) errors.push(`${label}: searched Chronica test path does not exist at ${sha}: ${path}`)
    }
  }

  const stale = currentHead ? currentHead !== sha : null
  if (stale) warnings.push(`Chronica semantic audit references ${sha}, while supplied Chronica HEAD is ${currentHead}`)
  if (checkedPaths === 0) warnings.push('No Chronica owner/code/test evidence paths were checked')
  return { errors, warnings, sha, currentHead, stale, checkedPaths }
}
