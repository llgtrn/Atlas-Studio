#!/usr/bin/env node
// link-slices.mjs — the NO-MISS slice<->canonical bridge (link-slices Pass A..D).
//
// Builds the survivable `slice_canonical` join table from the reviewed manifest
// (slice-domain-map.json) + deterministic domain/keyprefix/title heuristics, then
// derives canonical_capability.slice. EVERY one of the 1,793 canonical capabilities
// ends up either LINKED to a slice or UNASSIGNED (-> visible per-domain backlog in
// the cap-roadmap doc) — a builder cannot miss one.
//
// SAFETY: linking is STATUS-ORTHOGONAL. A wrong edge can mis-attribute a cap to a
// slice but can NEVER flip its build status. Every edge records match_method +
// confidence + reviewed, so `--report` (docs:link:review) surfaces every low-
// confidence edge and the full unassigned backlog for human review.
//
// Usage:
//   node tools/capabilities/link-slices.mjs            # (re)build slice_canonical + derive .slice
//   node tools/capabilities/link-slices.mjs --report   # print coverage report; do NOT write
import Database from 'better-sqlite3'
import { readFileSync } from 'node:fs'
import { join } from 'node:path'
import { CAPABILITIES_DB } from '../_paths.mjs'

const REPORT_ONLY = process.argv.includes('--report')
const manifest = JSON.parse(readFileSync(join(process.cwd(), 'tools', 'capabilities', 'slice-domain-map.json'), 'utf8'))
const db = new Database(CAPABILITIES_DB)
db.pragma('journal_mode = WAL')

// ── load canonical caps + slices ──────────────────────────────────────────────
const caps = db.prepare('SELECT key, canonical_name, domain, target_crate, moves_money FROM canonical_capability').all()
const slices = db.prepare("SELECT slice, title, target_crate FROM capability WHERE kind='slice'").all()
const sliceByNum = new Map(slices.map(s => [s.slice, s]))

// ── token-overlap (reused convention from build-registry sig()/STOP) ──────────
const STOP = new Set(['the','and','a','an','of','for','to','in','on','core','system','model','spec','chronica','company','project','template','templates','with','native','module','pipeline','core'])
const sig = (s) => new Set(String(s || '').toLowerCase().replace(/[^a-z0-9 ]/g, ' ').split(/\s+/).filter(w => w.length > 3 && !STOP.has(w)))
const overlap = (a, b) => { let n = 0; for (const w of a) if (b.has(w)) n++; return n }

// ── manifest → ownership index ────────────────────────────────────────────────
// For each domain, find: the slice that OWNS the bulk (keyPrefix present), the
// slices that take the DENY-remainder, and any plain domain-claimers.
const M = manifest.slices || {}
const domainOwners = {}   // domain -> [{slice, keyPrefix?, keyPrefixDeny?}]
for (const [k, v] of Object.entries(M)) {
  if (k.startsWith('_')) continue
  const sliceNum = Number(k)
  if (!Number.isFinite(sliceNum)) continue
  for (const d of (v.domains || [])) {
    (domainOwners[d] = domainOwners[d] || []).push({ slice: sliceNum, keyPrefix: v.keyPrefix, keyPrefixDeny: v.keyPrefixDeny })
  }
}
const keyPfx = (key) => { const i = key.indexOf('.'); return i > 0 ? key.slice(0, i) : key }

// reviewed per-key routing (highest confidence) + force-backlog set
const KEY_OVERRIDE = manifest.keyOverrides || {}
const KEY_BACKLOG = new Set(manifest.keyBacklog || [])

