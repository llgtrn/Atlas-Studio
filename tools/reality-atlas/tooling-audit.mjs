import { execFileSync } from 'node:child_process'
import { existsSync, readFileSync, statSync } from 'node:fs'
import { dirname, extname, join, normalize, posix as path, resolve } from 'node:path'

const TEXT_EXTENSIONS = new Set([
  '.cjs', '.css', '.html', '.js', '.json', '.jsx', '.md', '.mjs', '.ps1', '.py', '.rs', '.sh', '.sql', '.toml', '.ts', '.tsx', '.txt', '.yaml', '.yml',
])
const EXECUTABLE_EXTENSIONS = new Set(['.cjs', '.js', '.mjs', '.ps1', '.py', '.sh', '.sql', '.ts'])
const SOURCE_CALLER_EXTENSIONS = new Set(['.cjs', '.js', '.jsx', '.mjs', '.ps1', '.py', '.rs', '.sh', '.ts', '.tsx'])
const MAX_TEXT_BYTES = 1024 * 1024
const STALE_DAYS = 90

const LIFECYCLE_RULES = [
  {
    prefix: 'tools/capabilities/',
    kind: 'CAPABILITY_ERA_TRANSITIONAL',
    note: 'Capability-era operational tooling remains usable while capability debt exists, but it must converge or retire with the permanent Capability-layer extinction target.',
  },
  {
    prefix: 'tools/canonical-shards/',
    kind: 'CAPABILITY_ERA_TRANSITIONAL',
    note: 'Canonical-shard helpers are tied to capability-era census/proof infrastructure and should be reassessed as that substrate is absorbed or retired.',
  },
  {
    prefix: 'tools/first-l4/',
    kind: 'MILESTONE_SCOPED',
    note: 'First-L4 tooling is milestone-scoped. Once no current workflow/source/manual caller needs the milestone harness, prefer retirement over indefinite preservation.',
  },
]

const ROOT_LEGACY_RULES = [
  {
    path: '.cargo',
    id: 'HIDDEN_CARGO_CONTROL_SURFACE',
    note: 'Project-local Cargo aliases are retired in Chronica. Build tooling must be explicit through canonical crates, root scripts, docs and CI rather than a hidden root control surface.',
  },
  {
    path: '.claude',
    id: 'PROVIDER_LOCAL_PROJECT_SCAFFOLD',
    note: 'Project-local Claude/spec-kit skills are retired. Provider-specific agent scaffolding must not become repository architecture or a competing workflow universe.',
  },
  {
    path: '.specify',
    id: 'SPECKIT_PROJECT_SCAFFOLD',
    note: 'Spec-Kit project-local constitution/templates are retired. Canonical planning and architecture ownership live under docs/ and executable repo contracts.',
  },
]

function slash(value) {
  return String(value).replaceAll('\\', '/')
}

function git(root, args, { allowFailure = false } = {}) {
  try {
    // maxBuffer: see tools/reality-atlas/lib.mjs's identical gitLines fix -- the tracked donor
    // corpus under temporary/ (736k+ files) exceeds the default 1MB buffer on a plain `git ls-files`.
    return execFileSync('git', args, {
      cwd: root,
      encoding: 'utf8',
      stdio: ['ignore', 'pipe', 'pipe'],
      maxBuffer: 256 * 1024 * 1024,
    }).trim()
  } catch (error) {
    if (allowFailure) return ''
    throw error
  }
}

function gitLines(root, args) {
  return git(root, args).split('\n').map((line) => slash(line.trim())).filter(Boolean)
}

function safeRead(root, file) {
  try {
    const absolute = join(root, file)
    if (!existsSync(absolute) || statSync(absolute).size > MAX_TEXT_BYTES) return ''
    return readFileSync(absolute, 'utf8')
  } catch {
    return ''
  }
}

function isTextCarrier(file) {
  const extension = extname(file).toLowerCase()
  const base = path.basename(file)
  return TEXT_EXTENSIONS.has(extension) || ['Cargo.toml', 'package.json', 'Dockerfile', 'Makefile'].includes(base)
}

function isExecutableTool(file) {
  return file.startsWith('tools/') && EXECUTABLE_EXTENSIONS.has(extname(file).toLowerCase())
}

function isSourceCaller(file) {
  return SOURCE_CALLER_EXTENSIONS.has(extname(file).toLowerCase())
}

function topGroup(file) {
  if (!file.startsWith('tools/')) return null
  const rel = file.slice('tools/'.length)
  const first = rel.split('/')[0]
  if (!first) return null
  return rel.includes('/') ? `tools/${first}` : 'tools'
}

function lifecycleFor(group) {
  const prefix = `${group}/`
  return LIFECYCLE_RULES.find((rule) => prefix.startsWith(rule.prefix) || rule.prefix.startsWith(prefix)) ?? null
}

