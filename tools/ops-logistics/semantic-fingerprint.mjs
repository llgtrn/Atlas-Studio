const FINGERPRINT_FIELDS = [
  'resource',
  'transition',
  'effect',
  'authority',
  'normative',
  'evidence',
  'idempotency',
  'unknown_outcome',
  'risk',
  'persistence',
  'provider_dependency',
]

function normalize(value) {
  if (value === null || value === undefined) return ''
  if (Array.isArray(value)) return value.map(normalize).sort().join('|')
  if (typeof value === 'object') return JSON.stringify(Object.fromEntries(Object.entries(value).sort(([a], [b]) => a.localeCompare(b)).map(([key, item]) => [key, normalize(item)])))
  return String(value).trim().toLowerCase().replace(/\s+/g, ' ')
}

export function fingerprintKey(fingerprint = {}) {
  return FINGERPRINT_FIELDS.map((field) => `${field}=${normalize(fingerprint?.[field])}`).join(';')
}

export function detectSemanticFingerprintCollisions(manifest) {
  const mappings = Array.isArray(manifest?.semantic_convergence?.mappings) ? manifest.semantic_convergence.mappings : []
  const groups = new Map()
  for (const mapping of mappings) {
    if (!mapping?.fingerprint || typeof mapping.fingerprint !== 'object') continue
    const key = fingerprintKey(mapping.fingerprint)
    groups.set(key, [...(groups.get(key) ?? []), mapping])
  }

  const collisions = []
  const errors = []
  for (const [key, group] of groups.entries()) {
    if (group.length < 2) continue
    const canonicalTerms = [...new Set(group.map((mapping) => mapping?.canonical_term_after).filter(Boolean))]
    const semanticIds = [...new Set(group.map((mapping) => mapping?.semantic_id).filter(Boolean))]
    const record = {
      fingerprint: key,
      semanticIds,
      canonicalTerms,
      mappings: group.map((mapping) => ({ semanticId: mapping?.semantic_id ?? null, canonicalTerm: mapping?.canonical_term_after ?? null, disposition: mapping?.disposition ?? null })),
    }
    collisions.push(record)
    if (canonicalTerms.length > 1) errors.push(`same semantic fingerprint maps to multiple canonical terms: ${semanticIds.join(', ')} -> ${canonicalTerms.join(', ')}`)
  }
  return { errors, collisions }
}
