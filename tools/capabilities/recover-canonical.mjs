#!/usr/bin/env node
// recover-canonical.mjs — dedicated, safety-gated recovery for a docs/capabilities.db whose
// canonical_capability table is MISSING or EMPTY while its survivable side tables
// (canonical_status_override, impl_evidence, slice_canonical, agent_note, donor_file_census, ...)
// still carry real prior work (issue #713).
//
// This tool is ADDITIVE ONLY: it never deletes or updates a row in any table except INSERTing
// fresh rows into canonical_capability (which is empty/missing to begin with) and then
// re-projecting canonical_status_override onto those new rows exactly the way
// reapply-overrides.mjs already does after every normal rebuild. Every other side table
// (impl_evidence, slice_canonical, agent_note, donor_file_census, donor_absorption,
// package_retirement, execution_state, tracking_issue, opportunity, ...) is never touched, so it
// is preserved by construction, not by a special-cased copy step.
//
// SAFETY MODEL:
//   - DRY-RUN BY DEFAULT. Nothing is written unless --apply is passed.
//   - Refuses to run at all if canonical_capability already has rows (not a recovery scenario;
//     use `pnpm caps:rebuild` for a normal rebuild). Re-running after a successful recovery is
//     therefore a safe, idempotent no-op.
//   - Refuses if the recovery sources are malformed (bad schema, duplicate keys, empty keys) or
//     entirely absent, WITHOUT touching the live db.
//   - Before any write, copies the live db file (+ -wal/-shm sidecars) byte-for-byte to an
//     explicit backup path.
//   - All mutation runs inside one sqlite transaction. On ANY error (including a test-only
//     injected fault simulating an interrupted rebuild) or a failed post-write verification, the
//     live db file is restored from the backup bytes, so the original file is always either
//     untouched or byte-identical to its pre-apply state.
//
// Usage:
//   node tools/capabilities/recover-canonical.mjs [--db <path>]                 # dry-run (default)
//   node tools/capabilities/recover-canonical.mjs --apply [--db <path>] [--backup-path <path>]
import Database from 'better-sqlite3'
import { copyFileSync, existsSync, statSync, unlinkSync } from 'node:fs'
import { dirname, join } from 'node:path'
import { CAPABILITIES_DB } from '../_paths.mjs'
import {
  CANONICAL_CAPABILITY_DDL,
  REQUIRED_ARCH_LINK_COLUMNS,
  REQUIRED_CLOUD_CORE_COLUMNS,
  assertNoInventedVerified,
  mergeRecoverySources,
  missingColumns,
} from './canonical-recovery-lib.mjs'

function arg(flag) {
  const i = process.argv.indexOf(flag)
  return i >= 0 ? process.argv[i + 1] : undefined
}
const APPLY = process.argv.includes('--apply')
const DB_PATH = arg('--db') || CAPABILITIES_DB
const BACKUP_PATH = arg('--backup-path') || `${DB_PATH}.recover-backup-${new Date().toISOString().replace(/[:.]/g, '-')}`
// test-only fault injection: throws inside the write transaction after the insert step, to prove
// an interrupted rebuild leaves the original db byte-restored. Never set in normal operation.
const INJECT_FAILURE_AFTER_INSERT = process.env.RECOVER_CANONICAL_INJECT_FAILURE_AFTER_INSERT === '1'

function report(status, body) {
  console.log(JSON.stringify({ status, ...body }, null, 2))
}
function fail(status, body) {
  report(status, body)
  process.exit(status === 'ALREADY_RECOVERED_NOOP' ? 0 : 1)
}

