#!/usr/bin/env node
// sync-docs.mjs — the docs->DB write-back (the second half of the two-surface round-trip).
//
// An agent builds a capability and flips its line in a numbered doc:
//   - [ ] ⬜ spec `erp.invoice.create_sales` …   ->   - [x] ✅ verified `erp.invoice.create_sales` … · 🧪 my_test
// This tool scans ONLY the `<!-- chronica:caps … -->` fenced blocks across docs/NNN-*.md,
// parses each capability line, and writes the authored status/test ids into the SURVIVABLE
// `canonical_status_override` table (re-applied after every census rebuild by
// reapply-overrides.mjs). Prose outside the fences is NEVER read. Transactional + idempotent.
//
// MONEY INVARIANT enforced at write time: a money capability cannot be set `verified`
// without a real financial_control_test (gate→approval→CostRecord→audit). Such a write is
// REFUSED and reported, mirroring check-coverage.mjs's MONEY-GAP gate.
//
// Usage:
//   node tools/capabilities/sync-docs.mjs            # parse all docs, write overrides
//   node tools/capabilities/sync-docs.mjs --dry-run  # report what WOULD change; write nothing
import Database from 'better-sqlite3'
import { existsSync, readdirSync, readFileSync } from 'node:fs'
import { join } from 'node:path'
import { CAPABILITIES_DB, GENERATED, BENCHMARKS } from '../_paths.mjs'

const DRY = process.argv.includes('--dry-run')
const db = new Database(CAPABILITIES_DB)
db.pragma('journal_mode = WAL')

// status-token <-> canonical_capability.status bijection
const TOKEN_STATUS = { '⬜ spec': 'unimplemented', '🟡 unverified': 'implemented_unverified', '✅ verified': 'verified', '⛔ blocked': 'blocked', '🚫 excluded': 'excluded' }

// the capability-line grammar emitted by gen-numbered-docs.mjs emitLine():
//   - [<box>] <status-token> `<key>` — <name> → <target> · <gate> [· 🧪 <accept>] [· 💰 <fin>] [· ⛔ <blocker>]
const LINE = /^- \[([ x~])\]\s+(⬜ spec|🟡 unverified|✅ verified|⛔ blocked|🚫 excluded)\s+`([^`]+)`\s+—\s+(.+)$/
const capsFenceG = /<!--\s*chronica:caps\s+\d+\b[^>]*-->([\s\S]*?)<!--\s*\/chronica:caps\s*-->/g

// extract optional trailing test ids from the segment tail (after the key/name)
function parseTail(tail) {
  const accept = (tail.match(/·\s*🧪\s*([^·]+?)(?=\s*·|\s*$)/) || [])[1]?.trim()
  const finRaw = (tail.match(/·\s*💰\s*([^·]+?)(?=\s*·|\s*$)/) || [])[1]?.trim()
  const fin = finRaw && !/^REQUIRED/i.test(finRaw) ? finRaw : null
  return { accept: accept && accept !== '—' ? accept : null, fin }
}

// gather all canonical caps' money flag for the money invariant
const moneyOf = new Map(db.prepare('SELECT key, moves_money FROM canonical_capability').all().map(r => [r.key, r.moves_money]))
const canonKeys = new Set(moneyOf.keys())

// docs/_generated/ and docs/benchmarks/ can contain a `<!-- chronica:caps -->` fence (see
// gen-numbered-docs.mjs) — hand-authored docs/ prose is never fence-injected, so it's
// excluded from this scan by construction. docs/benchmarks/ inherits 4 already-fenced
// hybrid docs (106/116/120/124) migrated out of docs/_generated/; skip it here and the
// docs->DB write-back silently stops covering them the moment they move.
const genDocs_ = readdirSync(GENERATED).filter(f => /^\d{3}-.*\.md$/.test(f) && f !== '000-INDEX.md').map(f => ({ f, dir: GENERATED }))
const benchDocs_ = existsSync(BENCHMARKS) ? readdirSync(BENCHMARKS).filter(f => /^\d{3}-.*\.md$/.test(f)).map(f => ({ f, dir: BENCHMARKS })) : []
const docs = [...genDocs_, ...benchDocs_]
const authored = []      // {key, status, accept, fin, doc}
let lineCount = 0, orphanKeys = []
for (const { f, dir } of docs) {
  const raw = readFileSync(join(dir, f), 'utf8')
  for (const fence of raw.matchAll(capsFenceG)) {
    for (const line of fence[1].split('\n')) {
      const m = line.match(LINE)
      if (!m) continue
      lineCount++
      const [, , token, key, tail] = m
      const status = TOKEN_STATUS[token]
      const { accept, fin } = parseTail(tail)
      if (!canonKeys.has(key)) { orphanKeys.push({ key, doc: f }); continue }
      authored.push({ key, status, accept, fin, doc: f })
    }
  }
}

// keep only AUTHORED progress: rows that differ from the unimplemented default, i.e. an
// agent actually advanced them (status != unimplemented OR a test id was filled in).
const progressed = authored.filter(a => a.status !== 'unimplemented' || a.accept || a.fin)

const moneyRefused = []
const toWrite = []
for (const a of progressed) {
  if (a.status === 'verified' && moneyOf.get(a.key) === 1 && !a.fin) { moneyRefused.push(a.key); continue }
  toWrite.push(a)
}

console.log(`sync-docs${DRY ? ' (DRY-RUN)' : ''}: scanned ${docs.length} docs, ${lineCount} capability lines, ${progressed.length} with authored progress.`)
if (orphanKeys.length) console.log(`  ⚠ ${orphanKeys.length} ORPHAN keys (line key not in canonical_capability): ${orphanKeys.slice(0, 5).map(o => o.key + ' @' + o.doc).join(', ')}${orphanKeys.length > 5 ? '…' : ''}`)
if (moneyRefused.length) console.log(`  ⛔ ${moneyRefused.length} REFUSED money-verified-without-fin-test: ${moneyRefused.slice(0, 5).join(', ')}`)

if (DRY) {
  for (const a of toWrite.slice(0, 20)) console.log(`  would write: ${a.key} -> ${a.status}${a.accept ? ' 🧪 ' + a.accept : ''}${a.fin ? ' 💰 ' + a.fin : ''}`)
  db.close(); process.exit(0)
}

const now = meta('built_at') || ''
const up = db.prepare(`INSERT INTO canonical_status_override (canonical_key,status,acceptance_test,financial_control_test,set_at,set_by)
  VALUES (@key,@status,@accept,@fin,@now,'docs:sync')
  ON CONFLICT(canonical_key) DO UPDATE SET status=@status, acceptance_test=@accept, financial_control_test=@fin, set_at=@now, set_by='docs:sync'`)
const tx = db.transaction(() => { for (const a of toWrite) up.run({ key: a.key, status: a.status, accept: a.accept, fin: a.fin, now }) })
tx()

// immediately re-apply onto canonical_capability so a follow-up docs:gen reflects it.
const applyUp = db.prepare(`UPDATE canonical_capability SET status=@status,
  acceptance_test=COALESCE(@accept,acceptance_test), financial_control_test=COALESCE(@fin,financial_control_test)
  WHERE key=@key`)
const tx2 = db.transaction(() => { for (const a of toWrite) applyUp.run({ key: a.key, status: a.status, accept: a.accept, fin: a.fin }) })
tx2()

console.log(`  WROTE ${toWrite.length} status overrides (survivable) + applied to canonical_capability.`)
db.close()

function meta(k) { try { return db.prepare('SELECT v FROM meta WHERE k=?').get(k)?.v } catch { return null } }
