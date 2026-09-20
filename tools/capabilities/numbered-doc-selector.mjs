// Build the canonical-capability predicate used by numbered-doc projections.
// Domain roadmap docs intentionally cover their whole domain; all other routed
// docs must honor their curated slice selector to avoid duplicating that backlog.
export function numberedDocCapabilityScope(sel, { domainDoc = false } = {}) {
  const params = { d: sel.domain }
  const predicates = ['c.domain=@d']

  if (!domainDoc && sel.slices?.length) {
    const placeholders = sel.slices.map((slice, index) => {
      const key = `slice${index}`
      params[key] = slice
      return `@${key}`
    })
    predicates.push(`c.slice IN (${placeholders.join(', ')})`)
  }

  // Preserve the existing ERP projection cleanup for every selector it affects.
  if (sel.domain === 'erp-finance') predicates.push("c.key NOT LIKE 'noise.%'")

  return { pred: predicates.join(' AND '), params }
}

export const DEFAULT_FACET_INLINE_CAP_LIMIT = 40

export function shouldCompactNumberedDocCaps(capCount, { limit = DEFAULT_FACET_INLINE_CAP_LIMIT } = {}) {
  return capCount > limit
}
