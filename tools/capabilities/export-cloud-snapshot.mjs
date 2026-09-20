#!/usr/bin/env node
// export-cloud-snapshot.mjs - build small, git-trackable SQLite shards.
//
// Two source modes:
//   local_capabilities_db  - read the local-only docs/capabilities.db (~309MB,
//                             never git-tracked) and docs/architecture.db, when
//                             present. This remains the richest source.
//   git_canonical_shards   - when docs/capabilities.db / docs/architecture.db
//                             are absent (any fresh checkout, cloud environment),
//                             rebuild equivalent caches from git-tracked
//                             docs/capabilities-canonical/**/*.jsonl and
//                             docs/architecture-canonical/**/*.jsonl instead, so
//                             this script - and its downstream consumer
//                             subdb:gen - work standalone from git alone. See
//                             tools/capabilities/build-cloud-source-db.mjs for
//                             exactly which tables are and aren't reproducible
//                             this way; genuine gaps (donor_file_census,
//                             source_file_capability_link, agent_note - none of
//                             which are git-tracked yet) are recorded in
//                             meta.json's git_source_gaps, not silently emptied.
//
// Usage:
//   node tools/capabilities/export-cloud-snapshot.mjs
//   node tools/capabilities/export-cloud-snapshot.mjs --pending-limit 20000 --link-limit 20000
import Database from 'better-sqlite3'
import { existsSync, mkdirSync, statSync, unlinkSync, writeFileSync, readFileSync, readdirSync, rmSync } from 'node:fs'
import { basename, join, relative } from 'node:path'
import { createHash } from 'node:crypto'
import { ARCHITECTURE_DB, CAPABILITIES_DB, DOCS, GENERATED, CRATE_DOCS } from '../_paths.mjs'
import { buildCapabilitySourceDbFromGit } from './build-cloud-source-db.mjs'
import { buildArchitectureDbFromCanonicalShards } from '../architecture/build-db-from-canonical-shards.mjs'

const DEFAULT_OUT_DIR = join(DOCS, 'capabilities-cloud')
const args = process.argv.slice(2)

const readArg = (name, fallback) => {
  const idx = args.indexOf(name)
  if (idx === -1) return fallback
  return args[idx + 1] ?? fallback
}

const hasFlag = (name) => args.includes(name)
const OUT_DIR = readArg('--out', DEFAULT_OUT_DIR)
const SOURCE_EXPLICIT = hasFlag('--source')
const ARCH_EXPLICIT = hasFlag('--arch')
let SOURCE_DB = readArg('--source', CAPABILITIES_DB)
let ARCH_DB = readArg('--arch', ARCHITECTURE_DB)
const PENDING_LIMIT = Number(readArg('--pending-limit', '20000'))
const LINK_LIMIT = Number(readArg('--link-limit', '20000'))
const GENERATED_AT = new Date().toISOString()

const SHARDS = {
  core: 'cap-core.db',
  provenance: 'cap-provenance.db',
  census: 'cap-census-summary.db',
  architecture: 'cap-architecture.db',
  workqueue: 'cap-workqueue.db',
}

const q = (name) => `"${String(name).replace(/"/g, '""')}"`
const sqlString = (value) => `'${String(value).replace(/'/g, "''")}'`
const rel = (path) => relative(process.cwd(), path).replace(/\\/g, '/')

const printUsage = () => {
  console.log(`export Chronica cloud capability shards

Options:
  --source <path>          source SQLite DB (default docs/capabilities.db; falls
                            back to a git-tracked-shard rebuild when absent and
                            --source is not passed explicitly)
  --arch <path>            architecture SQLite DB (default docs/architecture.db;
                            same git-tracked-shard fallback as --source)
  --out <dir>              output directory (default docs/capabilities-cloud)
  --pending-limit <n>      pending file-census sample size (default 20000)
  --link-limit <n>         source-file link sample size (default 20000)
  --help                   show this help`)
}

if (hasFlag('--help')) {
  printUsage()
  process.exit(0)
}

let SOURCE_MODE = 'local_capabilities_db'
let CAPABILITY_GIT_BUILD = null
let ARCHITECTURE_GIT_BUILD = null
let ARCHITECTURE_GIT_BUILD_ERROR = null

if (!existsSync(SOURCE_DB) && !SOURCE_EXPLICIT) {
  console.log(`export-cloud-snapshot: ${rel(SOURCE_DB)} not present; rebuilding capability source from git-tracked canonical shards`)
  CAPABILITY_GIT_BUILD = buildCapabilitySourceDbFromGit({ root: process.cwd() })
  SOURCE_DB = CAPABILITY_GIT_BUILD.dbPath
  SOURCE_MODE = 'git_canonical_shards'
}

if (!existsSync(SOURCE_DB)) {
  console.error(`missing source DB: ${rel(SOURCE_DB)}`)
  process.exit(2)
}

if (!existsSync(ARCH_DB) && !ARCH_EXPLICIT) {
  try {
    console.log(`export-cloud-snapshot: ${rel(ARCH_DB)} not present; rebuilding architecture source from git-tracked canonical shards`)
    ARCHITECTURE_GIT_BUILD = buildArchitectureDbFromCanonicalShards({ root: process.cwd() })
    ARCH_DB = ARCHITECTURE_GIT_BUILD.dbPath
  } catch (error) {
    ARCHITECTURE_GIT_BUILD_ERROR = error.message
    console.error(`export-cloud-snapshot: could not rebuild architecture source from docs/architecture-canonical: ${error.message}`)
  }
}

