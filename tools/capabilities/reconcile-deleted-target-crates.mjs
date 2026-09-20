#!/usr/bin/env node
// reconcile-deleted-target-crates.mjs
//
// Guarded capability metadata repair for legacy target_crate values that name
// crates deleted from the current Rust workspace. This does not promote status,
// change money/approval flags, or claim a replacement implementation owner.
// It clears only stale target_crate metadata on unimplemented rows so cloud
// shards stop advertising deleted crates as build targets.
import Database from 'better-sqlite3'
import { existsSync } from 'node:fs'
import { CAPABILITIES_DB } from '../_paths.mjs'

const DELETED_TARGET_CRATES = [
  'chronica-execution',
  'chronica-infra-execution',
  'chronica-execution-state',
]

const dryRun = process.argv.includes('--dry-run')

const fail = (message, details = {}) => {
  console.error(JSON.stringify({ error: message, ...details }, null, 2))
  process.exit(2)
}

if (!existsSync(CAPABILITIES_DB)) {
  fail('LOCAL_AUDIT_REQUIRED: docs/capabilities.db is absent.')
}

const db = new Database(CAPABILITIES_DB)

const tableExists = (name) =>
  db.prepare("SELECT count(*) n FROM sqlite_master WHERE type='table' AND name=?").get(name).n > 0

const placeholders = DELETED_TARGET_CRATES.map(() => '?').join(',')
const staleRows = db
  .prepare(
    `SELECT key, canonical_name, domain, target_crate, target_module, status, moves_money, requires_approval
       FROM canonical_capability
      WHERE target_crate IN (${placeholders})
      ORDER BY target_crate, domain, key`,
  )
  .all(...DELETED_TARGET_CRATES)

const byTarget = new Map()
for (const row of staleRows) {
  byTarget.set(row.target_crate, (byTarget.get(row.target_crate) || 0) + 1)
}

const unsafeRows = staleRows.filter((row) => row.status !== 'unimplemented')
if (unsafeRows.length) {
  fail('Deleted target-crate reconciliation refused to touch non-unimplemented rows.', {
    unsafe_count: unsafeRows.length,
    examples: unsafeRows.slice(0, 12),
  })
}

const moneyRows = staleRows.filter((row) => Number(row.moves_money) === 1)
const approvalRows = staleRows.filter((row) => Number(row.requires_approval) === 1)

console.log('reconcile-deleted-target-crates: stale rows by target')
for (const target of DELETED_TARGET_CRATES) {
  console.log(`  ${target}: ${byTarget.get(target) || 0}`)
}
console.log(`  total: ${staleRows.length}`)
console.log(`  money rows left status-unchanged: ${moneyRows.length}`)
console.log(`  approval rows left status-unchanged: ${approvalRows.length}`)

if (dryRun || staleRows.length === 0) {
  console.log(dryRun ? 'reconcile-deleted-target-crates: DRY_RUN no writes.' : 'reconcile-deleted-target-crates: nothing to update.')
  db.close()
  process.exit(0)
}

const now = new Date().toISOString()
const tx = db.transaction(() => {
  db.prepare(
    `UPDATE canonical_capability
        SET target_crate = NULL
      WHERE target_crate IN (${placeholders})
        AND status = 'unimplemented'`,
  ).run(...DELETED_TARGET_CRATES)

  if (tableExists('agent_note')) {
    db.prepare(
      `INSERT INTO agent_note (author, kind, status, capability_key, donor, body, created_at)
       VALUES (@author, @kind, @status, @capability_key, @donor, @body, @created_at)`,
    ).run({
      author: 'codex',
      kind: 'census_correction',
      status: 'open',
      capability_key: null,
      donor: null,
      body:
        `Cleared deleted target_crate metadata for ${staleRows.length} unimplemented canonical capabilities ` +
        `(${DELETED_TARGET_CRATES.join(', ')}). No status, moves_money, requires_approval, ` +
        `target_module, acceptance test, blocker, or implementation-evidence rows were changed. ` +
        `Rows now require local target-owner re-triage before implementation.`,
      created_at: now,
    })
  }
})

tx()

const remaining = db
  .prepare(`SELECT count(*) n FROM canonical_capability WHERE target_crate IN (${placeholders})`)
  .get(...DELETED_TARGET_CRATES).n

console.log(`reconcile-deleted-target-crates: updated ${staleRows.length} row(s); remaining deleted target refs=${remaining}`)
db.close()
