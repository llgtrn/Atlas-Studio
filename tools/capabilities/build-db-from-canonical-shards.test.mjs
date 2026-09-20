import test from 'node:test'
import assert from 'node:assert/strict'
import { mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join } from 'node:path'

import { buildCapabilityDbFromCanonicalShards } from './build-db-from-canonical-shards.mjs'
import { exportCanonicalCapabilityShards } from './export-canonical-shards.mjs'

// Regression test for #2869: build-db-from-canonical-shards.mjs -> export-canonical-shards.mjs
// is not a safe round trip for real canonical data. A large fraction of this repo's real
// source_refs entries are freeform prose citations, not the strict `key=value;key=value`
// micro-format `parseRef()` understands. Rebuilding the DB from the canonical JSONL shards and
// re-exporting used to silently discard the original prose and replace it with a lossy
// `source_id=N;donor=unknown;module=unknown;...` synthetic stub on every capability, not just
// ones actually being promoted.

function makeRoot() {
  const root = mkdtempSync(join(tmpdir(), 'chronica-build-db-round-trip-'))
  mkdirSync(join(root, 'docs', 'capabilities-canonical', 'domains'), { recursive: true })
  return root
}

const PROSE_REF =
  'frappe/frappe@7dfdd9b frappe/email/doctype/notification/notification.py (lines 22-24 ' +
  'DATE_BASED_EVENTS, 27 class Notification) -- a declarative doctype that fires on document ' +
  'lifecycle events or on a date-relative schedule'

const STRUCTURED_REF = 'source_id=42;donor=known-donor;module=known-module;name=Known;files=a.rs;symbols=foo'

function record(overrides = {}) {
  return {
    schema_version: 1,
    capability_key: 'other.one',
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
    implemented_unverified_count: 1,
    money_count: 0,
    record_count: 1,
    source_db: 'docs/capabilities.db',
    verified_count: 0,
    ...overrides,
  }
  writeFileSync(join(root, 'docs', 'capabilities-canonical', 'meta.json'), JSON.stringify(meta, null, 2) + '\n')
}

function readExportedRecord(root) {
  const exported = readFileSync(join(root, 'docs', 'capabilities-canonical', 'domains', 'other.jsonl'), 'utf8')
  return JSON.parse(exported.trim().split('\n')[0])
}

test('a freeform prose source_ref round-trips byte-identical through build-db + export', () => {
  const root = makeRoot()
  writeShard(root, [record({ source_refs: [PROSE_REF] })])
  writeMeta(root)
  const dbPath = join(root, 'docs', 'capabilities.db')

  try {
    // the corrupting bug: without the fix, this row would carry donor=NULL/module=NULL
    // because parseRef() can't decompose free prose, and export reconstructed a lossy stub.
    buildCapabilityDbFromCanonicalShards({ root, outPath: dbPath })
    exportCanonicalCapabilityShards({ root, dbPath })

    const exportedRecord = readExportedRecord(root)
    assert.deepEqual(exportedRecord.source_refs, [PROSE_REF])
    assert.ok(
      !exportedRecord.source_refs.some((ref) => ref.includes('donor=unknown')),
      `source_refs corrupted into a lossy stub: ${JSON.stringify(exportedRecord.source_refs)}`,
    )
  } finally {
    rmSync(root, { recursive: true, force: true })
  }
})

test('a well-formed structured source_ref still round-trips correctly', () => {
  const root = makeRoot()
  writeShard(root, [record({ source_refs: [STRUCTURED_REF] })])
  writeMeta(root)
  const dbPath = join(root, 'docs', 'capabilities.db')

  try {
    buildCapabilityDbFromCanonicalShards({ root, outPath: dbPath })
    exportCanonicalCapabilityShards({ root, dbPath })

    const exportedRecord = readExportedRecord(root)
    assert.deepEqual(exportedRecord.source_refs, [STRUCTURED_REF])
  } finally {
    rmSync(root, { recursive: true, force: true })
  }
})
