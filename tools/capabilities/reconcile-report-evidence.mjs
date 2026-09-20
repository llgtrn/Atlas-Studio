#!/usr/bin/env node
// reconcile-report-evidence.mjs
//
// Re-derive verified canonical capability rows from committed per-crate reconcile
// reports, but only when the claim is backed by an actual Rust test function on
// disk. This keeps docs/capabilities.db rebuildable from tracked inputs instead
// of relying on ignored SQLite state.
import Database from 'better-sqlite3'
import { existsSync, readFileSync, readdirSync } from 'node:fs'
import { isAbsolute, join, relative, resolve, sep } from 'node:path'
import { CAPABILITIES_DB, DOCS } from '../_paths.mjs'

const REPORT_DIR = join(DOCS, '_machine', 'reconcile-reports')
const NOW = process.env.CHRONICA_RECONCILE_NOW || new Date().toISOString()

const slash = (p) => p.split(sep).join('/')

function walkRustFiles(dir) {
  const out = []
  if (!existsSync(dir)) return out
  for (const ent of readdirSync(dir, { withFileTypes: true })) {
    const full = join(dir, ent.name)
    if (ent.isDirectory()) out.push(...walkRustFiles(full))
    else if (ent.isFile() && ent.name.endsWith('.rs')) out.push(full)
  }
  return out
}

function reportFiles() {
  if (!existsSync(REPORT_DIR)) return []
  return readdirSync(REPORT_DIR)
    .filter((f) => /^chronica-[a-z0-9-]+\.json$/.test(f))
    .sort()
    .map((f) => join(REPORT_DIR, f))
}

function candidateKeys(text, keys) {
  const found = []
  for (const m of text.matchAll(/\b[a-z][a-z0-9-]+\.[a-z0-9_.-]+\b/g)) {
    const key = m[0].replace(/[.,;:]+$/, '')
    if (keys.has(key) && !found.includes(key)) found.push(key)
  }
  return found
}

function candidateSymbols(text) {
  const stop = new Set([
    'acceptance_test',
    'architecture_target_gap',
    'canonical_capability',
    'capability_architecture_link',
    'financial_control_test',
    'side_effect_class',
    'target_crate',
    'target_module',
    'moves_money',
  ])
  return [...new Set([...text.matchAll(/\b[a-z][a-z0-9_]{4,}\b/g)]
    .map((m) => m[0])
    .filter((s) => s.includes('_') && !stop.has(s)))]
}

function validIdent(value) {
  const s = String(value || '').trim()
  return /^[A-Za-z_][A-Za-z0-9_]*$/.test(s) ? s : ''
}

function structuredEvidence(change) {
  const ev = change?.evidence || change?.impl_evidence || change?.implementation_evidence
  if (!ev || typeof ev !== 'object' || Array.isArray(ev)) {
    return { ok: false, reason: 'missing structured implementation evidence' }
  }
  const implFile = String(ev.impl_file || ev.implFile || '').trim()
  const testFile = String(ev.test_file || ev.testFile || implFile).trim()
  const testSymbol = validIdent(ev.test_symbol || ev.testSymbol || ev.test)
  const rawSymbols = Array.isArray(ev.impl_symbols || ev.implSymbols)
    ? (ev.impl_symbols || ev.implSymbols)
    : String(ev.impl_symbols || ev.implSymbols || '').split(',')
  const implSymbols = [...new Set(rawSymbols.map(validIdent).filter(Boolean))]
  if (!implFile || !testFile) return { ok: false, reason: 'structured evidence missing impl_file/test_file' }
  if (!testSymbol) return { ok: false, reason: 'structured evidence missing valid test_symbol' }
  if (!implSymbols.length) return { ok: false, reason: 'structured evidence missing non-empty impl_symbols' }
  return { ok: true, implFile, testFile, testSymbol, implSymbols }
}

function resolveInside(base, input) {
  if (!input || isAbsolute(input)) return null
  const baseFull = resolve(base)
  const full = resolve(process.cwd(), input)
  const relPath = relative(baseFull, full)
  if (relPath === '' || (!relPath.startsWith('..') && !isAbsolute(relPath))) return full
  return null
}

function hasTestAttributeBefore(source, fnIndex) {
  const prefix = source.slice(Math.max(0, fnIndex - 240), fnIndex)
  return /#\s*\[\s*(?:tokio::)?test(?:\s*\([^)]*\))?\s*\]/.test(prefix)
}

function hasTestFn(source, symbol) {
  const match = new RegExp(`\\bfn\\s+${symbol}\\b`).exec(source)
  return Boolean(match && hasTestAttributeBefore(source, match.index))
}