function tableColumns(db, name) {
  return db.prepare(`PRAGMA table_info(${name})`).all().map((r) => r.name)
}
function hasTable(db, name) {
  return db.prepare("SELECT count(*) n FROM sqlite_master WHERE type='table' AND name=?").get(name).n > 0
}
function rowCount(db, name) {
  return db.prepare(`SELECT count(*) n FROM "${name}"`).get().n
}
function integrityOk(db) {
  return db.prepare('PRAGMA integrity_check').get().integrity_check === 'ok'
}
function canonicalReferenceIssues(db, expectedRows) {
  const expectedById = new Map(expectedRows.map((row) => [row.id, row.key]))
  const issues = []
  for (const table of ['source_capability', 'provenance']) {
    if (!hasTable(db, table) || !tableColumns(db, table).includes('canonical_id')) continue
    const references = db.prepare(`SELECT DISTINCT canonical_id FROM "${table}" WHERE canonical_id IS NOT NULL`).all()
    const missingIds = references.map((row) => row.canonical_id).filter((id) => !expectedById.has(id))
    if (missingIds.length) {
      issues.push({ type: 'DANGLING_CANONICAL_REFERENCE', table, count: missingIds.length, sample_ids: missingIds.slice(0, 20) })
    }
  }
  if (
    hasTable(db, 'source_capability') && hasTable(db, 'provenance') &&
    tableColumns(db, 'source_capability').includes('canonical_id') &&
    tableColumns(db, 'provenance').includes('source_id')
  ) {
    const mismatches = db.prepare(`
      SELECT p.source_id, p.canonical_id provenance_canonical_id, s.canonical_id source_canonical_id
      FROM provenance p
      LEFT JOIN source_capability s ON s.id=p.source_id
      WHERE s.id IS NULL OR s.canonical_id IS NULL OR s.canonical_id != p.canonical_id
      LIMIT 20
    `).all()
    if (mismatches.length) issues.push({ type: 'SOURCE_PROVENANCE_REFERENCE_MISMATCH', sample: mismatches })
  }
  return issues
}
function sideTableCounts(db) {
  const names = [
    'canonical_status_override', 'impl_evidence', 'slice_canonical', 'agent_note',
    'donor_file_census', 'donor_absorption', 'source_capability', 'provenance',
  ]
  const out = {}
  for (const name of names) out[name] = hasTable(db, name) ? rowCount(db, name) : null
  return out
}

if (!existsSync(DB_PATH)) {
  fail('BLOCKED_WITH_EVIDENCE', {
    reason: 'LOCAL_AUDIT_REQUIRED: the target db file does not exist. Recovery only ever repairs an ' +
      'EXISTING db whose canonical_capability table was lost — it never creates a brand-new db from ' +
      'nothing, and it never operates on a file that is not the user\'s own copy.',
    db_path: DB_PATH,
  })
}

// ── 1) inspect current state (read-only) ──────────────────────────────────────────────────────
const probe = new Database(DB_PATH, { readonly: true, fileMustExist: true })
if (!integrityOk(probe)) {
  probe.close()
  fail('BLOCKED_WITH_EVIDENCE', { reason: 'target db failed PRAGMA integrity_check; refusing to touch a possibly-corrupt file.', db_path: DB_PATH })
}
const preCanonicalExists = hasTable(probe, 'canonical_capability')
const preCanonicalRows = preCanonicalExists ? rowCount(probe, 'canonical_capability') : 0
const preSideTables = sideTableCounts(probe)
probe.close()

if (preCanonicalRows > 0) {
  fail('ALREADY_RECOVERED_NOOP', {
    reason: 'canonical_capability already has rows; this is not a missing/empty-table recovery scenario. ' +
      'Re-running this command after a successful recovery is expected to no-op here. Use `pnpm caps:rebuild` ' +
      'for a normal rebuild.',
    canonical_capability_rows: preCanonicalRows,
  })
}

// ── 2) load + validate recovery sources (read-only; never touches the live db) ────────────────
const docsDir = dirname(DB_PATH)
const cloudCorePath = join(docsDir, 'capabilities-cloud', 'cap-core.db')
const cloudProvenancePath = join(docsDir, 'capabilities-cloud', 'cap-provenance.db')
const archDbPath = join(docsDir, 'architecture.db')

let cloudCoreRows = []
let cloudCoreSourceOk = false
let cloudCoreGeneration = null
const sourceIssues = []
if (existsSync(cloudCorePath)) {
  const core = new Database(cloudCorePath, { readonly: true, fileMustExist: true })
  if (!integrityOk(core)) {
    sourceIssues.push({ type: 'MALFORMED_SNAPSHOT_INTEGRITY_CHECK', source: cloudCorePath })
  } else if (!hasTable(core, 'canonical_capability')) {
    sourceIssues.push({ type: 'MALFORMED_SNAPSHOT_MISSING_TABLE', source: cloudCorePath, table: 'canonical_capability' })
  } else {
    if (hasTable(core, 'cloud_snapshot_info')) {
      cloudCoreGeneration = core.prepare("SELECT value FROM cloud_snapshot_info WHERE key='generated_at'").get()?.value ?? null
    }
    const missing = missingColumns(tableColumns(core, 'canonical_capability'), REQUIRED_CLOUD_CORE_COLUMNS)
    if (missing.length) {
      sourceIssues.push({ type: 'MALFORMED_SNAPSHOT_MISSING_COLUMNS', source: cloudCorePath, table: 'canonical_capability', missing })
    } else if (
      hasTable(core, 'meta') &&
      tableColumns(core, 'meta').includes('k') &&
      core.prepare("SELECT count(*) n FROM meta WHERE k='canonical_recovery'").get().n > 0
    ) {
      sourceIssues.push({
        type: 'UNTRUSTED_RECOVERED_CORE_IDS',
        source: cloudCorePath,
        reason: 'cap-core.db was itself reconstructed from generated docs; its synthetic IDs are not safe to join to source/provenance side tables',
      })
    } else {
      cloudCoreRows = core.prepare('SELECT * FROM canonical_capability').all()
      cloudCoreSourceOk = true
    }
  }
  core.close()
}

