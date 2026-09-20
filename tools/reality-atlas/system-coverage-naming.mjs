// 2026-09-16 hard refoundation: crates/{core,runtime,adapter,cap}/ no longer exists (crates/ is
// retired -- see docs/decisions/0016-legacy-backend-retired-to-git-history.md). core/, runtime/,
// adapter/, organism/ are top-level single-package crates now, each owning its own Cargo.toml
// directly, not nested under a crates/<tier>/<domain>/<component>/ wrapper -- see
// docs/architecture/foundation/naming-topology.md section 2 for the canonical topology this
// module projects against.
const TIERS = new Set(['core', 'runtime', 'adapter', 'organism'])
const ARTIFACT_RE = /^chronica-(core|runtime|adapter|organism)(?:-(.+))?$/
const CAP_RE = /^chronica-cap-(.+)$/

function splitSlug(slug) {
  const parts = String(slug ?? '').split('-').filter(Boolean)
  return {
    domain: parts[0] ?? null,
    component: parts.length > 1 ? parts.slice(1).join('-') : null,
  }
}

function canonicalPath(tier, slug) {
  if (!tier) return null
  if (!slug) return tier
  const { domain, component } = splitSlug(slug)
  if (!domain) return tier
  return component ? `${tier}/${domain}/${component}` : `${tier}/${domain}`
}

function crateLocation(manifest) {
  if (!manifest || manifest === 'Cargo.toml') return null
  const match = String(manifest).match(/^(core|runtime|adapter|organism)\/Cargo\.toml$/)
  if (!match) return null
  return {
    physicalTier: match[1],
    physicalRelative: null,
    physicalPath: match[1],
  }
}

function displayFrom(tier, slug) {
  if (!tier) return slug || 'unknown'
  if (!slug) return tier
  const { domain, component } = splitSlug(slug)
  return component ? `${tier}/${domain}/${component}` : `${tier}/${domain}`
}

function deriveCrateTopology(node) {
  if (node.kind !== 'crate') return null
  if (node.manifest === 'Cargo.toml') {
    return {
      artifactIdentity: node.label,
      physicalPath: '.',
      canonicalPath: '.',
      architectureTier: 'workspace',
      domain: null,
      component: null,
      displayLabel: 'workspace',
      namingState: 'CANONICAL_PATH',
    }
  }

  const location = crateLocation(node.manifest)
  if (!location) {
    return {
      artifactIdentity: node.label,
      physicalPath: node.manifest ? node.manifest.replace(/\/Cargo\.toml$/, '') : null,
      canonicalPath: null,
      architectureTier: null,
      domain: null,
      component: null,
      displayLabel: node.label,
      namingState: 'LEGACY_TIER',
    }
  }

  const capMatch = String(node.label).match(CAP_RE)
  if (capMatch) {
    const { domain, component } = splitSlug(capMatch[1])
    return {
      artifactIdentity: node.label,
      physicalPath: location.physicalPath,
      canonicalPath: null,
      architectureTier: location.physicalTier,
      domain,
      component,
      displayLabel: component ? `cap/${domain}/${component}` : `cap/${domain}`,
      namingState: 'CAP_LEGACY_UNRESOLVED',
    }
  }

  const artifactMatch = String(node.label).match(ARTIFACT_RE)
  if (!artifactMatch) {
    return {
      artifactIdentity: node.label,
      physicalPath: location.physicalPath,
      canonicalPath: null,
      architectureTier: TIERS.has(location.physicalTier) ? location.physicalTier : null,
      domain: null,
      component: null,
      displayLabel: node.label,
      namingState: TIERS.has(location.physicalTier) ? 'PACKAGE_NAMESPACE_MISMATCH' : 'LEGACY_TIER',
    }
  }

  const artifactTier = artifactMatch[1]
  const slug = artifactMatch[2] ?? null
  const { domain, component } = splitSlug(slug)
  const targetPath = canonicalPath(artifactTier, slug)
  const displayLabel = displayFrom(artifactTier, slug)

  let namingState = 'CANONICAL_PATH'
  if (location.physicalTier !== artifactTier) namingState = 'TIER_MISMATCH'
  else if (location.physicalPath !== targetPath) namingState = 'LEGACY_FLAT_PATH'

  return {
    artifactIdentity: node.label,
    physicalPath: location.physicalPath,
    canonicalPath: targetPath,
    architectureTier: artifactTier,
    domain,
    component,
    displayLabel,
    namingState,
  }
}

