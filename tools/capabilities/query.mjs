#!/usr/bin/env node
// query.mjs — query canonical capability JSONL first, with docs/capabilities.db as a cache fallback.
// The point: an agent can ADDRESS a capability and get its execution contract, not read prose.
//
// Usage:
//   node tools/capabilities/query.mjs scope                       # the headline numbers
//   node tools/capabilities/query.mjs next-money                  # next buildable money capability
//   node tools/capabilities/query.mjs by-crate chronica-erp       # capabilities for a crate
//   node tools/capabilities/query.mjs by-status spec --limit 20   # capabilities by status
//   node tools/capabilities/query.mjs get slice.19                # one capability's full execution spec
//   node tools/capabilities/query.mjs donors                      # per-donor true scope vs coverage
//   node tools/capabilities/query.mjs target-crate-census [crate]  # target_crate hygiene, or one crate's build queue
//   node tools/capabilities/query.mjs full-census                  # full canonical cap DB build/truth census
//   node tools/capabilities/query.mjs sql "SELECT ..."            # raw query (read-only)
import { existsSync, readFileSync, readdirSync } from 'node:fs'
import { join } from 'node:path'
import { loadCapabilityShards } from '../canonical-shards/jsonl-lib.mjs'
import { attachCanonicalCapabilityFallback, isUsableCanonicalCapabilityTable } from './canonical-fallback.mjs'

const CAPABILITIES_DB = join(process.cwd(), 'docs', 'capabilities.db')
const [cmd, ...rest] = process.argv.slice(2)
const limIdx = rest.indexOf('--limit')
const limit = limIdx >= 0 ? Number(rest[limIdx + 1]) : 30
const positional = rest.filter((v, i) => v !== '--limit' && !(limIdx >= 0 && i === limIdx + 1))
const arg = positional[0]
const print = (rows) => console.log(JSON.stringify(rows, null, 2))
const fail = (message, details = {}) => {
  console.error(JSON.stringify({ error: message, ...details }, null, 2))
  process.exit(2)
}

function knownCrateNames(root = process.cwd()) {
  const cratesDir = join(root, 'crates')
  if (!existsSync(cratesDir)) return new Set()
  return new Set(readdirSync(cratesDir, { withFileTypes: true })
    .filter((entry) => entry.isDirectory() && entry.name.startsWith('chronica-'))
    .map((entry) => entry.name))
}

function targetCrateCensus(records, { root = process.cwd(), limit = 30, source, targetFilter = null }) {
  const known = knownCrateNames(root)
  const byTarget = new Map()
  const filteredRows = []
  const filterHasExactTarget = targetFilter
    ? records.some((record) => record.target_crate === targetFilter) || known.has(targetFilter)
    : false
  const matchesTargetFilter = (target) => {
    if (!targetFilter) return false
    const value = String(target ?? '')
    return filterHasExactTarget ? value === targetFilter : value.includes(targetFilter)
  }
  let null_target_crate = 0
  let known_target_crate_rows = 0
  let nonexistent_target_crate_rows = 0

  for (const record of records) {
    const target = record.target_crate || null
    if (matchesTargetFilter(target)) {
      filteredRows.push(record)
    }
    if (!target) {
      null_target_crate += 1
      continue
    }
    const current = byTarget.get(target) ?? {
      target_crate: target,
      exists: known.has(target),
      rows: 0,
      verified: 0,
      implemented_unverified: 0,
      unimplemented: 0,
      blocked: 0,
      moves_money: 0,
      examples: [],
    }
    current.rows += 1
    if (record.status === 'verified') current.verified += 1
    else if (record.status === 'implemented_unverified') current.implemented_unverified += 1
    else if (record.status === 'unimplemented') current.unimplemented += 1
    else if (record.status === 'blocked') current.blocked += 1
    if (record.moves_money === true || record.moves_money === 1) current.moves_money += 1
    if (current.examples.length < 5) {
      current.examples.push({
        key: record.capability_key ?? record.key,
        status: record.status,
        domain: record.domain,
        moves_money: record.moves_money === true || record.moves_money === 1 ? 1 : 0,
      })
    }
    byTarget.set(target, current)
  }

  for (const row of byTarget.values()) {
    if (row.exists) known_target_crate_rows += row.rows
    else nonexistent_target_crate_rows += row.rows
  }

  const missing_target_crates = [...byTarget.values()]
    .filter((row) => !row.exists)
    .sort((a, b) => b.rows - a.rows || b.moves_money - a.moves_money || a.target_crate.localeCompare(b.target_crate))
    .slice(0, limit)

  if (targetFilter) {
    const matchingTargetCrates = [...byTarget.values()]
      .filter((row) => matchesTargetFilter(row.target_crate))
      .sort((a, b) => a.target_crate.localeCompare(b.target_crate))
    const actionableRows = filteredRows
      .filter((record) => record.status !== 'verified')
      .sort((a, b) =>
        (isMoney(b) ? 1 : 0) - (isMoney(a) ? 1 : 0) ||
        donorCount(b) - donorCount(a) ||
        capabilityKey(a).localeCompare(capabilityKey(b)))
      .slice(0, limit)
      .map((record) => ({
        key: capabilityKey(record),
        canonical_name: record.canonical_name,
        domain: record.domain,
        target_crate: record.target_crate,
        status: record.status,
        moves_money: isMoney(record) ? 1 : 0,
        donor_count: donorCount(record),
        blocker: record.blocker ?? null,
      }))

    return {
      capability_schema: source,
      local_audit_required: false,
      known_crates_on_disk: known.size,
      canonical_rows: records.length,
      target_filter: targetFilter,
      summary: {
        matching_rows: filteredRows.length,
        matching_not_verified_rows: filteredRows.filter((record) => record.status !== 'verified').length,
        matching_moves_money_rows: filteredRows.filter((record) => isMoney(record)).length,
      },
      matching_target_crates: matchingTargetCrates,
      actionable_rows: actionableRows,
    }
  }

  return {
    capability_schema: source,
    local_audit_required: false,
    known_crates_on_disk: known.size,
    canonical_rows: records.length,
    target_crate_summary: {
      known_target_crate_rows,
      nonexistent_target_crate_rows,
      null_target_crate,
      distinct_nonexistent_target_crates: [...byTarget.values()].filter((row) => !row.exists).length,
    },
    missing_target_crates,
  }
}

