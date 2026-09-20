#!/usr/bin/env node
// verify-cloud-snapshot.mjs - validate generated cloud-readable capability shards.
//
// Usage:
//   node tools/capabilities/verify-cloud-snapshot.mjs
//   node tools/capabilities/verify-cloud-snapshot.mjs --max-mib 50
import Database from 'better-sqlite3'
import { existsSync, readFileSync, statSync } from 'node:fs'
import { join, relative } from 'node:path'
import { DOCS } from '../_paths.mjs'

const args = process.argv.slice(2)
const readArg = (name, fallback) => {
  const idx = args.indexOf(name)
  if (idx === -1) return fallback
  return args[idx + 1] ?? fallback
}

const OUT_DIR = readArg('--dir', join(DOCS, 'capabilities-cloud'))
const MAX_MIB = Number(readArg('--max-mib', '50'))
const SHARDS = {
  core: 'cap-core.db',
  provenance: 'cap-provenance.db',
  census: 'cap-census-summary.db',
  architecture: 'cap-architecture.db',
  workqueue: 'cap-workqueue.db',
}
const rel = (path) => relative(process.cwd(), path).replace(/\\/g, '/')

const issues = []
const fail = (message) => issues.push(message)
const warnings = []
const warn = (message) => warnings.push(message)

const open = (name) => new Database(join(OUT_DIR, SHARDS[name]), { readonly: true })
const one = (db, sql) => db.prepare(sql).get()
const count = (db, table) => one(db, `SELECT count(*) n FROM "${table}"`).n

const requireFile = (path) => {
  if (!existsSync(path)) fail(`missing file: ${rel(path)}`)
}

const integrityCheck = (name) => {
  const path = join(OUT_DIR, SHARDS[name])
  if (!existsSync(path)) return
  const db = new Database(path, { readonly: true })
  const result = db.prepare('PRAGMA integrity_check').get().integrity_check
  db.close()
  if (result !== 'ok') fail(`${SHARDS[name]} integrity_check failed: ${result}`)
}

const checkSize = (name) => {
  const path = join(OUT_DIR, SHARDS[name])
  if (!existsSync(path)) return
  const mib = statSync(path).size / 1024 / 1024
  if (mib > MAX_MIB) fail(`${SHARDS[name]} is ${mib.toFixed(2)} MiB, above ${MAX_MIB} MiB`)
}

