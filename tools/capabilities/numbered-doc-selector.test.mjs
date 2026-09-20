import assert from 'node:assert/strict'
import test from 'node:test'
import { numberedDocCapabilityScope, shouldCompactNumberedDocCaps } from './numbered-doc-selector.mjs'

test('facet projections are restricted to their selected slices', () => {
  const scope = numberedDocCapabilityScope({ domain: 'observability-analytics', slices: [15] })
  assert.equal(scope.pred, 'c.domain=@d AND c.slice IN (@slice0)')
  assert.deepEqual(scope.params, { d: 'observability-analytics', slice0: 15 })
})

test('domain roadmap projections retain full-domain coverage', () => {
  const scope = numberedDocCapabilityScope(
    { domain: 'observability-analytics', slices: [15] },
    { domainDoc: true },
  )
  assert.equal(scope.pred, 'c.domain=@d')
  assert.deepEqual(scope.params, { d: 'observability-analytics' })
})

test('ERP selectors retain the noise exclusion', () => {
  const scope = numberedDocCapabilityScope({ domain: 'erp-finance', slices: [19] })
  assert.equal(scope.pred, "c.domain=@d AND c.slice IN (@slice0) AND c.key NOT LIKE 'noise.%'")
})

test('large non-domain projections compact instead of duplicating domain roadmap lists', () => {
  assert.equal(shouldCompactNumberedDocCaps(798), true)
  assert.equal(shouldCompactNumberedDocCaps(40), false)
})
