import test from 'node:test'
import assert from 'node:assert/strict'
import Database from 'better-sqlite3'
import { mkdirSync, mkdtempSync, rmSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join } from 'node:path'
import { validateFileReview4dReport } from './validate-file-review-4d.mjs'

function withRoot(fn) {
  const root = mkdtempSync(join(tmpdir(), 'chronica-file-review-4d-'))
  try {
    mkdirSync(join(root, 'docs'), { recursive: true })
    return fn(root)
  } finally {
    rmSync(root, { recursive: true, force: true })
  }
}

function seedDbs(root) {
  const caps = new Database(join(root, 'docs', 'capabilities.db'))
  caps.exec(`
    CREATE TABLE donor_file_census (
      donor TEXT NOT NULL,
      path TEXT NOT NULL,
      classification TEXT NOT NULL,
      read_status TEXT NOT NULL,
      mapped_source_ids TEXT,
      exclusion_reason TEXT,
      PRIMARY KEY (donor, path)
    );
    CREATE TABLE source_capability (
      id INTEGER PRIMARY KEY,
      donor TEXT NOT NULL,
      canonical_name TEXT NOT NULL
    );
    CREATE TABLE source_file_capability_link (
      source_id INTEGER NOT NULL,
      donor TEXT NOT NULL,
      path TEXT NOT NULL,
      symbol TEXT,
      evidence_note TEXT,
      PRIMARY KEY (source_id, donor, path, symbol)
    );
    INSERT INTO donor_file_census VALUES
      ('demo-donor', 'src/app.py', 'mapped', 'reviewed', '101', 'Read app.py and mapped run() to source capability 101.'),
      ('demo-donor', 'README.md', 'non_behavioral_support', 'reviewed', '', 'Read README; product usage docs only, no separate behavior beyond mapped source.'),
      ('other-donor', 'left.py', 'capability_review_pending', 'unread_pending', '', 'still pending');
    INSERT INTO source_capability VALUES (101, 'demo-donor', 'Demo source capability');
    INSERT INTO source_file_capability_link VALUES
      (101, 'demo-donor', 'src/app.py', 'run', 'run() implements the mapped behavior.');
  `)
  caps.close()

  const arch = new Database(join(root, 'docs', 'architecture.db'))
  arch.exec(`
    CREATE TABLE capability_architecture_link (
      capability_key TEXT NOT NULL,
      architecture_node TEXT NOT NULL,
      relationship TEXT NOT NULL,
      status TEXT
    );
    CREATE TABLE architecture_target_gap (
      capability_key TEXT NOT NULL,
      gap_kind TEXT NOT NULL,
      status TEXT NOT NULL
    );
    INSERT INTO capability_architecture_link VALUES
      ('demo.capability', 'crate:chronica-demo', 'targets', 'unimplemented');
  `)
  arch.close()
}

function validReport(overrides = {}) {
  return {
    batch_id: 'demo-batch-001',
    donor: 'demo-donor',
    reviewed_by: 'codex:file-review-4d-test',
    scope: 'demo-donor file-review pilot',
    require_zero_pending_for_donor: true,
    meaning: {
      purpose: 'Turn literal donor-file review into auditable Chronica capability evidence.',
      source_reality: 'The donor has a behavior file and README whose behavior was read directly.',
      chronica_action: 'Mapped the behavior file to source capability 101 and excluded README as support evidence.',
      judgment: 'The donor file surface is exhausted for this scope; no runtime implementation claim is made.',
    },
    file_review: {
      pending_before: 2,
      pending_after: 0,
      files: [
        {
          path: 'src/app.py',
          classification: 'mapped',
          read_status: 'reviewed',
          mapped_source_ids: [101],
          evidence: 'Read app.py and mapped run() to source capability 101.',
        },
        {
          path: 'README.md',
          classification: 'non_behavioral_support',
          read_status: 'reviewed',
          evidence: 'Read README; product usage docs only, no separate behavior beyond mapped source.',
        },
      ],
    },
    four_way: {
      code_tests: {
        status: 'CONFIRMED',
        evidence: ['node --test tools/reconcile/validate-file-review-4d.test.mjs'],
      },
      docs_meaning: {
        status: 'CONFIRMED',
        evidence: ['docs/doctrines/023-five-dimension-cloud-shard-contract.md describes file-level donor review 4D evidence.'],
      },
      capability_tracking: {
        status: 'CONFIRMED',
        evidence: ['donor_file_census rows reviewed; source_file_capability_link row exists.'],
        source_ids: [101],
      },
      architecture_tracking: {
        status: 'CONFIRMED',
        evidence: ['demo.capability is accounted in architecture DB.'],
        capability_keys: ['demo.capability'],
      },
    },
    ...overrides,
  }
}

test('accepts a DB-backed file-review report with all four dimensions', () => withRoot((root) => {
  seedDbs(root)

  const result = validateFileReview4dReport(validReport(), { root, checkDb: true })

  assert.equal(result.ok, true, result.errors.join('\n'))
  assert.deepEqual(result.errors, [])
  assert.equal(result.summary.files_reviewed, 2)
  assert.equal(result.summary.mapped_files, 1)
}))

test('rejects missing meaning fields and missing four-way dimensions', () => {
  const report = validReport()
  delete report.meaning.judgment
  delete report.four_way.architecture_tracking

  const result = validateFileReview4dReport(report)

  assert.equal(result.ok, false)
  assert.ok(result.errors.includes('meaning.judgment is required'))
  assert.ok(result.errors.includes('four_way.architecture_tracking is required'))
})

test('rejects DB-backed reports when mapped file links or architecture accounting are missing', () => withRoot((root) => {
  seedDbs(root)
  const caps = new Database(join(root, 'docs', 'capabilities.db'))
  caps.prepare('DELETE FROM source_file_capability_link').run()
  caps.close()

  const result = validateFileReview4dReport(
    validReport({
      four_way: {
        ...validReport().four_way,
        architecture_tracking: {
          status: 'CONFIRMED',
          evidence: ['missing.capability should fail architecture accounting.'],
          capability_keys: ['missing.capability'],
        },
      },
    }),
    { root, checkDb: true },
  )

  assert.equal(result.ok, false)
  assert.ok(result.errors.includes('source_file_capability_link missing for demo-donor/src/app.py source_id=101'))
  assert.ok(result.errors.includes('architecture accounting missing for capability key: missing.capability'))
}))
