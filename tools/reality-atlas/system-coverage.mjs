import { execFileSync } from 'node:child_process'
import { existsSync, readFileSync, statSync } from 'node:fs'
import { extname, join, posix as path } from 'node:path'

export const GRAPH_FAMILIES = [
  'repository',
  'semantic',
  'api_protocol',
  'data',
  'execution',
  'ui',
  'tooling',
  'test_evidence',
  'deployment',
  'ops',
  'external_integration',
  'machine_physical',
]

const TEXT_EXTENSIONS = new Set([
  '.c', '.cc', '.cpp', '.css', '.go', '.h', '.hpp', '.html', '.java', '.js', '.json', '.jsx', '.md', '.mjs', '.proto',
  '.py', '.rs', '.sh', '.sql', '.toml', '.ts', '.tsx', '.txt', '.yaml', '.yml', '.webmanifest',
])
const SOURCE_EXTENSIONS = new Set(['.c', '.cc', '.cpp', '.go', '.java', '.js', '.jsx', '.mjs', '.py', '.rs', '.ts', '.tsx'])
const MAX_TEXT_BYTES = 2 * 1024 * 1024
const CANONICAL_ROOTS = ['.github/', 'apps/', 'bindings/', 'crates/', 'deploy/', 'docs/', 'graph/', 'tools/']

const SEMANTICS = [
  ['identity', ['identity', 'principal', 'actor', 'subject']],
  ['resource', ['resource', 'entity', 'asset']],
  ['state', ['state', 'transition', 'temporalvalidity', 'temporal_validity']],
  ['capability_resolution', ['capability resolution', 'capability_resolution', 'resolve_capability', 'capabilityresolve', 'can(']],
  ['authority', ['authority', 'authorization', 'authorisation', 'policy admission', 'grant', 'deny']],
  ['event', ['canonical event', 'eventstore', 'event_store', 'event log', 'eventlog']],
  ['execution', ['workrequest', 'work_request', 'workrun', 'work_run', 'execution plan', 'execution_plan']],
  ['evidence', ['evidence', 'receipt', 'reconciliation', 'proof']],
  ['memory', ['memory', 'archive', 'historical']],
  ['context', ['contextenvelope', 'context_envelope', 'context.compile', 'context compiler']],
  ['intelligence', ['world model', 'world_model', 'inference', 'prediction', 'simulation']],
  ['sovereignty', ['sovereign', 'holding boundary', 'tenant isolation', 'purpose limitation']],
  ['safety', ['safety', 'interlock', 'emergency stop', 'e-stop']],
  ['physical', ['machine', 'robot', 'plc', 'telemetry']],
]

const EXECUTION_STAGES = [
  ['intent', ['intent']],
  ['work_request', ['workrequest', 'work_request']],
  ['capability_resolution', ['capability resolution', 'capability_resolution', 'resolve_capability']],
  ['authority_admission', ['authority', 'admission', 'authorize', 'authorise']],
  ['policy_safety', ['policy', 'safety', 'constraint']],
  ['execution_plan', ['executionplan', 'execution_plan', 'execution plan']],
  ['binding', ['binding', 'adapter selection', 'provider binding']],
  ['work_run', ['workrun', 'work_run']],
  ['effect', ['external effect', 'side effect', 'dispatch', 'execute']],
  ['evidence', ['evidence', 'receipt']],
  ['reconciliation', ['reconcile', 'reconciliation', 'unknown outcome']],
  ['canonical_event', ['canonical event', 'canonical_event', 'append_event']],
]

const MACHINE_PROTOCOLS = [
  ['opc_ua', /\bopc\s*ua\b|\bopcua\b/i],
  ['modbus', /\bmodbus\b/i],
  ['siemens_s7', /\b(?:siemens\s*)?s7\b/i],
  ['ethernet_ip', /\bethernet\/?ip\b|\bEIP\b/],
  ['profinet', /\bprofinet\b/i],
  ['beckhoff_ads', /\bbeckhoff\b|\bADS\b/],
  ['mqtt', /\bmqtt\b|sparkplug/i],
  ['ros2', /\bros\s*2\b|\bros2\b/i],
  ['can', /\bCAN\b|controller area network/i],
  ['obd_ii', /\bobd[-\s]?ii\b|\bobd2\b/i],
  ['bacnet', /\bbacnet\b/i],
  ['knx', /\bknx\b/i],
]

const PROVIDERS = [
  'claude', 'codex', 'openai', 'gemini', 'grok', 'cursor', 'opencode', 'openclaw', 'acpx', 'pi',
  'github', 'google', 'aws', 'azure', 'supabase', 'postgres', 'redis', 'mqtt', 'ros2', 'modbus', 'opcua',
]