const removeIfExists = (path) => {
  if (existsSync(path)) unlinkSync(path)
}

const openWritableShard = (fileName) => {
  mkdirSync(OUT_DIR, { recursive: true })
  const shardPath = join(OUT_DIR, fileName)
  removeIfExists(shardPath)
  const db = new Database(shardPath)
  db.pragma('journal_mode = DELETE')
  db.pragma('synchronous = OFF')
  db.pragma('temp_store = MEMORY')
  return db
}

const attach = (db, path, alias = 'src') => {
  db.exec(`ATTACH DATABASE ${sqlString(path)} AS ${q(alias)}`)
}

const detach = (db, alias = 'src') => {
  db.exec(`DETACH DATABASE ${q(alias)}`)
}

const tableExists = (db, table, schema = 'main') => {
  return db
    .prepare(`SELECT count(*) n FROM ${q(schema)}.sqlite_master WHERE type='table' AND name=?`)
    .get(table).n > 0
}

const sourceTableExists = (path, table) => {
  if (!existsSync(path)) return false
  const db = new Database(path, { readonly: true })
  const exists = tableExists(db, table)
  db.close()
  return exists
}

const createTableFromRows = (db, table, rows, columns) => {
  db.exec(`DROP TABLE IF EXISTS ${q(table)}`)
  db.exec(`CREATE TABLE ${q(table)} (${columns.map((col) => `${q(col.name)} ${col.type || 'TEXT'}`).join(', ')})`)
  if (!rows.length) return
  const names = columns.map((col) => col.name)
  const insert = db.prepare(
    `INSERT INTO ${q(table)} (${names.map(q).join(', ')}) VALUES (${names.map((name) => `@${name}`).join(', ')})`,
  )
  const tx = db.transaction(() => {
    for (const row of rows) insert.run(row)
  })
  tx()
}