const hasSurvivingCanonicalReferences =
  (preSideTables.source_capability ?? 0) > 0 || (preSideTables.provenance ?? 0) > 0
if (sourceIssues.length === 0 && hasSurvivingCanonicalReferences) {
  if (!existsSync(cloudProvenancePath)) {
    sourceIssues.push({ type: 'MISSING_MATCHED_PROVENANCE_SHARD', source: cloudProvenancePath })
  } else {
    const provenanceShard = new Database(cloudProvenancePath, { readonly: true, fileMustExist: true })
    if (!integrityOk(provenanceShard)) {
      sourceIssues.push({ type: 'MALFORMED_PROVENANCE_SNAPSHOT_INTEGRITY_CHECK', source: cloudProvenancePath })
    } else if (!hasTable(provenanceShard, 'source_capability') || !hasTable(provenanceShard, 'provenance')) {
      sourceIssues.push({ type: 'MALFORMED_PROVENANCE_SNAPSHOT_MISSING_TABLE', source: cloudProvenancePath })
    } else {
      const provenanceGeneration = hasTable(provenanceShard, 'cloud_snapshot_info')
        ? provenanceShard.prepare("SELECT value FROM cloud_snapshot_info WHERE key='generated_at'").get()?.value ?? null
        : null
      if (!cloudCoreGeneration || !provenanceGeneration || cloudCoreGeneration !== provenanceGeneration) {
        sourceIssues.push({
          type: 'MIXED_OR_UNKNOWN_SNAPSHOT_GENERATION',
          core_generated_at: cloudCoreGeneration,
          provenance_generated_at: provenanceGeneration,
        })
      }

      const localMappings = new Database(DB_PATH, { readonly: true, fileMustExist: true })
      for (const table of ['source_capability', 'provenance']) {
        const order = table === 'source_capability' ? 'id' : 'source_id, canonical_id'
        const projection = table === 'source_capability' ? 'id, canonical_id' : 'source_id, canonical_id'
        const localRows = localMappings.prepare(`SELECT ${projection} FROM ${table} ORDER BY ${order}`).all()
        const shardRows = provenanceShard.prepare(`SELECT ${projection} FROM ${table} ORDER BY ${order}`).all()
        if (JSON.stringify(localRows) !== JSON.stringify(shardRows)) {
          sourceIssues.push({
            type: 'LOCAL_PROVENANCE_DIFFERS_FROM_SNAPSHOT',
            table,
            local_rows: localRows.length,
            snapshot_rows: shardRows.length,
          })
        }
      }
      localMappings.close()
    }
    provenanceShard.close()
  }
}

let archLinkRows = []
if (sourceIssues.length === 0 && existsSync(archDbPath)) {
  const arch = new Database(archDbPath, { readonly: true, fileMustExist: true })
  if (integrityOk(arch) && hasTable(arch, 'capability_architecture_link')) {
    const missing = missingColumns(tableColumns(arch, 'capability_architecture_link'), REQUIRED_ARCH_LINK_COLUMNS)
    if (missing.length === 0) {
      archLinkRows = arch.prepare(`
        SELECT capability_key, target_crate, target_module, status
        FROM capability_architecture_link
        WHERE capability_key IS NOT NULL AND capability_key <> ''
      `).all()
    }
  }
  arch.close()
}

if (sourceIssues.length) {
  fail('BLOCKED_WITH_EVIDENCE', {
    reason: 'a recovery source is malformed; refusing to recover from a source this tool cannot trust. ' +
      'The live db was NOT touched.',
    source_issues: sourceIssues,
  })
}
if (!cloudCoreSourceOk && archLinkRows.length === 0) {
  fail('BLOCKED_WITH_EVIDENCE', {
    reason: 'no usable recovery source available (docs/capabilities-cloud/cap-core.db and ' +
      'docs/architecture.db are both absent or empty). Nothing to recover from.',
    cloud_core_path: cloudCorePath,
    architecture_db_path: archDbPath,
  })
}