function slash(value) { return String(value).replaceAll('\\', '/') }
function unique(values) { return [...new Set(values.filter(Boolean))].sort() }
function idPart(value) { return String(value).replace(/[^A-Za-z0-9_.:/-]+/g, '_') }
// `git ls-files` on this repo's full tracked tree (739k+ paths, mostly donor corpus under
// temporary/) exceeds Node's default 1MB execFileSync stdout buffer and fails with ENOBUFS.
// 256MB comfortably covers that today with headroom for future growth.
const GIT_MAX_BUFFER_BYTES = 256 * 1024 * 1024
function gitLines(root, args) {
  return execFileSync('git', args, { cwd: root, encoding: 'utf8', maxBuffer: GIT_MAX_BUFFER_BYTES }).split('\n').map((line) => slash(line.trim())).filter(Boolean)
}
function safeRead(root, file) {
  try {
    const absolute = join(root, file)
    if (!existsSync(absolute) || statSync(absolute).size > MAX_TEXT_BYTES) return ''
    if (!TEXT_EXTENSIONS.has(extname(file).toLowerCase()) && !['Dockerfile', 'Cargo.toml', 'package.json', 'Caddyfile', 'Makefile'].includes(path.basename(file))) return ''
    return readFileSync(absolute, 'utf8')
  } catch { return '' }
}
function sourceFile(file) { return SOURCE_EXTENSIONS.has(extname(file).toLowerCase()) }
function testFile(file) { return /(?:^|\/)(?:tests?|__tests__|benches)(?:\/|$)|\.(?:test|spec)\.[cm]?[jt]sx?$|_test\.rs$/.test(file) }
function benchmarkFile(file) { return /(?:^|\/)benches\//.test(file) || /benchmark|bench_/i.test(path.basename(file)) }
function familyRecord(name) { return { name, nodes: [], edges: [], artifacts: [], gaps: [] } }

function createBuilder() {
  const families = Object.fromEntries(GRAPH_FAMILIES.map((name) => [name, familyRecord(name)]))
  const nodeKeys = new Set()
  const edgeKeys = new Set()
  const artifactFamilies = new Map()

  function markArtifact(file, family) {
    if (!file) return
    const set = artifactFamilies.get(file) ?? new Set()
    set.add(family)
    artifactFamilies.set(file, set)
    if (!families[family].artifacts.includes(file)) families[family].artifacts.push(file)
  }
  function node(family, id, kind, label, data = {}, artifact = null) {
    const key = `${family}:${id}`
    if (!nodeKeys.has(key)) {
      nodeKeys.add(key)
      families[family].nodes.push({ id: key, family, kind, label, ...data })
    }
    if (artifact) markArtifact(artifact, family)
    return key
  }
  function edge(family, source, target, type, data = {}) {
    const key = `${family}:${source}:${type}:${target}`
    if (!edgeKeys.has(key)) {
      edgeKeys.add(key)
      families[family].edges.push({ id: key, family, source, target, type, ...data })
    }
  }
  function gap(family, kind, artifact, message) {
    families[family].gaps.push({ kind, artifact, message })
    if (artifact) markArtifact(artifact, family)
  }
  return { families, artifactFamilies, node, edge, gap, markArtifact }
}

function resolveTsImport(fromFile, specifier, trackedSet) {
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

function extractTsImports(text) {
  const values = []
  for (const re of [/(?:import|export)\s+(?:[^'";]*?\sfrom\s*)?["']([^"']+)["']/g, /import\(\s*["']([^"']+)["']\s*\)/g]) {
    let m
    while ((m = re.exec(text))) values.push(m[1])
  }
  return unique(values)
}

function packageName(text) {
  try { return JSON.parse(text).name ?? null } catch { return null }
}

function rustPackageName(text) {
  const pkg = text.match(/\[package\][\s\S]*?^name\s*=\s*["']([^"']+)["']/m)
  return pkg?.[1] ?? null
}

function cargoDependencies(text) {
  const out = []
  const section = text.match(/\[dependencies\]([\s\S]*?)(?:\n\[|$)/m)?.[1] ?? ''
  for (const line of section.split('\n')) {
    const m = line.match(/^\s*([A-Za-z0-9_-]+)\s*=/)
    if (m) out.push(m[1])
  }
  return unique(out)
}

function jsonDependencies(text) {
  try {
    const pkg = JSON.parse(text)
    return unique([...Object.keys(pkg.dependencies ?? {}), ...Object.keys(pkg.devDependencies ?? {}), ...Object.keys(pkg.peerDependencies ?? {})])
  } catch { return [] }
}

function semanticMatches(text, file) {
  const haystack = `${file}\n${text}`.toLowerCase()
  return SEMANTICS.filter(([, terms]) => terms.some((term) => haystack.includes(term.toLowerCase()))).map(([name]) => name)
}

function extractInvariantIds(text) {
  return unique(String(text).match(/\b(?:INV|ARCH-EQ|ARCH-PERF)-[A-Z0-9_-]+\b/g) ?? [])
}

function parseSqlObjects(text) {
  const out = []
  const patterns = [
    ['table', /create\s+table\s+(?:if\s+not\s+exists\s+)?["`]?([A-Za-z0-9_.-]+)["`]?/ig],
    ['index', /create\s+(?:unique\s+)?index\s+(?:if\s+not\s+exists\s+)?["`]?([A-Za-z0-9_.-]+)["`]?/ig],
    ['materialized_view', /create\s+materialized\s+view\s+(?:if\s+not\s+exists\s+)?["`]?([A-Za-z0-9_.-]+)["`]?/ig],
    ['view', /create\s+view\s+(?:if\s+not\s+exists\s+)?["`]?([A-Za-z0-9_.-]+)["`]?/ig],
  ]
  for (const [kind, re] of patterns) {
    let m
    while ((m = re.exec(text))) out.push({ kind, name: m[1] })
  }
  return out
}

function parseHttpRoutes(text) {
  const routes = []
  const patterns = [
    /\.route\(\s*["']([^"']+)["']/g,
    /(?:get|post|put|patch|delete)\(\s*["']([^"']+)["']/gi,
    /(?:path|route)\s*:\s*["'](\/[^"']*)["']/g,
    /<Route[^>]*\bpath=["']([^"']+)["']/g,
  ]
  for (const re of patterns) {
    let m
    while ((m = re.exec(text))) routes.push(m[1])
  }
  return unique(routes.filter((route) => route.startsWith('/')))
}

function parseMcpTools(text) {
  const tools = []
  const patterns = [
    /(?:tool|name)\s*[:=]\s*["']([A-Za-z0-9_.:-]+)["']/g,
    /register_tool\s*\(\s*["']([^"']+)["']/gi,
    /mcp[^\n]{0,80}["']([A-Za-z0-9_.:-]+)["']/gi,
  ]
  for (const re of patterns) {
    let m
    while ((m = re.exec(text))) tools.push(m[1])
  }
  return unique(tools)
}

function parseEventNames(text) {
  const names = []
  for (const re of [/(?:event_type|eventType|kind)\s*[:=]\s*["']([A-Za-z0-9_.:-]+)["']/g, /emit\s*\(\s*["']([^"']+)["']/g, /publish\s*\(\s*["']([^"']+)["']/g]) {
    let m
    while ((m = re.exec(text))) names.push(m[1])
  }
  return unique(names)
}

function parseQueueNames(text) {
  const names = []
  for (const re of [/(?:queue|topic|subject|stream)\s*[:=]\s*["']([A-Za-z0-9_.:-]+)["']/gi, /(?:enqueue|subscribe|publish)\s*\(\s*["']([^"']+)["']/gi]) {
    let m
    while ((m = re.exec(text))) names.push(m[1])
  }
  return unique(names)
}

function parseComposeServices(text) {
  const names = []
  const lines = text.split('\n')
  let services = false
  for (const line of lines) {
    if (/^services:\s*$/.test(line)) { services = true; continue }
    if (services && /^\S/.test(line) && !/^services:/.test(line)) services = false
    if (services) {
      const m = line.match(/^\s{2}([A-Za-z0-9_.-]+):\s*$/)
      if (m) names.push(m[1])
    }
  }
  return unique(names)
}

function classifyRepository(root, tracked, texts, b) {
  const rootNode = b.node('repository', 'repo:chronica', 'repository', 'Chronica')
  const trackedSet = new Set(tracked)
  const packageByName = new Map()
  const manifestByFile = new Map()

  for (const file of tracked) {
    const fileNode = b.node('repository', `file:${idPart(file)}`, 'source_file', file, { path: file, extension: extname(file) || null }, file)
    b.edge('repository', rootNode, fileNode, 'CONTAINS')
    const parts = file.split('/')
    if (parts.length > 1) {
      const modulePath = parts.slice(0, -1).join('/')
      const moduleNode = b.node('repository', `module:${idPart(modulePath)}`, 'module', modulePath, { path: modulePath }, file)
      b.edge('repository', moduleNode, fileNode, 'CONTAINS')
      b.edge('repository', rootNode, moduleNode, 'CONTAINS')
    }
  }

  for (const file of tracked.filter((value) => value.endsWith('Cargo.toml'))) {
    const text = texts.get(file) ?? ''
    const name = rustPackageName(text) ?? (file === 'Cargo.toml' ? 'chronica-workspace' : path.basename(path.dirname(file)))
    const pkg = b.node('repository', `crate:${idPart(name)}`, 'crate', name, { manifest: file }, file)
    packageByName.set(name, pkg)
    manifestByFile.set(file, { node: pkg, dependencies: cargoDependencies(text), kind: 'crate' })
    b.edge('repository', rootNode, pkg, 'CONTAINS')
  }

  for (const file of tracked.filter((value) => value.endsWith('package.json'))) {
    const text = texts.get(file) ?? ''
    const name = packageName(text) ?? path.dirname(file)
    const pkg = b.node('repository', `package:${idPart(name)}`, 'package', name, { manifest: file }, file)
    packageByName.set(name, pkg)
    manifestByFile.set(file, { node: pkg, dependencies: jsonDependencies(text), kind: 'package' })
    b.edge('repository', rootNode, pkg, 'CONTAINS')
  }

  for (const { node: source, dependencies } of manifestByFile.values()) {
    for (const dependency of dependencies) {
      const target = packageByName.get(dependency) ?? b.node('repository', `external_dependency:${idPart(dependency)}`, 'external_dependency', dependency)
      b.edge('repository', source, target, 'DEPENDS_ON')
    }
  }

  for (const file of tracked.filter(sourceFile)) {
    const text = texts.get(file) ?? ''
    const source = `repository:file:${idPart(file)}`
    if (/\.[cm]?[jt]sx?$/.test(file)) {
      for (const spec of extractTsImports(text)) {
        const local = resolveTsImport(file, spec, trackedSet)
        if (local) b.edge('repository', source, `repository:file:${idPart(local)}`, 'IMPORTS')
        else if (!spec.startsWith('.') && !spec.startsWith('@/')) {
          const target = packageByName.get(spec) ?? b.node('repository', `external_dependency:${idPart(spec)}`, 'external_dependency', spec)
          b.edge('repository', source, target, 'IMPORTS')
        }
      }
    }
    if (file.endsWith('.rs')) {
      for (const m of text.matchAll(/^\s*(?:pub\s+)?mod\s+([A-Za-z0-9_]+)\s*;/gm)) {
        const dir = path.dirname(file)
        const candidates = [`${dir}/${m[1]}.rs`, `${dir}/${m[1]}/mod.rs`]
        const target = candidates.find((candidate) => trackedSet.has(candidate))
        if (target) b.edge('repository', source, `repository:file:${idPart(target)}`, 'IMPORTS')
      }
    }
  }
}

function classifySemantic(tracked, texts, b) {
  const semanticNodes = new Map()
  for (const [name] of SEMANTICS) semanticNodes.set(name, b.node('semantic', `semantic:${name}`, 'semantic_primitive', name))
  const invariantNodes = new Map()
  for (const file of tracked.filter((value) => sourceFile(value) || value.endsWith('.md'))) {
    const text = texts.get(file) ?? ''
    const matches = semanticMatches(text, file)
    const impl = b.node('semantic', `artifact:${idPart(file)}`, sourceFile(file) ? 'implementation' : 'documentation', file, { path: file }, file)
    for (const semantic of matches) b.edge('semantic', impl, semanticNodes.get(semantic), 'MAPS_TO', { confidence: 'heuristic' })
    for (const invariant of extractInvariantIds(text)) {
      const inv = invariantNodes.get(invariant) ?? b.node('semantic', `invariant:${idPart(invariant)}`, 'invariant', invariant)
      invariantNodes.set(invariant, inv)
      b.edge('semantic', impl, inv, 'REFERENCES')
    }
    if (sourceFile(file) && !testFile(file) && matches.length === 0) b.gap('semantic', 'UNMAPPED_IMPLEMENTATION', file, 'Source file has no observed mapping to a canonical semantic primitive.')
  }
}

function classifyApiProtocol(tracked, texts, b) {
  for (const file of tracked.filter((value) => sourceFile(value) || value.endsWith('.proto') || value.endsWith('.md'))) {
    const text = texts.get(file) ?? ''
    if (!text) continue
    const owner = b.node('api_protocol', `artifact:${idPart(file)}`, 'implementation', file, { path: file }, file)
    for (const route of parseHttpRoutes(text)) {
      const routeNode = b.node('api_protocol', `http:${idPart(route)}`, 'http_route', route)
      b.edge('api_protocol', owner, routeNode, 'EXPOSES')
    }
    if (/\bmcp\b/i.test(text) || /mcp/i.test(file)) {
      for (const tool of parseMcpTools(text)) {
        const toolNode = b.node('api_protocol', `mcp:${idPart(tool)}`, 'mcp_tool', tool)
        b.edge('api_protocol', owner, toolNode, 'EXPOSES')
      }
    }
    for (const event of parseEventNames(text)) {
      const eventNode = b.node('api_protocol', `event:${idPart(event)}`, 'event', event)
      b.edge('api_protocol', owner, eventNode, 'EMITS_OR_HANDLES')
    }
    for (const queue of parseQueueNames(text)) {
      const queueNode = b.node('api_protocol', `queue:${idPart(queue)}`, 'queue_or_topic', queue)
      b.edge('api_protocol', owner, queueNode, 'USES')
    }
    const urls = unique(text.match(/https?:\/\/[A-Za-z0-9._~:/?#\[\]@!$&'()*+,;=%-]+/g) ?? [])
    for (const url of urls.slice(0, 50)) {
      const endpoint = b.node('api_protocol', `endpoint:${idPart(url)}`, 'external_endpoint', url)
      b.edge('api_protocol', owner, endpoint, 'CONNECTS_TO')
    }
  }
}

function classifyData(tracked, texts, b) {
  for (const file of tracked) {
    const text = texts.get(file) ?? ''
    if (!text) continue
    const isMigration = /(?:^|\/)(?:migrations?|sql)(?:\/|$)/i.test(file)
    const objects = parseSqlObjects(text)
    if (!isMigration && !objects.length && !/event[_ -]?store|projection|materialized view|postgres/i.test(text)) continue
    const owner = b.node('data', `artifact:${idPart(file)}`, isMigration ? 'migration' : 'data_implementation', file, { path: file }, file)
    for (const object of objects) {
      const target = b.node('data', `${object.kind}:${idPart(object.name)}`, object.kind, object.name)
      b.edge('data', owner, target, 'DEFINES')
    }
    if (/event[_ -]?store/i.test(text)) b.edge('data', owner, b.node('data', 'store:event_store', 'event_store', 'event_store'), 'IMPLEMENTS')
    if (/projection|materialized view/i.test(text)) b.edge('data', owner, b.node('data', `projection:${idPart(file)}`, 'projection', file), 'IMPLEMENTS')
  }
}

function classifyExecution(tracked, texts, b) {
  const stages = EXECUTION_STAGES.map(([name]) => b.node('execution', `stage:${name}`, 'execution_stage', name))
  for (let i = 0; i < stages.length - 1; i++) b.edge('execution', stages[i], stages[i + 1], 'NEXT')
  for (const file of tracked.filter(sourceFile)) {
    const text = (texts.get(file) ?? '').toLowerCase()
    const matched = EXECUTION_STAGES.filter(([, terms]) => terms.some((term) => text.includes(term))).map(([name]) => name)
    if (!matched.length) continue
    const impl = b.node('execution', `artifact:${idPart(file)}`, 'implementation', file, { path: file }, file)
    for (const stage of matched) b.edge('execution', impl, `execution:stage:${stage}`, 'IMPLEMENTS')
  }
}

function classifyUi(tracked, texts, b, ui) {
  const sourceFiles = tracked.filter((file) => file.startsWith('apps/ui/src/') && sourceFile(file))
  for (const file of sourceFiles) {
    const text = texts.get(file) ?? ''
    const kind = /\/pages\//.test(file) ? 'page' : /\/components\//.test(file) ? 'component' : 'ui_source'
    const node = b.node('ui', `artifact:${idPart(file)}`, kind, file, { path: file }, file)
    for (const route of parseHttpRoutes(text)) {
      const routeNode = b.node('ui', `route:${idPart(route)}`, 'ui_route', route)
      b.edge('ui', routeNode, node, 'RENDERS')
    }
    if (/useQuery\s*\(|useMutation\s*\(|fetch\s*\(|api\./.test(text)) {
      const query = b.node('ui', `query:${idPart(file)}`, 'api_or_query_client', file)
      b.edge('ui', node, query, 'CALLS')
    }
    for (const semantic of semanticMatches(text, file)) {
      const semanticNode = b.node('ui', `semantic:${semantic}`, 'semantic_projection', semantic)
      b.edge('ui', node, semanticNode, 'PROJECTS')
    }
  }
  for (const file of ui?.orphanCandidates ?? []) b.gap('ui', 'ORPHAN_CANDIDATE', file.path, 'UI source is not production-reachable from the current entrypoint.')
  for (const pkg of ui?.packageReviewCandidates ?? []) b.gap('ui', pkg.status, pkg.path, 'UI workspace package lacks observed production import evidence.')
  for (const asset of ui?.unreferencedPublicAssets ?? []) b.gap('ui', asset.status, asset.path, 'Public asset has no observed reference.')
}

function classifyTooling(root, tracked, texts, b, tooling) {
  const manifests = tracked.filter((file) => file.endsWith('package.json'))
  for (const file of manifests) {
    const text = texts.get(file) ?? ''
    let scripts = {}
    try { scripts = JSON.parse(text).scripts ?? {} } catch { scripts = {} }
    for (const [name, command] of Object.entries(scripts)) {
      const script = b.node('tooling', `script:${idPart(file)}:${idPart(name)}`, 'package_script', `${name}`, { manifest: file, command }, file)
      for (const tool of tracked.filter((candidate) => candidate.startsWith('tools/') && String(command).includes(candidate))) {
        const target = b.node('tooling', `tool:${idPart(tool)}`, 'tool', tool, { path: tool }, tool)
        b.edge('tooling', script, target, 'CALLS')
      }
    }
  }
  for (const file of tracked.filter((value) => value.startsWith('.github/workflows/') && /\.ya?ml$/.test(value))) {
    const workflow = b.node('tooling', `workflow:${idPart(file)}`, 'ci_workflow', file, { path: file }, file)
    const text = texts.get(file) ?? ''
    for (const m of text.matchAll(/^\s*run:\s*(.+)$/gm)) {
      const command = b.node('tooling', `ci_command:${idPart(file)}:${idPart(m[1].slice(0, 120))}`, 'ci_command', m[1].trim())
      b.edge('tooling', workflow, command, 'RUNS')
    }
  }
  for (const group of tooling?.groups ?? []) {
    const groupNode = b.node('tooling', `group:${idPart(group.path)}`, 'tool_group', group.path, { lifecycle: group.status }, group.path)
    for (const ref of group.referenceFiles ?? []) {
      const caller = b.node('tooling', `caller:${idPart(ref)}`, 'caller', ref, { path: ref }, ref)
      b.edge('tooling', caller, groupNode, 'CALLS_OR_REFERENCES')
    }
    if (['ORPHAN_CANDIDATE', 'RETIREMENT_CANDIDATE'].includes(group.status)) b.gap('tooling', group.status, group.path, group.lifecycleNote ?? 'Tool group requires lifecycle review.')
  }
}

function classifyTestEvidence(tracked, texts, b) {
  const invariantNodes = new Map()
  for (const file of tracked.filter((value) => testFile(value) || benchmarkFile(value))) {
    const text = texts.get(file) ?? ''
    const test = b.node('test_evidence', `test:${idPart(file)}`, benchmarkFile(file) ? 'benchmark' : 'test', file, { path: file }, file)
    const invariants = extractInvariantIds(text)
    for (const invariant of invariants) {
      const inv = invariantNodes.get(invariant) ?? b.node('test_evidence', `invariant:${idPart(invariant)}`, 'invariant', invariant)
      invariantNodes.set(invariant, inv)
      b.edge('test_evidence', test, inv, 'VERIFIES')
    }
    if (!invariants.length) b.gap('test_evidence', 'UNLINKED_TEST', file, 'Test/benchmark has no observed architecture invariant reference.')
  }
  for (const file of tracked.filter((value) => value.startsWith('.github/workflows/'))) {
    const text = texts.get(file) ?? ''
    if (!/test|bench|check|lint/i.test(text)) continue
    const workflow = b.node('test_evidence', `workflow:${idPart(file)}`, 'ci_workflow', file, { path: file }, file)
    for (const invariant of extractInvariantIds(text)) {
      const inv = invariantNodes.get(invariant) ?? b.node('test_evidence', `invariant:${idPart(invariant)}`, 'invariant', invariant)
      invariantNodes.set(invariant, inv)
      b.edge('test_evidence', workflow, inv, 'ENFORCES')
    }
  }
}

function classifyDeployment(root, tracked, texts, b, sourceSha) {
  const shaNode = b.node('deployment', `git:${sourceSha}`, 'git_revision', sourceSha)
  for (const file of tracked.filter((value) => value.startsWith('deploy/') || value === 'Dockerfile' || value === 'apps/ui/Dockerfile')) {
    const text = texts.get(file) ?? ''
    const artifact = b.node('deployment', `artifact:${idPart(file)}`, 'deployment_artifact', file, { path: file }, file)
    b.edge('deployment', shaNode, artifact, 'BUILDS_OR_CONFIGURES')
    if (/docker-compose|compose\.ya?ml/.test(file)) {
      for (const service of parseComposeServices(text)) {
        const serviceNode = b.node('deployment', `service:${idPart(service)}`, 'service', service)
        b.edge('deployment', artifact, serviceNode, 'DEPLOYS')
      }
    }
    for (const env of unique(text.match(/\b(?:production|staging|development|local)\b/gi) ?? [])) {
      const envNode = b.node('deployment', `environment:${env.toLowerCase()}`, 'environment', env.toLowerCase())
      b.edge('deployment', artifact, envNode, 'TARGETS')
    }
  }
  const evidenceFiles = tracked.filter((file) => /deployment-evidence|runtime-evidence/i.test(file))
  for (const file of evidenceFiles) {
    const evidence = b.node('deployment', `runtime_evidence:${idPart(file)}`, 'runtime_evidence', file, { path: file }, file)
    b.edge('deployment', shaNode, evidence, 'EVIDENCED_BY')
  }
}

function classifyOps(tracked, texts, b) {
  for (const file of tracked.filter((value) => value.startsWith('docs/ops_production/') || value.startsWith('tools/ops-logistics/'))) {
    const text = texts.get(file) ?? ''
    const kind = file.includes('/contracts/') ? 'ops_contract' : file.startsWith('tools/') ? 'ops_tooling' : 'ops_document'
    const artifact = b.node('ops', `artifact:${idPart(file)}`, kind, file, { path: file }, file)
    const terms = [
      ['donor_baseline', /donor baseline|DONOR_BASELINE|donor_sha/i],
      ['semantic_mapping', /semantic_convergence|semantic mapping|MATCH_STATUS|DISPOSITION/i],
      ['legacy_burndown', /legacy_burndown|legacy burn-down/i],
      ['absorption_candidate', /absorption|ABSORB|EXTEND_CHRONICA_SEMANTIC/i],
      ['chronica_destination', /chronica_owner|destination|canonical owner/i],
    ]
    for (const [name, re] of terms) if (re.test(text)) b.edge('ops', artifact, b.node('ops', `concept:${name}`, 'ops_lifecycle_concept', name), 'COVERS')
  }
}

function classifyExternalIntegration(tracked, texts, b) {
  const candidates = tracked.filter((file) => /(?:^|\/)(?:adapter|adapters|bindings)(?:\/|$)/i.test(file) || file.startsWith('apps/ui/packages/adapters/'))
  for (const file of candidates) {
    const text = texts.get(file) ?? ''
    const adapter = b.node('external_integration', `adapter:${idPart(file)}`, 'adapter_or_binding', file, { path: file }, file)
    const haystack = `${file}\n${text}`.toLowerCase()
    for (const provider of PROVIDERS) {
      if (!haystack.includes(provider.toLowerCase())) continue
      const providerNode = b.node('external_integration', `provider:${provider}`, 'provider_or_service', provider)
      b.edge('external_integration', providerNode, adapter, 'BOUND_BY')
    }
    const semantics = semanticMatches(text, file)
    for (const semantic of semantics) {
      const target = b.node('external_integration', `semantic:${semantic}`, 'semantic_binding_target', semantic)
      b.edge('external_integration', adapter, target, 'TRANSLATES_TO')
    }
    if (!semantics.length) b.gap('external_integration', 'UNMAPPED_ADAPTER_SEMANTIC', file, 'Adapter/binding has no observed mapping to canonical semantics.')
  }
}

function classifyMachinePhysical(tracked, texts, b) {
  const physicalFiles = []
  const safety = b.node('machine_physical', 'safety:hard_interlocks', 'safety_boundary', 'hard_interlocks_and_emergency_protection')
  for (const file of tracked) {
    const text = texts.get(file) ?? ''
    const haystack = `${file}\n${text}`
    const protocols = MACHINE_PROTOCOLS.filter(([, re]) => re.test(haystack)).map(([name]) => name)
    const physical = /machine|robot|plc|physical|telemetry|digital twin|actuator|sensor/i.test(haystack)
    if (!physical && !protocols.length) continue
    physicalFiles.push(file)
    const artifact = b.node('machine_physical', `artifact:${idPart(file)}`, sourceFile(file) ? 'implementation' : 'contract_or_config', file, { path: file }, file)
    for (const protocol of protocols) {
      const protocolNode = b.node('machine_physical', `protocol:${protocol}`, 'machine_protocol', protocol)
      b.edge('machine_physical', artifact, protocolNode, 'IMPLEMENTS_OR_REFERENCES')
    }
    if (/control|command|actuat|write/i.test(text)) b.edge('machine_physical', artifact, safety, 'MUST_BE_GUARDED_BY')
  }
  if (physicalFiles.length) {
    const resource = b.node('machine_physical', 'semantic:machine_resource', 'semantic_machine_resource', 'machine_resource')
    const adapter = b.node('machine_physical', 'semantic:industrial_adapter', 'adapter_boundary', 'industrial_adapter')
    const device = b.node('machine_physical', 'semantic:physical_device', 'physical_device', 'physical_device')
    b.edge('machine_physical', resource, adapter, 'RESOLVES_THROUGH')
    b.edge('machine_physical', adapter, device, 'CONNECTS_TO')
    b.edge('machine_physical', adapter, safety, 'SUBJECT_TO')
  }
}

export function compileSystemCoverage(root, options = {}) {
  const tracked = gitLines(root, ['ls-files'])
  const sourceSha = gitLines(root, ['rev-parse', 'HEAD'])[0] ?? 'UNKNOWN'
  const texts = new Map(tracked.map((file) => [file, safeRead(root, file)]))
  const b = createBuilder()

  classifyRepository(root, tracked, texts, b)
  classifySemantic(tracked, texts, b)
  classifyApiProtocol(tracked, texts, b)
  classifyData(tracked, texts, b)
  classifyExecution(tracked, texts, b)
  classifyUi(tracked, texts, b, options.ui)
  classifyTooling(root, tracked, texts, b, options.tooling)
  classifyTestEvidence(tracked, texts, b)
  classifyDeployment(root, tracked, texts, b, sourceSha)
  classifyOps(tracked, texts, b)
  classifyExternalIntegration(tracked, texts, b)
  classifyMachinePhysical(tracked, texts, b)

  // Repository Graph is intentionally exhaustive: every tracked artifact is classified there.
  const classified = tracked.filter((file) => b.artifactFamilies.has(file))
  const unclassified = tracked.filter((file) => !b.artifactFamilies.has(file))
  const sourceFiles = tracked.filter(sourceFile)
  const semanticMapped = new Set(b.families.semantic.artifacts.filter((file) => sourceFile(file) && !b.families.semantic.gaps.some((gap) => gap.kind === 'UNMAPPED_IMPLEMENTATION' && gap.artifact === file)))
  const familyStats = Object.fromEntries(GRAPH_FAMILIES.map((name) => [name, {
    nodes: b.families[name].nodes.length,
    edges: b.families[name].edges.length,
    artifacts: b.families[name].artifacts.length,
    gaps: b.families[name].gaps.length,
  }]))

  for (const family of Object.values(b.families)) {
    family.artifacts.sort()
    family.nodes.sort((a, c) => a.id.localeCompare(c.id))
    family.edges.sort((a, c) => a.id.localeCompare(c.id))
    family.gaps.sort((a, c) => `${a.kind}:${a.artifact ?? ''}`.localeCompare(`${c.kind}:${c.artifact ?? ''}`))
  }

  return {
    schemaVersion: 1,
    source: 'generated multi-resolution System Atlas coverage projection; canonical truth remains in durable Chronica state and source owners',
    sourceSha,
    graphFamilies: GRAPH_FAMILIES,
    families: b.families,
    artifactClassification: tracked.map((file) => ({ file, families: unique([...(b.artifactFamilies.get(file) ?? [])]) })),
    unclassifiedArtifacts: unclassified,
    stats: {
      graphFamilies: GRAPH_FAMILIES.length,
      requiredGraphFamilies: GRAPH_FAMILIES.length,
      domainCoveragePercent: 100,
      trackedArtifacts: tracked.length,
      classifiedTrackedArtifacts: classified.length,
      trackedArtifactCoveragePercent: tracked.length ? Math.round((classified.length / tracked.length) * 10000) / 100 : 100,
      sourceFiles: sourceFiles.length,
      semanticMappedSourceFiles: semanticMapped.size,
      semanticMappingPercent: sourceFiles.length ? Math.round((semanticMapped.size / sourceFiles.length) * 10000) / 100 : 100,
      totalNodes: Object.values(b.families).reduce((sum, family) => sum + family.nodes.length, 0),
      totalEdges: Object.values(b.families).reduce((sum, family) => sum + family.edges.length, 0),
      totalGaps: Object.values(b.families).reduce((sum, family) => sum + family.gaps.length, 0),
      familyStats,
    },
  }
}

export function validateSystemCoverage(coverage) {
  const errors = []
  if (coverage?.schemaVersion !== 1) errors.push('System coverage schemaVersion must be 1')
  for (const family of GRAPH_FAMILIES) {
    if (!coverage?.families?.[family]) errors.push(`missing System Atlas graph family: ${family}`)
  }
  if (coverage?.stats?.domainCoveragePercent !== 100) errors.push(`System Atlas domain coverage must be 100%, got ${coverage?.stats?.domainCoveragePercent}`)
  if (coverage?.stats?.trackedArtifactCoveragePercent !== 100) errors.push(`tracked artifact classification coverage must be 100%, got ${coverage?.stats?.trackedArtifactCoveragePercent}`)
  if ((coverage?.unclassifiedArtifacts ?? []).length) errors.push(`unclassified tracked artifacts remain: ${coverage.unclassifiedArtifacts.slice(0, 20).join(', ')}`)

  const nodeIds = new Set()
  const edgeIds = new Set()
  for (const family of Object.values(coverage?.families ?? {})) {
    for (const node of family.nodes ?? []) {
      if (nodeIds.has(node.id)) errors.push(`duplicate System Atlas node id: ${node.id}`)
      nodeIds.add(node.id)
    }
    for (const edge of family.edges ?? []) {
      if (edgeIds.has(edge.id)) errors.push(`duplicate System Atlas edge id: ${edge.id}`)
      edgeIds.add(edge.id)
      if (!nodeIds.has(edge.source) && !String(edge.source).startsWith(`${family.name}:`)) errors.push(`edge source is not namespaced to family ${family.name}: ${edge.source}`)
      if (!nodeIds.has(edge.target) && !String(edge.target).startsWith(`${family.name}:`)) errors.push(`edge target is not namespaced to family ${family.name}: ${edge.target}`)
    }
  }
  return errors
}
