#!/usr/bin/env node
// verify-impl-evidence.mjs — audit that every VERIFIED capability's recorded implementation
// evidence still points at REAL code: the proving test symbol must exist in its test_file, and
// every declared impl symbol must exist in its impl_file. Catches "verified" rows that dangle
// because the code was renamed/deleted. Read-only; exits 1 on any dangling evidence (CI-gateable).
//
// Also reports verified caps that have NO impl_evidence row at all (a verified claim with zero
// Chronica-side proof).
//
// Usage: node tools/capabilities/verify-impl-evidence.mjs [--report]
import Database from 'better-sqlite3'
import { readFileSync, existsSync } from 'node:fs'
import { CAPABILITIES_DB } from '../_paths.mjs'
import { isUsableCanonicalCapabilityTable } from './canonical-fallback.mjs'

const REPORT = process.argv.includes('--report')
const db = new Database(CAPABILITIES_DB, { readonly: true })
const hasTable = (name) => db.prepare("SELECT count(*) n FROM sqlite_master WHERE type='table' AND name=?").get(name).n > 0

const evidence = db.prepare('SELECT canonical_key, impl_file, impl_symbols, test_file, test_symbol FROM impl_evidence').all()
// an EMPTY canonical_capability table (0 rows) carries no more truth than a missing one — both
// fall back to canonical_status_override, so a lost/not-yet-rebuilt canonical table cannot
// silently report zero verified capabilities as if that were real (issue #713).
const canonicalUsable = isUsableCanonicalCapabilityTable(db, 'canonical_capability')
const verified = canonicalUsable
  ? db.prepare("SELECT key FROM canonical_capability WHERE status='verified'").all().map(r => r.key)
  : hasTable('canonical_status_override')
    ? db.prepare("SELECT canonical_key key FROM canonical_status_override WHERE status='verified'").all().map(r => r.key)
    : []
const schema = canonicalUsable ? 'canonical_capability' : 'canonical_status_override'

const fileCache = new Map()
const read = (p) => {
  if (!p) return null
  if (fileCache.has(p)) return fileCache.get(p)
  const s = existsSync(p) ? readFileSync(p, 'utf8') : null
  fileCache.set(p, s)
  return s
}

const dangling = []      // evidence whose test/impl symbol is gone
const noEvidence = []    // verified cap with no evidence row at all (excluding pre-loop)
const evidenceKeys = new Set(evidence.map(e => e.canonical_key))

for (const e of evidence) {
  const tsrc = read(e.test_file)
  const isrc = read(e.impl_file)
  const problems = []
  const testSymbols = (e.test_symbol || '').split(/[;,]/).map(s => s.trim()).filter(Boolean)
  if (!tsrc) problems.push(`test_file missing: ${e.test_file}`)
  else {
    for (const sym of testSymbols) {
      if (!new RegExp(`\\bfn\\s+${sym}\\b`).test(tsrc)) problems.push(`test fn \`${sym}\` absent`)
    }
  }
  if (!isrc) problems.push(`impl_file missing: ${e.impl_file}`)
  else {
    for (const sym of (e.impl_symbols || '').split(',').map(s => s.trim()).filter(Boolean)) {
      if (!new RegExp(`\\b${sym}\\b`).test(isrc)) problems.push(`impl symbol \`${sym}\` absent`)
    }
  }
  if (problems.length) dangling.push({ key: e.canonical_key, problems })
}

// verified caps with no evidence (the pre-loop obs.audit cap is the only legitimate one).
const KNOWN_PRELOOP = new Set(['obs.audit.record_audit_log'])
for (const k of verified) if (!evidenceKeys.has(k) && !KNOWN_PRELOOP.has(k)) noEvidence.push(k)

console.log(`verify-impl-evidence: ${verified.length} verified caps · ${evidence.length} with impl evidence · schema=${schema}.`)
if (dangling.length) {
  console.log(`  ✗ DANGLING evidence (${dangling.length}): code was renamed/deleted but cap still 'verified':`)
  for (const d of dangling) console.log(`    ${d.key}: ${d.problems.join('; ')}`)
}
if (noEvidence.length) {
  console.log(`  ⚠ VERIFIED WITHOUT EVIDENCE (${noEvidence.length}): ${noEvidence.join(', ')}`)
}
const issues = dangling.length + noEvidence.length
if (issues === 0) console.log('  ✓ every verified cap has real, present Chronica code behind it.')
db.close()
process.exit(REPORT ? 0 : (dangling.length > 0 ? 1 : 0))
