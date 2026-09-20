import assert from 'node:assert/strict'
import { mkdirSync, mkdtempSync, rmSync, writeFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join } from 'node:path'
import test from 'node:test'
import {
  buildCapabilityEvidenceIndex,
  computeAllDomainEvidence,
  computeDomainEvidence,
  domainMappingFor,
  evidenceStrengthScore,
  FIC_DOMAIN_MAPPING,
  loadV1Snapshot,
  MAPPING_METHOD,
  reviewPriority,
  V1_JSON_RELATIVE_PATH,
} from './fic-domain-evidence-lib.mjs'

// ── mapping-table sanity (this must never silently drift from doc 180's denominator) ──────────

test('FIC_DOMAIN_MAPPING has exactly one entry per v1.json domain key, no more, no less', () => {
  const root = process.cwd()
  const v1 = loadV1Snapshot(root)
  const v1Keys = v1.domains.map((d) => d.key).sort()
  const mapKeys = Object.keys(FIC_DOMAIN_MAPPING).sort()
  assert.deepEqual(mapKeys, v1Keys)
})

test('every mapping entry is a recognized method with the fields that method needs', () => {
  for (const [key, mapping] of Object.entries(FIC_DOMAIN_MAPPING)) {
    assert.ok(Object.values(MAPPING_METHOD).includes(mapping.method), `${key}: unrecognized method ${mapping.method}`)
    if (mapping.method === MAPPING_METHOD.NOT_MAPPABLE) {
      assert.ok(mapping.note && mapping.note.length > 0, `${key}: NOT_MAPPABLE requires a note`)
    } else {
      assert.ok(Array.isArray(mapping.crate_names) && mapping.crate_names.length > 0, `${key}: mapped domain needs crate_names`)
    }
  }
})

test('domainMappingFor falls back to NOT_MAPPABLE for an unknown key instead of throwing', () => {
  const result = domainMappingFor('totally-unknown-domain-key')
  assert.equal(result.method, MAPPING_METHOD.NOT_MAPPABLE)
})

test('no crate name is reused across two different domains\' crate_names (would double-count evidence)', () => {
  const owners = new Map()
  for (const [key, mapping] of Object.entries(FIC_DOMAIN_MAPPING)) {
    for (const crate of mapping.crate_names ?? []) {
      assert.ok(!owners.has(crate), `crate ${crate} is claimed by both ${owners.get(crate)} and ${key}`)
      owners.set(crate, key)
    }
  }
})

// ── fixture workspace: two crates, one mapped by crate-name heuristic, one by explicit domain field ─

function makeFixtureWorkspace() {
  const root = mkdtempSync(join(tmpdir(), 'chronica-fic-evidence-'))
  writeFileSync(
    join(root, 'Cargo.toml'),
    '[workspace]\nmembers = ["crates/chronica-fixture-identity", "crates/chronica-fixture-commerce"]\n',
  )
  mkdirSync(join(root, 'docs', 'crates'), { recursive: true })

  // crate 1: mapped to "identity-session" via crate-name heuristic (mirrors the real mapping table)
  const identityPath = join(root, 'crates', 'chronica-fixture-identity')
  mkdirSync(join(identityPath, 'src'), { recursive: true })
  mkdirSync(join(identityPath, '.chronica'), { recursive: true })
  writeFileSync(join(identityPath, 'Cargo.toml'), '[package]\nname = "chronica-fixture-identity"\nversion = "0.1.0"\n')
  writeFileSync(
    join(identityPath, 'src', 'lib.rs'),
    'pub mod session;\n\npub fn dispatch() {\n    session::run();\n}\n',
  )
  writeFileSync(
    join(identityPath, 'src', 'session.rs'),
    `
pub struct Session { pub id: u32 }
impl Session {
    pub fn touch(&mut self) -> u32 {
        self.id += 1;
        for i in 0..self.id {
            if i % 2 == 0 { self.id += 1; }
        }
        self.id
    }
}
pub fn run() -> u32 { 1 }

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn run_is_one() { assert_eq!(run(), 1); }
}
`,
  )
  const identityShard = [
    { record_type: 'meta', crate: 'chronica-fixture-identity' },
    {
      record_type: 'capability',
      capability_key: 'fixture.identity_session',
      canonical_name: 'Session',
      domain: 'agent-knowledge',
      target_crate: 'chronica-fixture-identity',
      target_module: 'session',
      status: 'unimplemented',
    },
  ]
  writeFileSync(
    join(identityPath, '.chronica', 'sub-cap-arch.jsonl'),
    identityShard.map((r) => JSON.stringify(r)).join('\n') + '\n',
  )
  writeFileSync(
    join(root, 'docs', 'crates', '900-crate-chronica-fixture-identity.md'),
    '# 900 - crate chronica-fixture-identity\n\n## 16. capabilities.db row(s)\n\n| Key |Status |Money |Side effect |Target crate |Target module |Acceptance test |\n| --- |--- |--- |--- |--- |--- |--- |\n',
  )

  // crate 2: mapped to "commerce" via explicit shard-domain-field match ("commerce")
  const commercePath = join(root, 'crates', 'chronica-fixture-commerce')
  mkdirSync(join(commercePath, 'src'), { recursive: true })
  mkdirSync(join(commercePath, '.chronica'), { recursive: true })
  writeFileSync(join(commercePath, 'Cargo.toml'), '[package]\nname = "chronica-fixture-commerce"\nversion = "0.1.0"\n')
  writeFileSync(join(commercePath, 'src', 'lib.rs'), 'mod order;\n')
  writeFileSync(join(commercePath, 'src', 'order.rs'), 'pub fn run() {\n    todo!()\n}\n')
  const commerceShard = [
    { record_type: 'meta', crate: 'chronica-fixture-commerce' },
    {
      record_type: 'capability',
      capability_key: 'fixture.order_capture',
      canonical_name: 'Order capture',
      domain: 'commerce',
      target_crate: 'chronica-fixture-commerce',
      target_module: 'order',
      status: 'unimplemented',
    },
  ]
  writeFileSync(
    join(commercePath, '.chronica', 'sub-cap-arch.jsonl'),
    commerceShard.map((r) => JSON.stringify(r)).join('\n') + '\n',
  )

  return { root, identityPath, commercePath }
}