const merged = mergeRecoverySources({ cloudCoreRows, archLinkRows })
if (merged.issues.length) {
  fail('BLOCKED_WITH_EVIDENCE', {
    reason: 'recovery sources contain malformed/duplicate rows; refusing a partial or ambiguous merge. ' +
      'The live db was NOT touched.',
    source_issues: merged.issues,
  })
}
try {
  assertNoInventedVerified(merged.rows)
} catch (error) {
  fail('BLOCKED_WITH_EVIDENCE', { reason: String(error.message || error) })
}

const referenceProbe = new Database(DB_PATH, { readonly: true, fileMustExist: true })
const preReferenceIssues = canonicalReferenceIssues(referenceProbe, merged.rows)
referenceProbe.close()
if (preReferenceIssues.length) {
  fail('BLOCKED_WITH_EVIDENCE', {
    reason: 'recovery candidates cannot preserve every surviving source/provenance canonical reference. ' +
      'Refusing to attach side-table evidence to a different key or leave dangling references.',
    reference_issues: preReferenceIssues,
  })
}

// ── 3) dry-run report (always computed; this is as far as a plain invocation goes) ────────────
const preExistingKeysNote = preCanonicalExists
  ? 'canonical_capability table exists but has 0 rows (an empty shell — treated identically to a missing table).'
  : 'canonical_capability table does not exist yet.'
const dryRunReport = {
  db_path: DB_PATH,
  pre_state: { canonical_capability_table_exists: preCanonicalExists, canonical_capability_rows: preCanonicalRows, note: preExistingKeysNote, side_tables: preSideTables },
  recovery_sources: {
    cloud_core: { path: cloudCorePath, available: cloudCoreSourceOk, rows: cloudCoreRows.length },
    architecture_db: { path: archDbPath, available: archLinkRows.length > 0, distinct_capability_keys: new Set(archLinkRows.map((r) => r.capability_key)).size },
  },
  candidate_insert_count: merged.rows.length,
  candidate_insert_by_source: {
    'docs/capabilities-cloud/cap-core.db': merged.rows.filter((r) => r.recovery_source === 'docs/capabilities-cloud/cap-core.db').length,
    'docs/architecture.db': merged.rows.filter((r) => r.recovery_source === 'docs/architecture.db').length,
  },
  destructive_changes: [], // by design: this tool only ever INSERTs into an empty/missing table
  post_row_count_predicted: merged.rows.length,
  preserved_side_tables_untouched: Object.keys(preSideTables),
}

if (!APPLY) {
  report('DRY_RUN', dryRunReport)
  process.exit(0)
}

// ── 4) apply: backup -> transaction -> verify -> commit-or-restore ────────────────────────────
for (const suffix of ['', '-wal', '-shm']) {
  const src = `${DB_PATH}${suffix}`
  if (existsSync(src)) copyFileSync(src, `${BACKUP_PATH}${suffix}`)
}
const originalBytes = statSync(DB_PATH).size

function restoreFromBackup() {
  for (const suffix of ['', '-wal', '-shm']) {
    const bak = `${BACKUP_PATH}${suffix}`
    const live = `${DB_PATH}${suffix}`
    if (existsSync(bak)) copyFileSync(bak, live)
    else if (existsSync(live)) unlinkSync(live)
  }
}

const db = new Database(DB_PATH)
db.pragma('journal_mode = WAL')

