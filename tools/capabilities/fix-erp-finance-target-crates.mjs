#!/usr/bin/env node
// fix-erp-finance-target-crates.mjs — one-shot correction of erp-finance target_crate values that
// name crates never created in the live 80-crate workspace (chronica-ledger, chronica-hr,
// chronica-inventory, chronica-fleet, chronica-delivery, chronica-einvoice, chronica-manufacturing,
// chronica-loyalty, chronica-observability [for an item-pricing cap], chronica-project,
// chronica-org, chronica-finance, chronica-crm [for a cap whose real code lives in
// crates/chronica-erp/src/crm/], and chronica-erp itself for two caps whose real code was found in
// chronica-analytics/chronica-governance instead) with the crate that actually holds the real,
// tested implementation found during the 2026-08 erp-finance native-code census.
//
// SAFETY (same guard shape as reconcile-deleted-target-crates.mjs / backfill-missing-target-crates.mjs):
// only rewrites target_crate on rows that are still status='unimplemented' in this domain, never
// touches status/moves_money/requires_approval/acceptance_test/blocker/impl_evidence. Idempotent.
import Database from 'better-sqlite3'
import { CAPABILITIES_DB } from '../_paths.mjs'

const FIXES = {
  'erp.account_balance_query': 'chronica-erp',
  'erp.accounting.post_cogs_entry': 'chronica-erp',
  'erp.crm.convert_lead': 'chronica-erp',
  'erp.edi.export_ubl_cii_xml': 'chronica-erp',
  'erp.fleet.manage_vehicle': 'chronica-erp',
  'erp.fulfillment.deliver_sales_items': 'chronica-erp',
  'erp.hr.org_chart': 'chronica-erp',
  'erp.hr.recruitment.manage_pipeline': 'chronica-erp',
  'erp.hr.timeoff.request': 'chronica-erp',
  'erp.inventory.post_stock_ledger_entry': 'chronica-erp',
  'erp.inventory.reserve_stock': 'chronica-erp',
  'erp.inventory.track_movements': 'chronica-erp',
  'erp.loyalty.redeem_reward': 'chronica-erp',
  'erp.mfg.run_production_plan': 'chronica-erp',
  'erp.organisation_management': 'chronica-organization',
  'erp.pricing_catalog': 'chronica-erp',
  'erp.project.manage_project': 'chronica-erp',
  'erp.revenue_analytics': 'chronica-analytics',
  'erp.tax.compute_python_formula': 'chronica-erp',
  'erp.team_management': 'chronica-governance',
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
console.log(`fix-erp-finance-target-crates: fixed=${fixed} skipped=${skipped}`)
db.close()
