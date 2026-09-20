// Exact-SHA provenance. A report from SHA A must never silently represent SHA B.
// Tests passing never imply CURRENT or authoritative architecture.

import { execFileSync } from 'node:child_process'
import { FRESHNESS, GENERATOR_VERSION, BENCHMARK_SCHEMA_VERSION } from './vocab.mjs'
import { SHA40, isObject, text } from './util.mjs'

export function readRepositorySha(cwd) {
  try {
    return execFileSync('git', ['rev-parse', 'HEAD'], { cwd, encoding: 'utf8' }).trim()
  } catch {
    return null
  }
}

export function bindProvenance(input = {}) {
  const repository_sha = text(input.repository_sha).toLowerCase()
  const evaluated_sha = text(input.evaluated_sha || input.repository_sha).toLowerCase()
  const base_sha = text(input.base_sha).toLowerCase()
  const product_truth_sha = input.product_truth_sha == null || input.product_truth_sha === ''
    ? null
    : text(input.product_truth_sha).toLowerCase()
  const generated_at = text(input.generated_at)
  const tests_passing = input.tests_passing === true

  const provenance = {
    repository_sha,
    evaluated_sha,
    base_sha,
    product_truth_sha,
    benchmark_schema_version: text(input.benchmark_schema_version) || BENCHMARK_SCHEMA_VERSION,
    generator_version: text(input.generator_version) || GENERATOR_VERSION,
    generated_at,
    tests_passing,
    tests_passing_does_not_imply_authoritative: true,
    authoritative_architecture_verdict: false,
  }

  const classified = classifyFreshness(provenance)
  return { ...provenance, ...classified }
}

export function classifyFreshness(provenance) {
  const errors = []
  if (!isObject(provenance)) {
    return { freshness: 'INVALID', stale_reason: 'provenance missing', errors: ['provenance missing'] }
  }

  const requiredShas = [
    ['repository_sha', provenance.repository_sha],
    ['evaluated_sha', provenance.evaluated_sha],
    ['base_sha', provenance.base_sha],
  ]
  for (const [name, value] of requiredShas) {
    if (!SHA40.test(text(value))) errors.push(`${name} must be a 40-char git SHA`)
  }
  if (provenance.product_truth_sha != null && !SHA40.test(text(provenance.product_truth_sha))) {
    errors.push('product_truth_sha must be a 40-char git SHA when present')
  }
  if (!text(provenance.benchmark_schema_version)) errors.push('benchmark_schema_version required')
  if (!text(provenance.generator_version)) errors.push('generator_version required')
  if (!text(provenance.generated_at)) errors.push('generated_at required')
  if (text(provenance.repository_sha) && text(provenance.evaluated_sha) && provenance.repository_sha !== provenance.evaluated_sha) {
    errors.push('repository_sha must equal evaluated_sha')
  }

  if (errors.length) {
    return { freshness: 'INVALID', stale_reason: 'INVALID_PROVENANCE', errors }
  }

  if (text(provenance.base_sha) !== text(provenance.evaluated_sha)) {
    return {
      freshness: 'STALE',
      stale_reason: 'STALE_BASELINE',
      errors: [],
      note: 'audit.base_sha != evaluated_sha',
    }
  }

  if (provenance.product_truth_sha == null) {
    return {
      freshness: 'PROVISIONAL',
      stale_reason: 'PRODUCT_TRUTH_SHA_UNKNOWN',
      errors: [],
      note: 'current_product_truth_sha unknown → PROVISIONAL',
    }
  }

  if (text(provenance.evaluated_sha) !== text(provenance.product_truth_sha)) {
    return {
      freshness: 'STALE',
      stale_reason: 'NOT_PRODUCT_TRUTH_MAIN',
      errors: [],
    }
  }

  return {
    freshness: 'CURRENT',
    stale_reason: null,
    errors: [],
    note: 'CURRENT is SHA identity only. It is not AUTHORITATIVE_ARCHITECTURE_DECISION.',
  }
}

export function assertNotSilentShaSwap(report, evaluated_sha) {
  const bound = text(report?.provenance?.repository_sha || report?.repository_sha)
  const evaluated = text(evaluated_sha)
  if (!SHA40.test(bound) || !SHA40.test(evaluated)) {
    return { ok: false, errors: ['SHA binding missing'] }
  }
  if (bound !== evaluated) {
    return { ok: false, errors: [`SHA_MISMATCH report=${bound} evaluated=${evaluated}`] }
  }
  return { ok: true, errors: [] }
}

export function freshnessAllowed(value) {
  return FRESHNESS.includes(value)
}
