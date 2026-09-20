import { test } from 'node:test'
import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import { fileURLToPath } from 'node:url'
import { dirname, join } from 'node:path'
import { parse as parseYaml } from 'yaml'

const HERE = dirname(fileURLToPath(import.meta.url))
const ROOT = join(HERE, '..', '..')
const POLICY_PATH = join(HERE, 'native-technology-promotion.yaml')
const CORPUS_PATH = join(ROOT, 'tools', 'refoundation', 'donor-corpus.yaml')
const D237_CENSUS_PATH = join(ROOT, 'provenance', 'donors', 'D237-technology-census.json')

function policy() {
  return parseYaml(readFileSync(POLICY_PATH, 'utf8'))
}

function corpus() {
  return parseYaml(readFileSync(CORPUS_PATH, 'utf8'))
}

function d237Census() {
  return JSON.parse(readFileSync(D237_CENSUS_PATH, 'utf8'))
}

test('native technology policy requires full independence before terminal promotion', () => {
  const p = policy()
  assert.equal(p.schema_version, 2)
  assert.equal(p.terminal_doctrine?.promoted_core_end_state, 'FULLY_NATIVE')
  assert.equal(p.terminal_doctrine?.census_is_completion, false)
  assert.equal(p.terminal_doctrine?.donor_source_may_be_runtime_dependency, false)
  assert.equal(p.terminal_doctrine?.donor_library_may_be_required_at_fully_native, false)
  assert.equal(p.terminal_doctrine?.donor_database_engine_may_be_required_at_fully_native, false)
  assert.ok(p.fully_native_required?.includes('ZERO_REQUIRED_DONOR_RUNTIME_DEPENDENCY'))
  assert.ok(p.hard_rules?.includes('CURRENT_SLICE_COMPLETE_IS_NOT_EXTINCTION_AUTHORITY'))
  assert.ok(p.hard_rules?.includes('EXTINCTION_CANNOT_DESTROY_REQUIRED_TECHNOLOGICAL_STRENGTH'))
})

test('promoted strategic/foundational corpus entries cannot claim extinction before an extinction-safe lifecycle state', () => {
  const doc = corpus()
  const safe = new Set(['FULLY_NATIVE', 'DONOR_EXTINCT', 'REJECTED_FOR_NATIVE_PROMOTION'])
  for (const donor of doc.donors ?? []) {
    const promotion = donor.native_promotion
    if (!promotion) continue
    if (!['STRATEGIC', 'FOUNDATIONAL'].includes(promotion.strategic_class)) continue
    if (safe.has(promotion.lifecycle_state)) continue

    assert.equal(donor.can_delete_source, false, `${donor.id} ${donor.repo} allows source deletion at ${promotion.lifecycle_state}`)
    assert.equal(promotion.extinction_eligible, false, `${donor.id} ${donor.repo} claims extinction eligibility at ${promotion.lifecycle_state}`)
  }
})

test('D237 keeps the rehydrated donor lineage while the native census is incomplete', () => {
  const d237 = corpus().donors.find((d) => d.id === 'D237')
  assert.ok(d237)
  assert.equal(d237.repo, 'postgres/postgres')
  assert.equal(d237.status, 'PARTIALLY_ABSORBED')
  assert.equal(d237.census_complete, false)
  assert.equal(d237.source_present, true)
  assert.equal(d237.absorption_state, 'partial')
  assert.equal(d237.can_delete_source, false)
  assert.equal(d237.native_promotion?.technology_id, 'chronica.durable_world_engine')
  assert.equal(d237.native_promotion?.strategic_class, 'FOUNDATIONAL')
  assert.equal(d237.native_promotion?.lifecycle_state, 'NATIVE_PROTOTYPE')
  assert.equal(d237.native_promotion?.source_rehydration_required, false)
  assert.match(d237.native_promotion?.remaining_donor_runtime_dependencies?.[0] ?? '', /PostgreSQL database engine/)
})

test('D237 technology census satisfies the promoted-technology Atlas projection without claiming completion', () => {
  const census = d237Census()
  assert.equal(census.donor_id, 'D237')
  assert.equal(census.technology_id, 'chronica.durable_world_engine')
  assert.equal(census.strategic_class, 'FOUNDATIONAL')
  assert.equal(census.technology_value_score?.total, 39)
  assert.equal(census.native_owner?.canonical_semantic_owner, 'runtime::world')
  assert.ok(Array.isArray(census.replacement_scope?.included))
  assert.ok(Array.isArray(census.dependency_graph?.primary_sequence))
  assert.equal(census.lifecycle_state, 'NATIVE_PROTOTYPE')
  assert.equal(census.next_native_slice?.id, 'D237-NATIVE-LOG-001')
  assert.match(census.remaining_donor_runtime_dependencies?.[0] ?? '', /PostgreSQL database engine/)
  assert.equal(census.proof_status?.P0_CENSUS_PROOF, 'IN_PROGRESS')
  assert.equal(census.extinction_eligibility?.eligible, false)
  assert.equal(census.census_complete, false)
  assert.equal(census.extinction_eligibility?.source_must_remain_present, true)
})
