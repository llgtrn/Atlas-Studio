#!/usr/bin/env node

import { readFileSync } from 'node:fs'
import { dirname, resolve } from 'node:path'
import { fileURLToPath } from 'node:url'
import { canonicalArchitectureOwners } from './architecture-registry.mjs'
import { compileAtlas, docNodeId, isSemanticAtlasEdge, validateAtlas } from '../docs-atlas/lib.mjs'

const toolDir = dirname(fileURLToPath(import.meta.url))
const root = resolve(toolDir, '../..')
const read = (path) => readFileSync(resolve(root, path), 'utf8')
const atlas = compileAtlas(root)
const atlasErrors = validateAtlas(atlas)
const nodeByPath = new Map(atlas.nodes.filter((node) => node.path).map((node) => [node.path, node]))
const semanticEdges = atlas.edges.filter(isSemanticAtlasEdge)
const semanticKeys = new Set(semanticEdges.map((edge) => `${edge.source}|${edge.type}|${edge.target}`))

function relation(source, type, target) {
  return semanticKeys.has(`${docNodeId(source)}|${type}|${docNodeId(target)}`)
}

function allOwners(predicate) {
  return canonicalArchitectureOwners.every((path) => predicate(nodeByPath.get(path), path))
}

function contractsFor(path) {
  return nodeByPath.get(path)?.contractIds ?? []
}

function hasAll(text, patterns) {
  return patterns.every((pattern) => pattern.test(text))
}

const index = read('docs/INDEX.md')
const learning = read('docs/learning.md')
const architectureReadme = read('docs/architecture/README.md')
const northStar = read('docs/architecture/constitution/NORTH-STAR.md')
const normative = read('docs/architecture/governance/normative.md')
const machine = read('docs/architecture/physical/machine.md')
const safety = read('docs/architecture/physical/safety.md')
const organism = read('docs/architecture/organism/organism.md')
const fabric = read('docs/architecture/operations/fabric.md')
const template = read('docs/TEMPLATE.md')
const atlasGraphUi = read('apps/ui/src/control-plane/components/DocsAtlasGraph.tsx')
const atlasPageUi = read('apps/ui/src/control-plane/pages/DocsAtlasPage.tsx')

const formalOwners = [
  'docs/architecture/foundation/world.md',
  'docs/architecture/foundation/capability-resolution.md',
  'docs/architecture/governance/authority.md',
  'docs/architecture/governance/normative.md',
  'docs/architecture/governance/execution.md',
  'docs/architecture/intelligence/context.md',
  'docs/architecture/physical/safety.md',
  'docs/architecture/operations/performance.md',
  'docs/architecture/organism/organism.md',
  'docs/architecture/operations/fabric.md',
]

