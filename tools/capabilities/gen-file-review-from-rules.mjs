#!/usr/bin/env node
// gen-file-review-from-rules.mjs — expand an agent's evidence-backed cluster rules into a
// file-EXHAUSTIVE review manifest for apply-file-review-manifest.mjs.
//
// WHY: huge donors (odoo 45k files, ladybird 22k) cannot be reviewed as 45k hand-written
// per-file rows. Instead a reviewer agent INSPECTS the tree at file level and returns a
// small ordered ruleset (each rule = a path predicate + classification + evidence + optional
// mapped source ids), justified by reading representative files of each cluster. This engine
// expands those rules against the AUTHORITATIVE unread donor_file_census rows and REFUSES to
// emit a manifest if ANY unread file is left unmatched — that zero-unmatched assertion is what
// makes the result exhaustive rather than a sample. First matching rule wins (order matters).
//
// Usage:
//   node tools/capabilities/gen-file-review-from-rules.mjs --rules <rules.json> > manifest.json
//   node tools/capabilities/gen-file-review-from-rules.mjs --rules <rules.json> --validate-only
//   cat manifest.json | node tools/capabilities/apply-file-review-manifest.mjs
//
// rules.json shape:
//   {
//     "donor": "erpnext-main",
//     "reviewed_by": "codex:file-exhaustive-review:erpnext-main",   // optional
//     "created_pass": "file-exhaustive-review-2026-06-03",          // optional
//     "sources": [ ... new source_capability rows (must carry canonical_id) ... ],  // optional
//     "rules": [
//       { "classification": "mapped", "ids": [160,161], "symbol": "Account.validate",
//         "evidence": "...", "match": { "exact": ["erpnext/accounts/doctype/account/account.py"] } },
//       { "classification": "test_fixture", "evidence": "pytest unit tests under */tests/",
//         "match": { "regex": "(^|/)test_[^/]*\\.py$", "glob": ["**/tests/**"] } },
//       { "classification": "non_behavioral_support", "evidence": "gettext catalogs",
//         "match": { "suffix": [".po", ".pot", ".csv"], "contains": ["/locale/"] } },
//       { "classification": "behavioral_excluded", "evidence": "...", "match": { "all": true } }
//     ]
//   }
import Database from 'better-sqlite3'
import { readFileSync } from 'node:fs'
import { CAPABILITIES_DB } from '../_paths.mjs'

const ALLOWED = new Set([
  'mapped', 'duplicate_variant_mapped', 'behavioral_excluded',
  'non_behavioral_support', 'generated_vendor_build_artifact', 'test_fixture', 'blocked',
])
const NEEDS_IDS = new Set(['mapped', 'duplicate_variant_mapped'])

function argValue(name) {
  const i = process.argv.indexOf(name)
  return i >= 0 ? process.argv[i + 1] : null
}
const rulesPath = argValue('--rules')
const validateOnly = process.argv.includes('--validate-only')
if (!rulesPath) {
  console.error('usage: gen-file-review-from-rules.mjs --rules <rules.json> [--validate-only]')
  process.exit(2)
}

const spec = JSON.parse(readFileSync(rulesPath, 'utf8'))
const donor = String(spec.donor || '').trim()
if (!donor) { console.error('rules.donor is required'); process.exit(2) }
const rules = spec.rules || []
if (!rules.length) { console.error('rules[] is required and non-empty'); process.exit(2) }

function globToRegExp(glob) {
  let re = ''
  for (let i = 0; i < glob.length; i++) {
    const c = glob[i]
    if (c === '*') {
      if (glob[i + 1] === '*') { re += '.*'; i++; if (glob[i + 1] === '/') i++ } else re += '[^/]*'
    } else if (c === '?') re += '[^/]'
    else if ('\\^$+.()|[]{}'.includes(c)) re += '\\' + c
    else re += c
  }
  return new RegExp('^' + re + '$')
}

function compileRule(rule, idx) {
  const cls = String(rule.classification || '').trim()
  if (!ALLOWED.has(cls)) throw new Error(`rule[${idx}]: unsupported classification '${cls}'`)
  const evidence = String(rule.evidence || rule.reason || rule.evidence_note || '').trim()
  if (!evidence) throw new Error(`rule[${idx}] (${cls}): evidence is required`)
  const ids = (rule.ids || rule.mapped_source_ids || []).map(String).map(s => s.trim()).filter(Boolean)
  if (NEEDS_IDS.has(cls) && ids.length === 0) throw new Error(`rule[${idx}] (${cls}): ids are required`)
  const m = rule.match || {}
  const exact = new Set((m.exact || []).map(String))
  const prefix = (m.prefix || []).map(String)
  const suffix = (m.suffix || []).map(String)
  const contains = (m.contains || []).map(String)
  const globs = (m.glob || []).map(globToRegExp)
  const regex = m.regex ? new RegExp(m.regex) : null
  const all = m.all === true
  if (!all && !exact.size && !prefix.length && !suffix.length && !contains.length && !globs.length && !regex) {
    throw new Error(`rule[${idx}] (${cls}): empty match — give all/exact/prefix/suffix/contains/glob/regex`)
  }
  const test = (p) => all
    || exact.has(p)
    || prefix.some(x => p.startsWith(x))
    || suffix.some(x => p.endsWith(x))
    || contains.some(x => p.includes(x))
    || (regex ? regex.test(p) : false)
    || globs.some(rx => rx.test(p))
  return {
    idx, cls, evidence, all,
    ids, symbol: (rule.symbol || '').slice(0, 180),
    read: rule.read,
    label: rule.label || `${cls}: ${evidence.slice(0, 60)}`,
    test, count: 0,
  }
}