function capabilityKey(record) {
  return record.capability_key ?? record.key
}

function donorCount(record) {
  return Number(record.donor_count ?? record.source_refs?.length ?? 0)
}

function isMoney(record) {
  return record.moves_money === true || record.moves_money === 1
}

function increment(map, key, by = 1) {
  map.set(key, (map.get(key) ?? 0) + by)
}

function sortedCounter(map) {
  return [...map.entries()]
    .map(([key, rows]) => ({ key, rows }))
    .sort((a, b) => b.rows - a.rows || String(a.key).localeCompare(String(b.key)))
}

function fullCapabilityCensus(records, { root = process.cwd(), limit = 30, source }) {
  const known = knownCrateNames(root)
  const status = new Map()
  const domains = new Map()
  const targetCrates = new Map()
  const moneyByStatus = new Map()
  const buildable = []
  const missingTargetExamples = []
  const nullTargetExamples = []
  let movesMoneyRows = 0
  let knownTargetRows = 0
  let missingTargetRows = 0
  let nullTargetRows = 0

  for (const record of records) {
    const rowStatus = record.status || 'unknown'
    const domain = record.domain || 'unknown'
    const target = record.target_crate || null
    const money = isMoney(record)
    const rowDonorCount = donorCount(record)

    increment(status, rowStatus)
    increment(domains, domain)
    if (money) {
      movesMoneyRows += 1
      increment(moneyByStatus, rowStatus)
    }

    if (!target) {
      nullTargetRows += 1
      if (nullTargetExamples.length < limit) {
        nullTargetExamples.push({
          key: capabilityKey(record),
          canonical_name: record.canonical_name,
          domain,
          status: rowStatus,
          moves_money: money ? 1 : 0,
          donor_count: rowDonorCount,
        })
      }
      continue
    }

    const targetExists = known.has(target)
    const currentTarget = targetCrates.get(target) ?? {
      target_crate: target,
      exists: targetExists,
      rows: 0,
      verified: 0,
      implemented_unverified: 0,
      unimplemented: 0,
      blocked: 0,
      unknown: 0,
      moves_money: 0,
    }
    currentTarget.rows += 1
    if (rowStatus === 'verified') currentTarget.verified += 1
    else if (rowStatus === 'implemented_unverified') currentTarget.implemented_unverified += 1
    else if (rowStatus === 'unimplemented') currentTarget.unimplemented += 1
    else if (rowStatus === 'blocked') currentTarget.blocked += 1
    else currentTarget.unknown += 1
    if (money) currentTarget.moves_money += 1
    targetCrates.set(target, currentTarget)

    if (targetExists) {
      knownTargetRows += 1
      if (rowStatus !== 'verified') {
        buildable.push({
          key: capabilityKey(record),
          canonical_name: record.canonical_name,
          domain,
          target_crate: target,
          status: rowStatus,
          moves_money: money ? 1 : 0,
          donor_count: rowDonorCount,
        })
      }
    } else {
      missingTargetRows += 1
      if (missingTargetExamples.length < limit) {
        missingTargetExamples.push({
          key: capabilityKey(record),
          canonical_name: record.canonical_name,
          domain,
          target_crate: target,
          status: rowStatus,
          moves_money: money ? 1 : 0,
          donor_count: rowDonorCount,
        })
      }
    }
  }

  const targetRows = [...targetCrates.values()]
  const missingTargetCrates = targetRows
    .filter((row) => !row.exists)
    .sort((a, b) => b.rows - a.rows || b.moves_money - a.moves_money || a.target_crate.localeCompare(b.target_crate))
    .slice(0, limit)
  const hottestExistingCrates = targetRows
    .filter((row) => row.exists && (row.unimplemented || row.implemented_unverified || row.blocked || row.unknown))
    .sort((a, b) =>
      (b.unimplemented + b.implemented_unverified + b.blocked + b.unknown) -
        (a.unimplemented + a.implemented_unverified + a.blocked + a.unknown) ||
      b.moves_money - a.moves_money ||
      a.target_crate.localeCompare(b.target_crate))
    .slice(0, limit)
  const actionable_existing_crate_queue = buildable
    .sort((a, b) => b.moves_money - a.moves_money || b.donor_count - a.donor_count || a.key.localeCompare(b.key))
    .slice(0, limit)

  return {
    capability_schema: source,
    local_audit_required: false,
    canonical_rows: records.length,
    known_crates_on_disk: known.size,
    summary: {
      verified_rows: status.get('verified') ?? 0,
      not_verified_rows: records.length - (status.get('verified') ?? 0),
      moves_money_rows: movesMoneyRows,
      known_target_crate_rows: knownTargetRows,
      nonexistent_target_crate_rows: missingTargetRows,
      null_target_crate_rows: nullTargetRows,
      distinct_target_crates: targetRows.length,
      distinct_nonexistent_target_crates: targetRows.filter((row) => !row.exists).length,
    },
    status_counts: sortedCounter(status),
    money_by_status: sortedCounter(moneyByStatus),
    domain_counts: sortedCounter(domains).slice(0, limit),
    hottest_existing_crates: hottestExistingCrates,
    missing_target_crates: missingTargetCrates,
    actionable_existing_crate_queue,
    missing_target_examples: missingTargetExamples,
    null_target_examples: nullTargetExamples,
  }
}

