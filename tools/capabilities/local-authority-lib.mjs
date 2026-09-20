// local-authority-lib.mjs - read git-tracked raw table dumps under
// docs/capabilities-canonical/local-authority/*.jsonl (PR #2608). These are
// one-file-per-table snapshots of local-only docs/capabilities.db tables that
// have no other canonical-JSONL mirror (see local-authority/meta.json). They
// are NOT continuously regenerated the way docs/capabilities-canonical/domains/
// is - treat them as a point-in-time export, not a live sync target.
import { existsSync, readFileSync, readdirSync, statSync } from 'node:fs'
import { join } from 'node:path'

export const LOCAL_AUTHORITY_DIR = join('docs', 'capabilities-canonical', 'local-authority')

export function localAuthorityTablePath(root, table) {
  return join(root, LOCAL_AUTHORITY_DIR, `${table}.jsonl`)
}

export function localAuthorityShardedTableDir(root, table) {
  return join(root, LOCAL_AUTHORITY_DIR, table)
}

export function loadLocalAuthorityMeta(root) {
  const path = join(root, LOCAL_AUTHORITY_DIR, 'meta.json')
  if (!existsSync(path)) return null
  return JSON.parse(readFileSync(path, 'utf8'))
}

function parseJsonlFile(path) {
  const raw = readFileSync(path, 'utf8')
  const rows = []
  raw.split(/\r?\n/).forEach((line, index) => {
    if (!line.trim()) return
    try {
      rows.push(JSON.parse(line))
    } catch (error) {
      const parseError = new Error(`${path}:${index + 1}: invalid JSONL: ${error.message}`)
      parseError.code = 'JSONL_PARSE_ERROR'
      throw parseError
    }
  })
  return rows
}

// Returns an array of row objects, or null if the table's dump file is absent
// (the caller must treat null as "not available from git", never as "empty").
export function loadLocalAuthorityTable(root, table) {
  const path = localAuthorityTablePath(root, table)
  if (!existsSync(path)) return null
  return parseJsonlFile(path)
}

// Like loadLocalAuthorityTable, but for tables donor-sharded into
// docs/capabilities-canonical/local-authority/<table>/<donor>.jsonl (PR #2612
// - donor_file_census, source_file_capability_link). Returns null if the
// shard directory is absent, never [] - same "not available from git" vs
// "empty" distinction as loadLocalAuthorityTable.
export function loadLocalAuthorityShardedTable(root, table) {
  const dir = localAuthorityShardedTableDir(root, table)
  if (!existsSync(dir) || !statSync(dir).isDirectory()) return null
  const shardFiles = readdirSync(dir)
    .filter((name) => name.endsWith('.jsonl'))
    .sort()
  const rows = []
  for (const name of shardFiles) rows.push(...parseJsonlFile(join(dir, name)))
  return rows
}

function sqlType(value) {
  if (typeof value === 'number') return Number.isInteger(value) ? 'INTEGER' : 'REAL'
  if (typeof value === 'boolean') return 'INTEGER'
  return 'TEXT'
}

function normalizeCellValue(value) {
  if (value == null) return null
  if (typeof value === 'boolean') return value ? 1 : 0
  if (typeof value === 'object') return JSON.stringify(value)
  return value
}

// Creates (dropping any existing table of the same name) a table whose schema
// is inferred from the union of keys across every row - the local-authority
// dumps are raw single-table exports, so their JSON keys are already the
// original SQLite column names.
export function createTableFromLocalAuthorityRows(db, table, rows) {
  const q = (name) => `"${String(name).replace(/"/g, '""')}"`
  const columns = new Map()
  for (const row of rows) {
    for (const [key, value] of Object.entries(row)) {
      if (!columns.has(key) && value != null) columns.set(key, sqlType(value))
      else if (!columns.has(key)) columns.set(key, 'TEXT')
    }
  }
  const columnNames = [...columns.keys()]
  db.exec(`DROP TABLE IF EXISTS ${q(table)}`)
  db.exec(`CREATE TABLE ${q(table)} (${columnNames.map((name) => `${q(name)} ${columns.get(name)}`).join(', ')})`)
  if (!rows.length) return
  const insert = db.prepare(
    `INSERT INTO ${q(table)} (${columnNames.map(q).join(', ')}) VALUES (${columnNames.map(() => '?').join(', ')})`,
  )
  const tx = db.transaction(() => {
    for (const row of rows) insert.run(columnNames.map((name) => normalizeCellValue(row[name])))
  })
  tx()
}
