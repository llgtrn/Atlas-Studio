#!/usr/bin/env node
// apply-extraction-plan.mjs --donor <d> — STEP 1: insert new canonical_capability rows from
// extraction/<d>/_plan.json (status=unimplemented; money->financial_control_test). STEP 2: assemble
// docs/_machine/capability-reviews/<d>.rules.json = sources[] (source_capability defs w/ resolved
// canonical_id) + the donor's existing test rules FIRST + the new mapped rules + the rest of the
// donor's existing rules. Does NOT touch donor_file_census — caller re-scans + runs gen|apply.
import Database from 'better-sqlite3'
import { readFileSync, writeFileSync, existsSync } from 'node:fs'
import { join } from 'node:path'
import { CAPABILITIES_DB } from '../_paths.mjs'

const donor = (() => { const i = process.argv.indexOf('--donor'); return i >= 0 ? process.argv[i + 1] : null })()
if (!donor) { console.error('usage: apply-extraction-plan.mjs --donor <donor>'); process.exit(2) }
const RDIR = join(process.cwd(), 'docs', '_machine', 'capability-reviews')
const planPath = join(RDIR, 'extraction', donor, '_plan.json')
const rulesPath = join(RDIR, donor + '.rules.json')
if (!existsSync(planPath)) { console.error('no plan: ' + planPath); process.exit(2) }
const plan = JSON.parse(readFileSync(planPath, 'utf8'))
const current = existsSync(rulesPath) ? JSON.parse(readFileSync(rulesPath, 'utf8')) : { rules: [] }

const S = (v) => v == null || v === '' ? null : (typeof v === 'object' ? (Array.isArray(v) ? v.join('; ') : JSON.stringify(v)) : String(v))
const db = new Database(CAPABILITIES_DB)
db.pragma('journal_mode = WAL')
const insCanon = db.prepare(`INSERT OR IGNORE INTO canonical_capability
  (key,canonical_name,domain,target_crate,target_module,side_effect_class,moves_money,requires_approval,acceptance_criteria,required_tests,financial_control_test,status,donor_count)
  VALUES (@key,@canonical_name,@domain,@target_crate,@target_module,@side_effect_class,@moves_money,@requires_approval,@acceptance_criteria,@required_tests,@financial_control_test,'unimplemented',0)`)
let inserted = 0
db.transaction(() => { for (const c of plan.new_canonicals) inserted += insCanon.run({ key: S(c.key), canonical_name: S(c.canonical_name), domain: S(c.domain), target_crate: S(c.target_crate), target_module: S(c.target_module), side_effect_class: S(c.side_effect_class) || 'internal_write', moves_money: c.moves_money ? 1 : 0, requires_approval: c.requires_approval ? 1 : 0, acceptance_criteria: S(c.acceptance_criteria), required_tests: S(c.required_tests), financial_control_test: S(c.financial_control_test) }).changes })()
const getId = db.prepare('SELECT id FROM canonical_capability WHERE key=?')
const keymap = new Map(); for (const c of plan.caps) { const r = getId.get(c.canonicalKey); if (r) keymap.set(c.canonicalKey, r.id) }
db.close()
const missing = plan.caps.filter(c => !keymap.has(c.canonicalKey))
if (missing.length) { console.error('FATAL canonical ids missing: ' + missing.map(c => c.canonicalKey).slice(0, 10).join(', ')); process.exit(1) }

const sources = plan.caps.map(c => ({ key: c.tempKey, canonical_id: keymap.get(c.canonicalKey), canonical_name: c.canonical_name, module: c.module, source_files: c.source_files, source_symbols: c.source_symbols, business_behavior: c.business_behavior, technical_behavior: c.technical_behavior, inputs: c.inputs, outputs: c.outputs, persistence: c.persistence, surface: c.surface, side_effect_class: c.side_effect_class, moves_money: c.moves_money, requires_approval: c.requires_approval, external_services: c.external_services, target_crate: c.target_crate, target_module: c.target_module }))
const testRules = current.rules.filter(r => r.classification === 'test_fixture')
const otherRules = current.rules.filter(r => r.classification !== 'test_fixture')
const out = { donor, reviewed_by: `codex:source-extraction:${donor}`, created_pass: 'source-extraction-2026-06-03', require_zero_pending: true, sources, rules: [...testRules, ...plan.rules, ...otherRules] }
writeFileSync(rulesPath, JSON.stringify(out, null, 2))
console.log(JSON.stringify({ donor, new_canonicals_inserted: inserted, ids_resolved: keymap.size, sources: sources.length, mapped_rules: plan.rules.length, total_rules: out.rules.length }))
