// NO_AI taxonomy: AI inference providers are not deterministic effect providers.
// Collapsing Stripe/email/carrier/webhook/browser-effect into REQUIRED_INFERENCE fails closed.

import { DETERMINISTIC_EFFECT_EXAMPLES, NO_AI_CLASSES, PROVIDER_KINDS } from './vocab.mjs'
import { asArray, hasText, isObject, text } from './util.mjs'

export const AI_INFERENCE_CLASS = 'REQUIRED_INFERENCE'
export const DETERMINISTIC_EFFECT_CLASS = 'STILL_WORKS_WITHOUT_AI'

const EFFECT_NEEDLES = Object.freeze([
  /stripe/i,
  /payment provider/i,
  /email transport/i,
  /\bgmail\b/i,
  /\bcarrier\b/i,
  /webhook/i,
  /browser[- ]effect/i,
  /bank transport/i,
  /refund/i,
])

const INFERENCE_NEEDLES = Object.freeze([
  /\bllm\b/i,
  /\bgpt\b/i,
  /\bclaude\b/i,
  /\bgemini\b/i,
  /inference/i,
  /chat completion/i,
])

export function classifyProviderSurface(surface) {
  const errors = []
  if (!isObject(surface)) return { ok: false, errors: ['provider surface must be an object'] }

  const kind = text(surface.provider_kind)
  const noAi = text(surface.no_ai_class || surface.no_ai)
  const name = text(surface.name || surface.domain || surface.operation)

  if (!PROVIDER_KINDS.includes(kind)) errors.push(`${name || 'surface'}: invalid provider_kind`)
  if (!NO_AI_CLASSES.includes(noAi)) errors.push(`${name || 'surface'}: invalid no_ai_class`)

  if (kind === 'AI_INFERENCE_PROVIDER' && noAi !== AI_INFERENCE_CLASS) {
    errors.push(`${name}: AI_INFERENCE_PROVIDER must be ${AI_INFERENCE_CLASS}`)
  }
  if (kind === 'DETERMINISTIC_EXTERNAL_EFFECT_PROVIDER' && noAi !== DETERMINISTIC_EFFECT_CLASS) {
    errors.push(`${name}: DETERMINISTIC_EXTERNAL_EFFECT_PROVIDER must be ${DETERMINISTIC_EFFECT_CLASS}`)
  }
  if (kind === 'DETERMINISTIC_EXTERNAL_EFFECT_PROVIDER' && noAi === 'REQUIRED_INFERENCE') {
    errors.push(`${name}: deterministic external effect must not be classified as inference`)
  }
  if (kind === 'AI_INFERENCE_PROVIDER' && EFFECT_NEEDLES.some((p) => p.test(name)) && !INFERENCE_NEEDLES.some((p) => p.test(name))) {
    errors.push(`${name}: Stripe/email/carrier/webhook/browser-effect/bank cannot be AI_INFERENCE_PROVIDER`)
  }

  return { ok: errors.length === 0, errors, kind, no_ai_class: noAi }
}

export function detectCollapsedProviderTaxonomy(surfaces) {
  const rows = asArray(surfaces)
  const errors = []
  const bundled = rows.filter((row) => {
    const blob = JSON.stringify(row)
    const mentionsEffect = EFFECT_NEEDLES.some((p) => p.test(blob))
    const classifiedInference = text(row.no_ai || row.no_ai_class) === 'REQUIRED_INFERENCE'
    const kind = text(row.provider_kind)
    const explicitlyBundled = text(row.taxonomy_status) === 'BUNDLED_PROVIDER_SURFACES_MUST_SPLIT_ON_RERUN'
    const collapsedKind = kind !== 'AI_INFERENCE_PROVIDER' && kind !== 'DETERMINISTIC_EXTERNAL_EFFECT_PROVIDER'
    return explicitlyBundled || (mentionsEffect && classifiedInference && collapsedKind)
  })

  const hasAi = rows.some((row) => text(row.provider_kind) === 'AI_INFERENCE_PROVIDER')
  const hasEffect = rows.some((row) => text(row.provider_kind) === 'DETERMINISTIC_EXTERNAL_EFFECT_PROVIDER')

  if (bundled.length && !(hasAi && hasEffect)) {
    errors.push('COLLAPSED_PROVIDER_TAXONOMY: split AI_INFERENCE_PROVIDER from DETERMINISTIC_EXTERNAL_EFFECT_PROVIDER')
  }
  if (hasAi && !hasEffect && rows.some((row) => EFFECT_NEEDLES.some((p) => p.test(JSON.stringify(row))))) {
    errors.push('COLLAPSED_PROVIDER_TAXONOMY: deterministic effect surfaces present without DETERMINISTIC_EXTERNAL_EFFECT_PROVIDER')
  }

  return { ok: errors.length === 0, errors, bundled_count: bundled.length, examples: DETERMINISTIC_EFFECT_EXAMPLES }
}

export function validateNoAiTaxonomy(surfaces) {
  const errors = []
  const rows = asArray(surfaces)
  if (rows.length === 0) errors.push('no_ai surfaces required')
  for (const row of rows) {
    if (text(row.provider_kind) === 'NOT_A_PROVIDER' || !hasText(row.provider_kind)) {
      const noAi = text(row.no_ai_class || row.no_ai)
      if (noAi && !NO_AI_CLASSES.includes(noAi)) errors.push(`${text(row.domain || row.name)}: invalid no_ai_class`)
      continue
    }
    const classified = classifyProviderSurface(row)
    if (!classified.ok) errors.push(...classified.errors)
  }
  const collapsed = detectCollapsedProviderTaxonomy(rows)
  if (!collapsed.ok) errors.push(...collapsed.errors)
  return { ok: errors.length === 0, errors }
}

export function engineProviderSplitFromFreeze(freeze) {
  const split = freeze?.rerun_corrections?.no_ai_provider_split || {}
  return [
    {
      domain: 'ai-inference-provider',
      provider_kind: 'AI_INFERENCE_PROVIDER',
      no_ai_class: text(split.ai_inference_provider) || AI_INFERENCE_CLASS,
      assessment_note: 'LLM/media inference is inherent. Not a Stripe/email/carrier path.',
    },
    {
      domain: 'deterministic-external-effect-provider',
      provider_kind: 'DETERMINISTIC_EXTERNAL_EFFECT_PROVIDER',
      no_ai_class: text(split.deterministic_external_effect_provider) || DETERMINISTIC_EFFECT_CLASS,
      assessment_note: `Approved refund/email/carrier/webhook/browser-effect/bank are deterministic. Examples: ${DETERMINISTIC_EFFECT_EXAMPLES.join(', ')}`,
    },
  ]
}
