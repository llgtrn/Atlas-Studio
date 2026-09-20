#!/usr/bin/env node
// mark-blocked.mjs — record a capability as explicitly BLOCKED with a reason, instead of leaving
// it silently unimplemented or wrongly promoting/building it. Used for the narrow case where a
// capability's own tracking says moves_money:false but a HONEST implementation would necessarily
// read/display/compute money-adjacent state (e.g. a billing/wallet dashboard UI) -- rather than
// build a decorative shell around fake data, this records why the item was deliberately not built.
//
// Same guard shape as record-verified.mjs: writes canonical_status_override (survivable side
// table) plus canonical_capability directly when present, then re-exports the canonical JSONL and
// runs the SAME verify used by `caps:canonical:verify`, scoped to just this capability's own shard
// record, rolling back on a new violation so a refused block never lands half-synced.
import Database from 'better-sqlite3'
import { CAPABILITIES_DB } from '../_paths.mjs'
import { isUsableCanonicalCapabilityTable } from './canonical-fallback.mjs'
import { exportCanonicalCapabilityShards } from './export-canonical-shards.mjs'
import { verifyCanonicalCapabilityShards } from './verify-canonical-shards.mjs'
import { loadCapabilityShards, rel } from '../canonical-shards/jsonl-lib.mjs'

function arg(flag) { const i = process.argv.indexOf(flag); return i >= 0 ? process.argv[i + 1] : undefined }
const key = arg('--key')
const reason = arg('--reason')
const by = arg('--by') || 'loop:verify'

if (!key || !reason) {
  console.error('Usage: --key <k> --reason "<why this is blocked>"')
  process.exit(2)
}

const db = new Database(CAPABILITIES_DB)
const hasTable = (name) => db.prepare("SELECT count(*) n FROM sqlite_master WHERE type='table' AND name=?").get(name).n > 0
if (!hasTable('canonical_status_override')) { console.error('CAPABILITY_DB_SCHEMA_BLOCKER: canonical_status_override missing'); process.exit(1) }

const hasCanonical = isUsableCanonicalCapabilityTable(db, 'canonical_capability')
const cap = hasCanonical ? db.prepare('SELECT key, status, moves_money FROM canonical_capability WHERE key=?').get(key) : null
if (hasCanonical && !cap) { console.error(`no such canonical capability: ${key}`); process.exit(1) }
if (cap && cap.status === 'verified') { console.error(`REFUSED: ${key} is already verified; will not overwrite with blocked`); process.exit(1) }

const nowIso = new Date().toISOString()
const preWrite = {
  canonicalCapabilityRow: hasCanonical
    ? db.prepare('SELECT status, blocker FROM canonical_capability WHERE key=?').get(key)
    : null,
  overrideRow: db.prepare('SELECT status, blocker, set_at, set_by FROM canonical_status_override WHERE canonical_key=?').get(key) ?? null,
}

const tx = db.transaction(() => {
  if (hasCanonical) {
    db.prepare('UPDATE canonical_capability SET status=?, blocker=? WHERE key=?').run('blocked', reason, key)
  }
  db.prepare(`INSERT INTO canonical_status_override (canonical_key,status,blocker,set_at,set_by)
    VALUES (?,?,?,?,?)
    ON CONFLICT(canonical_key) DO UPDATE SET status='blocked', blocker=excluded.blocker, set_at=excluded.set_at, set_by=excluded.set_by`)
    .run(key, 'blocked', reason, nowIso, by)
})
tx()
console.log(`recorded blocked: ${key} -- ${reason}`)
db.close()

const root = process.cwd()
if (hasCanonical) {
  // CONV0AR2: field authority must match this script's ACTUAL DB writes exactly (see the
  // canonical_capability/canonical_status_override UPDATE/UPSERT above) -- status and blocker only.
  exportCanonicalCapabilityShards({ root, expectedChanges: { [key]: ['status', 'blocker'] } })
  const verifyResult = verifyCanonicalCapabilityShards({ root })
  if (!verifyResult.ok) {
    const { entries } = loadCapabilityShards(root)
    const entry = entries.find((candidate) => candidate.record?.capability_key === key)
    const path = entry ? `${rel(root, entry.file)}:${entry.line}` : null
    const scoped = path ? verifyResult.errors.filter((error) => error.startsWith(`${path}:`) || error.startsWith(`${path}.`)) : []
    if (scoped.length) {
      const rollbackDb = new Database(CAPABILITIES_DB)
      rollbackDb.transaction(() => {
        if (preWrite.canonicalCapabilityRow) {
          rollbackDb.prepare('UPDATE canonical_capability SET status=?, blocker=? WHERE key=?')
            .run(preWrite.canonicalCapabilityRow.status, preWrite.canonicalCapabilityRow.blocker, key)
        }
        if (preWrite.overrideRow) {
          rollbackDb.prepare('UPDATE canonical_status_override SET status=?, blocker=?, set_at=?, set_by=? WHERE canonical_key=?')
            .run(preWrite.overrideRow.status, preWrite.overrideRow.blocker, preWrite.overrideRow.set_at, preWrite.overrideRow.set_by, key)
        } else {
          rollbackDb.prepare('DELETE FROM canonical_status_override WHERE canonical_key=?').run(key)
        }
      })()
      rollbackDb.close()
      exportCanonicalCapabilityShards({ root, expectedChanges: { [key]: ['status', 'blocker'] } })
      console.error(`REFUSED: blocking ${key} rolled back -- caps:canonical:verify found new violation(s):`)
      for (const error of scoped) console.error(`  ${error}`)
      process.exit(1)
    }
  }
}
