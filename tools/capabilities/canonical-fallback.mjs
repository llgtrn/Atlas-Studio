import { existsSync } from 'node:fs'
import { join } from 'node:path'

export const CLOUD_CANONICAL_SOURCE = 'docs/capabilities-cloud/cap-core.db fallback (LOCAL_AUDIT_REQUIRED)'

function hasMainTable(database, name) {
  return database.prepare("SELECT count(*) n FROM sqlite_master WHERE type='table' AND name=?").get(name).n > 0
}

function hasTempObject(database, name) {
  return database.prepare("SELECT count(*) n FROM sqlite_temp_master WHERE type IN ('table','view') AND name=?").get(name).n > 0
}

function hasAttachedTable(database, schema, name) {
  return database.prepare(`SELECT count(*) n FROM "${schema}".sqlite_master WHERE type='table' AND name=?`).get(name).n > 0
}

// A canonical_capability table that EXISTS but has zero rows carries the exact same "no real
// truth here" meaning as a MISSING table (e.g. caps:schema's `CREATE TABLE IF NOT EXISTS` just
// materialized an empty shell after the real table was lost). Treating "exists" as "usable"
// without checking row count is precisely how an empty table silently disables the cloud/side-table
// fallback and reports zero canonical capabilities as if that were the true count (issue #713).
export function isUsableCanonicalCapabilityTable(database, name) {
  if (!hasMainTable(database, name)) return false
  return database.prepare(`SELECT count(*) n FROM "${name}"`).get().n > 0
}

export function attachCanonicalCapabilityFallback(database, { root = process.cwd(), includeOverrideView = false } = {}) {
  if (isUsableCanonicalCapabilityTable(database, 'canonical_capability')) {
    return { source: 'docs/capabilities.db', attached: false }
  }
  if (hasTempObject(database, 'canonical_capability')) {
    return { source: CLOUD_CANONICAL_SOURCE, attached: true }
  }
  // A TEMP VIEW resolves before a same-named MAIN table for unqualified lookups in SQLite, so it
  // can shadow the empty main-schema table below without dropping it (dropping would require a
  // read-write connection; every caller here opens the DB read-only).

  const corePath = join(root, 'docs', 'capabilities-cloud', 'cap-core.db')
  if (!existsSync(corePath)) {
    throw new Error(`canonical_capability missing and fallback shard is absent: ${corePath}`)
  }

  try {
    database.prepare('ATTACH DATABASE ? AS cloud_core').run(corePath)
  } catch (error) {
    if (!/already in use/i.test(String(error?.message || error))) throw error
  }
  if (!hasAttachedTable(database, 'cloud_core', 'canonical_capability')) {
    throw new Error(`fallback shard has no canonical_capability table: ${corePath}`)
  }

  database.exec('CREATE TEMP VIEW canonical_capability AS SELECT * FROM cloud_core.canonical_capability')
  if (
    includeOverrideView &&
    !hasMainTable(database, 'canonical_status_override') &&
    !hasTempObject(database, 'canonical_status_override') &&
    hasAttachedTable(database, 'cloud_core', 'canonical_status_override')
  ) {
    database.exec('CREATE TEMP VIEW canonical_status_override AS SELECT * FROM cloud_core.canonical_status_override')
  }

  return { source: CLOUD_CANONICAL_SOURCE, attached: true }
}
