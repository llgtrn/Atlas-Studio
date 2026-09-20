import { readFileSync } from 'node:fs'
import { extname, join } from 'node:path'
import { execFileSync } from 'node:child_process'

function slash(value) { return String(value).replaceAll('\\', '/') }
function idPart(value) { return String(value).replace(/[^A-Za-z0-9_.:/-]+/g, '_') }
function unique(values) { return [...new Set(values.filter(Boolean))].sort() }
function gitLines(root, args) {
  return execFileSync('git', args, { cwd: root, encoding: 'utf8' }).split('\n').map((line) => slash(line.trim())).filter(Boolean)
}
function safeRead(root, file) {
  try { return readFileSync(join(root, file), 'utf8') } catch { return '' }
}
function ensureNode(family, id, kind, label, data = {}) {
  const full = `${family.name}:${id}`
  if (!family.nodes.some((node) => node.id === full)) family.nodes.push({ id: full, family: family.name, kind, label, ...data })
  return full
}
function ensureEdge(family, source, target, type, data = {}) {
  const id = `${family.name}:${source}:${type}:${target}`
  if (!family.edges.some((edge) => edge.id === id)) family.edges.push({ id, family: family.name, source, target, type, ...data })
}
function addArtifact(family, file) {
  if (!family.artifacts.includes(file)) family.artifacts.push(file)
}

function enrichProto(file, text, family) {
  if (extname(file) !== '.proto') return
  const artifact = ensureNode(family, `artifact:${idPart(file)}`, 'protocol_schema', file, { path: file })
  addArtifact(family, file)

  for (const message of text.matchAll(/\bmessage\s+([A-Za-z_][A-Za-z0-9_]*)\s*\{/g)) {
    const node = ensureNode(family, `proto_message:${idPart(file)}:${idPart(message[1])}`, 'proto_message', message[1], { schema: file })
    ensureEdge(family, artifact, node, 'DEFINES')
  }

  const servicePattern = /\bservice\s+([A-Za-z_][A-Za-z0-9_]*)\s*\{([\s\S]*?)\n\}/g
  let serviceMatch
  while ((serviceMatch = servicePattern.exec(text))) {
    const serviceName = serviceMatch[1]
    const service = ensureNode(family, `proto_service:${idPart(file)}:${idPart(serviceName)}`, 'proto_service', serviceName, { schema: file })
    ensureEdge(family, artifact, service, 'DEFINES')
    for (const rpc of serviceMatch[2].matchAll(/\brpc\s+([A-Za-z_][A-Za-z0-9_]*)\s*\(([^)]*)\)\s+returns\s*\(([^)]*)\)/g)) {
      const rpcName = rpc[1]
      const method = ensureNode(family, `proto_rpc:${idPart(file)}:${idPart(serviceName)}:${idPart(rpcName)}`, 'proto_rpc', `${serviceName}.${rpcName}`, {
        requestType: rpc[2].trim(),
        responseType: rpc[3].trim(),
        schema: file,
      })
      ensureEdge(family, service, method, 'EXPOSES_RPC')
    }
  }
}

export function enrichApiCoverage(root, coverage) {
  const tracked = gitLines(root, ['ls-files'])
  const family = coverage.families.api_protocol

  for (const file of tracked) {
    const text = safeRead(root, file)
    if (!text) continue
    enrichProto(file, text, family)

    const observed = unique([
      ...(text.match(/['"](\/api\/[^'"\s]*)['"]/g) ?? []).map((value) => value.slice(1, -1)),
      ...(text.match(/fetch\(\s*['"](\/[^'"]+)['"]/g) ?? []).map((value) => value.replace(/^fetch\(\s*['"]/, '').replace(/['"]$/, '')),
    ]).filter((value) => value.startsWith('/'))
    if (!observed.length) continue

    const artifact = ensureNode(family, `artifact:${idPart(file)}`, 'api_observer_or_implementation', file, { path: file })
    addArtifact(family, file)
    for (const route of observed) {
      const routeNode = ensureNode(family, `http:${idPart(route)}`, 'http_route', route)
      const relation = file.startsWith('apps/ui/') ? 'CALLS' : 'EXPOSES_OR_REFERENCES'
      ensureEdge(family, artifact, routeNode, relation)
    }
  }

  family.nodes.sort((a, b) => a.id.localeCompare(b.id))
  family.edges.sort((a, b) => a.id.localeCompare(b.id))
  family.artifacts.sort()
  coverage.stats.familyStats.api_protocol = {
    nodes: family.nodes.length,
    edges: family.edges.length,
    artifacts: family.artifacts.length,
    gaps: family.gaps.length,
  }
  coverage.stats.totalNodes = Object.values(coverage.families).reduce((sum, item) => sum + item.nodes.length, 0)
  coverage.stats.totalEdges = Object.values(coverage.families).reduce((sum, item) => sum + item.edges.length, 0)
  return coverage
}
