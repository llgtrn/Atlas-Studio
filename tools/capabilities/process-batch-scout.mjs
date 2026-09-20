#!/usr/bin/env node
// process-batch-scout.mjs — ingest a donor-batch-scout Workflow result into caps.db + the strict ledger.
// Usage: node tools/capabilities/process-batch-scout.mjs <workflow-output-file.json>
//
// For each scout with real content (total_files>0 AND l3.architecture_score>0): write the scout doc,
// ingest its capabilities as source_capability rows (dedup), and patch the strict ledger
// (deep_caps + l3_scouted_modules + l3_note). DONE stays HONEST — recomputed as
// strict File>=70 AND strict Module>=70 AND L3-documented AND caps-ingested. Money primitives untouched.
import Database from 'better-sqlite3'
import { readFileSync, writeFileSync } from 'node:fs'
import { CAPABILITIES_DB } from '../_paths.mjs'

const outFile = process.argv[2]
if (!outFile) { console.error('usage: process-batch-scout.mjs <output.json>'); process.exit(1) }
const j = JSON.parse(readFileSync(outFile, 'utf8'))
const scouts = (j.result?.scouts || j.scouts || []).filter((s) => s && s.total_files > 0 && s.l3?.architecture_score > 0)
const failed = (j.result?.scouts || j.scouts || []).filter((s) => s && !(s.total_files > 0 && s.l3?.architecture_score > 0)).map((s) => s.donor)

const db = new Database(CAPABILITIES_DB)
const PASS = 'batch-scout-2026-06-25'
const ins = db.prepare(`INSERT INTO source_capability (donor,module,canonical_name,source_files,business_behavior,technical_behavior,persistence,surface,side_effect_class,moves_money,requires_approval,target_crate,target_module,canonical_id,created_pass)
 VALUES (@donor,@module,@canonical_name,@source_files,@business_behavior,'',@persistence,@surface,@side_effect_class,@moves_money,0,@target_crate,'',NULL,@created_pass)`)
const led = JSON.parse(readFileSync('docs/doctrines/audit/donor-census-coverage-strict.json', 'utf8'))

const done = []
for (const s of scouts) {
  writeFileSync(`docs/backend-truth-audit/donor-scout-${s.donor}.json`, JSON.stringify({ generated: '2026-06-25', source: 'donor-batch-scout', ...s }, null, 2) + '\n')
  const existing = new Set(db.prepare('SELECT LOWER(canonical_name) n FROM source_capability WHERE donor=?').all(s.donor).map((r) => r.n))
  let n = 0
  const tx = db.transaction(() => { for (const c of (s.capabilities || [])) {
    if (existing.has((c.name || '').toLowerCase())) continue
    ins.run({ donor: s.donor, module: s.chronica_domain, canonical_name: c.name, source_files: c.key || '', business_behavior: c.description || '', persistence: '', surface: s.chronica_domain, side_effect_class: c.money_adjacent ? 'money' : 'config', moves_money: c.money_adjacent ? 1 : 0, target_crate: 'chronica-' + s.chronica_domain, created_pass: PASS })
    n++; existing.add((c.name || '').toLowerCase())
  } })
  tx()
  const total = db.prepare('SELECT COUNT(*) n FROM source_capability WHERE donor=?').get(s.donor).n
  const t = led.donors.find((d) => d.donor === s.donor)
  if (t) {
    t.deep_caps = total
    t.l3_scouted_modules = { [s.chronica_domain]: { architecture_score: s.l3.architecture_score, dims: s.l3_dimensions_understood.length, modules: `${s.modules_scouted}/${s.modules_total}`, scout: `donor-scout-${s.donor}.json` } }
    t.l3_note = `batch-scouted (score ${s.l3.architecture_score}/10, ${s.l3_dimensions_understood.length}/10 dims, ${s.modules_scouted}/${s.modules_total} modules read); donor-wide partial.`
    // HONEST DONE: strict File AND Module >=70, plus L3 documented + caps ingested.
    const wasDone = t.status === 'DONE'
    if (t.L1_file_pct >= 70 && t.L2_module_pct >= 70 && total > 0) { t.status = 'DONE'; if (!wasDone) done.push(s.donor) }
    // a donor that is now L3-scouted with caps is no longer truly UNSCOUTED (it has real knowledge),
    // even if its file/module census coverage is still ~0 — reflect that honestly.
    else if (t.status === 'UNSCOUTED' && total > 0) t.status = 'PARTIALLY_SCOUTED'
    console.log(`  ${s.donor}: +${n} caps -> ${total}; score ${s.l3.architecture_score}/10; File ${t.L1_file_pct}% Module ${t.L2_module_pct}% -> ${t.status}; absorb_now=${(s.absorption_now || []).length}`)
  }
}
writeFileSync('docs/doctrines/audit/donor-census-coverage-strict.json', JSON.stringify(led, null, 2) + '\n')
db.close()
console.log(`\nprocessed ${scouts.length} scouts | newly DONE: ${done.length ? done.join(', ') : 'none (big donors stay PARTIALLY — honest)'} | failed(retry): ${failed.join(', ') || 'none'}`)
// emit absorption targets for the Cloud queue
const targets = scouts.flatMap((s) => (s.absorption_now || []).filter((a) => a.money_free).map((a) => ({ donor: s.donor, ...a })))
writeFileSync('/tmp/absorption-queue.json', JSON.stringify(targets, null, 2))
console.log(`absorption targets (money-free) -> /tmp/absorption-queue.json: ${targets.length}`)
