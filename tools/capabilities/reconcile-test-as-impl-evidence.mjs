#!/usr/bin/env node
// reconcile-test-as-impl-evidence.mjs — fix the 10 "test-as-implementation" impl_evidence rows flagged by
// audit-truth.mjs: rows whose impl_symbols was just the test fn name (impl_file==test_file AND
// impl_symbols==test_symbol), so the recorded "implementation evidence" pointed at the TEST, not the real
// impl. This re-points impl_file -> the REAL sibling impl module + impl_symbols -> its real pub fns/structs,
// leaving test_file/test_symbol (the genuine proving test) UNCHANGED.
//
// MONEY-SAFE BY CONSTRUCTION: this touches ONLY impl_evidence.impl_file + impl_evidence.impl_symbols. It does
// NOT write canonical_capability or canonical_status_override at all — so moves_money / requires_approval /
// financial_control_test / status are all preserved exactly (the 4 money caps among these stay mm=1, ra=1).
// HARD GUARD: every new impl_symbol must actually appear in the new impl_file, and the existing test_symbol
// must still appear in its test_file, before any write (all-or-nothing). Idempotent.
//
// Run: node tools/capabilities/reconcile-test-as-impl-evidence.mjs
import Database from 'better-sqlite3'
import { readFileSync, existsSync } from 'node:fs'
import { CAPABILITIES_DB } from '../_paths.mjs'

// cap -> the real sibling impl module + real pub symbols (read from source; the GUARD re-verifies presence).
const REPOINTS = [
  { key: 'erp.inventory.post_stock_ledger_entry', impl_file: 'crates/chronica-erp/src/accounting/stock_ledger_entry.rs', impl_symbols: 'validate_sle, validate_sle_on_submit, StockLedgerEntryRecord' },
  { key: 'slice.124.company_portfolio_runtime', impl_file: 'crates/chronica-company/src/allocation/mod.rs', impl_symbols: 'PortfolioAllocationDecision, allocate_resource' },
  { key: 'slice.125.inter_company_collaboration_sharedworkflow', impl_file: 'crates/chronica-company/src/inter_company.rs', impl_symbols: 'InterCompanyTransfer, invoice_finance_legs, legs_balance' },
  { key: 'slice.126.sharedartifact_dataset_across_companies', impl_file: 'crates/chronica-memory/src/knowledge_lifecycle/dataset.rs', impl_symbols: 'create_dataset, ingest_document, document_for' },
  { key: 'slice.129.consolidated_kpi_group_budget_ceiling', impl_file: 'crates/chronica-company/src/allocation/mod.rs', impl_symbols: 'allocate_resource, capital_allocation_action' },
  { key: 'erp.tax.select_template', impl_file: 'crates/chronica-erp/src/accounting/tax_rule.rs', impl_symbols: 'get_tax_template, TaxSelectionRule, validate' },
  { key: 'erp.accounting.check_ledger_health', impl_file: 'crates/chronica-erp/src/accounting/ledger_health.rs', impl_symbols: 'voucher_wise_balance, general_vs_payment_ledger_comparison, LedgerHealthFinding' },
  { key: 'erp.stock.build_stock_closing_balance', impl_file: 'crates/chronica-erp/src/accounting/stock_closing.rs', impl_symbols: 'StockClosingBalanceRow, validate_duplicate_closing, next_closing_status' },
  { key: 'workflow-runtime.decide_step_execution', impl_file: 'crates/chronica-workflows/src/step_graph/decision.rs', impl_symbols: 'should_execute_step' },
  { key: 'workflow-runtime.propagate_step_failure', impl_file: 'crates/chronica-workflows/src/step_graph/failure_propagation.rs', impl_symbols: 'should_skip_step_execution, should_fail_safely' },
]

const fileCache = new Map()
const read = (p) => { if (fileCache.has(p)) return fileCache.get(p); const s = existsSync(p) ? readFileSync(p, 'utf8') : null; fileCache.set(p, s); return s }
const hasSym = (src, sym) => new RegExp(`\\b${sym.trim()}\\b`).test(src)
const hasTestFn = (src, sym) => new RegExp(`\\bfn\\s+${sym}\\b`).test(src)

const db = new Database(CAPABILITIES_DB)

