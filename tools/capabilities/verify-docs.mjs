#!/usr/bin/env node
// verify-docs.mjs — the BOTH-SIDES drift check. Diffs the numbered docs' fenced capability
// state against capabilities.db and reports divergence. Read-only; exits 1 on any drift so
// CI can gate `docs:verify && parity:check`.
//
// Drift classes:
//   DOC-AHEAD     a doc line shows a status the DB doesn't have -> run `pnpm docs:sync`.
//   DB-AHEAD      the DB status differs from the doc's line     -> run `pnpm docs:gen`.
//   ORPHAN-LINE   a `key` in a doc fence not in canonical_capability (typo / renamed key).
//   UNTRACKED-CAP a canonical cap that appears in NO doc fence  -> generator coverage gap.
//   MONEY-DRIFT   a money cap shown verified anywhere without a real financial-control test.
//
// Usage:
//   node tools/capabilities/verify-docs.mjs            # exit 1 on drift
//   node tools/capabilities/verify-docs.mjs --report   # never exit 1; print summary
import Database from 'better-sqlite3'
import { existsSync, readdirSync, readFileSync } from 'node:fs'
import { join } from 'node:path'
import { CAPABILITIES_DB, DOCS, GENERATED, CRATE_DOCS, BENCHMARKS } from '../_paths.mjs'
import { loadCapabilityShards } from '../canonical-shards/jsonl-lib.mjs'
import { isUsableCanonicalCapabilityTable } from './canonical-fallback.mjs'