const compiled = rules.map(compileRule)

const db = new Database(CAPABILITIES_DB, { readonly: true })
const sourceKeys = new Set((spec.sources || []).map(s => s.key).filter(Boolean))
const idExists = db.prepare('SELECT 1 FROM source_capability WHERE id=?')
for (const r of compiled) {
  for (const id of r.ids) {
    if (/^\d+$/.test(id)) { if (!idExists.get(Number(id))) throw new Error(`rule[${r.idx}]: source_capability id ${id} not found`) }
    else if (/^\d+-\d+$/.test(id)) {
      const [a, b] = id.split('-').map(Number)
      for (let n = a; n <= b; n++) if (!idExists.get(n)) throw new Error(`rule[${r.idx}]: source id ${n} (range ${id}) not found`)
    } else if (!sourceKeys.has(id)) throw new Error(`rule[${r.idx}]: id '${id}' is neither a numeric source id nor a declared sources[].key`)
  }
}

const rows = db.prepare(
  "SELECT path, kind FROM donor_file_census WHERE donor=? AND read_status='unread_pending' ORDER BY path"
).all(donor)
db.close()

if (rows.length === 0) {
  console.error(`${donor}: 0 unread_pending rows — nothing to do (already exhaustive).`)
  if (validateOnly) process.exit(0)
  process.exit(0)
}

// group matched rows by (classification|evidence|ids|symbol|read) into review rows with paths[]
const groups = new Map()
const unmatched = []
for (const { path } of rows) {
  const rule = compiled.find(r => r.test(path))
  if (!rule) { unmatched.push(path); continue }
  rule.count++
  const key = [rule.cls, rule.evidence, rule.ids.join(','), rule.symbol, rule.read ?? ''].join('')
  if (!groups.has(key)) {
    groups.set(key, { classification: rule.cls, evidence: rule.evidence, ids: rule.ids, symbol: rule.symbol, read: rule.read, paths: [] })
  }
  groups.get(key).paths.push(path)
}

// stats to stderr (never pollutes the stdout manifest)
const clsTotals = {}
for (const r of compiled) clsTotals[r.cls] = (clsTotals[r.cls] || 0) + r.count
console.error(`\n${donor}: ${rows.length} unread_pending files`)
console.error('  per-rule matches:')
for (const r of compiled) console.error(`    [${String(r.idx).padStart(2)}] ${r.count.toString().padStart(6)}  ${r.all ? '(CATCH-ALL) ' : ''}${r.label}`)
console.error('  classification totals:')
for (const [c, n] of Object.entries(clsTotals).sort((a, b) => b[1] - a[1])) console.error(`    ${n.toString().padStart(6)}  ${c}`)
console.error(`  unmatched: ${unmatched.length}`)

if (unmatched.length) {
  console.error(`\n✗ NOT EXHAUSTIVE — ${unmatched.length} unread file(s) match no rule. First 60:`)
  for (const p of unmatched.slice(0, 60)) console.error(`    ${p}`)
  process.exit(3)
}

if (validateOnly) {
  console.error('\n✓ exhaustive (every unread file matched a rule). validate-only: no manifest emitted.')
  process.exit(0)
}

const manifest = {
  donor,
  reviewed_by: spec.reviewed_by || `codex:file-exhaustive-review:${donor}`,
  created_pass: spec.created_pass || 'file-exhaustive-review-2026-06-03',
  require_zero_pending: true,
  sources: spec.sources || [],
  reviews: [...groups.values()].map(g => ({
    paths: g.paths,
    classification: g.classification,
    evidence: g.evidence,
    ...(g.ids.length ? { ids: g.ids } : {}),
    ...(g.symbol ? { symbol: g.symbol } : {}),
    ...(g.read !== undefined ? { read: g.read } : {}),
  })),
}
process.stdout.write(JSON.stringify(manifest))
console.error('\n✓ exhaustive — manifest emitted to stdout.')
