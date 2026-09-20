#!/usr/bin/env node
// apply-extraction.mjs --donor <d> — IN-PLACE serial integrator for source-capability extraction.
// Reads extraction/<d>/_plan.json. Inserts new canonical rows + source_capability rows (+provenance),
// then flips ONLY the donor's `blocked` census rows that match a plan rule to `mapped` (with source ids
// + source_file_capability_link). Does NOT re-scan and does NOT touch non-blocked rows — so it cannot
// break the file-census gate. First-match-wins over the plan's mapped rules.
import Database from 'better-sqlite3'
import { readFileSync, existsSync } from 'node:fs'
import { join } from 'node:path'
import { CAPABILITIES_DB } from '../_paths.mjs'

const donor = (() => { const i = process.argv.indexOf('--donor'); return i >= 0 ? process.argv[i + 1] : null })()
if (!donor) { console.error('usage: apply-extraction.mjs --donor <donor>'); process.exit(2) }
const planPath = join(process.cwd(), 'docs', '_machine', 'capability-reviews', 'extraction', donor, '_plan.json')
if (!existsSync(planPath)) { console.error('no plan: ' + planPath); process.exit(2) }
const plan = JSON.parse(readFileSync(planPath, 'utf8'))
const S = (v) => v == null || v === '' ? null : (typeof v === 'object' ? (Array.isArray(v) ? v.join('; ') : JSON.stringify(v)) : String(v))

function glob2re(g) { let re = ''; for (let i = 0; i < g.length; i++) { const c = g[i]; if (c === '*') { if (g[i + 1] === '*') { re += '.*'; i++; if (g[i + 1] === '/') i++ } else re += '[^/]*' } else if (c === '?') re += '[^/]'; else if ('\\^$+.()|[]{}'.includes(c)) re += '\\' + c; else re += c } return new RegExp('^' + re + '$') }
function compile(m) {
  const exact = new Set((m.exact || []).map(String)), prefix = (m.prefix || []).map(String), suffix = (m.suffix || []).map(String), contains = (m.contains || []).map(String)
  const globs = (m.glob || []).map(glob2re), rx = m.regex ? new RegExp(m.regex) : null, all = m.all === true
  return (p) => all || exact.has(p) || prefix.some(x => p.startsWith(x)) || suffix.some(x => p.endsWith(x)) || contains.some(x => p.includes(x)) || (rx ? rx.test(p) : false) || globs.some(r => r.test(p))
}
const rules = plan.rules.map(r => ({ ...r, test: compile(r.match || {}) }))

const db = new Database(CAPABILITIES_DB)
db.pragma('journal_mode = WAL')
const now = new Date().toISOString()
const insCanon = db.prepare(`INSERT OR IGNORE INTO canonical_capability (key,canonical_name,domain,target_crate,target_module,side_effect_class,moves_money,requires_approval,acceptance_criteria,required_tests,financial_control_test,status,donor_count) VALUES (@key,@canonical_name,@domain,@target_crate,@target_module,@side_effect_class,@moves_money,@requires_approval,@acceptance_criteria,@required_tests,@financial_control_test,'unimplemented',0)`)
const getCanon = db.prepare('SELECT id FROM canonical_capability WHERE key=?')
const findSrc = db.prepare('SELECT id FROM source_capability WHERE donor=? AND canonical_name=?')
const insSrc = db.prepare(`INSERT INTO source_capability (donor,module,canonical_name,source_files,source_symbols,business_behavior,technical_behavior,inputs,outputs,persistence,surface,side_effect_class,moves_money,requires_approval,external_services,target_crate,target_module,canonical_id,created_pass) VALUES (@donor,@module,@canonical_name,@source_files,@source_symbols,@business_behavior,@technical_behavior,@inputs,@outputs,@persistence,@surface,@side_effect_class,@moves_money,@requires_approval,@external_services,@target_crate,@target_module,@canonical_id,'source-extraction-2026-06-03')`)
const insProv = db.prepare('INSERT OR IGNORE INTO provenance(source_id,canonical_id) VALUES(?,?)')
// "mappable" census rows = files the donor-file census flagged as carrying behaviour/capability and not
// yet mapped. The census emits capability_review_pending / behavior_review_pending (and legacy 'blocked');
// mapping any of them to a cap is what makes file-level coverage (source_file_capability_link) measurable.
const blockedRows = db.prepare("SELECT path FROM donor_file_census WHERE donor=? AND classification IN ('blocked','capability_review_pending','behavior_review_pending')").all(donor)
const updRow = db.prepare("UPDATE donor_file_census SET classification='mapped', read_status='reviewed', mapped_source_ids=?, reviewed_by='codex:source-extraction', reviewed_at=? WHERE donor=? AND path=?")
const insLink = db.prepare('INSERT OR IGNORE INTO source_file_capability_link(source_id,donor,path,symbol,evidence_note) VALUES(?,?,?,?,?)')