const REPORT = process.argv.includes('--report')
const fail = (message, details = {}) => {
  console.error(JSON.stringify({ error: message, ...details }, null, 2))
  process.exit(REPORT ? 0 : 2)
}
const TOKEN_STATUS = { '⬜ spec': 'unimplemented', '🟡 unverified': 'implemented_unverified', '✅ verified': 'verified', '⛔ blocked': 'blocked', '🚫 excluded': 'excluded' }
const LINE = /^- \[([ x~])\]\s+(⬜ spec|🟡 unverified|✅ verified|⛔ blocked|🚫 excluded)\s+`([^`]+)`\s+—\s+(.+)$/
const capsFenceG = /<!--\s*chronica:caps\s+\d+\b[^>]*-->([\s\S]*?)<!--\s*\/chronica:caps\s*-->/g
const tableExists = (database, name) => database.prepare("SELECT count(*) n FROM sqlite_master WHERE type='table' AND name=?").get(name).n > 0
const columns = (database, table) => database.prepare(`PRAGMA table_info(${table})`).all().map(row => row.name)
const requireColumns = (database, table, names, source) => {
  if (!tableExists(database, table)) fail('CAPABILITY_DB_SCHEMA_BLOCKER: required table is missing.', { table, source })
  const present = new Set(columns(database, table))
  const missing = names.filter(name => !present.has(name))
  if (missing.length) fail('CAPABILITY_DB_SCHEMA_BLOCKER: required column(s) are missing.', { table, missing, source })
}

function loadRowsFromCanonicalShards() {
  let shard
  try {
    shard = loadCapabilityShards(process.cwd())
  } catch {
    return null
  }
  if (!shard.meta || shard.entries.length === 0) return null
  return shard.entries.map((entry) => ({
    key: entry.record.capability_key,
    status: entry.record.status,
    moves_money: entry.record.moves_money ? 1 : 0,
    fin: entry.record.financial_control_test,
  }))
}

let db = null
let canonicalDb = null
let overrideSource = 'docs/capabilities-canonical/**/*.jsonl'
let canonicalSource = 'docs/capabilities-canonical/**/*.jsonl'
let dbRows = loadRowsFromCanonicalShards()

const CLOUD_CORE_DB = join(DOCS, 'capabilities-cloud', 'cap-core.db')
if (!dbRows) {
  if (!existsSync(CAPABILITIES_DB)) {
    fail('CANONICAL_SHARDS_REQUIRED: docs/capabilities.db is absent and docs/capabilities-canonical/**/*.jsonl is not available.')
  }
  db = new Database(CAPABILITIES_DB, { readonly: true })
  canonicalDb = db
  canonicalSource = 'docs/capabilities.db'
  // an EMPTY canonical_capability table (0 rows) carries no more truth than a missing one; both
  // must fall back to the cloud core shard rather than silently reporting "0 canonical caps" as
  // if that were real (issue #713).
  if (!isUsableCanonicalCapabilityTable(db, 'canonical_capability')) {
    if (!existsSync(CLOUD_CORE_DB)) {
      fail('CAPABILITY_DB_SCHEMA_BLOCKER: required table is missing and no cloud core shard is available.', {
        table: 'canonical_capability',
        source: 'docs/capabilities.db',
        fallback: 'docs/capabilities-cloud/cap-core.db',
      })
    }
    canonicalDb = new Database(CLOUD_CORE_DB, { readonly: true })
    canonicalSource = 'docs/capabilities-cloud/cap-core.db fallback (LOCAL_AUDIT_REQUIRED)'
  }

  let overrideDb = db
  overrideSource = 'docs/capabilities.db'
  if (!tableExists(db, 'canonical_status_override') && canonicalDb !== db && tableExists(canonicalDb, 'canonical_status_override')) {
    overrideDb = canonicalDb
    overrideSource = 'docs/capabilities-cloud/cap-core.db fallback'
  }

  requireColumns(canonicalDb, 'canonical_capability', ['key', 'status', 'moves_money', 'financial_control_test'], canonicalSource)
  requireColumns(overrideDb, 'canonical_status_override', ['canonical_key', 'status', 'financial_control_test'], overrideSource)

  // effective DB status (honors override) + money flag + fin-test presence
  const overrideRows = overrideDb.prepare('SELECT canonical_key, status, financial_control_test FROM canonical_status_override').all()
  const overrides = new Map(overrideRows.map(r => [r.canonical_key, r]))
  dbRows = canonicalDb.prepare('SELECT key, status, moves_money, financial_control_test fin FROM canonical_capability').all().map(r => {
    const override = overrides.get(r.key)
    return {
      key: r.key,
      status: override?.status ?? r.status,
      moves_money: r.moves_money,
      fin: override?.financial_control_test ?? r.fin,
    }
  })
}
const dbStatus = new Map(dbRows.map(r => [r.key, r]))

// scan doc fences — only docs/_generated/ (and, since the docs/benchmarks/ migration, the
// hybrid benchmark-gate docs that moved out of it) can contain a `<!-- chronica:caps -->`
// fence (see gen-numbered-docs.mjs); hand-authored docs/ prose is never fence-injected. The
// crate atlas docs (docs/crates/) never carry that fence either, but DO carry the separate
// `## N. capabilities.db row(s)` table gen-crate-docs.mjs generates (handled by the
// crate-specific branch below), so they must still be scanned here or the crate-doc <->
// DB cross-check silently stops running the moment a crate doc physically moves. Same
// reasoning applies to docs/benchmarks/: skip it here and the fence-based drift check
// silently stops covering those 4 docs the moment they move out of docs/_generated/.
const genDocs = existsSync(GENERATED) ? readdirSync(GENERATED).filter(f => /^\d{3}-.*\.md$/.test(f) && f !== '000-INDEX.md').map(f => ({ f, dir: GENERATED })) : []
const crateDocs_ = existsSync(CRATE_DOCS) ? readdirSync(CRATE_DOCS).filter(f => /^\d{3}-.*\.md$/.test(f)).map(f => ({ f, dir: CRATE_DOCS })) : []
const benchDocs_ = existsSync(BENCHMARKS) ? readdirSync(BENCHMARKS).filter(f => /^\d{3}-.*\.md$/.test(f)).map(f => ({ f, dir: BENCHMARKS })) : []
const docs = [...genDocs, ...crateDocs_, ...benchDocs_]
const docSeen = new Map()   // key -> {status, fin, doc}
const orphanLine = []
const rank = { unimplemented: 0, blocked: 1, implemented_unverified: 2, excluded: 2, verified: 3 }
const statusFromText = (text) => {
  const value = String(text || '').trim().toLowerCase()
  if (value === 'verified') return 'verified'
  if (value === 'implemented_unverified' || value === 'unverified') return 'implemented_unverified'
  if (value === 'blocked') return 'blocked'
  if (value === 'excluded') return 'excluded'
  return 'unimplemented'
}
const rememberDocKey = (key, status, fin, doc) => {
  if (!dbStatus.has(key)) { orphanLine.push({ key, doc }); return }
  const prev = docSeen.get(key)
  if (!prev || rank[status] > rank[prev.status]) docSeen.set(key, { status, fin, doc })
}
for (const { f, dir } of docs) {
  const raw = readFileSync(join(dir, f), 'utf8')
  for (const fence of raw.matchAll(capsFenceG)) {
    for (const line of fence[1].split('\n')) {
      const m = line.trimEnd().match(LINE); if (!m) continue
      const [, , token, key, tail] = m
      const status = TOKEN_STATUS[token]
      const finM = tail.match(/·\s*💰\s*([^·]+?)(?=\s*·|\s*$)/)
      const fin = finM && !/^REQUIRED/i.test(finM[1].trim()) ? finM[1].trim() : null
      // a cap can appear in multiple docs; the strongest authored status wins for the diff
      rememberDocKey(key, status, fin, f)
    }
  }
  if (/^\d{3}-crate-.*\.md$/.test(f)) {
    for (const line of raw.split(/\r?\n/)) {
      if (!line.startsWith('| ')) continue
      const cells = line.split('|').slice(1, -1).map((cell) => cell.trim())
      if (cells.length < 7) continue
      const [key, statusText, movesText, , , , testText] = cells
      if (!/^[a-z0-9][a-z0-9._-]*\.[a-z0-9][a-z0-9._-]*$/i.test(key)) continue
      const status = statusFromText(statusText)
      const fin = Number(movesText) === 1 && testText && !/^REQUIRED|none$/i.test(testText) ? testText : null
      rememberDocKey(key, status, fin, f)
    }
  }
}

