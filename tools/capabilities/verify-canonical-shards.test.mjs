import test from 'node:test'
import assert from 'node:assert/strict'
import { mkdirSync, mkdtempSync, writeFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join } from 'node:path'

import { verifyCanonicalCapabilityShards } from './verify-canonical-shards.mjs'

// Regression test for the recurring 2026-08 batch-build gap (#2872, #2883): a promotion script
// that moves N rows from `unimplemented` to `implemented_unverified` inside a domain shard, but
// never regenerates `docs/capabilities-canonical/meta.json`'s own `implemented_unverified_count`,
// used to pass this verifier with `ok: true` -- every OTHER aggregate count (`record_count`,
// `domain_count`, `verified_count`, `money_count`) was cross-checked against a live recompute
// except this one. Fixed by adding the missing check; this test proves both directions: a
// matching count passes, a stale one fails with a precise message naming both numbers.

function makeRoot() {
  const root = mkdtempSync(join(tmpdir(), 'chronica-verify-canonical-shards-'))
  mkdirSync(join(root, 'docs', 'capabilities-canonical', 'domains'), { recursive: true })
  return root
}

function record(overrides = {}) {
  return {
    schema_version: 1,
    canonical_name: 'Test capability',
    domain: 'other',
    target_crate: 'chronica-erp',
    target_module: 'test',
    status: 'implemented_unverified',
    side_effect_class: 'internal_write',
    moves_money: false,
    requires_approval: false,
    acceptance_criteria: 'Local audit must prove this is implemented.',
    required_tests: [],
    financial_control_test: null,
    acceptance_test: null,
    docs_refs: [],
    code_refs: [],
    test_refs: [],
    architecture_refs: [],
    source_refs: [],
    evidence_refs: [],
    blocker: null,
    ...overrides,
  }
}

function writeShard(root, rows) {
  const lines = rows.map((row) => JSON.stringify(row)).join('\n') + '\n'
  writeFileSync(join(root, 'docs', 'capabilities-canonical', 'domains', 'other.jsonl'), lines)
}

function writeMeta(root, overrides = {}) {
  const meta = {
    schema_version: 1,
    authority: 'canonical_jsonl',
    domain_count: 1,
    files: ['docs/capabilities-canonical/domains/other.jsonl'],
    generated_by: 'tools/capabilities/export-canonical-shards.mjs',
    implemented_unverified_count: 0,
    money_count: 0,
    record_count: 0,
    source_db: 'docs/capabilities.db',
    verified_count: 0,
    ...overrides,
  }
  writeFileSync(join(root, 'docs', 'capabilities-canonical', 'meta.json'), JSON.stringify(meta, null, 2) + '\n')
}

test('implemented_unverified_count matching the live JSONL recount passes', () => {
  const root = makeRoot()
  writeShard(root, [
    record({ capability_key: 'other.one' }),
    record({ capability_key: 'other.three', status: 'unimplemented' }),
    record({ capability_key: 'other.two' }),
  ])
  writeMeta(root, { record_count: 3, implemented_unverified_count: 2 })

  const result = verifyCanonicalCapabilityShards({ root })
  assert.equal(result.ok, true, JSON.stringify(result.errors))
  assert.equal(result.counts.implemented_unverified, 2)
})

test('a stale implemented_unverified_count (the #2872/#2883 batch-build gap) fails with a precise message', () => {
  const root = makeRoot()
  writeShard(root, [
    record({ capability_key: 'other.one' }),
    record({ capability_key: 'other.three', status: 'unimplemented' }),
    record({ capability_key: 'other.two' }),
  ])
  // meta.json was never regenerated after the batch promotion: still says 0, JSONL now has 2.
  writeMeta(root, { record_count: 3, implemented_unverified_count: 0 })

  const result = verifyCanonicalCapabilityShards({ root })
  assert.equal(result.ok, false)
  assert.equal(result.counts.implemented_unverified, 2)
  assert.ok(
    result.errors.some((e) => e.includes('implemented_unverified_count=0 but JSONL has 2')),
    JSON.stringify(result.errors),
  )
})