test('buildCapabilityEvidenceIndex evaluates capabilities across multiple crates and flags no-doc crates', () => {
  const { root } = makeFixtureWorkspace()
  try {
    const index = buildCapabilityEvidenceIndex(root, ['chronica-fixture-identity', 'chronica-fixture-commerce'])
    assert.equal(index.capabilities.length, 2)

    const identityCap = index.capabilities.find((c) => c.capability_key === 'fixture.identity_session')
    assert.equal(identityCap.crate, 'chronica-fixture-identity')
    assert.equal(identityCap.verdict, 'DISAGREE') // real+reachable code, shard says unimplemented
    assert.equal(identityCap.reachable, true)
    assert.equal(identityCap.tested, true) // inline #[test] references the module

    const commerceCap = index.capabilities.find((c) => c.capability_key === 'fixture.order_capture')
    assert.equal(commerceCap.verdict, 'AGREE') // stub body, shard correctly says unimplemented
    assert.equal(commerceCap.code_state, 'stub')

    assert.ok(index.cratesWithDocs.get('chronica-fixture-identity'))
    assert.equal(index.cratesWithDocs.get('chronica-fixture-commerce'), null)
  } finally {
    rmSync(root, { recursive: true, force: true })
  }
})

test('computeDomainEvidence: NOT_MAPPABLE domain returns an empty, explicit packet with no level', () => {
  const { root } = makeFixtureWorkspace()
  try {
    const evidence = computeDomainEvidence({
      domainKey: 'database',
      mapping: domainMappingFor('database'),
      capabilityIndex: buildCapabilityEvidenceIndex(root, []),
      workspaceCrateNames: new Set(),
    })
    assert.equal(evidence.mapping.method, MAPPING_METHOD.NOT_MAPPABLE)
    assert.equal(evidence.mapped_capabilities.total, 0)
    assert.equal(evidence.sync_anchor_v2, null)
    assert.ok(!('level' in evidence))
  } finally {
    rmSync(root, { recursive: true, force: true })
  }
})

test('computeDomainEvidence: crate-name-heuristic mapping aggregates AGREE/DISAGREE/reachable/tested', () => {
  const { root } = makeFixtureWorkspace()
  try {
    const capabilityIndex = buildCapabilityEvidenceIndex(root, ['chronica-fixture-identity'])
    const evidence = computeDomainEvidence({
      domainKey: 'identity-session',
      mapping: { method: MAPPING_METHOD.CRATE_NAME_HEURISTIC, crate_names: ['chronica-fixture-identity'] },
      capabilityIndex,
      workspaceCrateNames: new Set(['chronica-fixture-identity']),
    })
    assert.equal(evidence.mapped_capabilities.total, 1)
    assert.equal(evidence.mapped_capabilities.by_method.crate_name_heuristic, 1)
    assert.equal(evidence.sync_anchor_v2.disagree, 1)
    assert.equal(evidence.code_reachability.reachable, 1)
    assert.equal(evidence.test_evidence.tested, 1)
  } finally {
    rmSync(root, { recursive: true, force: true })
  }
})

test('computeDomainEvidence: explicit_domain_field mapping matches on the shard domain value', () => {
  const { root } = makeFixtureWorkspace()
  try {
    const capabilityIndex = buildCapabilityEvidenceIndex(root, ['chronica-fixture-commerce'])
    const evidence = computeDomainEvidence({
      domainKey: 'commerce',
      mapping: { method: MAPPING_METHOD.EXPLICIT_DOMAIN_FIELD, domain_field_values: ['commerce'], crate_names: [] },
      capabilityIndex,
      workspaceCrateNames: new Set(['chronica-fixture-commerce']),
    })
    assert.equal(evidence.mapped_capabilities.total, 1)
    assert.equal(evidence.mapped_capabilities.by_method.explicit_domain_field, 1)
    assert.equal(evidence.sync_anchor_v2.agree, 1)
  } finally {
    rmSync(root, { recursive: true, force: true })
  }
})

