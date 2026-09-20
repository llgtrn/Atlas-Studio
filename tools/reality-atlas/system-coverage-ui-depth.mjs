function ensureEdge(family, source, target, type, data = {}) {
  const id = `${family.name}:${source}:${type}:${target}`
  if (!family.edges.some((edge) => edge.id === id)) family.edges.push({ id, family: family.name, source, target, type, ...data })
}

export function enrichUiSemanticChains(coverage) {
  const family = coverage.families.ui
  const projectedBySource = new Map()
  for (const edge of family.edges.filter((candidate) => candidate.type === 'PROJECTS')) {
    const list = projectedBySource.get(edge.source) ?? []
    list.push(edge.target)
    projectedBySource.set(edge.source, list)
  }

  for (const edge of [...family.edges]) {
    if (!['CALLS', 'CALLS_API'].includes(edge.type)) continue
    for (const semantic of projectedBySource.get(edge.source) ?? []) {
      ensureEdge(family, edge.target, semantic, edge.type === 'CALLS_API' ? 'BOUND_TO_SEMANTIC' : 'INVOKES_SEMANTIC')
    }
  }

  family.edges.sort((a, b) => a.id.localeCompare(b.id))
  coverage.stats.familyStats.ui = {
    nodes: family.nodes.length,
    edges: family.edges.length,
    artifacts: family.artifacts.length,
    gaps: family.gaps.length,
  }
  coverage.stats.totalEdges = Object.values(coverage.families).reduce((sum, item) => sum + item.edges.length, 0)
  return coverage
}
