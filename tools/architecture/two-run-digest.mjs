#!/usr/bin/env node
// Deterministic two-run architecture-DB evidence tool.
//
// Usage:
//   node tools/architecture/two-run-digest.mjs <db1> <db2> [--out <json-path>]
//
// For each of the two SQLite database paths, computes:
//   - raw_sha256:        SHA-256 of the file's exact bytes on disk.
//   - normalized_sha256: SHA-256 of a fully specified, deterministic canonical serialization (see
//     the CANONICAL SERIALIZATION SPEC below) that excludes the specific fields
//     `tools/architecture/build-db.mjs` re-stamps on every rebuild even when a row's own content
//     is otherwise unchanged (autoincrement `id` columns on every table; the `meta` row keyed
//     `generated_at`; `architecture_evidence.verified_at`).
//
// This tool never claims the two DBs' raw bytes are identical -- raw digests legitimately differ
// across any two rebuilds because of those re-stamped fields. Only the normalized digest is a
// meaningful cross-run invariant, and this file is the single, git-tracked source of truth for
// exactly how that digest is computed, so its value can be independently re-derived by reading
// this file (not by trusting an unlogged, one-off script) and by re-running this exact command
// against a fresh rebuild pair from the same source.
//
// CANONICAL SERIALIZATION SPEC (exact, versioned -- see SPEC_VERSION):
//   1. Enumerate tables via `SELECT name FROM sqlite_master WHERE type='table' ORDER BY name ASC`
//      (deterministic alphabetical order, case-sensitive byte comparison).
//   2. For each table, enumerate columns via `PRAGMA table_info(<table>)`, keep only columns NOT
//      in that table's EXCLUDED_COLUMNS set (below), sorted alphabetically by column name
//      (case-sensitive byte comparison) -- NOT physical schema/definition order, so the spec does
//      not depend on column-declaration order ever staying the same.
//   3. EXCLUDED_COLUMNS: every table excludes a column literally named `id`. The
//      `architecture_evidence` table additionally excludes `verified_at`. No other column, on any
//      table, is excluded.
//   4. Row selection/order: `SELECT <kept columns, in the sorted order from step 2> FROM <table>
//      ORDER BY <the same columns, same order>` -- a deterministic row order independent of
//      physical insertion/rowid order. EXCEPTION: `meta` is a key-value table (columns `k`, `v`,
//      neither literally named `id`, so both are kept); after row selection/ordering, rows are
//      additionally filtered to drop any row where `k = 'generated_at'` -- a VALUE-level
//      exclusion (that timestamp lives in `meta.v`, not in a same-named column), needed because
//      `meta`'s own re-stamped generation timestamp is stored as one row, not one column.
//   5. Row encoding: each kept column value is encoded via `JSON.stringify(value)`, so SQL NULL
//      becomes the 4-byte string `null`, INTEGER/REAL values render as their JSON numeral, and
//      TEXT renders as a JSON string with standard JSON escaping. A BLOB value (not currently used
//      by this schema, but handled for completeness) is hex-encoded to a string before
//      `JSON.stringify`. Encoded column values within one row are joined with a single 0x1F byte
//      (ASCII Unit Separator, never produced by `JSON.stringify`) so column boundaries are
//      unambiguous even for strings containing arbitrary content.
//   6. Row framing: each encoded row is followed by a single 0x0A (newline) byte.
//   7. Table framing: each table's row block is preceded by the literal ASCII line
//      `TABLE:<table-name>\n` -- table names are fixed SQL identifiers in this schema, never
//      caller-supplied, so no escaping is applied to them.
//   8. The canonical byte sequence is the concatenation, in table order (step 1), of each table's
//      framing line (step 7) followed by its ordered, encoded, newline-terminated rows (steps
//      4-6). SHA-256 of this exact byte sequence, rendered as 64 lowercase hex characters, is the
//      normalized digest.

import { createHash } from 'node:crypto'
import { readFileSync, writeFileSync } from 'node:fs'
import { pathToFileURL } from 'node:url'
import Database from 'better-sqlite3'

export const SPEC_VERSION = '1'

const DEFAULT_EXCLUDED_COLUMNS = new Set(['id'])
const TABLE_EXCLUDED_COLUMNS = {
  architecture_evidence: new Set(['id', 'verified_at']),
}

function excludedColumnsFor(table) {
  return TABLE_EXCLUDED_COLUMNS[table] ?? DEFAULT_EXCLUDED_COLUMNS
}

function encodeValue(value) {
  if (Buffer.isBuffer(value)) {
    return JSON.stringify(value.toString('hex'))
  }
  return JSON.stringify(value)
}