function longestOwner(pathname, owners) {
  return owners
    .filter((entry) => pathname === entry.physicalPath || pathname.startsWith(`${entry.physicalPath}/`))
    .sort((a, b) => b.physicalPath.length - a.physicalPath.length)[0] ?? null
}

function addGap(repository, topology) {
  if (topology.namingState === 'CANONICAL_PATH') return
  const artifact = topology.physicalPath
  const key = `${topology.namingState}:${artifact}`
  if (repository.gaps.some((gap) => `${gap.kind}:${gap.artifact}` === key)) return
  repository.gaps.push({
    kind: topology.namingState,
    artifact,
    message: topology.canonicalPath
      ? `${topology.artifactIdentity} is physically located at ${topology.physicalPath}; canonical topology target is ${topology.canonicalPath}.`
      : `${topology.artifactIdentity} requires responsibility classification before a canonical topology target can be assigned.`,
  })
}

export function enrichNamingTopology(coverage) {
  const repository = coverage?.families?.repository
  if (!repository) return coverage

  const crateNodes = (repository.nodes ?? []).filter((node) => node.kind === 'crate')
  const crateTopologies = []

  for (const node of crateNodes) {
    const topology = deriveCrateTopology(node)
    if (!topology) continue
    node.artifactIdentity = topology.artifactIdentity
    node.displayLabel = topology.displayLabel
    node.topology = topology
    crateTopologies.push(topology)
    addGap(repository, topology)
  }

  const physicalOwners = crateTopologies.filter((entry) => entry.physicalPath && entry.physicalPath !== '.')
  for (const node of repository.nodes ?? []) {
    if (!['source_file', 'module'].includes(node.kind) || typeof node.path !== 'string') continue
    const owner = longestOwner(node.path, physicalOwners)
    if (!owner) continue
    const relative = node.path === owner.physicalPath ? '' : node.path.slice(owner.physicalPath.length + 1)
    node.artifactOwner = owner.artifactIdentity
    node.displayLabel = relative ? `${owner.displayLabel}/${relative}` : owner.displayLabel
    node.canonicalPath = owner.canonicalPath && relative ? `${owner.canonicalPath}/${relative}` : owner.canonicalPath
    node.namingState = owner.namingState
  }

  const measuredCrates = crateTopologies.filter((entry) => entry.architectureTier !== 'workspace')
  const states = Object.fromEntries([
    'CANONICAL_PATH',
    'LEGACY_FLAT_PATH',
    'TIER_MISMATCH',
    'CAP_LEGACY_UNRESOLVED',
    'PACKAGE_NAMESPACE_MISMATCH',
    'LEGACY_TIER',
  ].map((state) => [state, measuredCrates.filter((entry) => entry.namingState === state).length]))

  coverage.namingTopology = {
    schemaVersion: 1,
    contract: 'docs/architecture/foundation/naming-topology.md',
    crates: crateTopologies,
    states,
    stats: {
      totalCrates: measuredCrates.length,
      canonicalPathCrates: states.CANONICAL_PATH,
      namingDebtCrates: measuredCrates.filter((entry) => entry.namingState !== 'CANONICAL_PATH').length,
    },
  }
  coverage.stats.namingTopology = coverage.namingTopology.stats
  if (coverage.stats.familyStats?.repository) coverage.stats.familyStats.repository.gaps = repository.gaps.length
  coverage.stats.totalGaps = Object.values(coverage.families ?? {}).reduce((sum, family) => sum + (family.gaps?.length ?? 0), 0)
  return coverage
}

export function validateNamingTopology(coverage) {
  const errors = []
  const repository = coverage?.families?.repository
  if (!repository) return ['repository family is missing from System Atlas']
  if (!coverage?.namingTopology) return ['System Atlas naming topology projection is missing']
  if (coverage.namingTopology.schemaVersion !== 1) errors.push('System Atlas naming topology schemaVersion must be 1')

  for (const node of (repository.nodes ?? []).filter((candidate) => candidate.kind === 'crate')) {
    if (!node.topology) errors.push(`crate is missing naming topology metadata: ${node.label}`)
    if (!node.displayLabel) errors.push(`crate is missing stable displayLabel: ${node.label}`)
  }
  return errors
}
