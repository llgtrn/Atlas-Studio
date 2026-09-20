#!/usr/bin/env node
// fix-erp-finance-target-crates-build-phase.mjs — same guard shape as
// fix-erp-finance-target-crates.mjs (the census/promote-phase sibling), for the second batch of
// erp-finance keys corrected during the native-build phase of the same 2026-08 pass: target_crate
// values that named crates never created in the live workspace, or named an existing but wrong
// crate, now pointed at the crate the newly-built real implementation actually lives in.
import Database from 'better-sqlite3'
import { CAPABILITIES_DB } from '../_paths.mjs'

const FIXES = {
  'erp.lunch.place_order': 'chronica-erp',
  'erp.edi.peppol_send_document': 'chronica-erp',
  'erp.surfsense_incentive_tasks': 'chronica-erp',
  'erp.gsheets_replication_ee': 'chronica-erp',
  'erp.cloudrun_render_pricing': 'chronica-analytics',
  'erp.lambda_render_pricing': 'chronica-analytics',
  'erp.model_pricing': 'chronica-runtime',
  'erp.license_entitlement': 'chronica-authorization',
  'erp.trpc_org_team_routers': 'chronica-api',
}

const db = new Database(CAPABILITIES_DB)
const upd = db.prepare(
  `UPDATE canonical_capability SET target_crate = ? WHERE key = ? AND domain = 'erp-finance' AND status = 'unimplemented'`,
)
const get = db.prepare('SELECT key, target_crate, status FROM canonical_capability WHERE key = ?')

let fixed = 0
let skipped = 0
for (const [key, crate] of Object.entries(FIXES)) {
  const row = get.get(key)
  if (!row) { console.log(`NOT FOUND: ${key}`); skipped += 1; continue }
  if (row.status !== 'unimplemented') { console.log(`SKIP (status=${row.status}): ${key}`); skipped += 1; continue }
  const result = upd.run(crate, key)
  if (result.changes > 0) { fixed += 1; console.log(`fixed: ${key} -> ${crate} (was ${row.target_crate})`) }
  else skipped += 1
}
console.log(`fix-erp-finance-target-crates-build-phase: fixed=${fixed} skipped=${skipped}`)
db.close()
