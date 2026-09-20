// Runtime evidence graph. Distinguishes PRODUCT_RUNTIME from VALIDATE_ONLY / ISLAND / TOOLING_ONLY.
// Does not mutate canonical capability status.

import { RUNTIME_KINDS } from './vocab.mjs'
import { asArray, hasText, isObject, text } from './util.mjs'

export const GRAPH_NODES = Object.freeze([
  'entrypoint',
  'caller',
  'deterministic_owner',
  'persistence',
  'effect',
  'receipt',
  'event',
  'recovery',
  'witness',
])

export function validateEvidencePath(path) {
  const errors = []
  if (!isObject(path)) return { ok: false, errors: ['evidence path must be an object'] }
  if (!hasText(path.claim_id) && !hasText(path.claim)) errors.push('claim_id required')
  if (!RUNTIME_KINDS.includes(text(path.runtime_kind))) {
    errors.push(`${path.claim_id || path.claim}: invalid runtime_kind`)
  }

  const nodes = isObject(path.nodes) ? path.nodes : {}
  const present = GRAPH_NODES.filter((node) => hasText(nodes[node]) || nodes[node] === true)
  const missing = GRAPH_NODES.filter((node) => !present.includes(node))

  let mismatch = null
  const name = text(path.claim_id || path.claim || path.name)
  if (/create/i.test(name) && path.runtime_kind === 'VALIDATE_ONLY') {
    mismatch = 'VALIDATE_ONLY_CREATE_CLAIM'
  }
  if (/external_write/i.test(name) && !hasText(nodes.effect) && path.external_effect !== true) {
    mismatch = 'SIDE_EFFECT_CLASS_MISMATCH'
  }
  if (path.claimed_runtime_kind && path.claimed_runtime_kind !== path.runtime_kind) {
    mismatch = `CLAIMED_${path.claimed_runtime_kind}_OBSERVED_${path.runtime_kind}`
  }

  return {
    ok: errors.length === 0,
    errors,
    present_nodes: present,
    missing_nodes: missing,
    mismatch,
    runtime_kind: path.runtime_kind,
  }
}

export function evaluateEvidenceGraph(paths) {
  const errors = []
  const evaluated = []
  for (const path of asArray(paths)) {
    const result = validateEvidencePath(path)
    evaluated.push({ claim_id: path.claim_id || path.claim, ...result, path })
    if (!result.ok) errors.push(...result.errors)
  }
  return { ok: errors.length === 0, errors, paths: evaluated }
}
