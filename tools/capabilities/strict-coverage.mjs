#!/usr/bin/env node
// strict-coverage.mjs — HONEST donor-knowledge coverage (the Coverage Pyramid).
//
// WHY: the old coverage-audit.mjs reports ~100% for every donor because its denominator is
// SELF-REFERENTIAL — it counts only files already in the census, and every census file is 'mapped'.
// (mapped / census-files == 1.0 always.) That is the vanity metric the user rejected.
//
// This tool uses the REAL denominator: capability-bearing source files ON DISK in Temporary/<donor>.
// It computes the pyramid the user defined and applies a STRICT completion gate — a donor is only
// DONE when File >= 70% AND Module >= 70% (AND Architecture >= 70% once that level is measured);
// otherwise its honest status is PARTIALLY_SCOUTED (or UNSCOUTED below 5%).
//
//   L1 File Coverage     = mapped files            / real capability-bearing files on disk
//   L2 Module Coverage   = real modules with >=1 mapped file / real modules on disk
//   L3 Architecture Cov. = (deep-scout pending — not faked; null until per-donor arch is captured)
//   L4 Capability Cov.   = deep extractions (source_capability) / estimated surface (estimate = TODO)
//   L5 Materialization   = aggregate-only here (caps ported into Chronica / caps scouted)
//   Knowledge Reality    = files with a REAL extracted-capability link / mapped files
//                          (distinguishes "deeply understood" from "bulk-classified mapped")
//
// Mastery bands (per the user): 10% usable · 25% strategic · 50% moat · 80% donor-mastery.
//
// Usage: node tools/capabilities/strict-coverage.mjs [--donor <d>] [--json <out>]
import Database from 'better-sqlite3'
import { existsSync, readdirSync, writeFileSync } from 'node:fs'
import { CAPABILITIES_DB } from '../_paths.mjs'

const arg = (f) => { const i = process.argv.indexOf(f); return i >= 0 ? process.argv[i + 1] : null }
const onlyDonor = arg('--donor')
const outPath = arg('--json') || 'docs/doctrines/audit/donor-census-coverage-strict.json'
const GATE = 70 // strict completion threshold (%)

// capability-bearing source extensions (code that can carry behavior); excludes data/docs/config.
const EXT = ['ts', 'tsx', 'js', 'jsx', 'py', 'go', 'rb', 'rs', 'clj', 'cljc', 'java', 'kt', 'php', 'scala', 'ex', 'exs']
// not capability-bearing: vendored, generated, tests, builds, type stubs.
const EXCLUDE = /node_modules|\/dist\/|\/build\/|\.test\.|\.spec\.|__tests__|\/tests?\/|\.stories\.|\/\.git\/|\/generated\/|\.d\.ts$|\/vendor\/|\/migrations\/|\/__mocks__\/|\.min\.js$/

const db = new Database(CAPABILITIES_DB, { readonly: true })

// real capability-bearing files on disk for a donor (the HONEST denominator). Pure-Node recursive
// walk (portable — Node's execSync 'find' resolves to Windows find.exe under cmd.exe, not POSIX find).
const PRUNE = new Set(['node_modules', '.git', 'dist', 'build', 'target', '.next', 'vendor', '__pycache__', '.venv', 'coverage'])
const EXTSET = new Set(EXT)
function diskFiles(donorDir) {
  const root = `Temporary/${donorDir}`
  const out = []
  const stack = [root]
  while (stack.length) {
    const dir = stack.pop()
    let ents
    try { ents = readdirSync(dir, { withFileTypes: true }) } catch { continue }
    for (const e of ents) {
      const full = `${dir}/${e.name}`
      if (e.isDirectory()) { if (!PRUNE.has(e.name)) stack.push(full); continue }
      const ext = e.name.includes('.') ? e.name.slice(e.name.lastIndexOf('.') + 1) : ''
      if (EXTSET.has(ext) && !EXCLUDE.test(full)) out.push(full.slice(root.length + 1))
    }
  }
  return out
}

// module = first 3 path segments (a stable, repo-agnostic logical unit).
const moduleOf = (p) => p.split('/').slice(0, 3).join('/')

const donorDirs = (onlyDonor ? [onlyDonor] : readdirSync('Temporary').filter((d) => existsSync(`Temporary/${d}`)))
  .filter((d) => { try { return readdirSync(`Temporary/${d}`).length > 0 } catch { return false } })

const band = (p) => p >= 80 ? 'donor-mastery' : p >= 50 ? 'moat' : p >= 25 ? 'strategic' : p >= 10 ? 'usable' : 'minimal'

