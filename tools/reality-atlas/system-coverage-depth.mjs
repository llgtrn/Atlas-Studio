import { existsSync, readFileSync } from 'node:fs'
import { extname, join } from 'node:path'
import { execFileSync } from 'node:child_process'

const SOURCE_EXTENSIONS = new Set(['.c', '.cc', '.cpp', '.go', '.java', '.js', '.jsx', '.mjs', '.py', '.rs', '.ts', '.tsx'])
const EXTRA_SEMANTICS = [
  ['world', ['world', 'world graph', 'world_state', 'worldstate']],
  ['relation_binding', ['relation-binding', 'relation binding', 'binding', 'authorizedfor', 'temporal relation']],
  ['normative', ['normative', 'contract', 'legal', 'duty', 'right', 'obligation']],
  ['canonicalization', ['canonicalization', 'canonicalize', 'canonicalisation', 'canonical state']],
]

function slash(value) { return String(value).replaceAll('\\', '/') }
function idPart(value) { return String(value).replace(/[^A-Za-z0-9_.:/-]+/g, '_') }
function unique(values) { return [...new Set(values.filter(Boolean))].sort() }
function gitLines(root, args) {
  return execFileSync('git', args, { cwd: root, encoding: 'utf8' }).split('\n').map((line) => slash(line.trim())).filter(Boolean)
}
function safeRead(root, file) {
  try { return readFileSync(join(root, file), 'utf8') } catch { return '' }
}
function sourceFile(file) { return SOURCE_EXTENSIONS.has(extname(file).toLowerCase()) }
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
  return id
}
function addArtifact(family, file) {
  if (file && !family.artifacts.includes(file)) family.artifacts.push(file)
}

function enrichSemantic(root, tracked, coverage) {
  const family = coverage.families.semantic
  for (const [name, terms] of EXTRA_SEMANTICS) {
    const semantic = ensureNode(family, `semantic:${name}`, 'semantic_primitive', name).id
    for (const file of tracked.filter((value) => sourceFile(value) || value.endsWith('.md'))) {
      const text = `${file}\n${safeRead(root, file)}`.toLowerCase()
      if (!terms.some((term) => text.includes(term.toLowerCase()))) continue
      const artifact = ensureNode(family, `artifact:${idPart(file)}`, sourceFile(file) ? 'implementation' : 'documentation', file, { path: file }).id
      ensureEdge(family, artifact, semantic, 'MAPS_TO', { confidence: 'heuristic' })
      addArtifact(family, file)
      family.gaps = family.gaps.filter((gap) => !(gap.kind === 'UNMAPPED_IMPLEMENTATION' && gap.artifact === file))
    }
  }
}