function tryCanonicalShardCommand() {
  let shard
  try {
    shard = loadCapabilityShards(process.cwd())
  } catch {
    return false
  }
  if (!shard.meta || shard.entries.length === 0) return false
  const rows = shard.entries.map((entry) => entry.record).sort((a, b) => a.capability_key.localeCompare(b.capability_key))
  const meta = {
    capability_schema: 'canonical_jsonl_shards',
    local_audit_required: false,
    source: 'docs/capabilities-canonical/**/*.jsonl',
  }
  switch (cmd) {
    case 'scope': {
      const sourceMeta = shard.meta ?? {}
      print({
        ...sourceMeta,
        source_capability_count: String(rows.reduce((sum, row) => sum + (row.source_refs?.length ?? 0), 0)),
        canonical_capability_count: String(rows.length),
        money_canonical_count: String(rows.filter((row) => row.moves_money).length),
        true_ideal_denominator: String(rows.length),
        canonical_verified_count: String(rows.filter((row) => row.status === 'verified').length),
        capability_schema: meta.capability_schema,
        local_audit_required: 'false',
      })
      return true
    }
    case 'next-money':
      print({
        ...meta,
        rows: rows
          .filter((row) => row.moves_money && row.status === 'unimplemented')
          .sort((a, b) => (b.source_refs?.length ?? 0) - (a.source_refs?.length ?? 0) || a.capability_key.localeCompare(b.capability_key))
          .slice(0, limit)
          .map((row) => ({
            key: row.capability_key,
            canonical_name: row.canonical_name,
            target_crate: row.target_crate,
            status: row.status,
            blocker: row.blocker,
            donor_count: row.source_refs?.length ?? 0,
          })),
      })
      return true
    case 'by-crate':
      print({
        ...meta,
        rows: rows
          .filter((row) => String(row.target_crate ?? '').includes(arg ?? ''))
          .map((row) => ({
            key: row.capability_key,
            canonical_name: row.canonical_name,
            status: row.status,
            target_crate: row.target_crate,
            domain: row.domain,
            acceptance_test: row.acceptance_test,
          })),
      })
      return true
    case 'by-status':
      print({
        ...meta,
        rows: rows
          .filter((row) => row.status === (arg || 'unimplemented'))
          .slice(0, limit)
          .map((row) => ({
            key: row.capability_key,
            canonical_name: row.canonical_name,
            target_crate: row.target_crate,
            status: row.status,
          })),
      })
      return true
    case 'get':
    case 'get-canonical': {
      const row = rows.find((record) => record.capability_key === arg)
      print(row ? { ...row, ...meta } : { error: `not found: ${arg}`, ...meta })
      return true
    }
    case 'verified':
      print({
        ...meta,
        rows: rows
          .filter((row) => row.status === 'verified')
          .map((row) => ({
            key: row.capability_key,
            canonical_name: row.canonical_name,
            target_crate: row.target_crate,
            acceptance_test: row.acceptance_test,
            financial_control_test: row.financial_control_test,
          })),
      })
      return true
    case 'canonical-by-domain':
      print([...rows.reduce((acc, row) => {
        const current = acc.get(row.domain) ?? { domain: row.domain, n: 0, money: 0 }
        current.n += 1
        if (row.moves_money) current.money += 1
        acc.set(row.domain, current)
        return acc
      }, new Map()).values()].sort((a, b) => b.n - a.n || a.domain.localeCompare(b.domain)))
      return true
    case 'money-canonical':
      print(rows
        .filter((row) => row.moves_money)
        .sort((a, b) => (b.source_refs?.length ?? 0) - (a.source_refs?.length ?? 0) || a.capability_key.localeCompare(b.capability_key))
        .map((row) => ({
          key: row.capability_key,
          canonical_name: row.canonical_name,
          domain: row.domain,
          donor_count: row.source_refs?.length ?? 0,
          status: row.status,
        })))
      return true
    case 'next-unverified-canonical':
      print(rows
        .filter((row) => row.status === 'unimplemented')
        .sort((a, b) => (b.source_refs?.length ?? 0) - (a.source_refs?.length ?? 0) || a.capability_key.localeCompare(b.capability_key))
        .slice(0, limit)
        .map((row) => ({
          key: row.capability_key,
          canonical_name: row.canonical_name,
          domain: row.domain,
          target_crate: row.target_crate,
          moves_money: row.moves_money ? 1 : 0,
          donor_count: row.source_refs?.length ?? 0,
        })))
      return true
    case 'target-crate-census':
      print(targetCrateCensus(rows, { root: process.cwd(), limit, source: meta.capability_schema, targetFilter: arg || null }))
      return true
    case 'full-census':
      print(fullCapabilityCensus(rows, { root: process.cwd(), limit, source: meta.capability_schema }))
      return true
    default:
      return false
  }
}