let applyError = null
try {
  const tx = db.transaction(() => {
    db.exec(CANONICAL_CAPABILITY_DDL)
    const ins = db.prepare(`INSERT INTO canonical_capability
      (id, key, canonical_name, domain, target_crate, target_module, side_effect_class, moves_money,
       requires_approval, acceptance_criteria, required_tests, financial_control_test,
       acceptance_test, status, exclusion_note, blocker, slice, donor_count)
      VALUES (@id, @key, @canonical_name, @domain, @target_crate, @target_module, @side_effect_class,
       @moves_money, @requires_approval, @acceptance_criteria, @required_tests,
       @financial_control_test, @acceptance_test, @status, @exclusion_note, @blocker, @slice, @donor_count)`)
    for (const row of merged.rows) ins.run(row)

    if (INJECT_FAILURE_AFTER_INSERT) {
      throw new Error('RECOVER_CANONICAL_INJECT_FAILURE_AFTER_INSERT: simulated interrupted rebuild')
    }

    // re-project the survivable status overrides + derive .slice onto the freshly inserted rows,
    // exactly as reapply-overrides.mjs does after every normal rebuild.
    if (hasTable(db, 'canonical_status_override')) {
      const canonKeys = new Set(db.prepare('SELECT key FROM canonical_capability').all().map((r) => r.key))
      const upd = db.prepare(`UPDATE canonical_capability
        SET status = COALESCE(@status, status),
            acceptance_test = COALESCE(@acceptance_test, acceptance_test),
            financial_control_test = COALESCE(@financial_control_test, financial_control_test),
            moves_money = COALESCE(@moves_money, moves_money),
            requires_approval = COALESCE(@requires_approval, requires_approval),
            blocker = COALESCE(@blocker, blocker)
        WHERE key = @canonical_key`)
      for (const o of db.prepare('SELECT * FROM canonical_status_override').all()) {
        if (!canonKeys.has(o.canonical_key)) continue
        // money invariant: never re-apply a 'verified' override onto a money cap without a real
        // financial-control test (mirrors reapply-overrides.mjs's guard).
        const moneyRow = db.prepare('SELECT moves_money, financial_control_test FROM canonical_capability WHERE key=?').get(o.canonical_key)
        const money = (o.moves_money === 0 || o.moves_money === 1) ? o.moves_money === 1 : moneyRow?.moves_money === 1
        const fin = o.financial_control_test ?? moneyRow?.financial_control_test ?? ''
        const finOk = fin && !/^REQUIRED|^—$|^-$/.test(String(fin).trim())
        if (o.status === 'verified' && money && !finOk) continue
        upd.run({
          canonical_key: o.canonical_key,
          status: o.status ?? null,
          acceptance_test: o.acceptance_test ?? null,
          financial_control_test: o.financial_control_test ?? null,
          moves_money: (o.moves_money === 0 || o.moves_money === 1) ? o.moves_money : null,
          requires_approval: (o.requires_approval === 0 || o.requires_approval === 1) ? o.requires_approval : null,
          blocker: o.blocker ?? null,
        })
      }
    }
    if (hasTable(db, 'slice_canonical')) {
      db.exec(`UPDATE canonical_capability SET slice = (
        SELECT sc.slice FROM slice_canonical sc
        WHERE sc.canonical_key = canonical_capability.key
        ORDER BY (sc.confidence='high') DESC, (sc.confidence='medium') DESC, sc.slice ASC
        LIMIT 1)
        WHERE key IN (SELECT canonical_key FROM slice_canonical)`)
    }
    const referenceIssues = canonicalReferenceIssues(db, merged.rows)
    if (referenceIssues.length) {
      throw new Error(`post-insert canonical reference validation failed: ${JSON.stringify(referenceIssues)}`)
    }
  })
  tx()
} catch (error) {
  applyError = error
}

if (applyError) {
  db.close()
  restoreFromBackup()
  fail('FAILED_ROLLED_BACK', {
    reason: String(applyError.message || applyError),
    backup_path: BACKUP_PATH,
    original_db_restored: true,
  })
}

// ── 5) post-write verification; restore from backup if anything is off ────────────────────────
const postRowCount = rowCount(db, 'canonical_capability')
const postSideTables = sideTableCounts(db)
const postIntegrityOk = integrityOk(db)
db.close()

const sideTableDrift = Object.keys(preSideTables).filter((name) => preSideTables[name] !== postSideTables[name])
const countMismatch = postRowCount !== merged.rows.length
if (!postIntegrityOk || sideTableDrift.length > 0 || countMismatch) {
  restoreFromBackup()
  fail('FAILED_POST_VERIFICATION_RESTORED', {
    reason: 'post-write verification failed; the original db has been restored byte-for-byte from the backup.',
    integrity_check_ok: postIntegrityOk,
    side_table_drift: sideTableDrift,
    predicted_row_count: merged.rows.length,
    actual_row_count: postRowCount,
    backup_path: BACKUP_PATH,
  })
}

report('RECOVERED', {
  db_path: DB_PATH,
  backup_path: BACKUP_PATH,
  pre_row_count: preCanonicalRows,
  post_row_count: postRowCount,
  inserted: merged.rows.length,
  side_tables_preserved: postSideTables,
  original_db_size_bytes: originalBytes,
  next_step: 'run `pnpm caps:rebuild` (or at least `node tools/capabilities/reconcile-w9-target-keys.mjs`) to reconcile the Wave 9 target keys, then `pnpm docs:gen && pnpm docs:verify`.',
})
