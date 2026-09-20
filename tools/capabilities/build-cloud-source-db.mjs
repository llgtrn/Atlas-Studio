#!/usr/bin/env node
// build-cloud-source-db.mjs - assemble a capabilities.db-shaped SQLite cache
// entirely from git-tracked sources, for export-cloud-snapshot.mjs to read
// when the local-only docs/capabilities.db (~309MB, never git-tracked) is not
// present. Two git-tracked inputs are combined:
//
//   docs/capabilities-canonical/domains/**/*.jsonl   - canonical capability shards
//                                                       (tools/capabilities/build-db-from-canonical-shards.mjs
//                                                       already rebuilds canonical_capability + impl_evidence
//                                                       + a provenance/source_capability backbone from these)
//   docs/capabilities-canonical/local-authority/*.jsonl - raw dumps of local-only
//                                                       tables that have no other
//                                                       git mirror (PR #2608)
//
// IMPORTANT correctness note: local-authority/provenance.jsonl and
// local-authority/source_capability.jsonl carry canonical_id values from the
// ORIGINAL local docs/capabilities.db's canonical_capability.id numbering.
// The canonical_capability table rebuilt here from domains/*.jsonl assigns
// FRESH ids (sorted by capability_key - domains/*.jsonl never carried the
// original numeric ids). Reusing the local-authority dump's canonical_id
// values directly would silently mis-join source rows to the wrong capability.
// So: canonical_capability/impl_evidence/provenance/source_capability's
// identity+FK columns stay exactly as build-db-from-canonical-shards.mjs
// derives them (self-consistent within this rebuild); local-authority's
// source_capability.jsonl is only used to backfill descriptive columns
// (business_behavior, technical_behavior, inputs, outputs, persistence,
// surface, external_services) by matching on source_capability.id, which IS
// stable (it round-trips through the source_id=<n> token embedded in each
// domain shard record's source_refs).
//
// slice_canonical and canonical_status_override key off canonical_key (TEXT),
// not a numeric id, so they load directly from the local-authority dump.
import Database from 'better-sqlite3'
import { rmSync } from 'node:fs'
import { fileURLToPath } from 'node:url'
import { buildCapabilityDbFromCanonicalShards } from './build-db-from-canonical-shards.mjs'
import {
  createTableFromLocalAuthorityRows,
  loadLocalAuthorityMeta,
  loadLocalAuthorityShardedTable,
  loadLocalAuthorityTable,
} from './local-authority-lib.mjs'
import { parseArgs, rel } from '../canonical-shards/jsonl-lib.mjs'

const ENRICHABLE_SOURCE_COLUMNS = [
  'source_files',
  'source_symbols',
  'business_behavior',
  'technical_behavior',
  'inputs',
  'outputs',
  'persistence',
  'surface',
  'external_services',
]

// Tables that key off a TEXT capability_key/canonical_key (no numeric FK risk)
// and can be loaded wholesale from the local-authority dump. Each fallback
// schema is used only when the local-authority dump for that table is
// missing, so the shard still gets a validly-columned (if empty) table -
// export-cloud-snapshot.mjs indexes slice_canonical.canonical_key
// unconditionally and would fail on a zero-column table.
const DIRECT_LOAD_TABLES = {
  capability: [{ name: 'key', type: 'TEXT' }],
  canonical_status_override: [{ name: 'canonical_key', type: 'TEXT' }],
  slice_canonical: [
    { name: 'slice', type: 'INTEGER' },
    { name: 'canonical_key', type: 'TEXT' },
    { name: 'match_method', type: 'TEXT' },
    { name: 'confidence', type: 'TEXT' },
    { name: 'reviewed', type: 'INTEGER' },
    { name: 'note', type: 'TEXT' },
  ],
}

function createEmptyTable(db, table, columns) {
  const q = (name) => `"${String(name).replace(/"/g, '""')}"`
  db.exec(`DROP TABLE IF EXISTS ${q(table)}`)
  db.exec(`CREATE TABLE ${q(table)} (${columns.map((col) => `${q(col.name)} ${col.type}`).join(', ')})`)
}

