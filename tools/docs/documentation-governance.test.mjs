import test from 'node:test'
import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import {
  architectureOwnerIdFromPath,
  architectureResponsibilityDirs,
  canonicalArchitectureOwners,
  documentationRouterPaths,
  documentationTopLevelEntries,
  expectedGeneratedDocumentTarget,
  forbiddenDocumentationRoots,
  generatedDocumentKinds,
  isCanonicalArchitectureOwner,
  isValidGeneratedDocumentTarget,
} from './architecture-registry.mjs'
import { compileAtlas, docNodeId, isSemanticAtlasEdge, validateAtlas } from '../docs-atlas/lib.mjs'

function assertUnique(values, label) {
  assert.equal(new Set(values).size, values.length, `${label} must not contain duplicates`)
}

function assertSemanticMarkers(path, markers) {
  const text = readFileSync(path, 'utf8')
  for (const marker of markers) {
    assert.match(text, marker, `${path} missing canonical semantic boundary: ${marker}`)
  }
}

function semanticEdgeKey(edge) {
  return `${edge.source}|${edge.type}|${edge.target}`
}

test('documentation governance lists are unique and internally coherent', () => {
  assertUnique(documentationTopLevelEntries, 'documentationTopLevelEntries')
  assertUnique(documentationRouterPaths, 'documentationRouterPaths')
  assertUnique(forbiddenDocumentationRoots, 'forbiddenDocumentationRoots')
  assertUnique(architectureResponsibilityDirs, 'architectureResponsibilityDirs')
  assertUnique(canonicalArchitectureOwners, 'canonicalArchitectureOwners')
  assertUnique(generatedDocumentKinds, 'generatedDocumentKinds')

  for (const path of documentationRouterPaths) assert.match(path, /^docs\//)
  for (const path of canonicalArchitectureOwners) assert.match(path, /^docs\/architecture\//)
  for (const owner of canonicalArchitectureOwners) {
    for (const forbidden of forbiddenDocumentationRoots) {
      assert.equal(owner.startsWith(forbidden), false, `${owner} must not live under forbidden root ${forbidden}`)
    }
  }
})

test('canonical architecture owners are direct children of approved responsibility directories', () => {
  const ids = []
  for (const owner of canonicalArchitectureOwners) {
    const rel = owner.slice('docs/architecture/'.length)
    const parts = rel.split('/')
    assert.equal(parts.length, 2, `${owner} must be a direct responsibility child`)
    assert.ok(architectureResponsibilityDirs.includes(parts[0]), `${owner} has unapproved responsibility ${parts[0]}`)
    assert.equal(isCanonicalArchitectureOwner(owner), true)
    ids.push(architectureOwnerIdFromPath(owner))
  }
  assertUnique(ids, 'canonical architecture owner ids')
})

test('document generator taxonomy is fail-closed and direct-child only', () => {
  assert.equal(isValidGeneratedDocumentTarget('architecture-owner', 'docs/architecture/foundation/example-owner.md'), true)
  assert.equal(isValidGeneratedDocumentTarget('architecture-owner', 'docs/architecture/foundation/nested/example-owner.md'), false)
  assert.equal(isValidGeneratedDocumentTarget('architecture-owner', 'docs/architecture/constitution/example-owner.md'), false)
  assert.equal(isValidGeneratedDocumentTarget('architecture-owner', 'docs/architecture/foundation/ExampleOwner.md'), false)

  assert.equal(isValidGeneratedDocumentTarget('blueprint', 'docs/blueprints/example-flow.md'), true)
  assert.equal(isValidGeneratedDocumentTarget('blueprint', 'docs/blueprints/nested/example-flow.md'), false)
  assert.equal(isValidGeneratedDocumentTarget('decision', 'docs/decisions/0016-example-decision.md'), true)
  assert.equal(isValidGeneratedDocumentTarget('decision', 'docs/decisions/example-decision.md'), false)
  assert.equal(isValidGeneratedDocumentTarget('guide', 'docs/guides/example-guide.md'), true)
  assert.equal(isValidGeneratedDocumentTarget('guide', 'docs/runbooks/example-guide.md'), false)
  assert.equal(isValidGeneratedDocumentTarget('reference', 'docs/references/example-reference.md'), true)
  assert.equal(isValidGeneratedDocumentTarget('reference', 'docs/frontend/example-reference.md'), false)
  assert.equal(isValidGeneratedDocumentTarget('unknown', 'docs/guides/example.md'), false)
})

test('every generated document kind exposes a human-readable expected target', () => {
  for (const kind of generatedDocumentKinds) {
    const expected = expectedGeneratedDocumentTarget(kind)
    assert.equal(typeof expected, 'string')
    assert.ok(expected.length > 10)
    assert.notEqual(expected, '<unsupported document kind>')
  }
})

test('System Atlas is a valid rebuildable projection over the governed docs tree', () => {
  const atlas = compileAtlas(process.cwd())
  assert.deepEqual(validateAtlas(atlas), [])
  assert.match(atlas.sourceOfTruth, /generated projection/i)
  assert.ok(atlas.stats.documents > 0)
  assert.equal(atlas.stats.architectureOwners, canonicalArchitectureOwners.length)
  assert.equal(atlas.stats.semanticOwnerCoverage, 100)
  assert.equal(atlas.stats.ownersWithSemanticEdges, canonicalArchitectureOwners.length)
  assert.ok(atlas.stats.semanticEdges >= canonicalArchitectureOwners.length)

  const representedPaths = new Set(
    atlas.nodes.filter((node) => node.id.startsWith('doc:')).map((node) => node.path),
  )
  for (const owner of canonicalArchitectureOwners) {
    assert.equal(representedPaths.has(owner), true, `System Atlas missing canonical owner ${owner}`)
  }
  assert.equal(representedPaths.has('apps/ui/public/docs-atlas.json'), false)
})

test('every canonical architecture owner participates in at least one typed semantic relation', () => {
  const atlas = compileAtlas(process.cwd())
  const semanticEdges = atlas.edges.filter(isSemanticAtlasEdge)
  const connected = new Set()
  for (const edge of semanticEdges) {
    connected.add(edge.source)
    connected.add(edge.target)
  }
  for (const owner of canonicalArchitectureOwners) {
    assert.equal(connected.has(docNodeId(owner)), true, `semantic Atlas orphan owner: ${owner}`)
  }
})

test('System Atlas preserves the canonical Chronica dependency spine', () => {
  const atlas = compileAtlas(process.cwd())
  const keys = new Set(atlas.edges.filter(isSemanticAtlasEdge).map(semanticEdgeKey))
  const critical = [
    ['docs/architecture/constitution/NORTH-STAR.md', 'GOVERNS', 'docs/architecture/foundation/system-model.md'],
    ['docs/architecture/foundation/capability-resolution.md', 'DERIVES_FROM', 'docs/architecture/foundation/world.md'],
    ['docs/architecture/foundation/capability-resolution.md', 'CONTRIBUTES_TO_ADMISSION', 'docs/architecture/governance/execution.md'],
    ['docs/architecture/governance/authority.md', 'CONTRIBUTES_TO_ADMISSION', 'docs/architecture/governance/execution.md'],
    ['docs/architecture/governance/normative.md', 'CONTRIBUTES_TO_ADMISSION', 'docs/architecture/governance/execution.md'],
    ['docs/architecture/physical/safety.md', 'CONTRIBUTES_TO_ADMISSION', 'docs/architecture/governance/execution.md'],
    ['docs/architecture/governance/evidence.md', 'SUPPORTS_CANONICALIZATION', 'docs/architecture/foundation/event.md'],
    ['docs/architecture/intelligence/context.md', 'PROJECTS', 'docs/architecture/intelligence/memory.md'],
    ['docs/architecture/intelligence/adaptation.md', 'ACTIVATES_VIA', 'docs/architecture/governance/execution.md'],
    ['docs/architecture/physical/digital-twin.md', 'MODELS', 'docs/architecture/physical/machine.md'],
    ['docs/architecture/operations/fabric.md', 'PRESERVES_ACROSS_BOUNDARY', 'docs/architecture/foundation/relation-binding.md'],
    ['docs/blueprints/digital-organism.md', 'COMPOSES', 'docs/architecture/organism/organism.md'],
  ]
  for (const [source, type, target] of critical) {
    assert.equal(keys.has(`${docNodeId(source)}|${type}|${docNodeId(target)}`), true, `missing typed Atlas relation ${source} ${type} ${target}`)
  }
})

test('owner boundaries stay explicit for capability, organism and Fabric semantics', () => {
  assertSemanticMarkers('docs/architecture/foundation/capability-resolution.md', [
    /no permanent Capability registry\/database\/service\/crate universe/i,
    /Can != Authorized != May != Must != ExecutionAdmission/i,
  ])
  assertSemanticMarkers('docs/architecture/organism/organism.md', [
    /semantic owner/i,
    /end-to-end composition.*blueprints\/digital-organism\.md/is,
  ])
  assertSemanticMarkers('docs/architecture/operations/fabric.md', [
    /boundary-continuity/i,
    /Concrete.*composition.*blueprint/is,
  ])
})

test('composition blueprints preserve canonical execution/truth/disclosure boundaries', () => {
  const checks = new Map([
    ['docs/blueprints/README.md', [
      /ExecutionAdmission != Canonicalization/,
      /CandidateBinding != SelectedBinding/,
      /ContextCompiled != Disclosed/,
    ]],
    ['docs/blueprints/universal-execution.md', [
      /ExecutionAdmission/,
      /CandidateBinding != SelectedBinding/,
      /Candidate Effect Event/,
      /Canonicalization/,
    ]],
    ['docs/blueprints/universal-runtime.md', [
      /ExecutionAdmission/,
      /Selected Binding/,
      /Candidate Effect Event/,
      /ContextEnvelope.*governed effect/s,
    ]],
    ['docs/blueprints/context-compiler.md', [
      /ContextCompiled != Disclosed/,
      /Disclosure \/ ProviderInvocation WorkRequest/,
      /ExecutionAdmission/,
    ]],
    ['docs/blueprints/ai-provider-pipeline.md', [
      /ProviderInvocation WorkRequest/,
      /ProviderOutput != CanonicalFact/,
      /downstream WorkRequest/i,
      /Canonicalization/,
    ]],
    ['docs/blueprints/digital-organism.md', [
      /SelectedProposal != WorkRequest/,
      /Activation WorkRequest/,
      /qualified != active/,
      /ContextCompiled != Disclosed/,
      /Canonicalization/,
    ]],
    ['docs/blueprints/holding-black-box.md', [
      /ProjectionEligible != Disclosed/,
      /Disclosure or ProviderInvocation WorkRequest/,
      /Canonicalization/,
    ]],
    ['docs/blueprints/machine-control.md', [
      /Can\(\.\.\.\) != Authorized\(\.\.\.\) != May\/Must\(\.\.\.\)/,
      /Selected Industrial Binding/,
      /ExecutionAdmission=ALLOW != E-stop bypass permission/,
      /Candidate Effect Event/,
    ]],
    ['docs/blueprints/digital-twin-loop.md', [
      /SimulationSuccess != Can\(\.\.\.\)/,
      /Activation WorkRequest/,
      /Selected Industrial Binding/,
      /Canonicalization/,
    ]],
    ['docs/blueprints/ops-absorption.md', [
      /Capability Resolution \/ Can\(\.\.\.\)/,
      /MCP tool visible != ExecutionAdmission=ALLOW/,
      /ReconciledEffect != CanonicalizedEvent/,
    ]],
  ])

  for (const [path, markers] of checks) assertSemanticMarkers(path, markers)
})