if (tryCanonicalShardCommand()) {
  process.exit(0)
}
if (!existsSync(CAPABILITIES_DB)) {
  fail('CANONICAL_SHARDS_REQUIRED: docs/capabilities.db is absent and the requested command is not available from docs/capabilities-canonical/**/*.jsonl.')
}
const { openReadOnlyDatabase } = await import('../db/sqlite-open.mjs')
const db = openReadOnlyDatabase(CAPABILITIES_DB)

const hasTable = (name) =>
  db.prepare("SELECT count(*) n FROM sqlite_master WHERE type='table' AND name=?").get(name).n > 0 ||
  db.prepare("SELECT count(*) n FROM sqlite_temp_master WHERE type IN ('table','view') AND name=?").get(name).n > 0
// An empty (but present) `capability` table carries the same "no real data here" meaning as a
// missing one (see isUsableCanonicalCapabilityTable's issue #713 rationale) -- treat both as
// "use the canonical_capability fallback" rather than returning zero rows as if that were truth.
const hasUsableTable = (name) => hasTable(name) && db.prepare(`SELECT count(*) n FROM "${name}"`).get().n > 0
const columns = (table) => db.prepare(`PRAGMA table_info(${table})`).all().map(row => row.name)
const hasColumn = (table, column) => hasTable(table) && columns(table).includes(column)
const requireTable = (name) => {
  if (!hasTable(name)) fail('CAPABILITY_DB_SCHEMA_BLOCKER: required table is missing.', { table: name })
}
const requireColumns = (table, names) => {
  requireTable(table)
  const present = new Set(columns(table))
  const missing = names.filter(name => !present.has(name))
  if (missing.length) fail('CAPABILITY_DB_SCHEMA_BLOCKER: required column(s) are missing.', { table, missing })
}
const requireCoreSchema = () => {
  requireColumns('canonical_capability', ['key', 'canonical_name', 'domain', 'target_crate', 'moves_money', 'status'])
  requireColumns('source_capability', ['id', 'canonical_name', 'canonical_id'])
  requireTable('meta')
}
const requireCanonicalReadSchema = () => {
  if (!isUsableCanonicalCapabilityTable(db, 'canonical_capability')) {
    attachCanonicalCapabilityFallback(db)
  }
  requireColumns('canonical_capability', ['key', 'canonical_name', 'domain', 'target_crate', 'moves_money', 'status'])
}
// by-crate/get/verified/by-status/next-money read the retired `capability` table (singular --
// the old 195-slice parity table). That table is empty/absent in every environment that only has
// the cloud-tracked docs/capabilities-cloud/cap-core.db (e.g. Claude Code Cloud dispatches), so
// those commands fall back to canonical_capability the same way `scope`/`canonical-by-domain`
// already do, and stamp the result so fallback-derived output is never mistaken for the full
// local capability table.
const capabilityFallbackMeta = () => {
  let attach
  try {
    attach = attachCanonicalCapabilityFallback(db)
  } catch (error) {
    fail('CAPABILITY_FALLBACK_UNAVAILABLE: the capability table is missing and the canonical_capability cloud snapshot fallback could not be attached.', { detail: String(error?.message || error) })
  }
  return {
    capability_schema: attach.attached ? 'canonical_capability_cloud_snapshot_fallback' : 'canonical_capability',
    local_audit_required: attach.attached,
    schema_warning: 'the capability table is missing or empty locally; showing canonical_capability instead' +
      (attach.attached ? ' (attached from docs/capabilities-cloud/cap-core.db)' : '') +
      '. Column mapping vs. the retired capability table: title -> canonical_name, fin_test -> financial_control_test; ' +
      'percent, gate_class, and next_action have no canonical_capability equivalent and are omitted.',
  }
}
const requireFileCensus = () => {
  if (!hasTable('donor_file_census')) {
    print({ error: 'donor_file_census table missing; run pnpm caps:schema then pnpm caps:file-census' })
    return false
  }
  return true
}

