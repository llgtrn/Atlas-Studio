import test from 'node:test'
import assert from 'node:assert/strict'
import {
  ATLAS_SCHEMA_VERSION,
  ATLAS_SOURCE_OF_TRUTH,
  compileAtlas,
  contractNodeId,
  docNodeId,
  isSemanticAtlasEdge,
  parseDocumentText,
  parseFrontmatter,
  resolveDocReference,
  runtimeNodeId,
  validateAtlas,
} from './lib.mjs'

test('stable node ids distinguish docs, contracts and runtime owners', () => {
  assert.equal(docNodeId('docs/architecture/foundation/world.md'), 'doc:architecture/foundation/world')
  assert.equal(contractNodeId('INV-WORLD-001'), 'contract:INV-WORLD-001')
  assert.equal(runtimeNodeId('crates/core/'), 'runtime:crates/core')
})

test('frontmatter is optional and atlas metadata does not replace prose', () => {
  const text = `---\nstatus: active\natlas:\n  id: arch.world\n  edges:\n    - to: authority.md\n      type: governed_by\n  relations:\n    - from: world.md\n      to: authority.md\n      type: contributes_to\n---\n# World\n\nStatus: **ACTIVE**\n\nWorld explains durable truth in natural language.\n`
  const { attributes, body } = parseFrontmatter(text, 'fixture.md')
  assert.equal(attributes.atlas.id, 'arch.world')
  assert.ok(body.includes('World explains durable truth'))
  const parsed = parseDocumentText('docs/architecture/README.md', text)
  assert.equal(parsed.declaredRelations.length, 1)
  assert.equal(parsed.declaredRelations[0].type, 'contributes_to')
})

test('document parser extracts human and machine views together', () => {
  const doc = parseDocumentText('docs/architecture/foundation/world.md', `# World\n\nStatus: **ACTIVE ARCHITECTURE OWNER**\n\nWorld is reconstructed from durable admitted history and is canonical truth.\n\n## Purpose\n\nExplain the world.\n\n\`\`\`chronica-contract\n{"id":"INV-WORLD-001","kind":"invariant","owner":"world","severity":"hard","predicate":"replay reconstructs world","runtime_owner":["crates/core/"],"verification":["replay test"]}\n\`\`\`\n`)
  assert.equal(doc.title, 'World')
  assert.equal(doc.family, 'architecture')
  assert.equal(doc.kind, 'architecture-owner')
  assert.equal(doc.contractIds[0], 'INV-WORLD-001')
  assert.deepEqual(doc.runtimeOwners, ['crates/core/'])
  assert.match(doc.summary, /durable admitted history/)
})

test('presentation metadata is projected without becoming canonical or contract runtime ownership', () => {
  const doc = parseDocumentText('docs/frontend/PRODUCT-NORTH-STAR.md', `---\natlas:\n  presentation:\n    semantic_owners:\n      - docs/architecture/foundation/world.md\n      - docs/architecture/governance/execution.md\n    runtime_owners:\n      - apps/ui/\n    surfaces:\n      - feed\n      - inbox\n    vocabulary: semantic-id-first\n    localization: locale-compiled\n    state_projection: RECORD-ANALYZE-ACT\n    projection_contract: PresentationEnvelope\n    canonicality: projection-only\n---\n# Frontend Product North Star\n\nStatus: TARGET UX CONTRACT.\n\nPresentation is a projection of one canonical world, never a competing truth.\n`)

  assert.deepEqual(doc.presentation, {
    semanticOwners: [
      'docs/architecture/foundation/world.md',
      'docs/architecture/governance/execution.md',
    ],
    runtimeOwners: ['apps/ui/'],
    surfaces: ['feed', 'inbox'],
    vocabulary: 'semantic-id-first',
    localization: 'locale-compiled',
    stateProjection: 'RECORD-ANALYZE-ACT',
    projectionContract: 'PresentationEnvelope',
    canonicality: 'projection-only',
  })
  assert.deepEqual(doc.runtimeOwners, [])
})

