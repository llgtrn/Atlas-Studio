import assert from 'node:assert/strict'
import test from 'node:test'
import { enrichNamingTopology, validateNamingTopology } from './system-coverage-naming.mjs'

// 2026-09-16 hard refoundation: core/, runtime/, adapter/, organism/ are top-level
// single-package crates (their own Cargo.toml directly, not nested under a
// crates/<tier>/<domain>/<component>/ wrapper). See docs/architecture/foundation/naming-topology.md.
// Each real tier can physically host only one crate today, so this fixture spreads its debt
// examples across the four tier roots instead of stacking several under one.
function fixtureCoverage() {
  return {
    stats: {},
    families: {
      repository: {
        name: 'repository',
        nodes: [
          // Domain-suffixed artifact identity, but the repo hasn't physically split into
          // core/<domain>/<component>/ yet -- flat-path naming debt.
          { id: 'repository:crate:chronica-core-helpdesk-sla', family: 'repository', kind: 'crate', label: 'chronica-core-helpdesk-sla', manifest: 'core/Cargo.toml' },
          { id: 'repository:file:sla', family: 'repository', kind: 'source_file', label: 'core/src/lib.rs', path: 'core/src/lib.rs' },
          // Physically under runtime/ but artifact identity claims the adapter tier.
          { id: 'repository:crate:chronica-adapter-social-postgres-batch1', family: 'repository', kind: 'crate', label: 'chronica-adapter-social-postgres-batch1', manifest: 'runtime/Cargo.toml' },
          // Legacy chronica-cap-* package name -- responsibility not yet classified.
          { id: 'repository:crate:chronica-cap-helpdesk-kb', family: 'repository', kind: 'crate', label: 'chronica-cap-helpdesk-kb', manifest: 'adapter/Cargo.toml' },
          // Bare kernel facade identity with no domain suffix, physically at its own tier root.
          { id: 'repository:crate:chronica-organism', family: 'repository', kind: 'crate', label: 'chronica-organism', manifest: 'organism/Cargo.toml' },
          // The retired crates/ wrapper no longer classifies as a physical crate location at all.
          { id: 'repository:crate:chronica-core-legacy', family: 'repository', kind: 'crate', label: 'chronica-core-legacy', manifest: 'crates/core/chronica-core-legacy/Cargo.toml' },
        ],
        edges: [],
        artifacts: [],
        gaps: [],
      },
    },
  }
}

test('separates artifact identity, physical path and canonical topology', () => {
  const coverage = enrichNamingTopology(fixtureCoverage())
  const crates = new Map(coverage.namingTopology.crates.map((entry) => [entry.artifactIdentity, entry]))

  assert.equal(crates.get('chronica-core-helpdesk-sla').namingState, 'LEGACY_FLAT_PATH')
  assert.equal(crates.get('chronica-core-helpdesk-sla').canonicalPath, 'core/helpdesk/sla')
  assert.equal(crates.get('chronica-core-helpdesk-sla').displayLabel, 'core/helpdesk/sla')

  assert.equal(crates.get('chronica-adapter-social-postgres-batch1').namingState, 'TIER_MISMATCH')
  assert.equal(crates.get('chronica-adapter-social-postgres-batch1').canonicalPath, 'adapter/social/postgres-batch1')

  assert.equal(crates.get('chronica-cap-helpdesk-kb').namingState, 'CAP_LEGACY_UNRESOLVED')
  assert.equal(crates.get('chronica-cap-helpdesk-kb').canonicalPath, null)
  assert.equal(crates.get('chronica-cap-helpdesk-kb').displayLabel, 'cap/helpdesk/kb')

  assert.equal(crates.get('chronica-organism').namingState, 'CANONICAL_PATH')
  assert.equal(crates.get('chronica-organism').canonicalPath, 'organism')
  assert.equal(crates.get('chronica-organism').displayLabel, 'organism')

  assert.equal(crates.get('chronica-core-legacy').namingState, 'LEGACY_TIER')
  assert.equal(crates.get('chronica-core-legacy').architectureTier, null)

  const file = coverage.families.repository.nodes.find((node) => node.kind === 'source_file')
  assert.equal(file.displayLabel, 'core/helpdesk/sla/src/lib.rs')
  assert.equal(file.canonicalPath, 'core/helpdesk/sla/src/lib.rs')
  assert.deepEqual(validateNamingTopology(coverage), [])
})

test('records naming debt as Atlas gaps without pretending rename authority', () => {
  const coverage = enrichNamingTopology(fixtureCoverage())
  const kinds = new Set(coverage.families.repository.gaps.map((gap) => gap.kind))
  assert.ok(kinds.has('LEGACY_FLAT_PATH'))
  assert.ok(kinds.has('TIER_MISMATCH'))
  assert.ok(kinds.has('CAP_LEGACY_UNRESOLVED'))
  assert.ok(kinds.has('LEGACY_TIER'))
  assert.equal(coverage.namingTopology.stats.totalCrates, 5)
  assert.equal(coverage.namingTopology.stats.canonicalPathCrates, 1)
  assert.equal(coverage.namingTopology.stats.namingDebtCrates, 4)
})