/** SHA-256 of the exact file bytes on disk, as 64 lowercase hex characters. */
export function rawSha256(dbPath) {
  const bytes = readFileSync(dbPath)
  return createHash('sha256').update(bytes).digest('hex')
}

/** SHA-256 of the canonical normalized serialization described above, as 64 lowercase hex characters. */
export function normalizedSha256(dbPath) {
  const db = new Database(dbPath, { readonly: true })
  try {
    const hash = createHash('sha256')
    const tables = db
      .prepare("SELECT name FROM sqlite_master WHERE type='table' ORDER BY name ASC")
      .all()
      .map((r) => r.name)
    for (const table of tables) {
      hash.update(`TABLE:${table}\n`)
      const excluded = excludedColumnsFor(table)
      const allColumns = db
        .prepare(`PRAGMA table_info("${table}")`)
        .all()
        .map((c) => c.name)
      const cols = allColumns.filter((c) => !excluded.has(c)).sort()
      if (cols.length === 0) continue
      const colList = cols.map((c) => `"${c}"`).join(', ')
      let rows = db.prepare(`SELECT ${colList} FROM "${table}" ORDER BY ${colList}`).all()
      if (table === 'meta') {
        rows = rows.filter((r) => r.k !== 'generated_at')
      }
      for (const row of rows) {
        const encoded = cols.map((c) => encodeValue(row[c])).join('')
        hash.update(encoded)
        hash.update('\n')
      }
    }
    return hash.digest('hex')
  } finally {
    db.close()
  }
}

/** Table-level row counts (post row-selection, pre `meta` value filtering) -- diagnostic only. */
export function tableCounts(dbPath) {
  const db = new Database(dbPath, { readonly: true })
  try {
    const tables = db
      .prepare("SELECT name FROM sqlite_master WHERE type='table' ORDER BY name ASC")
      .all()
      .map((r) => r.name)
    const counts = {}
    for (const table of tables) {
      counts[table] = db.prepare(`SELECT count(*) n FROM "${table}"`).get().n
    }
    return counts
  } finally {
    db.close()
  }
}

/** Compute the full evidence record for a run1/run2 database pair. */
export function digestPair(db1Path, db2Path) {
  const run1 = {
    path: db1Path,
    raw_sha256: rawSha256(db1Path),
    normalized_sha256: normalizedSha256(db1Path),
    counts: tableCounts(db1Path),
  }
  const run2 = {
    path: db2Path,
    raw_sha256: rawSha256(db2Path),
    normalized_sha256: normalizedSha256(db2Path),
    counts: tableCounts(db2Path),
  }
  return {
    tool: 'tools/architecture/two-run-digest.mjs',
    spec_version: SPEC_VERSION,
    run1,
    run2,
    raw_identical: run1.raw_sha256 === run2.raw_sha256,
    normalized_identical: run1.normalized_sha256 === run2.normalized_sha256,
  }
}

function main() {
  const args = process.argv.slice(2).filter((a) => a !== '--')
  const outIdx = args.indexOf('--out')
  const outPath = outIdx >= 0 ? args[outIdx + 1] : null
  const positional = args.filter((a, i) => a !== '--out' && i !== outIdx + 1)
  const [db1Path, db2Path] = positional
  if (!db1Path || !db2Path) {
    console.error('usage: node tools/architecture/two-run-digest.mjs <db1> <db2> [--out <json-path>]')
    process.exit(2)
  }

  const result = digestPair(db1Path, db2Path)
  console.log(`two-run-digest: spec_version=${result.spec_version}`)
  console.log(`  run1 (${db1Path}): raw=${result.run1.raw_sha256} normalized=${result.run1.normalized_sha256}`)
  console.log(`  run2 (${db2Path}): raw=${result.run2.raw_sha256} normalized=${result.run2.normalized_sha256}`)
  console.log(`  raw_identical=${result.raw_identical} normalized_identical=${result.normalized_identical}`)
  if (!result.normalized_identical) {
    console.error('  ✗ normalized digests differ -- content is NOT deterministic across these two runs')
    if (outPath) writeFileSync(outPath, JSON.stringify(result, null, 2) + '\n')
    process.exit(1)
  }
  console.log('  ✓ normalized digests match -- content is deterministic across these two runs')
  if (outPath) {
    writeFileSync(outPath, JSON.stringify(result, null, 2) + '\n')
    console.log(`  wrote ${outPath}`)
  }
}

// `file://${process.argv[1]}` breaks on Windows: a drive-letter path like `C:\foo\bar.mjs`
// doesn't round-trip through that naive template into a valid `file:` URL (backslashes are left
// unescaped and the drive letter collides with the URL scheme separator), so the CLI would
// silently never run. `pathToFileURL` builds the URL the platform-correct way on every OS.
const isMain = process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href
if (isMain) {
  main()
}
