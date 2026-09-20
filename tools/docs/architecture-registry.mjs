// Canonical tooling metadata for Chronica documentation placement.
//
// This file is NOT an architecture authority. Meaning remains in the canonical
// owner documents. The purpose of this registry is narrower: give the checker,
// generator and architecture-contract tooling one shared source for paths and
// document taxonomy so those tools cannot drift into competing rules.

export const documentationTopLevelEntries = Object.freeze([
  'INDEX.md',
  'TEMPLATE.md',
  'README.md',
  'learning.md',
  'architecture',
  'blueprints',
  'decisions',
  'frontend',
  'guides',
  'ops_production',
  'references',
])

export const documentationRouterPaths = Object.freeze([
  'docs/INDEX.md',
  'docs/README.md',
  'docs/TEMPLATE.md',
  'docs/architecture/README.md',
  'docs/architecture/constitution/NORTH-STAR.md',
  'docs/architecture/constitution/ENGINEERING-CONTRACT.md',
  'docs/blueprints/README.md',
  'docs/decisions/README.md',
  'docs/frontend/README.md',
  'docs/ops_production/README.md',
])

export const forbiddenDocumentationRoots = Object.freeze([
  'docs/_archive/',
  'docs/_generated/',
  'docs/_machine/',
  'docs/doctrines/',
  'docs/capabilities-canonical/',
  'docs/capabilities-cloud/',
  'docs/architecture-canonical/',
  'docs/crates/',
  'docs/history/',
  'docs/legacy/',
  'docs/frontend/worldos-blueprints/',
  'docs/fabric/',
  'docs/legal/',
  'docs/tax/',
  'docs/fiscal/',
  'docs/compliance-platform/',
  'docs/normative/',
  'docs/policy-engine/',
  'docs/math/',
  'docs/mathematics/',
  'docs/formal-world/',
  'docs/performance-world/',
  'docs/organism/',
  'docs/world-model/',
  'docs/adr/',
  'docs/architecture/compatibility/',
  'docs/runbooks/',
])

export const architectureResponsibilityDirs = Object.freeze([
  'constitution',
  'foundation',
  'governance',
  'intelligence',
  'organism',
  'physical',
  'operations',
])

export const canonicalArchitectureOwners = Object.freeze([
  'docs/architecture/constitution/NORTH-STAR.md',
  'docs/architecture/constitution/ENGINEERING-CONTRACT.md',
  'docs/architecture/constitution/NATIVE-TECHNOLOGY-STRATEGY.md',
  'docs/architecture/foundation/system-model.md',
  'docs/architecture/foundation/naming-topology.md',
  'docs/architecture/foundation/product-language.md',
  'docs/architecture/foundation/world.md',
  'docs/architecture/foundation/identity.md',
  'docs/architecture/foundation/resource.md',
  'docs/architecture/foundation/state.md',
  'docs/architecture/foundation/relation-binding.md',
  'docs/architecture/foundation/event.md',
  'docs/architecture/foundation/capability-resolution.md',
  'docs/architecture/governance/authority.md',
  'docs/architecture/governance/organization.md',
  'docs/architecture/governance/normative.md',
  'docs/architecture/governance/execution.md',
  'docs/architecture/governance/evidence.md',
  'docs/architecture/governance/sovereignty.md',
  'docs/architecture/intelligence/memory.md',
  'docs/architecture/intelligence/context.md',
  'docs/architecture/intelligence/intelligence.md',
  'docs/architecture/intelligence/world-model.md',
  'docs/architecture/intelligence/learning.md',
  'docs/architecture/intelligence/adaptation.md',
  'docs/architecture/organism/organism.md',
  'docs/architecture/organism/embodiment.md',
  'docs/architecture/organism/lifecycle.md',
  'docs/architecture/physical/machine.md',
  'docs/architecture/physical/digital-twin.md',
  'docs/architecture/physical/safety.md',
  'docs/architecture/operations/reliability.md',
  'docs/architecture/operations/recovery.md',
  'docs/architecture/operations/performance.md',
  'docs/architecture/operations/fabric.md',
])

const canonicalArchitectureOwnerSet = new Set(canonicalArchitectureOwners)

export function isCanonicalArchitectureOwner(path) {
  return canonicalArchitectureOwnerSet.has(path)
}

export function architectureOwnerIdFromPath(path) {
  const name = String(path).split('/').at(-1) ?? ''
  return name.replace(/\.md$/i, '').toLowerCase()
}

// New semantic owners may be generated only inside one direct responsibility
// directory. Constitution files are deliberately hand-maintained and cannot be
// scaffolded by the generic owner generator.
export const architectureOwnerPrefixes = Object.freeze(
  architectureResponsibilityDirs
    .filter((dir) => dir !== 'constitution')
    .map((dir) => `docs/architecture/${dir}/`),
)

const KEBAB_MD = /^[a-z0-9]+(?:-[a-z0-9]+)*\.md$/
const DECISION_MD = /^\d{4}-[a-z0-9]+(?:-[a-z0-9]+)*\.md$/

function isDirectChild(path, directory, pattern = KEBAB_MD) {
  const prefix = `${directory}/`
  if (!path.startsWith(prefix)) return false
  const rest = path.slice(prefix.length)
  return !rest.includes('/') && pattern.test(rest)
}

export const generatedDocumentKinds = Object.freeze([
  'architecture-owner',
  'blueprint',
  'decision',
  'guide',
  'reference',
])

export function isValidGeneratedDocumentTarget(kind, path) {
  if (kind === 'architecture-owner') {
    return architectureOwnerPrefixes.some((prefix) => {
      const directory = prefix.slice(0, -1)
      return isDirectChild(path, directory)
    })
  }
  if (kind === 'blueprint') return isDirectChild(path, 'docs/blueprints')
  if (kind === 'decision') return isDirectChild(path, 'docs/decisions', DECISION_MD)
  if (kind === 'guide') return isDirectChild(path, 'docs/guides')
  if (kind === 'reference') return isDirectChild(path, 'docs/references')
  return false
}

export function expectedGeneratedDocumentTarget(kind) {
  if (kind === 'architecture-owner') return `${architectureOwnerPrefixes.join(' | ')}<slug>.md`
  if (kind === 'blueprint') return 'docs/blueprints/<slug>.md'
  if (kind === 'decision') return 'docs/decisions/<NNNN-slug>.md'
  if (kind === 'guide') return 'docs/guides/<slug>.md'
  if (kind === 'reference') return 'docs/references/<slug>.md'
  return '<unsupported document kind>'
}