let canonInserted = 0, srcCreated = 0, remapped = 0
const result = db.transaction(() => {
  for (const c of plan.new_canonicals) canonInserted += insCanon.run({ key: S(c.key), canonical_name: S(c.canonical_name), domain: S(c.domain), target_crate: S(c.target_crate), target_module: S(c.target_module), side_effect_class: S(c.side_effect_class) || 'internal_write', moves_money: c.moves_money ? 1 : 0, requires_approval: c.requires_approval ? 1 : 0, acceptance_criteria: S(c.acceptance_criteria), required_tests: S(c.required_tests), financial_control_test: S(c.financial_control_test) }).changes
  // ensure a source_capability per plan cap; map tempKey -> source_id
  const tk2src = new Map()
  for (const cap of plan.caps) {
    const cid = getCanon.get(cap.canonicalKey)
    if (!cid) throw new Error('canonical missing: ' + cap.canonicalKey)
    let sid = findSrc.get(donor, cap.canonical_name)
    if (!sid) { sid = { id: Number(insSrc.run({ donor, module: S(cap.module), canonical_name: S(cap.canonical_name), source_files: S(cap.source_files), source_symbols: S(cap.source_symbols), business_behavior: S(cap.business_behavior), technical_behavior: S(cap.technical_behavior), inputs: S(cap.inputs), outputs: S(cap.outputs), persistence: S(cap.persistence), surface: S(cap.surface), side_effect_class: S(cap.side_effect_class), moves_money: cap.moves_money ? 1 : 0, requires_approval: cap.requires_approval ? 1 : 0, external_services: S(cap.external_services), target_crate: S(cap.target_crate), target_module: S(cap.target_module), canonical_id: cid.id }).lastInsertRowid) }; srcCreated++ }
    insProv.run(sid.id, cid.id)
    tk2src.set(cap.tempKey, sid.id)
  }
  // remap blocked rows by first-matching rule
  for (const { path } of blockedRows) {
    const rule = rules.find(r => r.test(path))
    if (!rule) continue
    const sids = (rule.ids || []).map(tk => tk2src.get(tk)).filter(Boolean)
    if (!sids.length) continue
    updRow.run(sids.join(','), now, donor, path)
    for (const sid of sids) insLink.run(sid, donor, path, (rule.evidence || '').slice(0, 160), (rule.evidence || `extraction:${path}`).slice(0, 200))
    remapped++
  }
  // refresh donor_count + meta
  db.prepare(`UPDATE canonical_capability SET donor_count=(SELECT COUNT(DISTINCT donor) FROM source_capability WHERE canonical_id=canonical_capability.id)`).run()
  return { canonInserted, srcCreated, remapped }
})()
const remainingBlocked = db.prepare("SELECT count(*) n FROM donor_file_census WHERE donor=? AND classification='blocked'").get(donor).n
const donorSrc = db.prepare('SELECT count(*) n FROM source_capability WHERE donor=?').get(donor).n
db.close()
console.log(JSON.stringify({ donor, new_canonicals_inserted: result.canonInserted, source_caps_created: result.srcCreated, blocked_remapped: result.remapped, blocked_before: blockedRows.length, blocked_remaining: remainingBlocked, donor_source_caps_total: donorSrc }))