const main = () => {
  const metaPath = join(OUT_DIR, 'meta.json')
  requireFile(metaPath)
  for (const shard of Object.values(SHARDS)) requireFile(join(OUT_DIR, shard))
  if (issues.length) {
    console.error(`verify-cloud-snapshot: ${issues.length} missing file issue(s)`)
    for (const issue of issues) console.error(`  - ${issue}`)
    process.exit(1)
  }

  const meta = JSON.parse(readFileSync(metaPath, 'utf8'))
  for (const name of Object.keys(SHARDS)) {
    checkSize(name)
    integrityCheck(name)
  }

  const core = open('core')
  const canonical = count(core, 'canonical_capability')
  const money = one(core, 'SELECT count(*) n FROM canonical_capability WHERE moves_money=1').n
  const moneyWithoutApproval = one(
    core,
    "SELECT count(*) n FROM canonical_capability WHERE moves_money=1 AND COALESCE(requires_approval,0) != 1",
  ).n
  const verified = one(core, "SELECT count(*) n FROM canonical_capability WHERE status='verified'").n
  const implEvidence = count(core, 'impl_evidence')
  const canonicalIds = new Set(core.prepare('SELECT id FROM canonical_capability').all().map((row) => row.id))
  const coreWasRecoveredFromDocs = one(
    core,
    "SELECT count(*) n FROM sqlite_master WHERE type='table' AND name='meta'",
  ).n > 0 && one(core, "SELECT count(*) n FROM meta WHERE k='canonical_recovery'").n > 0
  const coreGeneration = one(
    core,
    "SELECT count(*) n FROM sqlite_master WHERE type='table' AND name='cloud_snapshot_info'",
  ).n > 0
    ? core.prepare("SELECT value FROM cloud_snapshot_info WHERE key='generated_at'").get()?.value ?? null
    : null
  core.close()

  if (canonical !== meta.source.canonical_capabilities) {
    fail(`canonical count mismatch: core=${canonical} meta=${meta.source.canonical_capabilities}`)
  }
  if (money !== meta.source.money_canonical) fail(`money count mismatch: core=${money} meta=${meta.source.money_canonical}`)
  if (moneyWithoutApproval !== 0) fail(`${moneyWithoutApproval} money capabilities are not approval-gated`)
  if (coreWasRecoveredFromDocs) {
    warn('cap-core.db canonical rows were recovered from generated docs; synthetic canonical IDs are not safe to join to cap-provenance.db')
  }

  const provenance = open('provenance')
  const sourceRows = count(provenance, 'source_capability')
  const provenanceRows = count(provenance, 'provenance')
  const provenanceGeneration = one(
    provenance,
    "SELECT count(*) n FROM sqlite_master WHERE type='table' AND name='cloud_snapshot_info'",
  ).n > 0
    ? provenance.prepare("SELECT value FROM cloud_snapshot_info WHERE key='generated_at'").get()?.value ?? null
    : null
  if (!coreGeneration || !provenanceGeneration || coreGeneration !== provenanceGeneration) {
    fail(`core/provenance snapshot generation mismatch: core=${coreGeneration ?? 'missing'} provenance=${provenanceGeneration ?? 'missing'}`)
  }
  for (const table of ['source_capability', 'provenance']) {
    const referenced = provenance
      .prepare(`SELECT DISTINCT canonical_id FROM ${table} WHERE canonical_id IS NOT NULL`)
      .all()
      .map((row) => row.canonical_id)
    const dangling = referenced.filter((id) => !canonicalIds.has(id))
    if (dangling.length) {
      const message = `${SHARDS.provenance}.${table} has ${dangling.length} canonical ID reference(s) absent from cap-core.db; sample=${dangling.slice(0, 20).join(',')}`
      if (coreWasRecoveredFromDocs) warn(message)
      else fail(message)
    }
  }
  const sourceProvenanceMismatch = one(provenance, `
    SELECT count(*) n
    FROM provenance p
    LEFT JOIN source_capability s ON s.id=p.source_id
    WHERE s.id IS NULL OR s.canonical_id IS NULL OR s.canonical_id != p.canonical_id
  `).n
  if (sourceProvenanceMismatch > 0) {
    fail(`${SHARDS.provenance} has ${sourceProvenanceMismatch} provenance row(s) that disagree with source_capability.canonical_id`)
  }
  provenance.close()
  if (sourceRows !== meta.source.source_capabilities) {
    fail(`source count mismatch: provenance=${sourceRows} meta=${meta.source.source_capabilities}`)
  }
  if (provenanceRows !== meta.source.provenance_rows) {
    fail(`provenance count mismatch: provenance=${provenanceRows} meta=${meta.source.provenance_rows}`)
  }

  const census = open('census')
  const censusTotals = one(census, 'SELECT * FROM donor_file_census_totals')
  census.close()
  if (censusTotals.rows !== meta.source.donor_file_census_rows) {
    fail(`file census total mismatch: census=${censusTotals.rows} meta=${meta.source.donor_file_census_rows}`)
  }
  if (censusTotals.unread_pending !== meta.source.donor_file_census_unread_pending) {
    fail(
      `file census pending mismatch: census=${censusTotals.unread_pending} meta=${meta.source.donor_file_census_unread_pending}`,
    )
  }

  const workqueue = open('workqueue')
  const policies = count(workqueue, 'cloud_policy')
  const next = count(workqueue, 'next_unverified_canonical')
  const moneyNext = count(workqueue, 'next_money_canonical')
  if (policies < 5) fail(`cloud_policy has ${policies} rows, expected at least 5`)

  // ── queue-content consistency: next_unverified_canonical / next_money_canonical are
  // denormalized materialized views of canonical_capability (see export-cloud-snapshot.mjs's
  // exportWorkqueue()). A scoped patch that updates canonical_capability without regenerating
  // these views leaves a live, documented query surface (query-cloud-snapshot.mjs's `next`
  // command) returning stale target_crate/target_module for corrected capabilities -- this
  // exact defect was found and fixed 2026-07-12 (issue #550 review comment #4951211110). These
  // checks make that class of drift a hard verifier failure, not just a row count.
  const DELETED_CRATE_TARGETS = new Set(['chronica-execution', 'chronica-infra-execution', 'chronica-execution-state'])
  const coreForJoin = open('core')
  const canonicalByKey = new Map(
    coreForJoin
      .prepare('SELECT key, target_crate, target_module, moves_money, requires_approval FROM canonical_capability')
      .all()
      .map((row) => [row.key, row]),
  )
  coreForJoin.close()

  for (const queueTable of ['next_unverified_canonical', 'next_money_canonical']) {
    const queueRows = workqueue.prepare(`SELECT key, target_crate, target_module, moves_money, requires_approval FROM ${queueTable}`).all()
    let staleTarget = 0
    let orphanKey = 0
    let deletedCrateRef = 0
    for (const row of queueRows) {
      if (DELETED_CRATE_TARGETS.has(row.target_crate)) deletedCrateRef++
      const canonical = canonicalByKey.get(row.key)
      if (!canonical) {
        orphanKey++
        continue
      }
      if (
        canonical.target_crate !== row.target_crate ||
        canonical.target_module !== row.target_module ||
        canonical.moves_money !== row.moves_money ||
        canonical.requires_approval !== row.requires_approval
      ) {
        staleTarget++
      }
    }
    if (orphanKey > 0) {
      fail(`${queueTable} has ${orphanKey} key(s) with no matching row in cap-core.db canonical_capability (retired-key leakage)`)
    }
    if (staleTarget > 0) {
      fail(`${queueTable} has ${staleTarget} row(s) whose target_crate/target_module/moves_money/requires_approval diverge from cap-core.db canonical_capability (stale remap / content drift)`)
    }
    if (deletedCrateRef > 0) {
      fail(`${queueTable} has ${deletedCrateRef} row(s) referencing a deleted/nonexistent crate (${[...DELETED_CRATE_TARGETS].join(', ')})`)
    }
  }
  // Also enforce the same deleted-crate-reference rule on canonical_capability itself, in both
  // cap-core.db and this shard's own copy -- the queue-content check above only covers keys that
  // are currently unverified/money-flagged; this covers every canonical row unconditionally.
  for (const [shardName, table] of [['core', 'canonical_capability'], ['workqueue', 'canonical_capability']]) {
    const db = shardName === 'core' ? open('core') : workqueue
    const badRefs = db
      .prepare(`SELECT count(*) n FROM ${table} WHERE target_crate IN (${[...DELETED_CRATE_TARGETS].map((v) => `'${v}'`).join(',')})`)
      .get().n
    if (badRefs > 0) fail(`${SHARDS[shardName]}.${table} has ${badRefs} row(s) referencing a deleted/nonexistent crate`)
    if (shardName === 'core') db.close()
  }
  workqueue.close()

  const architecture = open('architecture')
  const archLinks = count(architecture, 'capability_architecture_link')
  const archGaps = count(architecture, 'architecture_target_gap')
  architecture.close()
  if (archLinks !== meta.architecture.capability_architecture_links) {
    fail(`architecture link mismatch: shard=${archLinks} meta=${meta.architecture.capability_architecture_links}`)
  }
  if (archGaps !== meta.architecture.architecture_target_gaps) {
    fail(`architecture gap mismatch: shard=${archGaps} meta=${meta.architecture.architecture_target_gaps}`)
  }

  console.log(
    `verify-cloud-snapshot: canonical=${canonical}, money=${money}, verified=${verified}, impl_evidence=${implEvidence}, next=${next}, money_next=${moneyNext}`,
  )
  console.log(
    `verify-cloud-snapshot: source=${sourceRows}, provenance=${provenanceRows}, census_rows=${censusTotals.rows}, arch_links=${archLinks}, arch_gaps=${archGaps}`,
  )
  if (warnings.length) {
    console.log(`verify-cloud-snapshot: ${warnings.length} warning(s)`)
    for (const warning of warnings) console.log(`  - ${warning}`)
  }

  if (issues.length) {
    console.error(`verify-cloud-snapshot: ${issues.length} issue(s)`)
    for (const issue of issues) console.error(`  - ${issue}`)
    process.exit(1)
  }

  console.log(`verify-cloud-snapshot: OK (${rel(OUT_DIR)})`)
}

main()