// ── assign each canonical to its BEST single owning slice (no fan-out) ────────
const edges = []          // {slice, canonical_key, match_method, confidence, reviewed}
const unassigned = []
for (const c of caps) {
  // Pass A-exact: a reviewed per-key override wins outright (high confidence, reviewed).
  if (KEY_OVERRIDE[c.key] !== undefined) { edges.push({ slice: KEY_OVERRIDE[c.key], canonical_key: c.key, match_method: 'manual', confidence: 'high', reviewed: 1 }); continue }
  // reviewed force-backlog: genuinely mis-domained, leave unlinked on purpose.
  if (KEY_BACKLOG.has(c.key)) { unassigned.push(c); continue }

  const owners = domainOwners[c.domain] || []
  if (!owners.length) { unassigned.push(c); continue }

  const pfx = keyPfx(c.key)
  // Pass A/C: keyPrefix exact-owner (high) — the slice that claims this key prefix.
  let chosen = owners.find(o => o.keyPrefix && o.keyPrefix.includes(pfx))
  let method = 'manifest', conf = 'high'

  if (!chosen) {
    // deny-remainder owner: a slice that takes the domain MINUS some prefixes.
    const denyOwner = owners.find(o => o.keyPrefixDeny && !o.keyPrefixDeny.includes(pfx))
    if (denyOwner) { chosen = denyOwner; method = 'keyprefix'; conf = 'medium' }
  }
  if (!chosen) {
    // plain domain-claimer(s): if exactly one, high-ish; if several, pick by title overlap (low).
    const plain = owners.filter(o => !o.keyPrefix && !o.keyPrefixDeny)
    if (plain.length === 1) { chosen = plain[0]; method = 'domain'; conf = 'medium' }
    else if (plain.length > 1) {
      const cs = sig(c.canonical_name)
      let best = null, bestN = 0
      for (const o of plain) { const n = overlap(cs, sig(sliceByNum.get(o.slice)?.title)); if (n > bestN) { bestN = n; best = o } }
      if (best) { chosen = best; method = 'title'; conf = 'low' }
      else { chosen = plain[0]; method = 'domain'; conf = 'low' }
    }
  }
  if (!chosen) {
    // a keyPrefix-owner exists for the domain but this cap's prefix isn't claimed,
    // and no deny/plain owner — leave for a deny owner if any, else fall through.
    const anyDeny = owners.find(o => o.keyPrefixDeny)
    if (anyDeny) { chosen = anyDeny; method = 'keyprefix'; conf = 'low' }
  }

  if (chosen) edges.push({ slice: chosen.slice, canonical_key: c.key, match_method: method, confidence: conf, reviewed: 0 })
  else unassigned.push(c)
}

// ── report ────────────────────────────────────────────────────────────────────
const byConf = { high: 0, medium: 0, low: 0 }
for (const e of edges) byConf[e.confidence]++
const linkedKeys = new Set(edges.map(e => e.canonical_key))
const unassignedByDomain = {}
for (const c of unassigned) unassignedByDomain[c.domain] = (unassignedByDomain[c.domain] || 0) + 1

console.log('link-slices' + (REPORT_ONLY ? ' (REPORT — no write)' : '') + ':')
console.log(`  total canonical: ${caps.length}`)
console.log(`  LINKED:   ${linkedKeys.size}  (high ${byConf.high} · medium ${byConf.medium} · low ${byConf.low})`)
console.log(`  UNASSIGNED -> backlog: ${unassigned.length}`)
console.log(`  NO-MISS proof: linked(${linkedKeys.size}) + unassigned(${unassigned.length}) = ${linkedKeys.size + unassigned.length} == total(${caps.length}) -> ${linkedKeys.size + unassigned.length === caps.length ? 'OK' : 'MISMATCH!'}`)
console.log('  unassigned backlog by domain (surfaced in each domain cap-roadmap doc):')
for (const [d, n] of Object.entries(unassignedByDomain).sort((a, b) => b[1] - a[1])) console.log(`    ${(d || 'NULL').padEnd(26)} ${n}`)
if (byConf.low) {
  console.log(`  LOW-CONFIDENCE edges (review these — title-overlap or ambiguous domain):`)
  for (const e of edges.filter(e => e.confidence === 'low').slice(0, 25)) console.log(`    S${e.slice} <- ${e.canonical_key} (${e.match_method})`)
  if (byConf.low > 25) console.log(`    …and ${byConf.low - 25} more low-confidence edges`)
}

if (REPORT_ONLY) { db.close(); process.exit(0) }

// ── write slice_canonical + derive canonical_capability.slice ─────────────────
const tx = db.transaction(() => {
  db.exec('DELETE FROM slice_canonical')
  const ins = db.prepare('INSERT OR REPLACE INTO slice_canonical (slice,canonical_key,match_method,confidence,reviewed,note) VALUES (?,?,?,?,?,NULL)')
  for (const e of edges) ins.run(e.slice, e.canonical_key, e.match_method, e.confidence, e.reviewed || 0)
})
tx()

// derive the convenience .slice column (highest-confidence, lowest-slice wins)
db.exec('UPDATE canonical_capability SET slice = NULL')
db.prepare(`UPDATE canonical_capability SET slice = (
    SELECT sc.slice FROM slice_canonical sc WHERE sc.canonical_key = canonical_capability.key
    ORDER BY (sc.confidence='high') DESC, (sc.confidence='medium') DESC, sc.slice ASC LIMIT 1)
  WHERE key IN (SELECT canonical_key FROM slice_canonical)`).run()

const linkedCanon = db.prepare('SELECT count(*) n FROM canonical_capability WHERE slice IS NOT NULL').get().n
console.log(`  WROTE slice_canonical (${edges.length} edges); derived canonical.slice on ${linkedCanon} rows.`)
db.close()