function enrichExecution(tracked, coverage) {
  const family = coverage.families.execution
  const workRun = ensureNode(family, 'stage:work_run', 'execution_stage', 'work_run').id
  const adapter = ensureNode(family, 'stage:adapter', 'execution_stage', 'adapter').id
  const effect = ensureNode(family, 'stage:effect', 'execution_stage', 'effect').id
  family.edges = family.edges.filter((edge) => !(edge.type === 'NEXT' && edge.source === workRun && edge.target === effect))
  ensureEdge(family, workRun, adapter, 'NEXT')
  ensureEdge(family, adapter, effect, 'NEXT')

  for (const file of tracked.filter((value) => sourceFile(value) && (/(?:^|\/)adapter(?:s)?\//i.test(value) || value.startsWith('bindings/')))) {
    const impl = ensureNode(family, `artifact:${idPart(file)}`, 'implementation', file, { path: file }).id
    ensureEdge(family, impl, adapter, 'IMPLEMENTS')
    addArtifact(family, file)
  }
}

function enrichData(coverage) {
  const family = coverage.families.data
  const ownsByArtifact = new Map()
  for (const edge of family.edges.filter((candidate) => candidate.type === 'OWNS')) {
    const list = ownsByArtifact.get(edge.target) ?? []
    list.push(edge.source)
    ownsByArtifact.set(edge.target, list)
  }
  for (const edge of family.edges.filter((candidate) => candidate.type === 'DEFINES')) {
    for (const owner of ownsByArtifact.get(edge.source) ?? []) ensureEdge(family, owner, edge.target, 'OWNS_DATA_OBJECT')
  }
}

function composeServiceBlocks(text) {
  const out = []
  const lines = text.split('\n')
  let current = null
  let inServices = false
  for (const line of lines) {
    if (/^services:\s*$/.test(line)) { inServices = true; continue }
    if (inServices && /^\S/.test(line) && !/^services:/.test(line)) { inServices = false; current = null }
    if (!inServices) continue
    const service = line.match(/^\s{2}([A-Za-z0-9_.-]+):\s*$/)
    if (service) { current = { name: service[1], lines: [] }; out.push(current); continue }
    if (current) current.lines.push(line)
  }
  return out
}

function enrichDeployment(root, tracked, coverage) {
  const family = coverage.families.deployment
  for (const file of tracked.filter((candidate) => candidate.startsWith('deploy/') && /compose.*\.ya?ml$|docker-compose.*\.ya?ml$/.test(candidate))) {
    const text = safeRead(root, file)
    for (const service of composeServiceBlocks(text)) {
      if (!service.lines.some((line) => /^\s+healthcheck:\s*$/.test(line))) continue
      const serviceNode = ensureNode(family, `service:${idPart(service.name)}`, 'service', service.name).id
      const check = ensureNode(family, `healthcheck:${idPart(file)}:${idPart(service.name)}`, 'healthcheck_config', `${service.name}:healthcheck`, { path: file }).id
      ensureEdge(family, serviceNode, check, 'CONFIGURED_HEALTHCHECK')
      addArtifact(family, file)
    }
  }

  const evidencePath = '.chronica/deployment-evidence.json'
  if (!existsSync(join(root, evidencePath))) return
  try {
    const payload = JSON.parse(readFileSync(join(root, evidencePath), 'utf8'))
    const records = Array.isArray(payload) ? payload : Array.isArray(payload.records) ? payload.records : []
    let serviceScoped = 0
    for (const record of records) {
      const environmentName = String(record.environment ?? 'unknown')
      const healthId = `health:${idPart(environmentName)}:${idPart(record.gitSha ?? record.sha ?? 'unknown')}`
      const health = ensureNode(family, healthId, 'runtime_health', String(record.healthStatus ?? record.health ?? 'UNKNOWN'), {
        observedAt: record.observedAt ?? null,
        gitSha: record.gitSha ?? record.sha ?? null,
        deploymentId: record.deploymentId ?? null,
      }).id
      const serviceName = record.service ?? record.serviceName ?? null
      if (!serviceName) continue
      serviceScoped += 1
      const service = ensureNode(family, `service:${idPart(serviceName)}`, 'service', String(serviceName)).id
      ensureEdge(family, service, health, 'OBSERVED_RUNTIME_HEALTH')
    }
    if (records.length && serviceScoped === 0 && !family.gaps.some((gap) => gap.kind === 'RUNTIME_HEALTH_NOT_SERVICE_SCOPED')) {
      family.gaps.push({ kind: 'RUNTIME_HEALTH_NOT_SERVICE_SCOPED', artifact: evidencePath, message: 'Deployment evidence exists but records do not identify a service, so runtime health can only be attributed to the environment.' })
    }
  } catch {
    // Parse failure is already reported by the deployment enrichment layer.
  }
}

function enrichOps(root, coverage) {
  const family = coverage.families.ops
  const evidencePath = '.chronica/ops-evidence.json'
  if (!existsSync(join(root, evidencePath))) {
    if (!family.gaps.some((gap) => gap.kind === 'NO_LIVE_OPS_EVIDENCE')) {
      family.gaps.push({ kind: 'NO_LIVE_OPS_EVIDENCE', artifact: null, message: 'Ops contract topology is mapped, but no transient .chronica/ops-evidence.json is available for live donor/refactor/absorption instances.' })
    }
    return
  }
  try {
    const payload = JSON.parse(readFileSync(join(root, evidencePath), 'utf8'))
    const records = Array.isArray(payload) ? payload : Array.isArray(payload.ops) ? payload.ops : []
    for (const record of records) {
      const identity = String(record.repo ?? record.name ?? record.id ?? 'unknown-ops')
      const ops = ensureNode(family, `instance:${idPart(identity)}`, 'ops_instance', identity, { evidencePath }).id
      if (record.donorBaseline) {
        const donor = ensureNode(family, `donor:${idPart(identity)}:${idPart(record.donorBaseline)}`, 'donor_baseline', String(record.donorBaseline)).id
        ensureEdge(family, donor, ops, 'BASELINE_FOR')
      }
      for (const mapping of record.semanticMappings ?? []) {
        const label = String(mapping.canonicalTerm ?? mapping.semantic ?? mapping.id ?? 'semantic-mapping')
        const node = ensureNode(family, `mapping:${idPart(identity)}:${idPart(label)}`, 'semantic_mapping', label, { matchStatus: mapping.matchStatus ?? null, disposition: mapping.disposition ?? null }).id
        ensureEdge(family, ops, node, 'HAS_SEMANTIC_MAPPING')
      }
      if (record.legacyBurndown != null) {
        const burn = ensureNode(family, `burndown:${idPart(identity)}`, 'legacy_burndown', `${identity}:legacy-burndown`, { value: record.legacyBurndown }).id
        ensureEdge(family, ops, burn, 'HAS_LEGACY_BURNDOWN')
      }
      for (const candidate of record.absorptionCandidates ?? []) {
        const label = String(candidate.id ?? candidate.semantic ?? candidate)
        const node = ensureNode(family, `absorption:${idPart(identity)}:${idPart(label)}`, 'absorption_candidate', label).id
        ensureEdge(family, ops, node, 'PROPOSES_ABSORPTION')
      }
      for (const destination of record.chronicaDestinations ?? record.destinations ?? []) {
        const label = String(destination.owner ?? destination.path ?? destination)
        const node = ensureNode(family, `destination:${idPart(label)}`, 'chronica_destination', label).id
        ensureEdge(family, ops, node, 'CONVERGES_TO')
      }
    }
  } catch {
    family.gaps.push({ kind: 'INVALID_LIVE_OPS_EVIDENCE', artifact: evidencePath, message: 'Transient Ops evidence exists but could not be parsed.' })
  }
}

function recalculate(coverage) {
  for (const family of Object.values(coverage.families)) {
    family.nodes.sort((a, b) => a.id.localeCompare(b.id))
    family.edges.sort((a, b) => a.id.localeCompare(b.id))
    family.artifacts.sort()
    family.gaps.sort((a, b) => `${a.kind}:${a.artifact ?? ''}`.localeCompare(`${b.kind}:${b.artifact ?? ''}`))
  }
  coverage.stats.totalNodes = Object.values(coverage.families).reduce((sum, family) => sum + family.nodes.length, 0)
  coverage.stats.totalEdges = Object.values(coverage.families).reduce((sum, family) => sum + family.edges.length, 0)
  coverage.stats.totalGaps = Object.values(coverage.families).reduce((sum, family) => sum + family.gaps.length, 0)
  coverage.stats.familyStats = Object.fromEntries(coverage.graphFamilies.map((name) => [name, {
    nodes: coverage.families[name].nodes.length,
    edges: coverage.families[name].edges.length,
    artifacts: coverage.families[name].artifacts.length,
    gaps: coverage.families[name].gaps.length,
  }]))
  return coverage
}

export function enrichSystemCoverageDepth(root, coverage) {
  const tracked = gitLines(root, ['ls-files'])
  enrichSemantic(root, tracked, coverage)
  enrichExecution(tracked, coverage)
  enrichData(coverage)
  enrichDeployment(root, tracked, coverage)
  enrichOps(root, coverage)
  return recalculate(coverage)
}