function lastChange(root, group) {
  const raw = git(root, ['log', '-1', '--format=%ct', '--', group], { allowFailure: true })
  const epochSeconds = Number(raw)
  if (!Number.isFinite(epochSeconds) || epochSeconds <= 0) return { lastChangedAt: null, ageDays: null }
  const changed = new Date(epochSeconds * 1000)
  const ageDays = Math.max(0, Math.floor((Date.now() - changed.getTime()) / 86_400_000))
  return { lastChangedAt: changed.toISOString(), ageDays }
}

function extractRelativeImports(file, text) {
  const results = []
  const importPattern = /(?:from\s*|import\s*\(|require\s*\()\s*['"]([^'"]+)['"]/g
  for (const match of text.matchAll(importPattern)) {
    const spec = match[1]
    if (!spec.startsWith('.')) continue
    const base = slash(normalize(resolve('/', dirname(file), spec)).replace(/^\/+/, ''))
    results.push(base)
  }
  return results
}

function importMatchesTool(importPath, toolFile) {
  if (importPath === toolFile) return true
  if (importPath === toolFile.replace(/\.[^.]+$/, '')) return true
  if (toolFile.startsWith(`${importPath}/index.`)) return true
  return false
}

function strongReferenceFiles(refs) {
  return unique([...refs.packageScripts, ...refs.workflows, ...refs.imports, ...refs.docs, ...refs.source])
}

function classifyGroup({ refs, lifecycle, ageDays, hasReadme, executableFiles }) {
  const automated = refs.packageScripts.length + refs.workflows.length
  const imported = refs.imports.length + refs.source.length
  const documented = refs.docs.length
  const referenced = automated + imported + documented

  if (lifecycle?.kind === 'CAPABILITY_ERA_TRANSITIONAL' && referenced > 0) return 'ACTIVE_BUT_TRANSITIONAL'
  if (automated > 0) return 'ACTIVE_AUTOMATED'
  if (imported > 0) return 'ACTIVE_IMPORTED'
  if (lifecycle?.kind === 'MILESTONE_SCOPED' && referenced === 0) return 'RETIREMENT_CANDIDATE'
  if (documented > 0 || hasReadme) return lifecycle?.kind === 'MILESTONE_SCOPED' ? 'MANUAL_MILESTONE' : 'MANUAL_ENTRYPOINT'
  if (lifecycle?.kind === 'CAPABILITY_ERA_TRANSITIONAL') return 'RETIREMENT_CANDIDATE'
  if (typeof ageDays === 'number' && ageDays >= STALE_DAYS) return 'RETIREMENT_CANDIDATE'
  if (executableFiles > 0) return 'ORPHAN_CANDIDATE'
  return 'UNREFERENCED_SUPPORT_DATA'
}

function unique(values) {
  return [...new Set(values.filter(Boolean))].sort()
}

export function compileToolingReality(root) {
  // Excludes temporary/ (donor source snapshots) before any per-file read: isTextCarrier below
  // matches by extension only, so without this exclusion it would read the content of every
  // .md/.json/.ts/.js/etc. file under the 736k+-file donor corpus (see tools/reality-atlas/
  // lib.mjs's compileReality for the identical fix and full rationale) and then run the
  // carriers x executableToolFiles nested loop below over that inflated set -- the actual reason
  // `node tools/reality-atlas/check.mjs` never used to finish. No donor snapshot file is ever a
  // legitimate tool-carrier reference by construction.
  const tracked = gitLines(root, ['ls-files']).filter((file) => !file.startsWith('temporary/'))
  const toolFiles = tracked.filter((file) => file.startsWith('tools/'))
  const executableToolFiles = toolFiles.filter(isExecutableTool)
  const carrierFiles = tracked.filter(isTextCarrier)
  const carriers = carrierFiles.map((file) => ({ file, text: safeRead(root, file) })).filter((entry) => entry.text)

  const refsByGroup = new Map()
  const ensureRefs = (group) => {
    if (!refsByGroup.has(group)) refsByGroup.set(group, { packageScripts: [], workflows: [], imports: [], docs: [], source: [], metadata: [] })
    return refsByGroup.get(group)
  }

  for (const carrier of carriers) {
    const relativeImports = extractRelativeImports(carrier.file, carrier.text)
    const carrierGroup = topGroup(carrier.file)
    for (const toolFile of executableToolFiles) {
      if (carrier.file === toolFile) continue
      const group = topGroup(toolFile)
      if (!group || carrierGroup === group) continue
      const refs = ensureRefs(group)
      const exactPath = carrier.text.includes(toolFile)
      const groupPath = carrier.text.includes(`${group}/`)
      const relativeImport = relativeImports.some((candidate) => importMatchesTool(candidate, toolFile))
      if (!exactPath && !groupPath && !relativeImport) continue

      if (carrier.file === 'package.json' || /\/package\.json$/.test(carrier.file)) refs.packageScripts.push(carrier.file)
      else if (carrier.file.startsWith('.github/workflows/') || carrier.file.startsWith('.github/actions/')) refs.workflows.push(carrier.file)
      else if (relativeImport) refs.imports.push(carrier.file)
      else if (carrier.file.startsWith('docs/') || carrier.file.endsWith('.md') || carrier.file === 'README.md' || carrier.file === 'AGENTS.md') refs.docs.push(carrier.file)
      else if (isSourceCaller(carrier.file)) refs.source.push(carrier.file)
      else refs.metadata.push(carrier.file)
    }
  }

  const groups = unique(toolFiles.map(topGroup)).map((group) => {
    const files = toolFiles.filter((file) => topGroup(file) === group)
    const executables = files.filter(isExecutableTool)
    const refs0 = refsByGroup.get(group) ?? { packageScripts: [], workflows: [], imports: [], docs: [], source: [], metadata: [] }
    const refs = Object.fromEntries(Object.entries(refs0).map(([key, value]) => [key, unique(value)]))
    const lifecycle = lifecycleFor(group)
    const { lastChangedAt, ageDays } = lastChange(root, group)
    const hasReadme = files.some((file) => /\/README\.md$/i.test(file))
    const hasTests = files.some((file) => /(?:^|\/)(?:tests?|__tests__)(?:\/|$)|\.(?:test|spec)\.[cm]?[jt]sx?$/.test(file))
    const status = classifyGroup({ refs, lifecycle, ageDays, hasReadme, executableFiles: executables.length })
    const referenceFiles = strongReferenceFiles(refs)

    return {
      path: group,
      status,
      lifecycleKind: lifecycle?.kind ?? null,
      lifecycleNote: lifecycle?.note ?? null,
      trackedFiles: files.length,
      executableFiles: executables.length,
      hasReadme,
      hasTests,
      lastChangedAt,
      ageDays,
      references: refs,
      referenceFiles,
      referenceCount: referenceFiles.length,
      weakMetadataReferences: refs.metadata,
      weakMetadataReferenceCount: refs.metadata.length,
      sampleExecutables: executables.slice(0, 10),
    }
  }).sort((a, b) => a.path.localeCompare(b.path))

  const rootLegacySurfaces = ROOT_LEGACY_RULES.map((rule) => {
    const files = tracked.filter((file) => file === rule.path || file.startsWith(`${rule.path}/`))
    return files.length ? { ...rule, trackedFiles: files.length, sampleFiles: files.slice(0, 12) } : null
  }).filter(Boolean)

  const reviewStatuses = new Set(['ORPHAN_CANDIDATE', 'RETIREMENT_CANDIDATE'])
  const reviewCandidates = groups.filter((group) => reviewStatuses.has(group.status))
  const transitional = groups.filter((group) => group.status === 'ACTIVE_BUT_TRANSITIONAL')
  const statusCounts = Object.fromEntries(unique(groups.map((group) => group.status)).map((status) => [status, groups.filter((group) => group.status === status).length]))

  return {
    source: 'observed external tooling references and Git history; generated advisory projection, never deletion authority',
    staleThresholdDays: STALE_DAYS,
    groups,
    reviewCandidates,
    transitional,
    rootLegacySurfaces,
    stats: {
      trackedToolFiles: toolFiles.length,
      executableToolFiles: executableToolFiles.length,
      groups: groups.length,
      reviewCandidates: reviewCandidates.length,
      transitionalGroups: transitional.length,
      rootLegacySurfaces: rootLegacySurfaces.length,
      rootLegacyFiles: rootLegacySurfaces.reduce((sum, entry) => sum + entry.trackedFiles, 0),
      statusCounts,
    },
  }
}

export function validateToolingReality(tooling) {
  const errors = []
  if (!tooling || !Array.isArray(tooling.groups)) errors.push('tooling reality groups must be an array')
  if (!tooling?.stats || typeof tooling.stats.groups !== 'number') errors.push('tooling reality stats are required')
  if (!Array.isArray(tooling?.rootLegacySurfaces)) errors.push('tooling rootLegacySurfaces must be an array')
  if (Array.isArray(tooling?.groups)) {
    const paths = new Set()
    for (const group of tooling.groups) {
      if (!group?.path?.startsWith('tools')) errors.push(`invalid tooling group path: ${group?.path ?? '<missing>'}`)
      else if (paths.has(group.path)) errors.push(`duplicate tooling group: ${group.path}`)
      else paths.add(group.path)
      if (!group?.status) errors.push(`tooling group ${group?.path ?? '<missing>'} is missing status`)
    }
  }
  for (const surface of tooling?.rootLegacySurfaces ?? []) {
    errors.push(`retired root tooling surface must not exist: ${surface.path} (${surface.id}) files=${surface.trackedFiles}`)
  }
  return errors
}
