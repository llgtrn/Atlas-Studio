#!/usr/bin/env node
// donor-file-census.mjs — build the literal file-level proof layer under the
// source/canonical capability census. This does NOT claim capabilities are absorbed;
// it makes every reviewed and unreviewed donor file visible in docs/capabilities.db.
import Database from 'better-sqlite3'
import { existsSync, readdirSync } from 'node:fs'
import { join } from 'node:path'
import { CAPABILITIES_DB } from '../_paths.mjs'
import { scanDonorFiles } from './donor-file-census-lib.mjs'

const ROOT = process.cwd()
const TEMPORARY = join(ROOT, 'Temporary')
const DONOR_ARG = argValue('--donor')
const HASH = process.argv.includes('--hash')
const REPORT_ONLY = process.argv.includes('--report')

function argValue(name) {
  const i = process.argv.indexOf(name)
  return i >= 0 ? process.argv[i + 1] : null
}

function ensureSchema(db) {
  db.exec(`
CREATE TABLE IF NOT EXISTS donor_file_census (
  donor            TEXT NOT NULL,
  path             TEXT NOT NULL,
  is_directory     INTEGER DEFAULT 0,
  kind             TEXT NOT NULL,
  classification   TEXT NOT NULL,
  read_status      TEXT NOT NULL,
  mapped_source_ids TEXT,
  exclusion_reason TEXT,
  size_bytes       INTEGER,
  mtime_ms         INTEGER,
  sha256           TEXT,
  reviewed_by      TEXT,
  reviewed_at      TEXT,
  PRIMARY KEY (donor, path)
);
CREATE INDEX IF NOT EXISTS idx_dfc_donor ON donor_file_census(donor);
CREATE INDEX IF NOT EXISTS idx_dfc_class ON donor_file_census(classification);
CREATE INDEX IF NOT EXISTS idx_dfc_read ON donor_file_census(read_status);
CREATE INDEX IF NOT EXISTS idx_dfc_kind ON donor_file_census(kind);

CREATE TABLE IF NOT EXISTS source_file_capability_link (
  source_id       INTEGER NOT NULL,
  donor           TEXT NOT NULL,
  path            TEXT NOT NULL,
  symbol          TEXT,
  evidence_note   TEXT,
  PRIMARY KEY (source_id, donor, path, symbol)
);
CREATE INDEX IF NOT EXISTS idx_sfcl_file ON source_file_capability_link(donor, path);
CREATE INDEX IF NOT EXISTS idx_sfcl_source ON source_file_capability_link(source_id);
CREATE TABLE IF NOT EXISTS meta (k TEXT PRIMARY KEY, v TEXT);
`)
}

function donorDirs() {
  if (!existsSync(TEMPORARY)) throw new Error(`missing Temporary directory: ${TEMPORARY}`)
  const dirs = readdirSync(TEMPORARY, { withFileTypes: true })
    .filter(d => d.isDirectory())
    .map(d => d.name)
    .sort((a, b) => a.localeCompare(b))
  return DONOR_ARG ? dirs.filter(d => d === DONOR_ARG) : dirs
}

function summarize(db) {
  const totals = db.prepare(`SELECT
    count(*) rows,
    count(DISTINCT donor) donors,
    sum(CASE WHEN read_status='unread_pending' THEN 1 ELSE 0 END) unread_pending,
    sum(CASE WHEN classification='capability_review_pending' THEN 1 ELSE 0 END) capability_review_pending,
    sum(CASE WHEN classification='behavior_review_pending' THEN 1 ELSE 0 END) behavior_review_pending,
    sum(CASE WHEN classification='generated_vendor_build_artifact' THEN 1 ELSE 0 END) artifacts,
    sum(CASE WHEN classification='non_behavioral_support' THEN 1 ELSE 0 END) support
    FROM donor_file_census`).get()
  const byDonor = db.prepare(`SELECT donor, count(*) rows,
    sum(CASE WHEN read_status='unread_pending' THEN 1 ELSE 0 END) unread_pending,
    sum(CASE WHEN classification='generated_vendor_build_artifact' THEN 1 ELSE 0 END) artifacts
    FROM donor_file_census GROUP BY donor ORDER BY unread_pending DESC, rows DESC LIMIT 15`).all()
  return { totals, byDonor }
}

const db = new Database(CAPABILITIES_DB)
db.pragma('journal_mode = WAL')
ensureSchema(db)

if (!REPORT_ONLY) {
  const donors = donorDirs()
  if (DONOR_ARG && donors.length === 0) {
    console.error(`donor not found under Temporary/: ${DONOR_ARG}`)
    process.exit(2)
  }

  const insert = db.prepare(`INSERT OR REPLACE INTO donor_file_census
    (donor,path,is_directory,kind,classification,read_status,mapped_source_ids,exclusion_reason,size_bytes,mtime_ms,sha256,reviewed_by,reviewed_at)
    VALUES (@donor,@path,@isDirectory,@kind,@classification,@readStatus,@mappedSourceIds,@exclusionReason,@sizeBytes,@mtimeMs,@sha256,@reviewedBy,@reviewedAt)`)
  const del = db.prepare('DELETE FROM donor_file_census WHERE donor=?')
  const setMeta = db.prepare('INSERT OR REPLACE INTO meta(k,v) VALUES(?,?)')
  const tx = db.transaction((donor, rows) => {
    del.run(donor)
    for (const row of rows) {
      insert.run({
        ...row,
        reviewedBy: row.readStatus === 'classified_not_read' ? 'donor-file-census:auto' : null,
        reviewedAt: row.readStatus === 'classified_not_read' ? new Date().toISOString() : null,
      })
    }
  })

  for (const donor of donors) {
    const rows = scanDonorFiles(join(TEMPORARY, donor), donor, { hashFiles: HASH })
    tx(donor, rows)
    console.log(`${donor}: ${rows.length} census rows`)
  }

  const totals = summarize(db).totals
  setMeta.run('file_census_built_at', new Date().toISOString())
  setMeta.run('file_census_donors_scanned', String(totals.donors || 0))
  setMeta.run('file_census_rows', String(totals.rows || 0))
  setMeta.run('file_census_unread_pending', String(totals.unread_pending || 0))
}

const summary = summarize(db)
console.log(JSON.stringify(summary, null, 2))
db.close()