const docAhead = [], dbAhead = [], moneyDrift = []
for (const [key, d] of docSeen) {
  const db_ = dbStatus.get(key)
  if (d.status !== db_.status) {
    // doc shows MORE progress than DB -> DOC-AHEAD; DB shows more -> DB-AHEAD
    const rank = { unimplemented: 0, blocked: 1, implemented_unverified: 2, excluded: 2, verified: 3 }
    if (rank[d.status] > rank[db_.status]) docAhead.push({ key, doc: d.status, db: db_.status })
    else dbAhead.push({ key, doc: d.status, db: db_.status })
  }
  if (d.status === 'verified' && db_.moves_money === 1 && !d.fin) moneyDrift.push(key)
}
// money drift from the DB side too
for (const r of dbRows) if (r.status === 'verified' && r.moves_money === 1 && (!r.fin || /^REQUIRED/i.test(r.fin || ''))) if (!moneyDrift.includes(r.key)) moneyDrift.push(r.key)

// untracked caps: in DB but in NO doc fence (only count those that SHOULD be in a doc —
// i.e. linked to a slice or in a domain that has a cap-roadmap doc; every domain has one,
// so every cap should appear. Report the gap honestly.)
const untracked = dbRows.filter(r => !docSeen.has(r.key) && !orphanLine.find(o => o.key === r.key))

const drift = docAhead.length + dbAhead.length + orphanLine.length + moneyDrift.length
console.log(`canonical source: ${canonicalSource}`)
if (overrideSource !== 'docs/capabilities.db') console.log(`override source: ${overrideSource}`)
console.log(`verify-docs: ${dbRows.length} canonical caps · ${docSeen.size} surfaced in numbered docs · ${untracked.length} not surfaced.`)
const show = (label, arr, fmt) => { if (arr.length) { console.log(`  ${label}: ${arr.length}`); for (const x of arr.slice(0, 12)) console.log(`    ${fmt(x)}`); if (arr.length > 12) console.log(`    …and ${arr.length - 12} more`) } }
show('DOC-AHEAD (docs edited, run docs:sync)', docAhead, x => `${x.key}  doc=${x.doc} db=${x.db}`)
show('DB-AHEAD (docs stale, run docs:gen)', dbAhead, x => `${x.key}  doc=${x.doc} db=${x.db}`)
show('ORPHAN-LINE (key not in DB)', orphanLine, x => `${x.key} @${x.doc}`)
show('MONEY-DRIFT (verified money cap w/o fin test)', moneyDrift.map(k => ({ key: k })), x => x.key)
if (untracked.length) console.log(`  UNTRACKED-CAP (in DB, in no doc fence): ${untracked.length} — generator coverage gap (e.g. domains with no cap-roadmap doc yet).`)

if (drift === 0) { console.log('  ✓ both surfaces agree (no drift).') }
else console.log(`  ✗ DRIFT: ${drift} issue(s).`)
if (canonicalDb && canonicalDb !== db) canonicalDb.close()
if (db) db.close()
process.exit(REPORT ? 0 : (drift > 0 ? 1 : 0))