const parseGeneratedCapabilityDocs = () => {
  const rows = []
  const seen = new Set()
  const normalize = (value) => {
    const text = String(value ?? '').trim()
    return text && text !== 'none' ? text : null
  }
  const addRow = (row) => {
    if (!row.key || seen.has(row.key)) return
    seen.add(row.key)
    rows.push({ id: rows.length + 1, ...row })
  }
  const capDocs = readdirSync(GENERATED)
    .filter((file) => /^\d+-cap-.*\.md$/.test(file))
    .sort()
  for (const file of capDocs) {
    const text = readFileSync(join(GENERATED, file), 'utf8')
    const domain = text.match(/\| Domain \| ([^|]+) \|/)?.[1]?.trim() || file.replace(/^\d+-cap-/, '').replace(/\.md$/, '')
    for (const line of text.split(/\r?\n/)) {
      if (!line.startsWith('- [')) continue
      const keyMatch = line.match(/`([^`]+)`/)
      if (!keyMatch) continue
      const key = keyMatch[1]
      const before = line.slice(0, keyMatch.index)
      const after = line.slice(keyMatch.index + keyMatch[0].length)
      const title = after.match(/^\s*\u2014\s*(.*?)\s*\u2192\s*/u)?.[1]?.trim() || key
      const target = after.match(/\u2192\s*(.*?)\s*(?:\u00b7|$)/u)?.[1]?.trim() || ''
      const [targetCrate = '', targetModule = ''] = target.split('::')
      const money = /\u00b7\s*money\b/u.test(after) ? 1 : 0
      const finTest = money ? after.match(/\u{1F4B0}\s*(.*)$/u)?.[1]?.trim() || 'REQUIRED (local audit)' : null
      const status = before.includes('unverified') || before.includes('implemented') ? 'implemented_unverified'
        : before.includes('verified') ? 'verified'
          : before.includes('blocked') ? 'blocked'
            : before.includes('excluded') ? 'excluded' : 'unimplemented'
      addRow({
        key,
        canonical_name: title,
        domain,
        target_crate: targetCrate,
        target_module: targetModule,
        side_effect_class: money ? 'money' : after.match(/\u00b7\s*(read_only|internal_write|artifact_write|external_read|external_write|approval|side_effect)\b/u)?.[1] || '',
        moves_money: money,
        requires_approval: money ? 1 : 0,
        acceptance_criteria: money
          ? `Local audit must prove ${title} runs policy -> approval -> cost/audit chain before any money side effect.`
          : `Local audit must prove ${title} is implemented natively with behavior and failure-path tests.`,
        required_tests: null,
        financial_control_test: finTest,
        acceptance_test: null,
        status,
        exclusion_note: null,
        blocker: status === 'blocked' ? 'See generated roadmap line and local capabilities.db audit.' : null,
        slice: null,
        donor_count: 0,
      })
    }
  }
  const crateDocs = existsSync(CRATE_DOCS)
    ? readdirSync(CRATE_DOCS)
      .filter((file) => /^\d+-crate-.*\.md$/.test(file))
      .sort()
    : []
  for (const file of crateDocs) {
    const text = readFileSync(join(CRATE_DOCS, file), 'utf8')
    for (const line of text.split(/\r?\n/)) {
      if (!line.startsWith('| ')) continue
      const cells = line.split('|').slice(1, -1).map((cell) => cell.trim())
      if (cells.length < 7) continue
      const [key, statusRaw, movesRaw, sideRaw, targetCrateRaw, targetModuleRaw, acceptanceRaw] = cells
      if (!/^[a-z0-9][a-z0-9._-]*\.[a-z0-9][a-z0-9._-]*$/i.test(key)) continue
      if (seen.has(key)) continue
      const statusText = String(statusRaw || '').trim()
      const statusValue = statusText.toLowerCase()
      const status = statusValue === 'verified'
        ? 'verified'
        : statusValue === 'implemented_unverified' || statusValue === 'unverified'
          ? 'implemented_unverified'
          : statusValue === 'blocked'
            ? 'blocked'
            : statusValue === 'excluded'
              ? 'excluded'
              : 'unimplemented'
      const movesMoney = Number(movesRaw) === 1 ? 1 : 0
      const domain = key.split('.')[0]
      const title = key.split('.').slice(1).join(' ').replace(/_/g, ' ') || key
      addRow({
        key,
        canonical_name: title.charAt(0).toUpperCase() + title.slice(1),
        domain,
        target_crate: normalize(targetCrateRaw),
        target_module: normalize(targetModuleRaw),
        side_effect_class: normalize(sideRaw) || (movesMoney ? 'money' : 'internal_write'),
        moves_money: movesMoney,
        requires_approval: movesMoney ? 1 : 0,
        acceptance_criteria: normalize(acceptanceRaw)
          ? `Local audit must prove ${key} with ${acceptanceRaw}.`
          : `Local audit must prove ${key} is implemented natively with behavior and failure-path tests.`,
        required_tests: null,
        financial_control_test: movesMoney ? 'REQUIRED (local audit)' : null,
        acceptance_test: normalize(acceptanceRaw),
        status,
        exclusion_note: null,
        blocker: status === 'blocked' ? 'See generated crate doc and local capabilities.db audit.' : null,
        slice: null,
        donor_count: 0,
      })
    }
  }
  return rows
}

const createCanonicalFromGeneratedDocs = (db) => {
  const rows = parseGeneratedCapabilityDocs()
  db.exec(`
DROP TABLE IF EXISTS canonical_capability;
CREATE TABLE canonical_capability (
  id INTEGER PRIMARY KEY,
  key TEXT UNIQUE NOT NULL,
  canonical_name TEXT NOT NULL,
  domain TEXT,
  target_crate TEXT,
  target_module TEXT,
  side_effect_class TEXT,
  moves_money INTEGER DEFAULT 0,
  requires_approval INTEGER DEFAULT 0,
  acceptance_criteria TEXT,
  required_tests TEXT,
  financial_control_test TEXT,
  acceptance_test TEXT,
  status TEXT DEFAULT 'unimplemented',
  exclusion_note TEXT,
  blocker TEXT,
  slice INTEGER,
  donor_count INTEGER DEFAULT 0
);
`)
  const insert = db.prepare(`
INSERT INTO canonical_capability (
  id, key, canonical_name, domain, target_crate, target_module, side_effect_class,
  moves_money, requires_approval, acceptance_criteria, required_tests, financial_control_test,
  acceptance_test, status, exclusion_note, blocker, slice, donor_count
) VALUES (
  @id, @key, @canonical_name, @domain, @target_crate, @target_module, @side_effect_class,
  @moves_money, @requires_approval, @acceptance_criteria, @required_tests, @financial_control_test,
  @acceptance_test, @status, @exclusion_note, @blocker, @slice, @donor_count
)`)
  const tx = db.transaction(() => {
    for (const row of rows) insert.run(row)
  })
  tx()
  return rows.length
}

const copyTable = (db, table, columns = ['*'], options = {}) => {
  const sourcePath = options.sourcePath || SOURCE_DB
  const source = new Database(sourcePath, { readonly: true })
  if (!tableExists(source, table)) {
    source.close()
    throw new Error(`source table missing: ${rel(sourcePath)}:${table}`)
  }
  const sourceColumns = source.prepare(`PRAGMA table_info(${q(table)})`).all()
  const selectedColumns =
    columns[0] === '*'
      ? sourceColumns.map((col) => ({ name: col.name, type: col.type || 'TEXT' }))
      : columns.map((name) => {
          const found = sourceColumns.find((col) => col.name === name)
          if (!found) throw new Error(`source column missing: ${table}.${name}`)
          return { name: found.name, type: found.type || 'TEXT' }
        })
  const cols = selectedColumns.map((col) => q(col.name)).join(', ')
  const where = options.where ? ` WHERE ${options.where}` : ''
  const order = options.order ? ` ORDER BY ${options.order}` : ''
  const limit = options.limit ? ` LIMIT ${Number(options.limit)}` : ''
  const rows = source.prepare(`SELECT ${cols} FROM ${q(table)}${where}${order}${limit}`).all()
  source.close()
  createTableFromRows(db, table, rows, selectedColumns)
}

const createMetaTable = (db, shardName) => {
  db.exec(`
CREATE TABLE cloud_snapshot_info (
  key TEXT PRIMARY KEY,
  value TEXT NOT NULL
);
`)
  const insert = db.prepare('INSERT INTO cloud_snapshot_info(key, value) VALUES (?, ?)')
  insert.run('schema_version', '1')
  insert.run('shard', shardName)
  insert.run('generated_at', GENERATED_AT)
  insert.run('source_mode', SOURCE_MODE)
  insert.run('source_db', SOURCE_MODE === 'git_canonical_shards' ? 'docs/capabilities-canonical/**/*.jsonl' : 'docs/capabilities.db')
  insert.run('architecture_db', ARCHITECTURE_GIT_BUILD ? 'docs/architecture-canonical/**/*.jsonl' : 'docs/architecture.db')
  insert.run('authority', 'cloud-readable snapshot only; local docs/capabilities.db remains audit authority')
}

const finishShard = (db) => {
  db.exec('ANALYZE')
  db.exec('VACUUM')
  db.close()
}

const readSourceCounts = () => {
  const db = new Database(SOURCE_DB, { readonly: true })
  const has = (name) => tableExists(db, name)
  const hasCanonical = has('canonical_capability')
  const syntheticCanonicalIds = has('meta') && db.prepare("SELECT count(*) n FROM meta WHERE k IN ('canonical_recovery','synthetic_canonical_ids')").get().n > 0
  const canonicalFromLocalDb = hasCanonical && !syntheticCanonicalIds
  const docsRows = hasCanonical ? [] : parseGeneratedCapabilityDocs()
  const meta = has('meta')
    ? Object.fromEntries(db.prepare('SELECT k, v FROM meta').all().map((r) => [r.k, r.v]))
    : {}
  const canonicalSourceLabel = SOURCE_MODE === 'git_canonical_shards'
    ? 'docs/capabilities-canonical/domains/**/*.jsonl (git_canonical_shards mode)'
    : canonicalFromLocalDb ? 'docs/capabilities.db' : 'generated capability docs fallback'
  const counts = {
    source_db: rel(SOURCE_DB),
    generated_at: GENERATED_AT,
    canonical_source: canonicalSourceLabel,
    canonical_source_warning: SOURCE_MODE === 'git_canonical_shards'
      ? null
      : canonicalFromLocalDb
        ? null
        : 'source docs/capabilities.db is missing canonical_capability; cloud snapshot canonical rows were recovered from generated docs and require local audit before truth claims',
    meta,
    canonical_capabilities: has('canonical_capability')
      ? db.prepare('SELECT count(*) n FROM canonical_capability').get().n
      : docsRows.length,
    source_capabilities: has('source_capability')
      ? db.prepare('SELECT count(*) n FROM source_capability').get().n
      : 0,
    provenance_rows: has('provenance') ? db.prepare('SELECT count(*) n FROM provenance').get().n : 0,
    money_canonical: has('canonical_capability')
      ? db.prepare('SELECT count(*) n FROM canonical_capability WHERE moves_money=1').get().n
      : docsRows.filter((row) => row.moves_money === 1).length,
    money_without_approval: has('canonical_capability')
      ? db
          .prepare(
            "SELECT count(*) n FROM canonical_capability WHERE moves_money=1 AND COALESCE(requires_approval,0) != 1",
          )
          .get().n
      : docsRows.filter((row) => row.moves_money === 1 && row.requires_approval !== 1).length,
    verified_canonical: has('canonical_capability')
      ? db.prepare("SELECT count(*) n FROM canonical_capability WHERE status='verified'").get().n
      : docsRows.filter((row) => row.status === 'verified').length,
    impl_evidence_rows: has('impl_evidence') ? db.prepare('SELECT count(*) n FROM impl_evidence').get().n : 0,
    donor_file_census_rows: has('donor_file_census')
      ? db.prepare('SELECT count(*) n FROM donor_file_census').get().n
      : 0,
    donor_file_census_unread_pending: has('donor_file_census')
      ? db.prepare("SELECT count(*) n FROM donor_file_census WHERE read_status='unread_pending'").get().n
      : 0,
    source_file_capability_link_rows: has('source_file_capability_link')
      ? db.prepare('SELECT count(*) n FROM source_file_capability_link').get().n
      : 0,
  }
  db.close()
  return counts
}

const readArchitectureCounts = () => {
  if (!existsSync(ARCH_DB)) return { architecture_db: rel(ARCH_DB), present: false }
  const db = new Database(ARCH_DB, { readonly: true })
  const has = (name) => tableExists(db, name)
  const meta = has('meta')
    ? Object.fromEntries(db.prepare('SELECT k, v FROM meta').all().map((r) => [r.k, r.v]))
    : {}
  const counts = {
    architecture_db: rel(ARCH_DB),
    present: true,
    meta,
    capability_architecture_links: has('capability_architecture_link')
      ? db.prepare('SELECT count(*) n FROM capability_architecture_link').get().n
      : 0,
    architecture_target_gaps: has('architecture_target_gap')
      ? db.prepare('SELECT count(*) n FROM architecture_target_gap').get().n
      : 0,
    planned_architecture_nodes: has('planned_architecture_node')
      ? db.prepare('SELECT count(*) n FROM planned_architecture_node').get().n
      : 0,
  }
  db.close()
  return counts
}

const exportCore = () => {
  const db = openWritableShard(SHARDS.core)
  createMetaTable(db, 'core')
  if (sourceTableExists(SOURCE_DB, 'meta')) {
    copyTable(db, 'meta')
  } else {
    db.exec('CREATE TABLE meta(k TEXT PRIMARY KEY, v TEXT NOT NULL)')
    const insertMeta = db.prepare('INSERT INTO meta(k, v) VALUES (?, ?)')
    insertMeta.run('generated_by', 'tools/capabilities/export-cloud-snapshot.mjs')
    insertMeta.run('generated_at', GENERATED_AT)
    insertMeta.run('note', 'source docs/capabilities.db had no meta table; counts are in docs/capabilities-cloud/meta.json')
  }
  if (sourceTableExists(SOURCE_DB, 'canonical_capability')) {
    copyTable(db, 'canonical_capability')
  } else {
    const recovered = createCanonicalFromGeneratedDocs(db)
    db.prepare('INSERT OR REPLACE INTO meta(k, v) VALUES (?, ?)').run(
      'canonical_recovery',
      `canonical_capability recovered from generated docs (${recovered} rows); local audit required`,
    )
  }
  if (sourceTableExists(SOURCE_DB, 'capability')) copyTable(db, 'capability')
  else createTableFromRows(db, 'capability', [], [{ name: 'key', type: 'TEXT' }])
  if (sourceTableExists(SOURCE_DB, 'impl_evidence')) copyTable(db, 'impl_evidence')
  else createTableFromRows(db, 'impl_evidence', [], [{ name: 'canonical_key', type: 'TEXT' }])
  if (sourceTableExists(SOURCE_DB, 'canonical_status_override')) copyTable(db, 'canonical_status_override')
  else createTableFromRows(db, 'canonical_status_override', [], [{ name: 'canonical_key', type: 'TEXT' }])
  db.exec(`
CREATE INDEX idx_cloud_cc_key ON canonical_capability(key);
CREATE INDEX idx_cloud_cc_domain ON canonical_capability(domain);
CREATE INDEX idx_cloud_cc_status ON canonical_capability(status);
CREATE INDEX idx_cloud_cc_money ON canonical_capability(moves_money);
CREATE INDEX idx_cloud_capability_key ON capability(key);
CREATE INDEX idx_cloud_impl_key ON impl_evidence(canonical_key);
`)
  finishShard(db)
}

const exportProvenance = () => {
  const db = openWritableShard(SHARDS.provenance)
  createMetaTable(db, 'provenance')
  copyTable(db, 'source_capability')
  copyTable(db, 'provenance')
  copyTable(db, 'slice_canonical')
  db.exec(`
CREATE INDEX idx_cloud_sc_id ON source_capability(id);
CREATE INDEX idx_cloud_sc_donor ON source_capability(donor);
CREATE INDEX idx_cloud_sc_canonical_id ON source_capability(canonical_id);
CREATE INDEX idx_cloud_sc_money ON source_capability(moves_money);
CREATE INDEX idx_cloud_prov_source ON provenance(source_id);
CREATE INDEX idx_cloud_prov_canonical ON provenance(canonical_id);
CREATE INDEX idx_cloud_slice_key ON slice_canonical(canonical_key);
`)
  finishShard(db)
}

const exportCensusSummary = () => {
  const db = openWritableShard(SHARDS.census)
  createMetaTable(db, 'census-summary')
  const source = new Database(SOURCE_DB, { readonly: true })
  const hasDonorFileCensus = tableExists(source, 'donor_file_census')
  const hasSourceFileLink = tableExists(source, 'source_file_capability_link')
  if (!hasDonorFileCensus || !hasSourceFileLink) {
    const missing = [!hasDonorFileCensus && 'donor_file_census', !hasSourceFileLink && 'source_file_capability_link']
      .filter(Boolean)
      .join(', ')
    db.prepare('INSERT INTO cloud_snapshot_info(key, value) VALUES (?, ?)').run(
      'census_gap',
      `${missing} not present in ${rel(SOURCE_DB)}; this source is either the local-only capabilities.db missing these tables, or the git_canonical_shards fallback (neither donor_file_census nor source_file_capability_link is git-tracked yet - see docs/capabilities-canonical/local-authority/meta.json not_yet_exported). Census tables below are empty, not a real zero.`,
    )
  }
  const donorFileColumns = [
    { name: 'rows', type: 'INTEGER' },
    { name: 'donors', type: 'INTEGER' },
    { name: 'unread_pending', type: 'INTEGER' },
    { name: 'capability_review_pending', type: 'INTEGER' },
    { name: 'behavior_review_pending', type: 'INTEGER' },
    { name: 'mapped_missing_source_ids', type: 'INTEGER' },
    { name: 'blocked', type: 'INTEGER' },
    { name: 'size_bytes', type: 'INTEGER' },
  ]
  const zeroDonorFileTotals = Object.fromEntries(donorFileColumns.map((col) => [col.name, 0]))
  createTableFromRows(
    db,
    'donor_file_census_totals',
    [
      hasDonorFileCensus
        ? source
          .prepare(
            `SELECT
            count(*) AS rows,
            count(DISTINCT donor) AS donors,
            sum(CASE WHEN read_status='unread_pending' THEN 1 ELSE 0 END) AS unread_pending,
            sum(CASE WHEN classification='capability_review_pending' THEN 1 ELSE 0 END) AS capability_review_pending,
            sum(CASE WHEN classification='behavior_review_pending' THEN 1 ELSE 0 END) AS behavior_review_pending,
            sum(CASE WHEN classification IN ('mapped','duplicate_variant_mapped') AND COALESCE(mapped_source_ids,'')='' THEN 1 ELSE 0 END) AS mapped_missing_source_ids,
            sum(CASE WHEN classification='blocked' THEN 1 ELSE 0 END) AS blocked,
            sum(COALESCE(size_bytes,0)) AS size_bytes
          FROM donor_file_census`,
          )
          .get()
        : zeroDonorFileTotals,
    ],
    donorFileColumns,
  )
  createTableFromRows(
    db,
    'donor_file_census_by_donor',
    hasDonorFileCensus
      ? source
        .prepare(
          `SELECT
          donor,
          count(*) AS rows,
          sum(CASE WHEN read_status='unread_pending' THEN 1 ELSE 0 END) AS unread_pending,
          sum(CASE WHEN classification='capability_review_pending' THEN 1 ELSE 0 END) AS capability_review_pending,
          sum(CASE WHEN classification='behavior_review_pending' THEN 1 ELSE 0 END) AS behavior_review_pending,
          sum(CASE WHEN classification='mapped' THEN 1 ELSE 0 END) AS mapped,
          sum(CASE WHEN classification='duplicate_variant_mapped' THEN 1 ELSE 0 END) AS duplicate_variant_mapped,
          sum(CASE WHEN classification='generated_vendor_build_artifact' THEN 1 ELSE 0 END) AS generated_vendor_build_artifact,
          sum(COALESCE(size_bytes,0)) AS size_bytes
        FROM donor_file_census
        GROUP BY donor
        ORDER BY unread_pending DESC, rows DESC`,
        )
        .all()
      : [],
    [
      { name: 'donor', type: 'TEXT' },
      { name: 'rows', type: 'INTEGER' },
      { name: 'unread_pending', type: 'INTEGER' },
      { name: 'capability_review_pending', type: 'INTEGER' },
      { name: 'behavior_review_pending', type: 'INTEGER' },
      { name: 'mapped', type: 'INTEGER' },
      { name: 'duplicate_variant_mapped', type: 'INTEGER' },
      { name: 'generated_vendor_build_artifact', type: 'INTEGER' },
      { name: 'size_bytes', type: 'INTEGER' },
    ],
  )
  createTableFromRows(
    db,
    'donor_file_census_by_classification',
    hasDonorFileCensus
      ? source
        .prepare(
          `SELECT
          classification,
          read_status,
          kind,
          count(*) AS rows,
          count(DISTINCT donor) AS donors,
          sum(COALESCE(size_bytes,0)) AS size_bytes
        FROM donor_file_census
        GROUP BY classification, read_status, kind
        ORDER BY rows DESC`,
        )
        .all()
      : [],
    [
      { name: 'classification', type: 'TEXT' },
      { name: 'read_status', type: 'TEXT' },
      { name: 'kind', type: 'TEXT' },
      { name: 'rows', type: 'INTEGER' },
      { name: 'donors', type: 'INTEGER' },
      { name: 'size_bytes', type: 'INTEGER' },
    ],
  )
  createTableFromRows(
    db,
    'donor_file_census_pending_sample',
    hasDonorFileCensus
      ? source
        .prepare(
          `SELECT donor, path, is_directory, kind, classification, read_status, exclusion_reason, size_bytes, sha256
        FROM donor_file_census
        WHERE read_status='unread_pending' OR classification LIKE '%review_pending'
        ORDER BY donor, path
        LIMIT ?`,
        )
        .all(PENDING_LIMIT)
      : [],
    [
      { name: 'donor', type: 'TEXT' },
      { name: 'path', type: 'TEXT' },
      { name: 'is_directory', type: 'INTEGER' },
      { name: 'kind', type: 'TEXT' },
      { name: 'classification', type: 'TEXT' },
      { name: 'read_status', type: 'TEXT' },
      { name: 'exclusion_reason', type: 'TEXT' },
      { name: 'size_bytes', type: 'INTEGER' },
      { name: 'sha256', type: 'TEXT' },
    ],
  )
  createTableFromRows(
    db,
    'source_file_link_summary',
    hasSourceFileLink
      ? source
        .prepare(
          `SELECT
          donor,
          count(*) AS rows,
          count(DISTINCT path) AS distinct_paths,
          count(DISTINCT source_id) AS distinct_sources
        FROM source_file_capability_link
        GROUP BY donor
        ORDER BY rows DESC`,
        )
        .all()
      : [],
    [
      { name: 'donor', type: 'TEXT' },
      { name: 'rows', type: 'INTEGER' },
      { name: 'distinct_paths', type: 'INTEGER' },
      { name: 'distinct_sources', type: 'INTEGER' },
    ],
  )
  createTableFromRows(
    db,
    'source_file_link_sample',
    hasSourceFileLink
      ? source
        .prepare(
          `SELECT source_id, donor, path, symbol, evidence_note
        FROM source_file_capability_link
        ORDER BY donor, path, source_id
        LIMIT ?`,
        )
        .all(LINK_LIMIT)
      : [],
    [
      { name: 'source_id', type: 'INTEGER' },
      { name: 'donor', type: 'TEXT' },
      { name: 'path', type: 'TEXT' },
      { name: 'symbol', type: 'TEXT' },
      { name: 'evidence_note', type: 'TEXT' },
    ],
  )
  source.close()
  db.exec(`
CREATE INDEX idx_cloud_dfc_donor ON donor_file_census_by_donor(donor);
CREATE INDEX idx_cloud_dfc_sample_donor ON donor_file_census_pending_sample(donor);
CREATE INDEX idx_cloud_dfc_sample_class ON donor_file_census_pending_sample(classification);
CREATE INDEX idx_cloud_sfls_donor ON source_file_link_summary(donor);
`)
  finishShard(db)
}

const exportArchitecture = () => {
  const db = openWritableShard(SHARDS.architecture)
  createMetaTable(db, 'architecture')
  if (!existsSync(ARCH_DB)) {
    db.exec("CREATE TABLE architecture_missing(reason TEXT NOT NULL)")
    db.prepare('INSERT INTO architecture_missing(reason) VALUES (?)').run(`missing architecture DB: ${rel(ARCH_DB)}`)
    finishShard(db)
    return
  }
  for (const table of [
    'meta',
    'architecture_node',
    'architecture_edge',
    'architecture_invariant',
    'architecture_evidence',
    'architecture_status_override',
    'authority_rule',
    'data_flow',
    'capability_architecture_link',
    'architecture_target_gap',
    'planned_architecture_node',
  ]) {
    if (sourceTableExists(ARCH_DB, table)) copyTable(db, table, ['*'], { sourcePath: ARCH_DB })
  }
  db.exec(`
CREATE INDEX idx_cloud_arch_cap_key ON capability_architecture_link(capability_key);
CREATE INDEX idx_cloud_arch_node ON capability_architecture_link(architecture_node);
CREATE INDEX idx_cloud_arch_gap_key ON architecture_target_gap(capability_key);
CREATE INDEX idx_cloud_arch_gap_kind ON architecture_target_gap(gap_kind);
CREATE INDEX idx_cloud_arch_node_id ON architecture_node(id);
`)
  finishShard(db)
}

const exportWorkqueue = () => {
  const db = openWritableShard(SHARDS.workqueue)
  createMetaTable(db, 'workqueue')
  const hasCanonical = sourceTableExists(SOURCE_DB, 'canonical_capability')
  if (hasCanonical) copyTable(db, 'canonical_capability')
  if (!hasCanonical) createCanonicalFromGeneratedDocs(db)
  if (sourceTableExists(SOURCE_DB, 'impl_evidence')) copyTable(db, 'impl_evidence')
  else createTableFromRows(db, 'impl_evidence', [], [{ name: 'canonical_key', type: 'TEXT' }])
  db.exec(`
CREATE TABLE cloud_policy (
  key TEXT PRIMARY KEY,
  value TEXT NOT NULL
);

INSERT INTO cloud_policy(key, value) VALUES
  ('authority', 'cloud shards are read-only working context; local docs/capabilities.db remains final audit authority'),
  ('capability_claim_rule', 'cloud may propose code and tests, but final capability status claims require local audit'),
  ('money_rule', 'money-moving capabilities remain approval-gated and must be treated as unarmed unless local evidence proves otherwise'),
  ('meaning_rule', 'docs own meaning and intent; DB shards own mechanical facts; code/tests own executable proof'),
  ('five_dimension_cloud_shard_rule', 'cloud evidence must reconcile code, tests/evidence, docs, crate_sub_cap_db, and crate_sub_arch_db; crate shards live at crates/<crate>/.chronica/sub-cap-arch.jsonl plus .db; root capabilities.db and architecture.db remain local aggregate audit targets');

CREATE TABLE domain_summary AS
SELECT
  COALESCE(domain, '<none>') AS domain,
  count(*) AS canonical_capabilities,
  sum(CASE WHEN moves_money=1 THEN 1 ELSE 0 END) AS money_capabilities,
  sum(CASE WHEN status='verified' THEN 1 ELSE 0 END) AS verified,
  sum(CASE WHEN status='implemented_unverified' THEN 1 ELSE 0 END) AS implemented_unverified,
  sum(CASE WHEN status='unimplemented' THEN 1 ELSE 0 END) AS unimplemented,
  sum(CASE WHEN status='blocked' THEN 1 ELSE 0 END) AS blocked,
  sum(CASE WHEN status='excluded' THEN 1 ELSE 0 END) AS excluded,
  sum(donor_count) AS donor_links
FROM src.canonical_capability
GROUP BY COALESCE(domain, '<none>')
ORDER BY money_capabilities DESC, canonical_capabilities DESC;

CREATE TABLE next_unverified_canonical AS
SELECT
  key,
  canonical_name,
  domain,
  target_crate,
  target_module,
  side_effect_class,
  moves_money,
  requires_approval,
  status,
  donor_count,
  acceptance_criteria,
  required_tests,
  financial_control_test,
  acceptance_test,
  blocker
FROM src.canonical_capability
WHERE status != 'verified'
ORDER BY moves_money DESC, donor_count DESC, key;

CREATE TABLE next_money_canonical AS
SELECT
  key,
  canonical_name,
  domain,
  target_crate,
  target_module,
  side_effect_class,
  moves_money,
  requires_approval,
  status,
  donor_count,
  acceptance_criteria,
  required_tests,
  financial_control_test,
  acceptance_test,
  blocker
FROM src.canonical_capability
WHERE moves_money=1 AND status != 'verified'
ORDER BY donor_count DESC, key;

CREATE TABLE implementation_evidence_queue AS
SELECT
  c.key,
  c.canonical_name,
  c.domain,
  c.target_crate,
  c.status,
  c.acceptance_test,
  c.financial_control_test,
  e.impl_file,
  e.test_file,
  e.test_symbol,
  CASE WHEN e.canonical_key IS NULL THEN 1 ELSE 0 END AS local_evidence_missing
FROM src.canonical_capability c
LEFT JOIN impl_evidence e ON e.canonical_key = c.key
WHERE c.status='verified' OR c.status='implemented_unverified'
ORDER BY local_evidence_missing DESC, c.key;
`.replaceAll('src.canonical_capability', 'canonical_capability'))
  if (sourceTableExists(SOURCE_DB, 'agent_note')) {
    const source = new Database(SOURCE_DB, { readonly: true })
    createTableFromRows(
      db,
      'open_agent_note',
      source
        .prepare(
          `SELECT id, author, kind, status, capability_key, donor, body, created_at, resolved_by, resolved_at
          FROM agent_note
          WHERE status='open'
          ORDER BY id`,
        )
        .all(),
      [
        { name: 'id', type: 'INTEGER' },
        { name: 'author', type: 'TEXT' },
        { name: 'kind', type: 'TEXT' },
        { name: 'status', type: 'TEXT' },
        { name: 'capability_key', type: 'TEXT' },
        { name: 'donor', type: 'TEXT' },
        { name: 'body', type: 'TEXT' },
        { name: 'created_at', type: 'TEXT' },
        { name: 'resolved_by', type: 'TEXT' },
        { name: 'resolved_at', type: 'TEXT' },
      ],
    )
    source.close()
  } else {
    db.exec('CREATE TABLE open_agent_note(id INTEGER, author TEXT, kind TEXT, status TEXT, capability_key TEXT, donor TEXT, body TEXT, created_at TEXT, resolved_by TEXT, resolved_at TEXT)')
  }
  db.exec(`
CREATE INDEX idx_cloud_next_key ON next_unverified_canonical(key);
CREATE INDEX idx_cloud_next_domain ON next_unverified_canonical(domain);
CREATE INDEX idx_cloud_next_money ON next_unverified_canonical(moves_money);
CREATE INDEX idx_cloud_money_key ON next_money_canonical(key);
CREATE INDEX idx_cloud_domain_summary_domain ON domain_summary(domain);
`)
  finishShard(db)
}

const hashFile = (path) => {
  const hash = createHash('sha256')
  hash.update(readFileSync(path))
  return hash.digest('hex')
}

const writeManifest = (sourceCounts, architectureCounts) => {
  const files = Object.fromEntries(
    Object.entries(SHARDS).map(([name, file]) => {
      const path = join(OUT_DIR, file)
      const stat = statSync(path)
      return [
        name,
        {
          path: rel(path),
          file: basename(path),
          bytes: stat.size,
          mib: Number((stat.size / 1024 / 1024).toFixed(2)),
          sha256: hashFile(path),
        },
      ]
    }),
  )

  const meta = {
    schema_version: 1,
    generated_at: GENERATED_AT,
    authority: 'cloud-readable snapshot only; local docs/capabilities.db remains final audit authority',
    local_audit_required: true,
    source: sourceCounts,
    architecture: architectureCounts,
    source_mode: SOURCE_MODE,
    architecture_source_mode: ARCHITECTURE_GIT_BUILD ? 'git_canonical_shards' : 'local_architecture_db',
    git_source_gaps: SOURCE_MODE === 'git_canonical_shards' ? CAPABILITY_GIT_BUILD.gaps : null,
    architecture_git_source_error: ARCHITECTURE_GIT_BUILD_ERROR,
    export_options: {
      pending_limit: PENDING_LIMIT,
      link_limit: LINK_LIMIT,
    },
    files,
  }
  writeFileSync(join(OUT_DIR, 'meta.json'), `${JSON.stringify(meta, null, 2)}\n`)
  return meta
}

const cleanupGitBuilds = () => {
  if (CAPABILITY_GIT_BUILD?.tempDir) rmSync(CAPABILITY_GIT_BUILD.tempDir, { recursive: true, force: true })
  if (ARCHITECTURE_GIT_BUILD?.tempDir) rmSync(ARCHITECTURE_GIT_BUILD.tempDir, { recursive: true, force: true })
}

const main = () => {
  console.log(`export-cloud-snapshot: source=${rel(SOURCE_DB)} (${SOURCE_MODE}) out=${rel(OUT_DIR)}`)
  try {
    const sourceCounts = readSourceCounts()
    const architectureCounts = readArchitectureCounts()
    exportCore()
    exportProvenance()
    exportCensusSummary()
    exportArchitecture()
    exportWorkqueue()
    const meta = writeManifest(sourceCounts, architectureCounts)
    console.log(
      `export-cloud-snapshot: wrote ${Object.keys(SHARDS).length} shards; canonical=${meta.source.canonical_capabilities}; money=${meta.source.money_canonical}; source=${meta.source.source_capabilities}`,
    )
    console.log(`export-cloud-snapshot: meta=${rel(join(OUT_DIR, 'meta.json'))}`)
    if (SOURCE_MODE === 'git_canonical_shards') {
      const gaps = Object.keys(CAPABILITY_GIT_BUILD.gaps)
      console.log(`export-cloud-snapshot: git_canonical_shards mode; gaps not reproducible from git yet: ${gaps.join(', ')}`)
    }
  } finally {
    cleanupGitBuilds()
  }
}

main()