const rows = []
for (const d of donorDirs) {
  const disk = diskFiles(d)
  const realFiles = disk.length
  const realModules = new Set(disk.map(moduleOf))

  const mapped = db.prepare(`SELECT path FROM donor_file_census WHERE donor=? AND classification='mapped'`).all(d).map((r) => r.path)
  const caps = db.prepare(`SELECT COUNT(*) n FROM source_capability WHERE donor=?`).get(d).n
  const linked = db.prepare(`SELECT COUNT(DISTINCT path) n FROM source_file_capability_link WHERE donor=?`).get(d).n

  const mappedModules = new Set(mapped.map(moduleOf).filter((m) => realModules.has(m)))
  const L1 = realFiles ? +(100 * Math.min(mapped.length, realFiles) / realFiles).toFixed(1) : 0
  const L2 = realModules.size ? +(100 * mappedModules.size / realModules.size).toFixed(1) : 0
  // Knowledge Reality = of mapped files, how many carry a REAL extracted-capability link. Capped at
  // 100 (linked can exceed mapped when caps are attributed to support-classified files — a data-quality
  // signal preserved in the raw capability_linked_files / mapped_files columns).
  const reality = mapped.length ? Math.min(100, +(100 * linked / mapped.length).toFixed(1)) : 0

  // STRICT gate: DONE only when File AND Module clear 70% (Architecture folds in once measured).
  // UNSCOUTED is reserved for true zero-knowledge (no mapped files AND no deep extractions); a donor
  // with real caps but tiny coverage is PARTIALLY_SCOUTED (honestly low), not "unscouted".
  const status = (caps === 0 && mapped.length === 0) ? 'UNSCOUTED'
    : (L1 >= GATE && L2 >= GATE) ? 'DONE'
      : 'PARTIALLY_SCOUTED'
  // when the census mapped MORE files than the disk walk found, the File denominator is undercounted
  // (walk excludes something the census kept) -> File% is unreliable-high for that donor.
  const denominatorWarning = mapped.length > realFiles

  rows.push({ donor: d, real_files: realFiles, mapped_files: mapped.length, real_modules: realModules.size,
    mapped_modules: mappedModules.size, deep_caps: caps, capability_linked_files: linked,
    L1_file_pct: L1, L2_module_pct: L2, L3_architecture_pct: null, knowledge_reality_pct: reality,
    mastery_band: band(L1), status, denominator_warning: denominatorWarning })
}
rows.sort((a, b) => a.L1_file_pct - b.L1_file_pct)

const avg = (k) => +(rows.reduce((s, r) => s + (r[k] || 0), 0) / (rows.length || 1)).toFixed(1)
const summary = {
  generated: new Date(0).toISOString().slice(0, 10), // stamped by caller; engine is deterministic
  donors: rows.length,
  status_breakdown: rows.reduce((a, r) => ((a[r.status] = (a[r.status] || 0) + 1), a), {}),
  avg_L1_file_pct: avg('L1_file_pct'),
  avg_L2_module_pct: avg('L2_module_pct'),
  avg_knowledge_reality_pct: avg('knowledge_reality_pct'),
  total_real_files: rows.reduce((s, r) => s + r.real_files, 0),
  total_mapped_files: rows.reduce((s, r) => s + r.mapped_files, 0),
  total_deep_caps: rows.reduce((s, r) => s + r.deep_caps, 0),
  note: 'L1/L2 use REAL disk denominators (not the self-referential census). DONE requires L1>=70 AND L2>=70. L3 architecture + L4 capability-surface estimate + L5 per-donor materialization are deep-scout-pending (intentionally null, not faked).',
}

writeFileSync(outPath, JSON.stringify({ summary, donors: rows }, null, 2) + '\n')

// dashboard
console.log(`\n=== STRICT DONOR-KNOWLEDGE COVERAGE (real disk denominators, gate ${GATE}%) ===`)
console.log(`donors ${summary.donors} | DONE ${summary.status_breakdown.DONE || 0} | PARTIALLY_SCOUTED ${summary.status_breakdown.PARTIALLY_SCOUTED || 0} | UNSCOUTED ${summary.status_breakdown.UNSCOUTED || 0}`)
console.log(`avg File ${summary.avg_L1_file_pct}% | avg Module ${summary.avg_L2_module_pct}% | avg Knowledge-Reality ${summary.avg_knowledge_reality_pct}%`)
console.log(`total real files ${summary.total_real_files} | mapped ${summary.total_mapped_files} | deep caps ${summary.total_deep_caps}\n`)
console.log('lowest-coverage donors (the real frontier):')
for (const r of rows.slice(0, 14)) {
  console.log(`  ${r.status === 'DONE' ? '✓' : r.status === 'UNSCOUTED' ? '·' : '◐'} ${r.donor.padEnd(26)} File ${String(r.L1_file_pct).padStart(5)}%  Module ${String(r.L2_module_pct).padStart(5)}%  KReality ${String(r.knowledge_reality_pct).padStart(5)}%  (${r.mapped_files}/${r.real_files} files, ${r.deep_caps} caps)  [${r.mastery_band}]`)
}
console.log(`\nfull ledger -> ${outPath}`)
