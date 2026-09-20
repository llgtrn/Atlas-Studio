#!/usr/bin/env node
// record-blocker.mjs — durably record a "not yet buildable" blocker for a capability by writing
// into canonical_status_override.blocker (the DB table export-canonical-shards.mjs reads at export
// time), NOT by hand-editing the jsonl shard directly. A hand-edited jsonl blocker used to be
// silently wiped the next time any tool (e.g. record-verified.mjs) regenerated shards from the DB,
// because the DB never learned about it. exportCanonicalCapabilityShards() now REFUSES (rather than
// silently discarding) any such divergence outside the keys it is told to expect changed — see its
// STALE-DB EXPORT GUARD (DEBT.BATCH-0) — but writing through this tool, which keeps the DB and JSONL
// in sync in the first place, remains the correct way to record a blocker.
//
// Usage:
//   node tools/capabilities/record-blocker.mjs --key <k> --blocker "<text>" [--by <who>]
//   node tools/capabilities/record-blocker.mjs --keys-file <path.json> [--by <who>]
//     where path.json is { "cap.key": "blocker text", ... }
import { readFileSync } from 'node:fs'
import Database from 'better-sqlite3'
import { CAPABILITIES_DB } from '../_paths.mjs'
import { exportCanonicalCapabilityShards } from './export-canonical-shards.mjs'

function arg(flag) { const i = process.argv.indexOf(flag); return i >= 0 ? process.argv[i + 1] : undefined }
const keysArg = arg('--keys-file')
const singleKey = arg('--key')
const blockerText = arg('--blocker')
const by = arg('--by') || 'loop:blocker'
const now = arg('--now') || new Date().toISOString()

let entries = []
if (keysArg) {
  const map = JSON.parse(readFileSync(keysArg, 'utf8'))
  entries = Object.entries(map)
} else if (singleKey && blockerText) {
  entries = [[singleKey, blockerText]]
} else {
  console.error('Usage: --key <k> --blocker "<text>" [--by who] OR --keys-file <path.json>')
  process.exit(2)
}

const db = new Database(CAPABILITIES_DB)
const knownKeys = new Set(db.prepare('SELECT key FROM canonical_capability').all().map((r) => r.key))
const unknown = entries.filter(([key]) => !knownKeys.has(key)).map(([key]) => key)
if (unknown.length) {
  console.error(`REFUSED: unknown canonical_key(s) not in canonical_capability: ${unknown.join(', ')}`)
  process.exit(1)
}

const upsert = db.prepare(`INSERT INTO canonical_status_override
  (canonical_key,status,acceptance_test,financial_control_test,moves_money,requires_approval,blocker,set_at,set_by)
  VALUES (@key,NULL,NULL,NULL,NULL,NULL,@blocker,@now,@by)
  ON CONFLICT(canonical_key) DO UPDATE SET blocker=excluded.blocker, set_at=excluded.set_at, set_by=excluded.set_by`)

const tx = db.transaction((rows) => {
  for (const [key, text] of rows) upsert.run({ key, blocker: text, now, by })
})
tx(entries)
console.log(`recorded blocker for ${entries.length} capability key(s)`)
db.close()

const root = process.cwd()
// CONV0AR2: this script's ON CONFLICT clause above only ever updates blocker/set_at/set_by (set_at/
// set_by are not exported canonical fields) -- its field authority per declared key is 'blocker'
// only, never status/moves_money/impl-evidence fields, even on a repeat write.
const expectedChanges = Object.fromEntries(entries.map(([entryKey]) => [entryKey, ['blocker']]))
const result = exportCanonicalCapabilityShards({ root, expectedChanges })
console.log(`re-exported shards: ${result.record_count} records across ${result.domain_count} domains`)
