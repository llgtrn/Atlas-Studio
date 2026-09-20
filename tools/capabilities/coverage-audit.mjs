#!/usr/bin/env node
// coverage-audit.mjs — per-donor scout COMPLETENESS audit (DONOR EXHAUSTIVE SCOUT MODE).
//
// A donor is "touched" when capabilities were found; it is COMPLETE only when its capability-bearing
// file surface is actually mapped. This measures, per donor:
//   - file coverage   = mapped census files / capability-bearing census files
//   - module coverage = top-level modules with >=1 mapped file / all such modules
//   - big unmapped modules = directories with many capability-bearing files and ZERO mapped
// Completion gate: coverage >= 80% AND no big unmapped module remains. Coverage you can audit.
//
// Usage: node tools/capabilities/coverage-audit.mjs [--donor <d>] [--threshold 80]
import Database from 'better-sqlite3'
import { CAPABILITIES_DB } from '../_paths.mjs'

const arg = (f) => { const i = process.argv.indexOf(f); return i >= 0 ? process.argv[i + 1] : null }
const onlyDonor = arg('--donor')
const THRESH = Number(arg('--threshold') || 80)
const BIG_UNMAPPED = 12 // a module with >= this many capability-bearing files and 0 mapped blocks completion

const db = new Database(CAPABILITIES_DB, { readonly: true })
const CAPCLASS = "('capability_review_pending','behavior_review_pending','mapped')"
const donors = onlyDonor
  ? [onlyDonor]
  : db.prepare(`SELECT DISTINCT donor FROM donor_file_census ORDER BY donor`).all().map(r => r.donor)

const moduleOf = (path) => {
  const seg = path.split('/')
  if (seg.length > 1) return seg.slice(0, 2).join('/') // packages/foo, app/lib, pkg/bar, comfy/sd …
  return seg[0] || '.'
}

let complete = 0, total = 0, touched = 0
for (const d of donors) {
  const rows = db.prepare(`SELECT path, classification FROM donor_file_census WHERE donor=? AND classification IN ${CAPCLASS}`).all(d)
  if (!rows.length) continue
  total++
  const evidenced = db.prepare('SELECT COUNT(*) n FROM source_capability WHERE donor=?').get(d).n
  if (evidenced > 0) touched++
  const capFiles = rows.length
  const mapped = rows.filter(r => r.classification === 'mapped').length
  const cov = capFiles ? 100 * mapped / capFiles : 0
  const mod = new Map()
  for (const r of rows) { const m = moduleOf(r.path); const e = mod.get(m) || { files: 0, mapped: 0 }; e.files++; if (r.classification === 'mapped') e.mapped++; mod.set(m, e) }
  const unmapped = [...mod.entries()].filter(([, e]) => e.mapped === 0).sort((a, b) => b[1].files - a[1].files)
  const bigUnmapped = unmapped.filter(([, e]) => e.files >= BIG_UNMAPPED)
  const isComplete = cov >= THRESH && bigUnmapped.length === 0
  if (isComplete) complete++
  console.log(`${isComplete ? '✓' : (evidenced ? '·' : ' ')} ${d.padEnd(26)} cov ${cov.toFixed(1).padStart(5)}%  ${String(mapped).padStart(5)}/${String(capFiles).padStart(5)} files  modules ${mod.size - unmapped.length}/${mod.size}  big-unmapped:${bigUnmapped.length}  caps:${evidenced}`)
  if (onlyDonor) bigUnmapped.slice(0, 25).forEach(([m, e]) => console.log(`     ✗ unmapped module: ${m}  (${e.files} capability-bearing files, 0 mapped)`))
}
console.log(`\nSCOUT STATUS: ${complete}/${total} donors COMPLETE (>=${THRESH}% + no big unmapped module)  |  ${touched}/${total} touched  |  ${total - touched} untouched`)
db.close()