function enrichSourceCapability(db, localRows) {
  if (!localRows?.length) return { enriched: 0 }
  const byId = new Map(localRows.map((row) => [row.id, row]))
  const existing = db.prepare('SELECT id FROM source_capability').all()
  const update = db.prepare(
    `UPDATE source_capability SET ${ENRICHABLE_SOURCE_COLUMNS.map((c) => `"${c}" = ?`).join(', ')} WHERE id = ?`,
  )
  let enriched = 0
  const tx = db.transaction(() => {
    for (const { id } of existing) {
      const local = byId.get(id)
      if (!local) continue
      update.run(...ENRICHABLE_SOURCE_COLUMNS.map((c) => local[c] ?? null), id)
      enriched += 1
    }
  })
  tx()
  return { enriched, local_authority_rows: localRows.length }
}

export function buildCapabilitySourceDbFromGit({ root = process.cwd(), outPath = null } = {}) {
  const base = buildCapabilityDbFromCanonicalShards({ root, outPath })
  const db = new Database(base.dbPath)

  const localAuthorityMeta = loadLocalAuthorityMeta(root)
  const gaps = {}
  const loaded = {}

  for (const [table, fallbackColumns] of Object.entries(DIRECT_LOAD_TABLES)) {
    const rows = loadLocalAuthorityTable(root, table)
    if (rows == null) {
      gaps[table] = `docs/capabilities-canonical/local-authority/${table}.jsonl not present`
      if (table !== 'canonical_status_override') createEmptyTable(db, table, fallbackColumns)
      continue
    }
    createTableFromLocalAuthorityRows(db, table, rows)
    loaded[table] = rows.length
  }

  const sourceCapabilityRows = loadLocalAuthorityTable(root, 'source_capability')
  if (sourceCapabilityRows == null) {
    gaps.source_capability_enrichment = 'docs/capabilities-canonical/local-authority/source_capability.jsonl not present; source_capability rows carry only the fields embedded in domain shard source_refs'
  } else {
    const result = enrichSourceCapability(db, sourceCapabilityRows)
    loaded.source_capability_enrichment = result
  }

  // donor_file_census / source_file_capability_link are donor-sharded
  // directories (PR #2612: docs/capabilities-canonical/local-authority/<table>/<donor>.jsonl),
  // not single <table>.jsonl files - loaded wholesale, same as DIRECT_LOAD_TABLES,
  // just via the sharded reader.
  for (const table of ['donor_file_census', 'source_file_capability_link']) {
    const rows = loadLocalAuthorityShardedTable(root, table)
    if (rows == null) {
      const notYetExported = localAuthorityMeta?.not_yet_exported?.[table]
      gaps[table] = notYetExported
        ? `not git-tracked: ${notYetExported.reason} (rows=${notYetExported.rows}, size_bytes_approx=${notYetExported.size_bytes_approx})`
        : 'not git-tracked: no canonical JSONL mirror exists for this table yet'
      continue
    }
    createTableFromLocalAuthorityRows(db, table, rows)
    loaded[table] = rows.length
  }

  for (const table of ['agent_note']) {
    const notYetExported = localAuthorityMeta?.not_yet_exported?.[table]
    gaps[table] = notYetExported
      ? `not git-tracked: ${notYetExported.reason} (rows=${notYetExported.rows}, size_bytes_approx=${notYetExported.size_bytes_approx})`
      : 'not git-tracked: no canonical JSONL mirror exists for this table yet'
  }

  const meta = db.prepare('SELECT k, v FROM meta').all()
  const hasMetaKey = new Set(meta.map((row) => row.k))
  const insertMeta = db.prepare('INSERT OR REPLACE INTO meta (k, v) VALUES (?, ?)')
  if (!hasMetaKey.has('source_mode')) insertMeta.run('source_mode', 'git_canonical_shards')
  insertMeta.run('local_authority_gaps', JSON.stringify(gaps))
  insertMeta.run('local_authority_loaded', JSON.stringify(loaded))

  db.close()
  return { dbPath: base.dbPath, tempDir: base.tempDir, counts: base.counts, gaps, loaded }
}

function main() {
  const args = parseArgs(process.argv.slice(2))
  const outPath = args.out ? String(args.out) : null
  if (outPath) rmSync(outPath, { force: true })
  const result = buildCapabilitySourceDbFromGit({ root: process.cwd(), outPath })
  console.log(JSON.stringify({
    ok: true,
    db_path: result.tempDir ? result.dbPath : rel(process.cwd(), result.dbPath),
    temp_cache: Boolean(result.tempDir),
    counts: result.counts,
    gaps: result.gaps,
    loaded: result.loaded,
  }, null, 2))
}

if (process.argv[1] && fileURLToPath(import.meta.url) === process.argv[1]) {
  main()
}
