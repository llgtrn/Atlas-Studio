import { execFileSync } from 'node:child_process'
import { readFileSync } from 'node:fs'
import { dirname, resolve as resolveFilesystemPath, posix as path } from 'node:path'
import { parse as parseYaml } from 'yaml'
import { parseContractsFromText } from '../ci/architecture-contract-lib.mjs'
import { canonicalArchitectureOwners } from '../docs/architecture-registry.mjs'

export const ATLAS_SCHEMA_VERSION = 2
export const ATLAS_SOURCE_OF_TRUTH = 'tracked Markdown under docs/; Atlas is a generated projection, never canonical truth'

const canonicalOwnerSet = new Set(canonicalArchitectureOwners)

function unique(values) {
  return [...new Set(values.filter(Boolean))]
}

function cleanInlineMarkdown(value) {
  return value
    .replace(/`([^`]+)`/g, '$1')
    .replace(/\[([^\]]+)\]\([^\)]+\)/g, '$1')
    .replace(/[*_~]/g, '')
    .replace(/\s+/g, ' ')
    .trim()
}

export function trackedMarkdownDocs(root = process.cwd()) {
  const output = execFileSync('git', ['ls-files', 'docs'], { cwd: root, encoding: 'utf8' })
  return output
    .split('\n')
    .map((line) => line.trim())
    .filter((line) => line.endsWith('.md'))
    .sort()
}

export function docNodeId(file) {
  return `doc:${file.replace(/^docs\//, '').replace(/\.md$/i, '')}`
}

export function contractNodeId(id) {
  return `contract:${id}`
}

export function runtimeNodeId(owner) {
  return `runtime:${owner.replace(/\/$/, '')}`
}

export function parseFrontmatter(text, source = '<memory>') {
  if (!text.startsWith('---\n')) return { attributes: {}, body: text }
  const end = text.indexOf('\n---\n', 4)
  if (end < 0) return { attributes: {}, body: text }
  const raw = text.slice(4, end)
  let attributes = {}
  try {
    attributes = parseYaml(raw) ?? {}
  } catch (error) {
    throw new Error(`${source}: invalid YAML frontmatter: ${error.message}`)
  }
  return { attributes, body: text.slice(end + 5) }
}

function titleFromBody(body, file) {
  const match = body.match(/^#\s+(.+)$/m)
  return cleanInlineMarkdown(match?.[1] ?? path.basename(file, '.md'))
}

function statusFromBody(body, attributes) {
  if (typeof attributes.status === 'string' && attributes.status.trim()) return attributes.status.trim()
  const match = body.match(/^Status:\s*(?:\*\*)?(.+?)(?:\*\*)?\s*$/mi)
  return cleanInlineMarkdown(match?.[1] ?? '') || null
}

function summaryFromBody(body) {
  const withoutFences = body.replace(/```[\s\S]*?```/g, '')
  for (const block of withoutFences.split(/\n\s*\n/)) {
    const trimmed = block.trim()
    if (!trimmed || trimmed.startsWith('#') || /^Status:/i.test(trimmed)) continue
    if (/^(?:[-*+] |\d+\. )/.test(trimmed)) continue
    const cleaned = cleanInlineMarkdown(trimmed)
    if (cleaned.length >= 24) return cleaned.slice(0, 360)
  }
  return ''
}

function headingsFromBody(body) {
  return [...body.matchAll(/^(#{2,6})\s+(.+)$/gm)].map((match) => ({
    depth: match[1].length,
    title: cleanInlineMarkdown(match[2]),
  }))
}

function familyForPath(file) {
  if (file.startsWith('docs/architecture/')) return 'architecture'
  if (file.startsWith('docs/blueprints/')) return 'blueprint'
  if (file.startsWith('docs/frontend/')) return 'frontend'
  if (file.startsWith('docs/ops_production/')) return 'ops'
  if (file.startsWith('docs/guides/')) return 'guide'
  if (file.startsWith('docs/references/')) return 'reference'
  if (file.startsWith('docs/decisions/')) return 'decision'
  if (file === 'docs/learning.md') return 'learning'
  if (['docs/INDEX.md', 'docs/README.md'].includes(file)) return 'router'
  if (file === 'docs/TEMPLATE.md') return 'template'
  return 'other'
}

function groupForPath(file) {
  const parts = file.split('/')
  if (parts[1] === 'architecture') return parts[2] ? `architecture/${parts[2]}` : 'architecture'
  if (parts[1] === 'frontend') return parts[2] ? `frontend/${parts[2]}` : 'frontend'
  if (parts[1] === 'ops_production') return parts[2] ? `ops/${parts[2]}` : 'ops'
  return familyForPath(file)
}

function kindForPath(file) {
  if (canonicalOwnerSet.has(file)) return 'architecture-owner'
  if (file === 'docs/architecture/README.md' || file.endsWith('/README.md') || file === 'docs/INDEX.md') return 'router'
  if (file.startsWith('docs/blueprints/')) return 'blueprint'
  if (file.startsWith('docs/decisions/')) return 'decision'
  if (file.startsWith('docs/guides/')) return 'guide'
  if (file.startsWith('docs/references/')) return 'reference'
  if (file.startsWith('docs/frontend/')) return 'frontend-doc'
  if (file.startsWith('docs/ops_production/')) return 'ops-doc'
  if (file === 'docs/learning.md') return 'learning-doc'
  if (file === 'docs/TEMPLATE.md') return 'template'
  return 'doc'
}

function atlasMetadata(attributes) {
  const atlas = attributes?.atlas
  return atlas && typeof atlas === 'object' && !Array.isArray(atlas) ? atlas : {}
}

function stringList(value) {
  if (!Array.isArray(value)) return []
  return unique(value.filter((item) => typeof item === 'string').map((item) => item.trim()).filter(Boolean))
}

function stringValue(value) {
  return typeof value === 'string' && value.trim() ? value.trim() : null
}

function presentationMetadata(atlas) {
  const raw = atlas?.presentation
  if (!raw || typeof raw !== 'object' || Array.isArray(raw)) return null
  const presentation = {
    semanticOwners: stringList(raw.semantic_owners),
    runtimeOwners: stringList(raw.runtime_owners),
    surfaces: stringList(raw.surfaces),
    vocabulary: stringValue(raw.vocabulary),
    localization: stringValue(raw.localization),
    stateProjection: stringValue(raw.state_projection),
    projectionContract: stringValue(raw.projection_contract),
    canonicality: stringValue(raw.canonicality),
  }
  const hasValue = presentation.semanticOwners.length
    || presentation.runtimeOwners.length
    || presentation.surfaces.length
    || presentation.vocabulary
    || presentation.localization
    || presentation.stateProjection
    || presentation.projectionContract
    || presentation.canonicality
  return hasValue ? presentation : null
}

export function parseDocumentText(file, text) {
  const { attributes, body } = parseFrontmatter(text, file)
  const atlas = atlasMetadata(attributes)
  const contracts = parseContractsFromText(text, file).map(({ __source, ...contract }) => contract)
  return {
    id: typeof atlas.id === 'string' && atlas.id.trim() ? atlas.id.trim() : docNodeId(file),
    title: typeof atlas.title === 'string' && atlas.title.trim() ? atlas.title.trim() : titleFromBody(body, file),
    kind: typeof atlas.kind === 'string' && atlas.kind.trim() ? atlas.kind.trim() : kindForPath(file),
    family: typeof atlas.family === 'string' && atlas.family.trim() ? atlas.family.trim() : familyForPath(file),
    group: typeof atlas.group === 'string' && atlas.group.trim() ? atlas.group.trim() : groupForPath(file),
    status: statusFromBody(body, attributes),
    summary: typeof atlas.summary === 'string' && atlas.summary.trim() ? atlas.summary.trim() : summaryFromBody(body),
    path: file,
    headings: headingsFromBody(body),
    contractIds: contracts.map((contract) => contract.id).filter(Boolean),
    runtimeOwners: unique(contracts.flatMap((contract) => contract.runtime_owner ?? [])),
    presentation: presentationMetadata(atlas),
    contracts,
    explicitEdges: Array.isArray(atlas.edges) ? atlas.edges : [],
    declaredRelations: Array.isArray(atlas.relations) ? atlas.relations : [],
    body,
  }
}

function referenceCandidates(text) {
  const values = []
  for (const match of text.matchAll(/\[[^\]]*\]\(([^)]+\.md(?:#[^)]+)?)\)/g)) values.push(match[1])
  for (const match of text.matchAll(/`([^`\n]+\.md(?:#[^`\n]+)?)`/g)) values.push(match[1])
  for (const match of text.matchAll(/\b(docs\/[A-Za-z0-9_./-]+\.md)\b/g)) values.push(match[1])
  return unique(values)
}

function normalizeReference(value) {
  if (typeof value !== 'string') return null
  const trimmed = value.trim().split('#')[0].split('?')[0]
  if (!trimmed || /^(?:https?:|mailto:|#)/i.test(trimmed)) return null
  return trimmed.replace(/^\.\//, '')
}

export function resolveDocReference(sourceFile, value, docPaths, basenameIndex) {
  const normalized = normalizeReference(value)
  if (!normalized) return null
  let candidate
  if (normalized.startsWith('/docs/')) candidate = normalized.slice(1)
  else if (normalized.startsWith('docs/')) candidate = normalized
  else candidate = path.normalize(path.join(dirname(sourceFile), normalized))
  if (docPaths.has(candidate)) return candidate

  const basename = path.basename(normalized)
  const matches = basenameIndex.get(basename) ?? []
  return matches.length === 1 ? matches[0] : null
}

function edgeId(source, target, type) {
  return `${type}:${source}->${target}`
}

function normalizeEdgeType(value, fallback = 'REFERENCES') {
  if (typeof value !== 'string' || !value.trim()) return fallback
  return value.trim().toUpperCase().replace(/[^A-Z0-9]+/g, '_').replace(/^_+|_+$/g, '') || fallback
}

function resolveExplicitTarget(value, sourceFile, docsByPath, docsById, basenameIndex) {
  if (typeof value !== 'string' || !value.trim()) return null
  if (docsById.has(value.trim())) return value.trim()
  const resolvedPath = resolveDocReference(sourceFile, value.trim(), new Set(docsByPath.keys()), basenameIndex)
  return resolvedPath ? docsByPath.get(resolvedPath)?.id ?? null : null
}

export function isSemanticAtlasEdge(edge) {
  return edge?.inferredBy === 'frontmatter' || edge?.inferredBy === 'atlas-relation-map'
}

export function compileAtlas(root = process.cwd()) {
  const files = trackedMarkdownDocs(root)
  const parsed = files.map((file) => parseDocumentText(file, readFileSync(resolveFilesystemPath(root, file), 'utf8')))
  const docsByPath = new Map(parsed.map((doc) => [doc.path, doc]))
  const docsById = new Map(parsed.map((doc) => [doc.id, doc]))
  const basenameIndex = new Map()
  for (const file of files) {
    const base = path.basename(file)
    basenameIndex.set(base, [...(basenameIndex.get(base) ?? []), file])
  }

  const nodes = parsed.map(({ body, explicitEdges, declaredRelations, contracts, ...node }) => node)
  const edges = []
  const diagnostics = []
  const seenEdges = new Set()
  const addEdge = (edge) => {
    const key = edgeId(edge.source, edge.target, edge.type)
    if (seenEdges.has(key)) return
    seenEdges.add(key)
    edges.push({ id: key, ...edge })
  }

  for (const doc of parsed) {
    for (const ref of referenceCandidates(doc.body)) {
      const targetPath = resolveDocReference(doc.path, ref, new Set(files), basenameIndex)
      const target = targetPath ? docsByPath.get(targetPath) : null
      if (target && target.id !== doc.id) {
        addEdge({ source: doc.id, target: target.id, type: 'REFERENCES', inferredBy: 'markdown-reference' })
      }
    }

    for (const rawEdge of doc.explicitEdges) {
      if (!rawEdge || typeof rawEdge !== 'object') continue
      const target = resolveExplicitTarget(rawEdge.to, doc.path, docsByPath, docsById, basenameIndex)
      if (!target) {
        diagnostics.push({ level: 'error', source: doc.path, message: `atlas edge target not found: ${String(rawEdge.to)}` })
        continue
      }
      addEdge({
        source: doc.id,
        target,
        type: normalizeEdgeType(rawEdge.type, 'RELATES_TO'),
        label: typeof rawEdge.label === 'string' ? rawEdge.label : undefined,
        inferredBy: 'frontmatter',
      })
    }

    for (const contract of doc.contracts) {
      if (!contract.id) continue
      const contractId = contractNodeId(contract.id)
      if (!nodes.some((node) => node.id === contractId)) {
        const description = contract.equation ?? contract.predicate ?? contract.metric ?? ''
        nodes.push({
          id: contractId,
          title: contract.id,
          kind: 'contract',
          family: 'contract',
          group: contract.kind ?? 'contract',
          status: contract.gate ?? contract.severity ?? null,
          summary: String(description),
          path: doc.path,
          sourceDocId: doc.id,
          contractKind: contract.kind,
          severity: contract.severity,
          gate: contract.gate ?? null,
          headings: [],
          contractIds: [],
          runtimeOwners: contract.runtime_owner ?? [],
        })
      }
      addEdge({ source: doc.id, target: contractId, type: 'DECLARES', inferredBy: 'chronica-contract' })

      for (const owner of contract.runtime_owner ?? []) {
        const runtimeId = runtimeNodeId(owner)
        if (!nodes.some((node) => node.id === runtimeId)) {
          nodes.push({
            id: runtimeId,
            title: owner,
            kind: 'runtime-owner',
            family: 'runtime',
            group: owner.split('/').slice(0, 2).join('/'),
            status: 'runtime responsibility',
            summary: 'Runtime responsibility referenced by one or more architecture contracts.',
            path: owner,
            headings: [],
            contractIds: [],
            runtimeOwners: [],
          })
        }
        addEdge({ source: contractId, target: runtimeId, type: 'RUNTIME_OWNER', inferredBy: 'chronica-contract' })
      }
    }
  }

  for (const doc of parsed) {
    for (const rawRelation of doc.declaredRelations) {
      if (!rawRelation || typeof rawRelation !== 'object') continue
      const source = resolveExplicitTarget(rawRelation.from, doc.path, docsByPath, docsById, basenameIndex)
      const target = resolveExplicitTarget(rawRelation.to, doc.path, docsByPath, docsById, basenameIndex)
      if (!source || !target) {
        diagnostics.push({
          level: 'error',
          source: doc.path,
          message: `atlas relation endpoint not found: ${String(rawRelation.from)} -> ${String(rawRelation.to)}`,
        })
        continue
      }
      if (source === target) {
        diagnostics.push({ level: 'error', source: doc.path, message: `atlas relation cannot self-loop: ${String(rawRelation.from)}` })
        continue
      }
      addEdge({
        source,
        target,
        type: normalizeEdgeType(rawRelation.type, 'RELATES_TO'),
        label: typeof rawRelation.label === 'string' ? rawRelation.label : undefined,
        inferredBy: 'atlas-relation-map',
      })
    }
  }

  const contractIds = new Set(nodes.filter((node) => node.kind === 'contract').map((node) => node.id))
  for (const doc of parsed) {
    for (const contract of doc.contracts) {
      if (!contract.id || contract.kind !== 'performance') continue
      for (const invariant of contract.hard_invariants ?? []) {
        const source = contractNodeId(contract.id)
        const target = contractNodeId(invariant)
        if (contractIds.has(target)) addEdge({ source, target, type: 'GUARDED_BY', inferredBy: 'chronica-contract' })
      }
    }
  }

  nodes.sort((a, b) => a.id.localeCompare(b.id))
  edges.sort((a, b) => a.id.localeCompare(b.id))

  const architectureOwnerNodes = parsed.filter((doc) => canonicalOwnerSet.has(doc.path))
  const architectureOwnerIds = new Set(architectureOwnerNodes.map((doc) => doc.id))
  const semanticEdges = edges.filter(isSemanticAtlasEdge)
  const semanticallyConnectedOwners = new Set()
  for (const edge of semanticEdges) {
    if (architectureOwnerIds.has(edge.source)) semanticallyConnectedOwners.add(edge.source)
    if (architectureOwnerIds.has(edge.target)) semanticallyConnectedOwners.add(edge.target)
  }
  const semanticOwnerCoverage = architectureOwnerIds.size
    ? Number(((semanticallyConnectedOwners.size / architectureOwnerIds.size) * 100).toFixed(1))
    : 100

  const atlas = {
    schemaVersion: ATLAS_SCHEMA_VERSION,
    sourceOfTruth: ATLAS_SOURCE_OF_TRUTH,
    nodes,
    edges,
    diagnostics,
    stats: {
      documents: parsed.length,
      architectureOwners: architectureOwnerNodes.length,
      contracts: nodes.filter((node) => node.kind === 'contract').length,
      runtimeOwners: nodes.filter((node) => node.kind === 'runtime-owner').length,
      presentationAnnotatedDocs: parsed.filter((doc) => doc.presentation).length,
      edges: edges.length,
      semanticEdges: semanticEdges.length,
      ownersWithSemanticEdges: semanticallyConnectedOwners.size,
      semanticOwnerCoverage,
    },
  }
  return atlas
}

export function validateAtlas(atlas) {
  const errors = []
  if (atlas?.schemaVersion !== ATLAS_SCHEMA_VERSION) errors.push(`schemaVersion must be ${ATLAS_SCHEMA_VERSION}`)
  if (atlas?.sourceOfTruth !== ATLAS_SOURCE_OF_TRUTH) errors.push('sourceOfTruth must state that Atlas is a generated projection')
  if (!Array.isArray(atlas?.nodes) || !Array.isArray(atlas?.edges)) return [...errors, 'nodes and edges must be arrays']

  const ids = new Set()
  for (const node of atlas.nodes) {
    if (!node?.id || typeof node.id !== 'string') errors.push('node id must be a non-empty string')
    else if (ids.has(node.id)) errors.push(`duplicate node id: ${node.id}`)
    else ids.add(node.id)

    if (node?.presentation) {
      if (!Array.isArray(node.presentation.semanticOwners)) errors.push(`${node.id}: presentation semanticOwners must be an array`)
      if (!Array.isArray(node.presentation.runtimeOwners)) errors.push(`${node.id}: presentation runtimeOwners must be an array`)
      if (!Array.isArray(node.presentation.surfaces)) errors.push(`${node.id}: presentation surfaces must be an array`)
      if (node.presentation.canonicality && node.presentation.canonicality !== 'projection-only') {
        errors.push(`${node.id}: presentation canonicality must be projection-only when declared`)
      }
    }
  }
  for (const edge of atlas.edges) {
    if (!ids.has(edge.source)) errors.push(`${edge.id}: source does not exist: ${edge.source}`)
    if (!ids.has(edge.target)) errors.push(`${edge.id}: target does not exist: ${edge.target}`)
    if (!edge.type || typeof edge.type !== 'string') errors.push(`${edge.id}: edge type is required`)
  }
  for (const diagnostic of atlas.diagnostics ?? []) {
    if (diagnostic.level === 'error') errors.push(`${diagnostic.source}: ${diagnostic.message}`)
  }
  if (atlas?.stats?.architectureOwners && atlas?.stats?.semanticOwnerCoverage !== 100) {
    errors.push(`semantic architecture owner coverage must be 100%, got ${atlas?.stats?.semanticOwnerCoverage ?? 0}%`)
  }
  return errors
}