const rubric = [
  {
    id: 'architecture-coherence',
    label: 'Architecture coherence',
    pass: atlasErrors.length === 0
      && relation('docs/architecture/foundation/capability-resolution.md', 'CONTRIBUTES_TO_ADMISSION', 'docs/architecture/governance/execution.md')
      && relation('docs/architecture/governance/authority.md', 'CONTRIBUTES_TO_ADMISSION', 'docs/architecture/governance/execution.md')
      && relation('docs/architecture/governance/normative.md', 'CONTRIBUTES_TO_ADMISSION', 'docs/architecture/governance/execution.md')
      && relation('docs/architecture/governance/evidence.md', 'SUPPORTS_CANONICALIZATION', 'docs/architecture/foundation/event.md'),
    evidence: `atlasErrors=${atlasErrors.length}; canonical execution/canonicalization spine typed`,
  },
  {
    id: 'human-readability',
    label: 'Human readability',
    pass: allOwners((node) => Boolean(node?.title && node?.summary && node?.status && node?.headings?.length)),
    evidence: 'every canonical owner exposes title + natural-language summary + status + structured headings',
  },
  {
    id: 'canonical-ownership',
    label: 'Canonical ownership',
    pass: atlas.stats.architectureOwners === canonicalArchitectureOwners.length
      && new Set(canonicalArchitectureOwners).size === canonicalArchitectureOwners.length,
    evidence: `${atlas.stats.architectureOwners}/${canonicalArchitectureOwners.length} registered canonical owners`,
  },
  {
    id: 'natural-language-semantics',
    label: 'Natural-language explanation',
    pass: allOwners((node) => Boolean(node?.summary && !node.summary.trim().startsWith('{')))
      && /human-readable map of canonical architecture owners/i.test(architectureReadme)
      && /PROSE EXPLAINS/i.test(template),
    evidence: 'owner prose remains primary; machine metadata is projection annotation',
  },
  {
    id: 'formal-invariant-integration',
    label: 'Math / invariant integration',
    pass: formalOwners.every((path) => contractsFor(path).some((id) => id.startsWith('INV-')))
      && contractsFor('docs/architecture/foundation/world.md').some((id) => id.startsWith('ARCH-EQ-'))
      && contractsFor('docs/architecture/governance/execution.md').some((id) => id.startsWith('ARCH-EQ-')),
    evidence: `${formalOwners.length}/${formalOwners.length} critical formal owners declare hard invariants`,
  },
  {
    id: 'legal-tax-integration',
    label: 'Legal / tax integration',
    pass: hasAll(normative, [/SourceArtifact/, /NormativeInterpretation/, /Fiscal semantics/i, /UNRESOLVED/, /effective-time/i])
      && hasAll(northStar, [/Law, contracts and tax are temporal normative semantics/i, /Fiscal semantics derive from economic events/i]),
    evidence: 'source/provenance + jurisdiction/effective-time + unresolved + fiscal projection semantics present',
  },
  {
    id: 'machine-safety-model',
    label: 'Machine / safety model',
    pass: hasAll(machine, [/OperationalTelemetryObservation != CanonicalState/, /Local certified controllers\/interlocks retain veto power/i, /Can\(machine,action,t\)/])
      && hasAll(safety, [/independent local hard safety/i, /ExecutionAdmission=ALLOW != hard-safety guarantee/, /risk classes/i]),
    evidence: 'semantic machine layer + freshness-bounded telemetry + independent non-bypassable local safety',
  },
  {
    id: 'docs-consistency',
    label: 'Docs consistency of format',
    pass: hasAll(template, [/ONE SEMANTIC RESPONSIBILITY -> ONE CANONICAL OWNER/, /BLUEPRINT != NEW SUBSYSTEM/, /Canonical vocabulary/, /100% owner participation/i])
      && /ACTIVE SEMANTIC OWNER/.test(organism)
      && /ACTIVE BOUNDARY-CONTINUITY OWNER/.test(fabric)
      && /boundary-continuity invariants across providers\/adapters\/Ops\/machines/i.test(index)
      && /Capability Resolution \/ derived `Can\(\.\.\.\)`/i.test(learning),
    evidence: 'taxonomy/vocabulary/owner-vs-blueprint/Atlas rules align across INDEX, TEMPLATE, learning and canonical owners',
  },
  {
    id: 'machine-readable-graph',
    label: 'Machine-readable graph',
    pass: atlas.stats.semanticOwnerCoverage === 100
      && atlas.stats.ownersWithSemanticEdges === canonicalArchitectureOwners.length
      && semanticEdges.length >= canonicalArchitectureOwners.length,
    evidence: `${atlas.stats.ownersWithSemanticEdges}/${canonicalArchitectureOwners.length} owners connected; ${semanticEdges.length} typed semantic edges`,
  },
  {
    id: 'visual-system-atlas',
    label: 'Visual System Atlas',
    pass: hasAll(atlasGraphUi, [/function MiniMap/, /Semantic/, /Contracts \+ runtime/, /Search/, /ZoomIn/, /Inspector/, /edge\.label/])
      && hasAll(atlasPageUi, [/semanticOwnerCoverage/, /semanticEdges/, /typed architecture relations/i]),
    evidence: 'semantic/evidence/all views + search + zoom + minimap + inspector + edge labels + coverage telemetry',
  },
]

let failures = 0
for (const item of rubric) {
  if (!item.pass) failures += 1
  console.log(`${item.pass ? 'PASS' : 'FAIL'} ${item.label}: ${item.evidence}`)
}

const score = rubric.length - failures
console.log(`Chronica documentation scorecard: ${score}/${rubric.length}`)

if (failures) process.exit(1)