switch (cmd) {
  case 'scope': {
    const cloudMetaPath = join(process.cwd(), 'docs', 'capabilities-cloud', 'meta.json')
    const cloudMeta = existsSync(cloudMetaPath) ? JSON.parse(readFileSync(cloudMetaPath, 'utf8')) : null
    const m = hasTable('meta') ? Object.fromEntries(db.prepare('SELECT k,v FROM meta').all().map(r => [r.k, r.v])) : {}
    const hasCanonical = isUsableCanonicalCapabilityTable(db, 'canonical_capability')
    const hasOverrides = hasTable('canonical_status_override')
    const sourceCount = hasTable('source_capability') ? db.prepare('SELECT count(*) n FROM source_capability').get().n : 0
    const canonicalCount = hasCanonical
      ? db.prepare('SELECT count(*) n FROM canonical_capability').get().n
      : cloudMeta?.source?.canonical_capabilities ?? (hasTable('slice_canonical') ? db.prepare('SELECT count(DISTINCT canonical_key) n FROM slice_canonical').get().n : 0)
    const moneyCount = hasCanonical && hasColumn('canonical_capability', 'moves_money')
      ? db.prepare('SELECT count(*) n FROM canonical_capability WHERE moves_money=1').get().n
      : cloudMeta?.source?.money_canonical ?? (hasTable('source_capability') && hasColumn('source_capability', 'moves_money') ? db.prepare('SELECT count(*) n FROM source_capability WHERE moves_money=1').get().n : 0)
    const verifiedCount = hasCanonical && hasColumn('canonical_capability', 'status')
      ? db.prepare("SELECT count(*) n FROM canonical_capability WHERE status='verified'").get().n
      : hasOverrides
        ? db.prepare("SELECT count(*) n FROM canonical_status_override WHERE status='verified'").get().n
        : 0
    m.source_capability_count = String(sourceCount)
    m.canonical_capability_count = String(canonicalCount)
    m.money_canonical_count = String(moneyCount)
    m.true_ideal_denominator = String(canonicalCount)
    m.canonical_verified_count = String(verifiedCount)
    m.capability_schema = hasCanonical ? 'canonical_capability' : 'side_tables_plus_cloud_snapshot_fallback'
    m.local_audit_required = hasCanonical ? 'false' : 'true'
    if (!hasCanonical) {
      m.schema_warning = 'canonical_capability table is missing; status/progress is derived from survivable side-tables and cloud snapshot fallback until caps:schema/rebuild restores canonical rows'
    }
    if (m.money_flags_corrected) {
      m.money_flags_corrected = m.money_flags_corrected.replace(/->\d+/, `->${moneyCount}`)
    }
    if (hasTable('donor_file_census')) {
      const fileCensus = db.prepare(`SELECT
        count(*) rows,
        count(DISTINCT donor) donors,
        sum(CASE WHEN read_status='unread_pending' THEN 1 ELSE 0 END) unread_pending
        FROM donor_file_census`).get()
      const fullyReviewed = db.prepare(`SELECT count(*) n FROM (
        SELECT donor, sum(CASE WHEN read_status='unread_pending' THEN 1 ELSE 0 END) pending
        FROM donor_file_census GROUP BY donor HAVING pending=0
      )`).get().n
      m.file_census_rows = String(fileCensus.rows)
      m.file_census_donors_scanned = String(fileCensus.donors)
      m.file_census_unread_pending = String(fileCensus.unread_pending)
      m.file_census_fully_reviewed_donors = String(fullyReviewed)
    }
    print(m)
    break
  }
  case 'next-money': {
    // next buildable money capability: spec status, money gate, deps fewest
    if (hasUsableTable('capability')) {
      print(db.prepare(`SELECT key,title,target_crate,gate_class,status,percent,blocker,next_action
        FROM capability WHERE moves_money=1 AND status='spec' ORDER BY slice LIMIT ?`).all(limit))
      break
    }
    const meta = capabilityFallbackMeta()
    const rows = db.prepare(`SELECT key,canonical_name,target_crate,status,blocker,donor_count FROM canonical_capability
      WHERE moves_money=1 AND status='unimplemented' ORDER BY donor_count DESC, key LIMIT ?`).all(limit)
    print({
      ...meta,
      status_vocabulary_warning: "filtered to status='unimplemented' (canonical_capability's not-yet-built state); the retired capability table's finer 'spec' status has no canonical_capability equivalent",
      rows,
    })
    break
  }
  case 'by-crate': {
    if (hasUsableTable('capability')) {
      print(db.prepare(`SELECT key,title,status,percent,gate_class,acceptance_test FROM capability
        WHERE target_crate LIKE ? ORDER BY status DESC, slice`).all('%' + (arg || '') + '%'))
      break
    }
    const meta = capabilityFallbackMeta()
    const rows = db.prepare(`SELECT key,canonical_name,status,target_crate,domain,acceptance_test FROM canonical_capability
      WHERE target_crate LIKE ? ORDER BY status DESC, key`).all('%' + (arg || '') + '%')
    print({ ...meta, rows })
    break
  }
  case 'by-status': {
    if (hasUsableTable('capability')) {
      print(db.prepare(`SELECT key,title,target_crate,gate_class,percent FROM capability
        WHERE status=? ORDER BY slice LIMIT ?`).all(arg || 'spec', limit))
      break
    }
    const meta = capabilityFallbackMeta()
    const rows = db.prepare(`SELECT key,canonical_name,target_crate,status FROM canonical_capability
      WHERE status=? ORDER BY key LIMIT ?`).all(arg || 'unimplemented', limit)
    print({
      ...meta,
      status_vocabulary_warning: "canonical_capability status values are 'verified'/'unimplemented', not the retired capability table's finer slice-status vocabulary (e.g. 'spec')",
      rows,
    })
    break
  }
  case 'get': {
    if (hasUsableTable('capability')) {
      const row = db.prepare('SELECT * FROM capability WHERE key=?').get(arg)
      print(row || { error: 'not found: ' + arg })
      break
    }
    const meta = capabilityFallbackMeta()
    const row = db.prepare('SELECT * FROM canonical_capability WHERE key=?').get(arg)
    if (!row) { print({ error: 'not found: ' + arg, ...meta }); break }
    const { id: _unsafeCloudId, ...rowWithoutSyntheticId } = row
    print({ ...rowWithoutSyntheticId, ...meta })
    break
  }
  case 'donors':
    print(db.prepare('SELECT donor,true_capabilities,target_crates,priority FROM donor_scope ORDER BY true_capabilities DESC').all())
    break
  case 'verified': {
    if (hasUsableTable('capability')) {
      print(db.prepare(`SELECT key,title,target_crate,acceptance_test,fin_test FROM capability WHERE status='verified' ORDER BY slice`).all())
      break
    }
    const meta = capabilityFallbackMeta()
    const rows = db.prepare(`SELECT key,canonical_name,target_crate,acceptance_test,financial_control_test FROM canonical_capability
      WHERE status='verified' ORDER BY key`).all()
    print({ ...meta, rows })
    break
  }
  case 'file-census':
    if (!requireFileCensus()) break
    print(db.prepare(`SELECT
      count(*) rows,
      count(DISTINCT donor) donors,
      sum(CASE WHEN read_status='unread_pending' THEN 1 ELSE 0 END) unread_pending,
      sum(CASE WHEN classification='capability_review_pending' THEN 1 ELSE 0 END) capability_review_pending,
      sum(CASE WHEN classification='behavior_review_pending' THEN 1 ELSE 0 END) behavior_review_pending,
      sum(CASE WHEN classification='generated_vendor_build_artifact' THEN 1 ELSE 0 END) artifacts,
      sum(CASE WHEN classification='non_behavioral_support' THEN 1 ELSE 0 END) support
      FROM donor_file_census`).get())
    break
  case 'file-census-completion':
    if (!requireFileCensus()) break
    print({
      totals: db.prepare(`SELECT
        count(*) rows,
        count(DISTINCT donor) donors,
        sum(CASE WHEN read_status='unread_pending' THEN 1 ELSE 0 END) unread_pending,
        sum(CASE WHEN classification='capability_review_pending' THEN 1 ELSE 0 END) capability_review_pending,
        sum(CASE WHEN classification='behavior_review_pending' THEN 1 ELSE 0 END) behavior_review_pending,
        sum(CASE WHEN classification IN ('mapped','duplicate_variant_mapped') AND COALESCE(mapped_source_ids,'')='' THEN 1 ELSE 0 END) mapped_missing_source_ids,
        sum(CASE WHEN classification='blocked' THEN 1 ELSE 0 END) blocked
        FROM donor_file_census`).get(),
      fully_reviewed_donors: db.prepare(`SELECT count(*) n FROM (
        SELECT donor, sum(CASE WHEN read_status='unread_pending' THEN 1 ELSE 0 END) pending
        FROM donor_file_census GROUP BY donor HAVING pending=0
      )`).get().n,
      source_rows_unmapped: db.prepare(`SELECT count(*) n FROM source_capability WHERE canonical_id IS NULL`).get().n,
      canonical_rows_unsurfaced_hint: 'run pnpm docs:verify',
    })
    break
  case 'file-census-donors':
    if (!requireFileCensus()) break
    print(db.prepare(`SELECT donor, count(*) rows,
      sum(CASE WHEN read_status='unread_pending' THEN 1 ELSE 0 END) unread_pending,
      sum(CASE WHEN classification='capability_review_pending' THEN 1 ELSE 0 END) capability_review_pending,
      sum(CASE WHEN classification='behavior_review_pending' THEN 1 ELSE 0 END) behavior_review_pending,
      sum(CASE WHEN classification='generated_vendor_build_artifact' THEN 1 ELSE 0 END) artifacts
      FROM donor_file_census GROUP BY donor ORDER BY unread_pending DESC, rows DESC LIMIT ?`).all(limit))
    break
  case 'file-census-gaps':
    if (!requireFileCensus()) break
    print(db.prepare(`SELECT donor,path,kind,classification,read_status,exclusion_reason
      FROM donor_file_census
      WHERE read_status='unread_pending' OR classification LIKE '%review_pending'
      ORDER BY donor,path LIMIT ?`).all(limit))
    break
  case 'census': {
    const m = Object.fromEntries(db.prepare('SELECT k,v FROM meta').all().map(r => [r.k, r.v]))
    print({
      source_capabilities: m.source_capability_count, canonical_capabilities: m.canonical_capability_count,
      money_canonical: m.money_canonical_count, verified_slices: m.verified_slices,
      donors_extracted: m.donors_extracted, donors_blocked: m.donors_blocked,
      roadmap_pct_of_ideal: m.roadmap_pct_of_ideal, verified_pct_of_ideal: m.verified_pct_of_ideal,
      dedupe_method: m.dedupe_method, note: m.note,
    })
    break
  }
  case 'canonical-by-domain':
    requireCanonicalReadSchema()
    print(db.prepare('SELECT domain, count(*) n, sum(moves_money) money FROM canonical_capability GROUP BY domain ORDER BY n DESC').all())
    break
  case 'money-canonical':
    requireCanonicalReadSchema()
    print(db.prepare('SELECT key, canonical_name, domain, donor_count, status FROM canonical_capability WHERE moves_money=1 ORDER BY donor_count DESC').all())
    break
  case 'next-unverified-canonical':
    requireCanonicalReadSchema()
    print(db.prepare("SELECT key, canonical_name, domain, target_crate, moves_money, donor_count FROM canonical_capability WHERE status='unimplemented' ORDER BY donor_count DESC LIMIT ?").all(limit))
    break
  case 'target-crate-census':
    requireCanonicalReadSchema()
    print(targetCrateCensus(
      db.prepare('SELECT key, canonical_name, domain, target_crate, moves_money, status, donor_count FROM canonical_capability').all(),
      { root: process.cwd(), limit, source: 'canonical_capability', targetFilter: arg || null },
    ))
    break
  case 'full-census':
    requireCanonicalReadSchema()
    print(fullCapabilityCensus(
      db.prepare('SELECT key, canonical_name, domain, target_crate, moves_money, status, donor_count FROM canonical_capability').all(),
      { root: process.cwd(), limit, source: 'canonical_capability' },
    ))
    break
  case 'get-canonical': {
    // an EMPTY local canonical_capability table (0 rows) is treated the same as a missing one:
    // it falls through to the source_capability/side-table reconstruction below instead of
    // silently reporting "not found" against a table that lost its truth (issue #713).
    if (isUsableCanonicalCapabilityTable(db, 'canonical_capability')) {
      requireColumns('canonical_capability', ['id', 'key'])
      requireColumns('provenance', ['canonical_id', 'source_id'])
      requireColumns('source_capability', ['id', 'donor', 'canonical_name', 'source_files'])
      const cc = db.prepare('SELECT * FROM canonical_capability WHERE key=?').get(arg)
      if (!cc) { print({ error: 'not found: ' + arg }); break }
      cc.provenance = db.prepare('SELECT s.donor, s.canonical_name, s.source_files FROM provenance p JOIN source_capability s ON p.source_id=s.id WHERE p.canonical_id=?').all(cc.id)
      print(cc)
      break
    }
    attachCanonicalCapabilityFallback(db)
    requireColumns('canonical_capability', ['key', 'canonical_name'])
    const cloudCanonical = db.prepare('SELECT * FROM canonical_capability WHERE key=?').get(arg)
    if (!cloudCanonical) { print({ error: 'not found: ' + arg, capability_schema: 'side_tables_plus_cloud_snapshot_fallback' }); break }
    const status = hasTable('canonical_status_override')
      ? db.prepare('SELECT * FROM canonical_status_override WHERE canonical_key=?').get(arg) || null
      : null
    const impl = hasTable('impl_evidence')
      ? db.prepare('SELECT * FROM impl_evidence WHERE canonical_key=?').get(arg) || null
      : null
    const { id: _unsafeCloudId, ...cloudCanonicalWithoutSyntheticId } = cloudCanonical
    print({
      ...cloudCanonicalWithoutSyntheticId,
      capability_schema: 'side_tables_plus_cloud_snapshot_fallback',
      local_audit_required: true,
      status: status?.status || cloudCanonical.status || null,
      moves_money: status?.moves_money ?? cloudCanonical.moves_money ?? 0,
      requires_approval: status?.requires_approval ?? cloudCanonical.requires_approval ?? 0,
      acceptance_test: status?.acceptance_test || impl?.test_symbol || null,
      implementation: impl,
      provenance: [],
      provenance_status: 'LOCAL_AUDIT_REQUIRED_UNSAFE_SYNTHETIC_ID_JOIN',
      schema_warning: 'canonical_capability table is missing; showing key-addressed cloud/status/evidence data without numeric-ID provenance joins',
    })
    break
  }
  case 'coverage-gaps':
    print(db.prepare("SELECT donor, status, capabilities, substr(coverage_note,1,90) note FROM census_coverage WHERE status='blocked' OR coverage_note LIKE '%partial%' OR coverage_note LIKE '%65%' OR coverage_note LIKE '%35%' OR coverage_note LIKE '%25-30%' ORDER BY capabilities").all())
    break
  case 'sql':
    if (!/^\s*select/i.test(arg || '')) { console.error('read-only: SELECT only'); process.exit(2) }
    print(db.prepare(arg).all())
    break
  default:
    console.log(`capability store query — docs/capabilities.db
Commands:
  scope                      headline numbers (true denominator, %)
  next-money [--limit N]     next buildable money capabilities
  by-crate <crate>           capabilities for a crate
  by-status <status> [--limit N]
  get <key>                  full execution spec for one capability
  donors                     per-donor true scope
  verified                   the verified capabilities + their tests
  file-census                file-level donor census totals
  file-census-completion     literal completion gate counters
  file-census-donors         per-donor file census gaps
  file-census-gaps [--limit N]
  target-crate-census [crate-or-fragment] [--limit N]
  full-census [--limit N]
  sql "<SELECT ...>"         raw read-only query`)
}
db.close()