function hasSymbol(source, symbol) {
  return new RegExp(`\\b${symbol}\\b`).test(source)
}

function findRustTest(files, symbols) {
  for (const symbol of symbols) {
    const fnRe = new RegExp(`\\bfn\\s+${symbol}\\b`)
    for (const file of files) {
      const src = readFileSync(file, 'utf8')
      const match = fnRe.exec(src)
      if (match && hasTestAttributeBefore(src, match.index)) {
        return { testFile: file, testSymbol: symbol }
      }
    }
  }
  return null
}

function moduleFile(crateName, targetModule, fallbackFile) {
  const src = join(process.cwd(), 'crates', crateName, 'src')
  const normalized = String(targetModule || '')
    .replaceAll('::', '/')
    .replaceAll('\\', '/')
    .replace(/^\/+|\/+$/g, '')
  const candidates = normalized
    ? [
        join(src, `${normalized}.rs`),
        join(src, normalized, 'mod.rs'),
      ]
    : []
  for (const path of candidates) if (existsSync(path)) return path
  return fallbackFile
}

function isPartialClaim(text) {
  return /\b(partial|partially|caveat|with-caveat|implemented-as|out of scope|remains out)\b/i.test(text)
}

const db = new Database(CAPABILITIES_DB)
const caps = db.prepare(`
  SELECT key, target_crate, target_module, moves_money, status
  FROM canonical_capability
`).all()
const capByKey = new Map(caps.map((c) => [c.key, c]))
const allKeys = new Set(capByKey.keys())

const updateCap = db.prepare(`
  UPDATE canonical_capability
  SET status='verified', acceptance_test=@test, financial_control_test=NULL
  WHERE key=@key
`)
const upsertOverride = db.prepare(`
  INSERT INTO canonical_status_override
    (canonical_key, status, acceptance_test, financial_control_test, moves_money, set_at, set_by)
  VALUES
    (@key, 'verified', @test, NULL, NULL, @now, 'reconcile-report-evidence')
  ON CONFLICT(canonical_key) DO UPDATE SET
    status='verified',
    acceptance_test=excluded.acceptance_test,
    financial_control_test=NULL,
    moves_money=NULL,
    set_at=excluded.set_at,
    set_by=excluded.set_by
`)
const updateImplementedUnverified = db.prepare(`
  UPDATE canonical_capability
  SET status='implemented_unverified', acceptance_test=@test, financial_control_test=NULL
  WHERE key=@key
`)
const upsertImplementedUnverifiedOverride = db.prepare(`
  INSERT INTO canonical_status_override
    (canonical_key, status, acceptance_test, financial_control_test, moves_money, set_at, set_by)
  VALUES
    (@key, 'implemented_unverified', @test, NULL, NULL, @now, 'reconcile-report-evidence')
  ON CONFLICT(canonical_key) DO UPDATE SET
    status='implemented_unverified',
    acceptance_test=excluded.acceptance_test,
    financial_control_test=NULL,
    moves_money=NULL,
    set_at=excluded.set_at,
    set_by=excluded.set_by
`)
const upsertEvidence = db.prepare(`
  INSERT INTO impl_evidence
    (canonical_key, impl_file, impl_symbols, test_file, test_symbol, donor_source, verified_at, verified_by, notes)
  VALUES
    (@key, @implFile, @symbols, @testFile, @test, @report, @now, 'reconcile-report-evidence', @notes)
  ON CONFLICT(canonical_key) DO UPDATE SET
    impl_file=excluded.impl_file,
    impl_symbols=excluded.impl_symbols,
    test_file=excluded.test_file,
    test_symbol=excluded.test_symbol,
    donor_source=excluded.donor_source,
    verified_at=excluded.verified_at,
    verified_by=excluded.verified_by,
    notes=excluded.notes
`)
const deleteOwnedEvidence = db.prepare("DELETE FROM impl_evidence WHERE verified_by='reconcile-report-evidence' AND canonical_key=@key")

const decisions = []
const seen = new Set()