test('relative and basename-only document references resolve without inventing edges', () => {
  const docs = new Set([
    'docs/architecture/foundation/world.md',
    'docs/architecture/governance/authority.md',
  ])
  const basenames = new Map([
    ['world.md', ['docs/architecture/foundation/world.md']],
    ['authority.md', ['docs/architecture/governance/authority.md']],
  ])
  assert.equal(
    resolveDocReference('docs/architecture/foundation/world.md', '../governance/authority.md', docs, basenames),
    'docs/architecture/governance/authority.md',
  )
  assert.equal(
    resolveDocReference('docs/architecture/foundation/world.md', 'authority.md', docs, basenames),
    'docs/architecture/governance/authority.md',
  )
})

test('semantic edge classification excludes incidental references and implementation edges', () => {
  assert.equal(isSemanticAtlasEdge({ inferredBy: 'atlas-relation-map' }), true)
  assert.equal(isSemanticAtlasEdge({ inferredBy: 'frontmatter' }), true)
  assert.equal(isSemanticAtlasEdge({ inferredBy: 'markdown-reference' }), false)
  assert.equal(isSemanticAtlasEdge({ inferredBy: 'chronica-contract' }), false)
})

test('atlas validation rejects broken typed edges', () => {
  const errors = validateAtlas({
    schemaVersion: ATLAS_SCHEMA_VERSION,
    sourceOfTruth: ATLAS_SOURCE_OF_TRUTH,
    diagnostics: [],
    nodes: [{ id: 'doc:a' }],
    edges: [{ id: 'REFERENCES:doc:a->doc:b', source: 'doc:a', target: 'doc:b', type: 'REFERENCES' }],
  })
  assert.ok(errors.some((error) => error.includes('target does not exist')))
})

test('atlas validation rejects presentation metadata that claims canonicality', () => {
  const errors = validateAtlas({
    schemaVersion: ATLAS_SCHEMA_VERSION,
    sourceOfTruth: ATLAS_SOURCE_OF_TRUTH,
    diagnostics: [],
    nodes: [{
      id: 'doc:frontend',
      presentation: {
        semanticOwners: [],
        runtimeOwners: ['apps/ui/'],
        surfaces: ['feed'],
        canonicality: 'canonical',
      },
    }],
    edges: [],
    stats: { architectureOwners: 0, semanticOwnerCoverage: 100 },
  })
  assert.ok(errors.some((error) => error.includes('presentation canonicality must be projection-only')))
})

test('atlas validation fails closed when canonical owner semantic coverage is incomplete', () => {
  const errors = validateAtlas({
    schemaVersion: ATLAS_SCHEMA_VERSION,
    sourceOfTruth: ATLAS_SOURCE_OF_TRUTH,
    diagnostics: [],
    nodes: [{ id: 'doc:a' }],
    edges: [],
    stats: { architectureOwners: 1, semanticOwnerCoverage: 0 },
  })
  assert.ok(errors.some((error) => error.includes('semantic architecture owner coverage must be 100%')))
})


test('universal connector blueprint is indexed and references physical/fabric owners', () => {
  const atlas = compileAtlas(process.cwd())
  const connectorId = docNodeId('docs/blueprints/universal-physical-digital-connector.md')
  const connector = atlas.nodes.find((node) => node.id === connectorId)
  assert.ok(connector, 'connector blueprint must be indexed')
  assert.equal(connector.kind, 'blueprint')

  const targets = new Set(
    atlas.edges
      .filter((edge) => edge.source === connectorId && edge.type === 'REFERENCES')
      .map((edge) => edge.target),
  )

  for (const path of [
    'docs/architecture/physical/machine.md',
    'docs/architecture/physical/safety.md',
    'docs/architecture/physical/digital-twin.md',
    'docs/architecture/operations/fabric.md',
    'docs/architecture/governance/sovereignty.md',
  ]) {
    assert.ok(targets.has(docNodeId(path)), `connector blueprint must reference ${path}`)
  }
})
