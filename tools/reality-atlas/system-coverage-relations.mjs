import { existsSync, readFileSync } from 'node:fs'
import { extname, join, posix as path } from 'node:path'
import { execFileSync } from 'node:child_process'

const SOURCE_EXTENSIONS = new Set(['.c', '.cc', '.cpp', '.go', '.java', '.js', '.jsx', '.mjs', '.py', '.rs', '.ts', '.tsx'])

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
function testFile(file) { return /(?:^|\/)(?:tests?|__tests__|benches)(?:\/|$)|\.(?:test|spec)\.[cm]?[jt]sx?$|_test\.rs$/.test(file) }
function integrationTest(file) { return /(?:^|\/)(?:integration|e2e)(?:[_-]?tests?)?(?:\/|$)|e2e/i.test(file) }
function benchmarkFile(file) { return /(?:^|\/)benches\//.test(file) || /benchmark|bench_/i.test(path.basename(file)) }
function invariantIds(text) { return unique(String(text).match(/\b(?:INV|ARCH-EQ|ARCH-PERF)-[A-Z0-9_-]+\b/g) ?? []) }

function ensureNode(family, id, kind, label, data = {}) {
  const full = `${family.name}:${id}`
  let node = family.nodes.find((candidate) => candidate.id === full)
  if (!node) {
    node = { id: full, family: family.name, kind, label, ...data }
    family.nodes.push(node)
  }
  return full
}
function ensureEdge(family, source, target, type, data = {}) {
  const id = `${family.name}:${source}:${type}:${target}`
  if (!family.edges.some((edge) => edge.id === id)) family.edges.push({ id, family: family.name, source, target, type, ...data })
  return id
}
function addArtifact(family, file) {
  if (file && !family.artifacts.includes(file)) family.artifacts.push(file)
}

function extractImports(text) {
  const specs = []
  for (const re of [/(?:import|export)\s+(?:[^'";]*?\sfrom\s*)?["']([^"']+)["']/g, /import\(\s*["']([^"']+)["']\s*\)/g]) {
    let m
    while ((m = re.exec(text))) specs.push(m[1])
  }
  return unique(specs)
}
function resolveUiImport(fromFile, specifier, trackedSet) {
  let base = null
  if (specifier.startsWith('@/')) base = `apps/ui/src/${specifier.slice(2)}`
  else if (specifier.startsWith('.')) base = path.normalize(path.join(path.dirname(fromFile), specifier))
  else return null
  const candidates = [base]
  if (!extname(base)) {
    for (const suffix of ['.ts', '.tsx', '.js', '.jsx', '.mjs']) candidates.push(`${base}${suffix}`)
    for (const suffix of ['.ts', '.tsx', '.js', '.jsx', '.mjs']) candidates.push(`${base}/index${suffix}`)
  }
  return candidates.find((candidate) => trackedSet.has(candidate)) ?? null
}

function enrichRepository(coverage) {
  const family = coverage.families.repository
  for (const edge of [...family.edges]) {
    if (edge.type === 'IMPORTS') ensureEdge(family, edge.target, edge.source, 'IMPORTED_BY')
    if (edge.type === 'DEPENDS_ON') ensureEdge(family, edge.target, edge.source, 'DEPENDENCY_OF')
    if (edge.type === 'CONTAINS') ensureEdge(family, edge.target, edge.source, 'CONTAINED_BY')
  }
}

function enrichUi(root, tracked, coverage) {
  const family = coverage.families.ui
  const trackedSet = new Set(tracked)
  const uiSources = tracked.filter((file) => file.startsWith('apps/ui/src/') && sourceFile(file))
  const importsByFile = new Map()

  for (const file of uiSources) {
    const source = ensureNode(family, `artifact:${idPart(file)}`, /\/pages\//.test(file) ? 'page' : /\/components\//.test(file) ? 'component' : 'ui_source', file, { path: file })
    addArtifact(family, file)
    const resolved = unique(extractImports(safeRead(root, file)).map((specifier) => resolveUiImport(file, specifier, trackedSet)))
    importsByFile.set(file, resolved)
    for (const targetFile of resolved) {
      const target = ensureNode(family, `artifact:${idPart(targetFile)}`, /\/pages\//.test(targetFile) ? 'page' : /\/components\//.test(targetFile) ? 'component' : 'ui_source', targetFile, { path: targetFile })
      ensureEdge(family, source, target, 'USES_COMPONENT')
      ensureEdge(family, target, source, 'USED_BY')
    }
  }

  for (const routeEdge of family.edges.filter((edge) => edge.type === 'RENDERS')) {
    const declaring = family.nodes.find((node) => node.id === routeEdge.target)?.path
    if (!declaring) continue
    const pages = (importsByFile.get(declaring) ?? []).filter((file) => /\/pages\//.test(file))
    for (const pageFile of pages) ensureEdge(family, routeEdge.source, `ui:artifact:${idPart(pageFile)}`, 'RESOLVES_TO_PAGE')
  }

  for (const node of family.nodes.filter((candidate) => candidate.kind === 'page' || candidate.kind === 'component')) {
    const file = node.path
    if (!file) continue
    const text = safeRead(root, file)
    for (const apiPath of unique(text.match(/['"](\/api\/[^'"]*)['"]/g)?.map((value) => value.slice(1, -1)) ?? [])) {
      const api = ensureNode(family, `api:${idPart(apiPath)}`, 'api_endpoint', apiPath)
      ensureEdge(family, node.id, api, 'CALLS_API')
    }
  }
}

function inferOwner(file) {
  const parts = file.split('/')
  if (parts[0] === 'crates' && parts.length >= 3) return parts.slice(0, 3).join('/')
  if (parts[0] === 'apps' && parts[1] === 'ui' && parts[2] === 'packages' && parts.length >= 4) return parts.slice(0, 4).join('/')
  if (parts[0] === 'apps' && parts.length >= 2) return parts.slice(0, 2).join('/')
  if (parts[0] === 'bindings') return 'bindings'
  if (parts[0] === 'graph') return 'graph'
  return parts[0] || 'root'
}

function enrichData(coverage) {
  const family = coverage.families.data
  for (const node of family.nodes.filter((candidate) => candidate.path)) {
    const ownerName = inferOwner(node.path)
    const owner = ensureNode(family, `owner:${idPart(ownerName)}`, 'data_owner', ownerName, { path: ownerName })
    ensureEdge(family, owner, node.id, 'OWNS')
  }
}

function enrichTestEvidence(root, tracked, coverage) {
  const family = coverage.families.test_evidence
  const invariants = new Map(family.nodes.filter((node) => node.kind === 'invariant').map((node) => [node.label, node.id]))
  const testNodes = new Map()

  for (const file of tracked.filter(sourceFile)) {
    const text = safeRead(root, file)
    const ids = invariantIds(text)
    if (!ids.length) continue
    if (testFile(file)) {
      const kind = benchmarkFile(file) ? 'benchmark' : integrationTest(file) ? 'integration_test' : 'unit_test'
      const test = ensureNode(family, `test:${idPart(file)}`, kind, file, { path: file })
      testNodes.set(file, test)
      addArtifact(family, file)
      for (const invariant of ids) {
        const inv = invariants.get(invariant) ?? ensureNode(family, `invariant:${idPart(invariant)}`, 'invariant', invariant)
        invariants.set(invariant, inv)
        ensureEdge(family, inv, test, 'VERIFIED_BY')
      }
    } else {
      const impl = ensureNode(family, `implementation:${idPart(file)}`, 'implementation', file, { path: file })
      addArtifact(family, file)
      for (const invariant of ids) {
        const inv = invariants.get(invariant) ?? ensureNode(family, `invariant:${idPart(invariant)}`, 'invariant', invariant)
        invariants.set(invariant, inv)
        ensureEdge(family, inv, impl, 'IMPLEMENTED_BY')
        ensureEdge(family, impl, inv, 'CLAIMS')
      }
    }
  }

  for (const file of tracked.filter((candidate) => candidate.startsWith('.github/workflows/') && /\.ya?ml$/.test(candidate))) {
    const text = safeRead(root, file)
    if (!/(?:cargo|pnpm|npm|node|vitest).*\b(?:test|bench|check)\b/i.test(text)) continue
    const workflow = ensureNode(family, `workflow:${idPart(file)}`, 'ci_workflow', file, { path: file })
    const suite = ensureNode(family, `suite:${idPart(file)}`, 'test_suite_execution', `${file}:tests`)
    ensureEdge(family, workflow, suite, 'RUNS_TEST_SUITE')
    for (const test of testNodes.values()) ensureEdge(family, suite, test, 'MAY_EXECUTE')
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
  const sha = ensureNode(family, `git:${coverage.sourceSha}`, 'git_revision', coverage.sourceSha)
  for (const file of tracked.filter((candidate) => candidate.startsWith('deploy/') && /compose.*\.ya?ml$|docker-compose.*\.ya?ml$/.test(path.basename(candidate)))) {
    const text = safeRead(root, file)
    const environmentName = file.includes('/production/') ? 'production' : file.includes('/staging/') ? 'staging' : file.includes('/compose/') ? 'local' : 'unspecified'
    const environment = ensureNode(family, `environment:${environmentName}`, 'environment', environmentName)
    addArtifact(family, file)
    for (const service of composeServiceBlocks(text)) {
      const serviceNode = ensureNode(family, `service:${idPart(service.name)}`, 'service', service.name)
      ensureEdge(family, environment, serviceNode, 'RUNS')
      const body = service.lines.join('\n')
      const image = body.match(/^\s+image:\s*([^\s#]+)/m)?.[1]
      const build = /\bbuild\s*:/m.test(body)
      if (image) {
        const artifact = ensureNode(family, `image:${idPart(image)}`, 'build_artifact', image)
        ensureEdge(family, sha, artifact, 'BUILDS_OR_SELECTS')
        ensureEdge(family, artifact, environment, 'PROMOTED_TO')
        ensureEdge(family, serviceNode, artifact, 'RUNS_ARTIFACT')
      } else if (build) {
        const artifact = ensureNode(family, `image:${idPart(service.name)}:local-build`, 'build_artifact', `${service.name}:local-build`)
        ensureEdge(family, sha, artifact, 'BUILDS')
        ensureEdge(family, artifact, environment, 'PROMOTED_TO')
        ensureEdge(family, serviceNode, artifact, 'RUNS_ARTIFACT')
      }
    }
  }

  const evidencePath = '.chronica/deployment-evidence.json'
  if (existsSync(join(root, evidencePath))) {
    try {
      const payload = JSON.parse(readFileSync(join(root, evidencePath), 'utf8'))
      const records = Array.isArray(payload) ? payload : Array.isArray(payload.records) ? payload.records : []
      for (const record of records) {
        const environmentName = String(record.environment ?? 'unknown')
        const environment = ensureNode(family, `environment:${idPart(environmentName)}`, 'environment', environmentName)
        const healthLabel = String(record.healthStatus ?? record.health ?? 'UNKNOWN')
        const health = ensureNode(family, `health:${idPart(environmentName)}:${idPart(record.gitSha ?? record.sha ?? 'unknown')}`, 'runtime_health', healthLabel, {
          observedAt: record.observedAt ?? null,
          gitSha: record.gitSha ?? record.sha ?? null,
          deploymentId: record.deploymentId ?? null,
        })
        ensureEdge(family, environment, health, 'OBSERVED_AS')
      }
    } catch {
      family.gaps.push({ kind: 'INVALID_DEPLOYMENT_EVIDENCE', artifact: evidencePath, message: 'Deployment evidence exists but could not be parsed.' })
    }
  } else {
    family.gaps.push({ kind: 'NO_LOCAL_RUNTIME_HEALTH_EVIDENCE', artifact: null, message: 'No local .chronica/deployment-evidence.json is available in this working tree.' })
  }
}

function enrichOps(coverage) {
  const family = coverage.families.ops
  const order = ['donor_baseline', 'semantic_mapping', 'legacy_burndown', 'absorption_candidate', 'chronica_destination']
  const nodes = order.map((name) => ensureNode(family, `concept:${name}`, 'ops_lifecycle_concept', name))
  for (let index = 0; index < nodes.length - 1; index++) ensureEdge(family, nodes[index], nodes[index + 1], 'NEXT')
}

function enrichMachine(coverage) {
  const family = coverage.families.machine_physical
  const resource = ensureNode(family, 'semantic:machine_resource', 'semantic_machine_resource', 'machine_resource')
  const adapter = ensureNode(family, 'semantic:industrial_adapter', 'adapter_boundary', 'industrial_adapter')
  const device = ensureNode(family, 'semantic:physical_device', 'physical_device', 'physical_device')
  const safety = ensureNode(family, 'safety:hard_interlocks', 'safety_boundary', 'hard_interlocks_and_emergency_protection')
  ensureEdge(family, resource, adapter, 'RESOLVES_THROUGH')
  const protocols = family.nodes.filter((node) => node.kind === 'machine_protocol')
  for (const protocol of protocols) {
    ensureEdge(family, adapter, protocol.id, 'USES_PROTOCOL')
    ensureEdge(family, protocol.id, device, 'CONNECTS_TO')
  }
  ensureEdge(family, adapter, safety, 'SUBJECT_TO')
  ensureEdge(family, safety, device, 'PROTECTS')
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

export function enrichSystemCoverage(root, coverage) {
  const tracked = gitLines(root, ['ls-files'])
  enrichRepository(coverage)
  enrichUi(root, tracked, coverage)
  enrichData(coverage)
  enrichTestEvidence(root, tracked, coverage)
  enrichDeployment(root, tracked, coverage)
  enrichOps(coverage)
  enrichMachine(coverage)
  return recalculate(coverage)
}