test('computeDomainEvidence: mapped crate with zero capability rows reports zero, not a crash', () => {
  const { root } = makeFixtureWorkspace()
  try {
    const capabilityIndex = buildCapabilityEvidenceIndex(root, ['chronica-fixture-identity'])
    const evidence = computeDomainEvidence({
      domainKey: 'some-domain',
      mapping: { method: MAPPING_METHOD.CRATE_NAME_HEURISTIC, crate_names: ['chronica-does-not-exist'] },
      capabilityIndex: buildCapabilityEvidenceIndex(root, ['chronica-does-not-exist']),
      workspaceCrateNames: new Set(['chronica-fixture-identity']),
    })
    assert.equal(evidence.mapped_capabilities.total, 0)
    assert.equal(evidence.sync_anchor_v2, null)
    assert.deepEqual(evidence.mapping.crate_names_missing_from_workspace, ['chronica-does-not-exist'])
  } finally {
    rmSync(root, { recursive: true, force: true })
  }
})

test('computeAllDomainEvidence: reads the real v1.json taxonomy and never emits a level field on evidence', () => {
  const root = process.cwd()
  const result = computeAllDomainEvidence(root)
  assert.equal(result.domains_total, 55)
  assert.equal(result.per_domain.length, 55)
  for (const domain of result.per_domain) {
    assert.ok(domain.hand_assigned.level, `${domain.key} missing hand_assigned.level (read-only copy)`)
    assert.ok(!('level' in domain), `${domain.key}: evidence packet must never carry its own level field`)
    assert.ok(!('suggested_level' in domain))
  }
})

test('computeAllDomainEvidence: every domain key is a real V1_JSON_RELATIVE_PATH entry', () => {
  const root = process.cwd()
  const v1 = loadV1Snapshot(root)
  assert.ok(V1_JSON_RELATIVE_PATH.includes('world-class-infrastructure-coverage-v1.json'))
  const result = computeAllDomainEvidence(root)
  assert.deepEqual(
    result.per_domain.map((d) => d.key),
    v1.domains.map((d) => d.key),
  )
})

// ── review-priority scoring (a sort key, never a level) ────────────────────────────────────────

test('evidenceStrengthScore: null when no capabilities are mapped', () => {
  const score = evidenceStrengthScore({ mapped_capabilities: { total: 0 } })
  assert.equal(score, null)
})

test('evidenceStrengthScore: averages agree/reachable/tested rates', () => {
  const score = evidenceStrengthScore({
    mapped_capabilities: { total: 10 },
    sync_anchor_v2: { agree_percent: 100 },
    code_reachability: { reachable_percent: 50 },
    test_evidence: { tested_percent: 0 },
  })
  assert.equal(score, 0.5)
})

test('reviewPriority: not-evaluable domains are marked and never given a contradiction score', () => {
  const priority = reviewPriority({
    mapping: { method: MAPPING_METHOD.NOT_MAPPABLE },
    mapped_capabilities: { total: 0 },
    hand_assigned: { level: 'L2' },
  })
  assert.equal(priority.evaluable, false)
  assert.equal(priority.reason, 'NOT_MAPPABLE')
  assert.equal(priority.contradiction_score, null)
})

test('reviewPriority: strong evidence against a low hand level is flagged EVIDENCE_STRONGER_THAN_HAND_LEVEL', () => {
  const priority = reviewPriority({
    mapping: { method: MAPPING_METHOD.CRATE_NAME_HEURISTIC },
    mapped_capabilities: { total: 5 },
    sync_anchor_v2: { agree_percent: 100 },
    code_reachability: { reachable_percent: 100 },
    test_evidence: { tested_percent: 100 },
    hand_assigned: { level: 'L0' },
  })
  assert.equal(priority.evaluable, true)
  assert.equal(priority.direction, 'EVIDENCE_STRONGER_THAN_HAND_LEVEL')
  assert.ok(priority.contradiction_score > 0)
  assert.ok(!('suggested_level' in priority))
})

test('reviewPriority: weak evidence against a high hand level is flagged EVIDENCE_WEAKER_THAN_HAND_LEVEL', () => {
  const priority = reviewPriority({
    mapping: { method: MAPPING_METHOD.CRATE_NAME_HEURISTIC },
    mapped_capabilities: { total: 5 },
    sync_anchor_v2: { agree_percent: 0 },
    code_reachability: { reachable_percent: 0 },
    test_evidence: { tested_percent: 0 },
    hand_assigned: { level: 'L3' },
  })
  assert.equal(priority.direction, 'EVIDENCE_WEAKER_THAN_HAND_LEVEL')
})
