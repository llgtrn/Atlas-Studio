function ensureNode(family, id, kind, label, data = {}) {
  const full = `${family.name}:${id}`
  let node = family.nodes.find((candidate) => candidate.id === full)
  if (!node) {
    node = { id: full, family: family.name, kind, label, ...data }
    family.nodes.push(node)
  }
  return node
}
function ensureEdge(family, source, target, type, data = {}) {
  const id = `${family.name}:${source}:${type}:${target}`
  if (!family.edges.some((edge) => edge.id === id)) family.edges.push({ id, family: family.name, source, target, type, ...data })
}
function classify(path) {
  const value = String(path ?? '')
  if (/(?:^|\/)benches\//.test(value) || /benchmark|bench_/i.test(value)) return 'benchmark'
  if (/(?:^|\/)(?:integration|e2e)(?:[_-]?tests?)?(?:\/|$)|e2e/i.test(value)) return 'integration_test'
  return 'unit_test'
}

export function enrichEvidenceMaturity(coverage) {
  const family = coverage.families.test_evidence
  const unitLayer = ensureNode(family, 'evidence_level:unit', 'evidence_level', 'unit_test').id
  const integrationLayer = ensureNode(family, 'evidence_level:integration', 'evidence_level', 'integration_test').id
  const benchmarkLayer = ensureNode(family, 'evidence_level:benchmark', 'evidence_level', 'benchmark').id
  const ciLayer = ensureNode(family, 'evidence_level:ci', 'evidence_level', 'ci_execution').id

  // This is an evidence pipeline/order, not a claim that benchmark evidence is semantically
  // stronger than integration correctness evidence. The evidence dimensions are complementary.
  ensureEdge(family, unitLayer, integrationLayer, 'EVIDENCE_PIPELINE_NEXT')
  ensureEdge(family, integrationLayer, benchmarkLayer, 'EVIDENCE_PIPELINE_NEXT')
  ensureEdge(family, benchmarkLayer, ciLayer, 'EVIDENCE_PIPELINE_NEXT')
  ensureEdge(family, integrationLayer, benchmarkLayer, 'COMPLEMENTED_BY')

  for (const node of family.nodes) {
    if (!node.path || !['test', 'unit_test', 'integration_test', 'benchmark'].includes(node.kind)) continue
    const kind = classify(node.path)
    node.kind = kind
    const target = kind === 'unit_test' ? unitLayer : kind === 'integration_test' ? integrationLayer : benchmarkLayer
    ensureEdge(family, node.id, target, 'EVIDENCE_LEVEL')
  }

  for (const node of family.nodes.filter((candidate) => candidate.kind === 'ci_workflow')) {
    ensureEdge(family, node.id, ciLayer, 'EXECUTES_AT')
  }

  family.nodes.sort((a, b) => a.id.localeCompare(b.id))
  family.edges.sort((a, b) => a.id.localeCompare(b.id))
  coverage.stats.familyStats.test_evidence = {
    nodes: family.nodes.length,
    edges: family.edges.length,
    artifacts: family.artifacts.length,
    gaps: family.gaps.length,
  }
  coverage.stats.totalNodes = Object.values(coverage.families).reduce((sum, item) => sum + item.nodes.length, 0)
  coverage.stats.totalEdges = Object.values(coverage.families).reduce((sum, item) => sum + item.edges.length, 0)
  return coverage
}
