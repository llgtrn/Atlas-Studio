import test from 'node:test'
import assert from 'node:assert/strict'
import Database from 'better-sqlite3'
import { mkdtempSync, rmSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join } from 'node:path'
import { applyReviewManifest, normalizeReviewManifest, validateReviewManifest } from './donor-file-review-lib.mjs'

test('normalizes a donor review manifest into DB-safe rows', () => {
  const manifest = normalizeReviewManifest({
    donor: 'demo-donor',
    file_reviews: [
      {
        path: '\\src\\app.py',
        classification: 'mapped',
        read_status: 'reviewed',
        mapped_source_ids: [1, 2],
        exclusion_reason: 'Read fully',
        evidence_note: 'Functions map to source rows',
      },
    ],
    links: [
      { source_id: '1', path: 'src/app.py', symbol: 'run()', evidence_note: 'line 10' },
    ],
  })

  assert.equal(manifest.file_reviews[0].path, 'src/app.py')
  assert.equal(manifest.file_reviews[0].mapped_source_ids, '1,2')
  assert.equal(manifest.links[0].source_id, 1)
})

test('accepts reviewer shorthand read status', () => {
  const manifest = normalizeReviewManifest({
    donor: 'demo-donor',
    file_reviews: [
      {
        path: 'README.md',
        classification: 'non_behavioral_support',
        read_status: 'read',
        evidence_note: 'license-only text',
      },
    ],
  })

  assert.equal(manifest.file_reviews[0].read_status, 'reviewed')
  assert.doesNotThrow(() => validateReviewManifest(manifest))
})

test('normalizes Temporary donor-prefixed review paths', () => {
  const manifest = normalizeReviewManifest({
    donor: 'blackbox_exporter-master',
    file_reviews: [
      {
        path: 'Temporary/blackbox_exporter-master/.circleci/config.yml',
        classification: 'non_behavioral_support',
        read_status: 'read',
        evidence_note: 'CI only',
      },
    ],
    links: [
      {
        source_id: 1,
        path: 'Temporary/blackbox_exporter-master/main.go',
        symbol: 'run',
        evidence_note: 'entrypoint',
      },
    ],
  })

  assert.equal(manifest.file_reviews[0].path, '.circleci/config.yml')
  assert.equal(manifest.links[0].path, 'main.go')
  assert.doesNotThrow(() => validateReviewManifest(manifest))
})

test('converts reviewer shorthand read status to blocked for blocked rows', () => {
  const manifest = normalizeReviewManifest({
    donor: 'demo-donor',
    file_reviews: [
      {
        path: 'src/new_behavior.py',
        classification: 'blocked',
        read_status: 'read',
        evidence_note: 'read fully; missing source capability row',
      },
    ],
  })

  assert.equal(manifest.file_reviews[0].read_status, 'blocked')
  assert.doesNotThrow(() => validateReviewManifest(manifest))
})

test('rejects incomplete or unsupported review classifications', () => {
  assert.throws(
    () => validateReviewManifest({
      donor: 'demo-donor',
      file_reviews: [{ path: 'src/app.py', classification: 'maybe', read_status: 'reviewed' }],
    }),
    /unsupported classification/
  )

  assert.throws(
    () => validateReviewManifest({ donor: '', file_reviews: [] }),
    /donor is required/
  )
})

test('applies review manifests when meta table is absent', () => {
  const root = mkdtempSync(join(tmpdir(), 'chronica-donor-file-review-'))
  let db
  try {
    db = new Database(join(root, 'caps.db'))
    db.exec(`
      CREATE TABLE donor_file_census (
        donor TEXT NOT NULL,
        path TEXT NOT NULL,
        classification TEXT NOT NULL,
        read_status TEXT NOT NULL,
        mapped_source_ids TEXT,
        exclusion_reason TEXT,
        reviewed_by TEXT,
        reviewed_at TEXT,
        PRIMARY KEY (donor, path)
      );
      CREATE TABLE source_capability (
        id INTEGER PRIMARY KEY,
        donor TEXT NOT NULL
      );
      CREATE TABLE source_file_capability_link (
        source_id INTEGER NOT NULL,
        donor TEXT NOT NULL,
        path TEXT NOT NULL,
        symbol TEXT,
        evidence_note TEXT,
        PRIMARY KEY (source_id, donor, path, symbol)
      );
      INSERT INTO donor_file_census VALUES ('demo-donor', 'src/app.py', 'capability_review_pending', 'unread_pending', '', 'pending', NULL, NULL);
      INSERT INTO source_capability VALUES (101, 'demo-donor');
    `)

    const result = applyReviewManifest(db, {
      donor: 'demo-donor',
      file_reviews: [{
        path: 'src/app.py',
        classification: 'mapped',
        read_status: 'reviewed',
        mapped_source_ids: [101],
        evidence_note: 'Read app.py and mapped run().',
      }],
      links: [{
        source_id: 101,
        path: 'src/app.py',
        symbol: 'run',
        evidence_note: 'run() evidence',
      }],
    }, { reviewedBy: 'test' })

    assert.equal(result.updated, 1)
    assert.equal(result.linked, 1)
    assert.equal(db.prepare("SELECT v FROM meta WHERE k='file_census_unread_pending'").get().v, '0')
  } finally {
    if (db?.open) db.close()
    rmSync(root, { recursive: true, force: true })
  }
})