for (const file of reportFiles()) {
  const report = JSON.parse(readFileSync(file, 'utf8'))
  const changes = Array.isArray(report.tracking_changes) ? report.tracking_changes : []
  for (const change of changes.filter((c) => c.db === 'capabilities')) {
    const text = JSON.stringify(change)
    for (const key of candidateKeys(text, allKeys)) {
      if (seen.has(key)) continue
      seen.add(key)
      const cap = capByKey.get(key)
      const crateName = cap.target_crate || report.crate
      const crateSrc = join(process.cwd(), 'crates', crateName, 'src')
      if (!existsSync(crateSrc)) {
        decisions.push({ key, action: 'skipped', reason: `missing crate src: ${crateName}` })
        continue
      }
      if (cap.moves_money === 1) {
        decisions.push({ key, action: 'skipped', reason: 'money capability requires explicit financial-control import' })
        continue
      }
      if (isPartialClaim(text)) {
        decisions.push({ key, action: 'skipped', reason: 'partial or caveated report claim' })
        continue
      }
      const files = walkRustFiles(crateSrc)
      const evidence = structuredEvidence(change)
      if (!evidence.ok) {
        const hit = findRustTest(files, candidateSymbols(text))
        if (hit) {
          decisions.push({
            key,
            action: 'implemented_unverified',
            test: hit.testSymbol,
            testFile: slash(hit.testFile),
            reason: evidence.reason,
          })
        } else {
          decisions.push({ key, action: 'skipped', reason: `${evidence.reason}; no named Rust #[test] function found from report text` })
        }
        continue
      }
      const crateRoot = join(process.cwd(), 'crates', crateName)
      const implFileFull = resolveInside(crateRoot, evidence.implFile)
      const testFileFull = resolveInside(crateRoot, evidence.testFile)
      if (!implFileFull || !testFileFull || !implFileFull.endsWith('.rs') || !testFileFull.endsWith('.rs')) {
        decisions.push({ key, action: 'skipped', reason: 'structured evidence path outside target crate or not Rust' })
        continue
      }
      const implSrc = readFileSync(implFileFull, 'utf8')
      const testSrc = testFileFull === implFileFull ? implSrc : readFileSync(testFileFull, 'utf8')
      if (!hasTestFn(testSrc, evidence.testSymbol)) {
        decisions.push({ key, action: 'skipped', reason: `structured test fn absent or not #[test]: ${evidence.testSymbol}` })
        continue
      }
      const missingSymbols = evidence.implSymbols.filter((symbol) => !hasSymbol(implSrc, symbol))
      if (missingSymbols.length) {
        decisions.push({ key, action: 'skipped', reason: `structured impl symbol(s) absent: ${missingSymbols.join(', ')}` })
        continue
      }
      decisions.push({
        key,
        action: 'verified',
        test: evidence.testSymbol,
        testFile: slash(testFileFull),
        implFile: slash(implFileFull),
        implSymbols: evidence.implSymbols.join(', '),
        report: slash(file),
      })
    }
  }
}

const tx = db.transaction(() => {
  for (const d of decisions.filter((x) => x.action === 'implemented_unverified' || x.action === 'skipped')) {
    deleteOwnedEvidence.run({ key: d.key })
  }
  for (const d of decisions.filter((x) => x.action === 'implemented_unverified')) {
    updateImplementedUnverified.run({ key: d.key, test: d.test })
    upsertImplementedUnverifiedOverride.run({ key: d.key, test: d.test, now: NOW })
  }
  for (const d of decisions.filter((x) => x.action === 'verified')) {
    updateCap.run({ key: d.key, test: d.test })
    upsertOverride.run({ key: d.key, test: d.test, now: NOW })
    upsertEvidence.run({
      key: d.key,
      implFile: d.implFile,
      symbols: d.implSymbols,
      testFile: d.testFile,
      test: d.test,
      report: d.report,
      now: NOW,
      notes: 'Derived from committed crate reconcile report and validated against structured, non-empty implementation evidence plus an on-disk Rust test function.',
    })
  }
})
tx()

const verified = decisions.filter((d) => d.action === 'verified')
const implementedUnverified = decisions.filter((d) => d.action === 'implemented_unverified')
const skipped = decisions.filter((d) => d.action === 'skipped')
console.log(`reconcile-report-evidence: verified ${verified.length} report-backed cap(s); marked ${implementedUnverified.length} implemented_unverified; skipped ${skipped.length}.`)
for (const d of verified) {
  console.log(`  verified ${d.key} :: ${d.test} (${d.testFile}) [${d.implSymbols}]`)
}
for (const d of implementedUnverified) {
  console.log(`  implemented_unverified ${d.key} :: ${d.test} (${d.testFile}) — ${d.reason}`)
}
if (skipped.length) {
  const byReason = new Map()
  for (const d of skipped) byReason.set(d.reason, (byReason.get(d.reason) || 0) + 1)
  console.log('  skipped by reason:')
  for (const [reason, count] of [...byReason.entries()].sort()) console.log(`    ${count} - ${reason}`)
}

db.close()
