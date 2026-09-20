function keyBy(values, key) {
  const map = new Map()
  for (const value of values) {
    const k = key(value)
    if (!k) continue
    const list = map.get(k) ?? []
    list.push(value)
    map.set(k, list)
  }
  return map
}

function relationId(source, type, target) {
  return `unified:${source}:${type}:${target}`
}

function pushEdge(edges, seen, source, target, type, data = {}) {
  if (!source || !target || source === target) return
  const id = relationId(source, type, target)
  if (seen.has(id)) return
  seen.add(id)
  edges.push({ id, family: 'unified', source, target, type, ...data })
}

function bridgePairs(matches, callback) {
  for (let i = 0; i < matches.length; i++) {
    for (let j = i + 1; j < matches.length; j++) {
      if (matches[i].family === matches[j].family) continue
      callback(matches[i], matches[j])
    }
  }
}

export function unifySystemCoverage(coverage) {
  const nodes = Object.values(coverage.families).flatMap((family) => family.nodes)
  const familyEdges = Object.values(coverage.families).flatMap((family) => family.edges)
  const crossFamilyEdges = []
  const seen = new Set()

  const byPath = keyBy(nodes.filter((node) => typeof node.path === 'string'), (node) => node.path)
  for (const [artifact, matches] of byPath) {
    bridgePairs(matches, (left, right) => {
      pushEdge(crossFamilyEdges, seen, left.id, right.id, 'SAME_ARTIFACT', { artifact })
      pushEdge(crossFamilyEdges, seen, right.id, left.id, 'SAME_ARTIFACT', { artifact })
    })
  }

  const invariantNodes = nodes.filter((node) => node.kind === 'invariant')
  const byInvariant = keyBy(invariantNodes, (node) => node.label)
  for (const [invariant, matches] of byInvariant) {
    bridgePairs(matches, (left, right) => {
      pushEdge(crossFamilyEdges, seen, left.id, right.id, 'SAME_INVARIANT', { invariant })
      pushEdge(crossFamilyEdges, seen, right.id, left.id, 'SAME_INVARIANT', { invariant })
    })
  }

  const endpointKinds = new Set(['http_route', 'api_endpoint'])
  const endpointNodes = nodes.filter((node) => endpointKinds.has(node.kind))
  const byEndpoint = keyBy(endpointNodes, (node) => node.label)
  for (const [endpoint, matches] of byEndpoint) {
    bridgePairs(matches, (left, right) => {
      pushEdge(crossFamilyEdges, seen, left.id, right.id, 'SAME_ENDPOINT', { endpoint })
      pushEdge(crossFamilyEdges, seen, right.id, left.id, 'SAME_ENDPOINT', { endpoint })
    })
  }

  const semanticKinds = new Set(['semantic_primitive', 'semantic_projection', 'semantic_binding_target', 'execution_stage'])
  const semanticNodes = nodes.filter((node) => semanticKinds.has(node.kind))
  const normalizeSemantic = (label) => String(label).toLowerCase().replace(/[^a-z0-9]+/g, '_').replace(/^_+|_+$/g, '')
  const bySemantic = keyBy(semanticNodes, (node) => normalizeSemantic(node.label))
  for (const [semantic, matches] of bySemantic) {
    if (!semantic) continue
    bridgePairs(matches, (left, right) => {
      pushEdge(crossFamilyEdges, seen, left.id, right.id, 'SAME_SEMANTIC', { semantic })
      pushEdge(crossFamilyEdges, seen, right.id, left.id, 'SAME_SEMANTIC', { semantic })
    })
  }

  const providerNodes = nodes.filter((node) => node.kind === 'provider_or_service')
  const protocolNodes = nodes.filter((node) => node.kind === 'machine_protocol')
  for (const provider of providerNodes) {
    const providerKey = normalizeSemantic(provider.label)
    for (const protocol of protocolNodes) {
      const protocolKey = normalizeSemantic(protocol.label)
      if (provider.family === protocol.family || !providerKey || providerKey !== protocolKey) continue
      pushEdge(crossFamilyEdges, seen, provider.id, protocol.id, 'SAME_EXTERNAL_PROTOCOL', { key: providerKey })
      pushEdge(crossFamilyEdges, seen, protocol.id, provider.id, 'SAME_EXTERNAL_PROTOCOL', { key: providerKey })
    }
  }

  const repoFiles = new Map(
    (coverage.families.repository.nodes ?? [])
      .filter((node) => node.kind === 'source_file' && node.path)
      .map((node) => [node.path, node]),
  )
  for (const [familyName, family] of Object.entries(coverage.families)) {
    if (familyName === 'repository') continue
    for (const artifact of family.artifacts ?? []) {
      const repoNode = repoFiles.get(artifact)
      if (!repoNode) continue
      const familyNode = family.nodes.find((node) => node.path === artifact)
      if (!familyNode || familyNode.family === repoNode.family) continue
      pushEdge(crossFamilyEdges, seen, repoNode.id, familyNode.id, 'PROJECTED_AS', { family: familyName, artifact })
      pushEdge(crossFamilyEdges, seen, familyNode.id, repoNode.id, 'BACKED_BY_REPOSITORY_ARTIFACT', { artifact })
    }
  }

  const allEdges = [...familyEdges, ...crossFamilyEdges]
  coverage.unified = {
    nodes,
    familyEdges,
    crossFamilyEdges,
    edges: allEdges,
    stats: {
      nodes: nodes.length,
      familyEdges: familyEdges.length,
      crossFamilyEdges: crossFamilyEdges.length,
      edges: allEdges.length,
      artifactBridges: crossFamilyEdges.filter((edge) => edge.type === 'SAME_ARTIFACT' || edge.type === 'PROJECTED_AS').length,
      semanticBridges: crossFamilyEdges.filter((edge) => edge.type === 'SAME_SEMANTIC').length,
      invariantBridges: crossFamilyEdges.filter((edge) => edge.type === 'SAME_INVARIANT').length,
      endpointBridges: crossFamilyEdges.filter((edge) => edge.type === 'SAME_ENDPOINT').length,
    },
  }
  coverage.stats.unifiedNodes = nodes.length
  coverage.stats.unifiedEdges = allEdges.length
  coverage.stats.crossFamilyEdges = crossFamilyEdges.length
  return coverage
}

export function validateUnifiedSystemCoverage(coverage) {
  const errors = []
  if (!coverage?.unified) return ['unified System Atlas graph is missing']
  const nodeIds = new Set(coverage.unified.nodes.map((node) => node.id))
  const nodesById = new Map(coverage.unified.nodes.map((node) => [node.id, node]))
  for (const edge of coverage.unified.edges) {
    if (!nodeIds.has(edge.source)) errors.push(`unified edge source is missing: ${edge.source}`)
    if (!nodeIds.has(edge.target)) errors.push(`unified edge target is missing: ${edge.target}`)
  }
  for (const edge of coverage.unified.crossFamilyEdges) {
    const source = nodesById.get(edge.source)
    const target = nodesById.get(edge.target)
    if (source && target && source.family === target.family) errors.push(`cross-family edge does not cross families: ${edge.id}`)
  }
  if (coverage.stats.graphFamilies > 1 && coverage.unified.stats.crossFamilyEdges === 0) errors.push('unified System Atlas has no cross-family edges')
  return errors
}