// ── GUARD: validate every re-point against real source + the existing row's test, before any write. ──
// A cap with no impl_evidence row yet (e.g. a fresh rebuild before the original recording ran) is SKIPPED,
// not failed — so this is safe to run in rebuild-pipeline.mjs. A genuine symbol mismatch still aborts.
const failures = []
const skipped = []
const rows = {}
const todo = []
for (const r of REPOINTS) {
  const ev = db.prepare('SELECT impl_file,impl_symbols,test_file,test_symbol FROM impl_evidence WHERE canonical_key=?').get(r.key)
  if (!ev) { skipped.push(r.key); continue }
  rows[r.key] = ev
  todo.push(r)
  const isrc = read(r.impl_file)
  if (!isrc) { failures.push(`${r.key}: new impl_file missing ${r.impl_file}`); continue }
  for (const s of r.impl_symbols.split(',').map(x => x.trim()).filter(Boolean)) {
    if (!hasSym(isrc, s)) failures.push(`${r.key}: impl symbol \`${s}\` absent in ${r.impl_file}`)
  }
  const tsrc = read(ev.test_file)
  if (!tsrc) failures.push(`${r.key}: existing test_file missing ${ev.test_file}`)
  else if (!hasTestFn(tsrc, ev.test_symbol)) failures.push(`${r.key}: existing test fn \`${ev.test_symbol}\` absent in ${ev.test_file}`)
  // ensure we actually break the test-as-impl signature.
  if (r.impl_symbols.trim() === ev.test_symbol.trim()) failures.push(`${r.key}: impl_symbols still equals test_symbol`)
}
if (failures.length) {
  console.error(`REFUSED: ${failures.length} guard failure(s) — wrote NOTHING:`)
  for (const f of failures) console.error(`  ✗ ${f}`)
  db.close(); process.exit(1)
}

// snapshot money flags BEFORE, to prove they are untouched AFTER.
const moneyBefore = {}
for (const r of todo) moneyBefore[r.key] = db.prepare('SELECT moves_money mm, requires_approval ra, status FROM canonical_capability WHERE key=?').get(r.key)

const upd = db.prepare('UPDATE impl_evidence SET impl_file=?, impl_symbols=? WHERE canonical_key=?')
const tx = db.transaction(() => { for (const r of todo) upd.run(r.impl_file, r.impl_symbols, r.key) })
tx()

// ── money-safety self-check: NO money/status flag may have changed (we only wrote impl_evidence). ──
let bad = 0
for (const r of todo) {
  const a = db.prepare('SELECT moves_money mm, requires_approval ra, status FROM canonical_capability WHERE key=?').get(r.key)
  const b = moneyBefore[r.key]
  if (a.mm !== b.mm || a.ra !== b.ra || a.status !== b.status) { console.error(`  ✗ flag drift ${r.key}: ${JSON.stringify(b)} -> ${JSON.stringify(a)}`); bad++ }
  // the re-pointed evidence must not dangle.
  const ev = db.prepare('SELECT impl_file,impl_symbols,test_file,test_symbol FROM impl_evidence WHERE canonical_key=?').get(r.key)
  const isrc = read(ev.impl_file), tsrc = read(ev.test_file)
  if (!isrc || ev.impl_symbols.split(',').some(s => s.trim() && !hasSym(isrc, s)) || !tsrc || !hasTestFn(tsrc, ev.test_symbol)) { console.error(`  ✗ post-check dangle ${r.key}`); bad++ }
}
db.close()
const moneyCaps = todo.filter(r => moneyBefore[r.key].mm === 1).map(r => r.key)
console.log(`reconcile-test-as-impl-evidence: re-pointed ${todo.length} caps' impl_evidence to their real sibling impl modules (impl_file+impl_symbols only).${skipped.length ? ` skipped ${skipped.length} (no evidence row yet): ${skipped.join(', ')}` : ''}`)
console.log(`  money caps preserved (mm=1, untouched): ${moneyCaps.join(', ') || '(none in this run)'}`)
if (bad) { console.error(`  ✗ ${bad} self-check failure(s)`); process.exit(1) }
console.log('  ✓ self-check clean — no money/status flag changed; no dangling evidence.')
