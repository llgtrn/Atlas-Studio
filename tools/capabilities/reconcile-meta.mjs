#!/usr/bin/env node
// reconcile-meta.mjs — recompute the caps.db `meta` headline counts from the LIVE tables, so the
// human-facing tracking surface (`pnpm caps:query census`, the doc index generator) never lies.
//
// WHY: `meta` was last written by finalize.mjs ~2026-06-02. The canonical set has grown and been
// re-verified many times since (apply-clusters / reapply-overrides / the reconcile-* scripts mutate
// canonical_capability without re-running finalize). So the STORED meta drifted badly —
// canonical_verified_count=20 vs an actual 1189, money_canonical_count=88 vs 99,
// true_ideal_denominator=2007 vs 3013, source_capability_count=4251 vs 5677. `caps:query scope`
// masked the rot by recomputing some counts live; `caps:query census` and gen-index.mjs read the
// stale STORED values. The capability ROWS are correct — only the cached meta is wrong.
//
// This is a SURVIVABLE reconcile (like reconcile-helpdesk-evidence.mjs / reapply-overrides.mjs):
// committed + idempotent, wired into rebuild-pipeline.mjs AFTER the other reconciles so meta reflects
// the post-reconcile state on every `caps:rebuild`. caps.db is gitignored, so THIS SCRIPT is the
// durable artifact.
//
// MODES:
//   node tools/capabilities/reconcile-meta.mjs            # default: recompute + upsert meta
//   node tools/capabilities/reconcile-meta.mjs --check    # read-only: exit 1 if any stored count drifted
//
// The --check mode is the CI gate (the role verify-impl-evidence.mjs plays for evidence): meta can
// never silently rot again. It compares only the DETERMINISTIC count/percent keys — not the prose
// `note` or the `meta_reconciled_at` stamp.
//
// NO money flags, NO capability rows, NO tenant surface are touched — this writes ONLY the `meta` k/v
// table. Event-timestamp / historical keys (built_at, census_built_at, file_census_*, dedupe_method,
// donors_*, ai_work_*, company_group_runtime_reconciled_at) are PRESERVED, never clobbered.
import Database from 'better-sqlite3'
import { CAPABILITIES_DB } from '../_paths.mjs'
import { attachCanonicalCapabilityFallback } from './canonical-fallback.mjs'

const CHECK = process.argv.includes('--check')

function hasColumn(db, table, column) {
  try {
    return db.prepare(`PRAGMA table_info(${table})`).all().some((r) => r.name === column)
  } catch {
    return false
  }
}

// ── the deterministic, drift-checked count/percent keys — each recomputed purely from live tables. ──
// Order matters: denominator-derived keys read earlier results from `computed`.
const COUNT_KEYS = [
  ['source_capability_count', (db) => db.prepare('SELECT count(*) n FROM source_capability').get().n],
  ['canonical_capability_count', (db) => db.prepare('SELECT count(*) n FROM canonical_capability').get().n],
  ['canonical_verified_count', (db) => db.prepare("SELECT count(*) n FROM canonical_capability WHERE status='verified'").get().n],
  ['money_canonical_count', (db) => db.prepare('SELECT count(*) n FROM canonical_capability WHERE moves_money=1').get().n],
  ['true_ideal_denominator', (_db, c) => c.canonical_capability_count], // the denominator IS the canonical census
  ['current_slices', (db) => hasColumn(db, 'capability', 'kind') ? db.prepare("SELECT count(*) n FROM capability WHERE kind='slice'").get().n : 0],
  ['verified_slices', (db) => hasColumn(db, 'capability', 'kind') && hasColumn(db, 'capability', 'status') ? db.prepare("SELECT count(*) n FROM capability WHERE kind='slice' AND status='verified'").get().n : 0],
  // legacy slice-progress percentages (slices ÷ canonical denominator), kept internally consistent.
  // NOTE: the HONEST progress metric is caps-level: canonical_verified_count / canonical_capability_count
  // (= 1189/3013 ≈ 39%). These slice ratios are retained as-formulated, not silently redefined.
  ['roadmap_pct_of_ideal', (_db, c) => (100 * c.current_slices / c.canonical_capability_count).toFixed(2)],
  ['verified_pct_of_ideal', (_db, c) => (100 * c.verified_slices / c.canonical_capability_count).toFixed(2)],
]

function computeAll(db) {
  const computed = {}
  for (const [k, fn] of COUNT_KEYS) computed[k] = fn(db, computed)
  return computed
}

if (CHECK) {
  // ── read-only drift gate ──
  const db = new Database(CAPABILITIES_DB, { readonly: true })
  attachCanonicalCapabilityFallback(db)
  const computed = computeAll(db)
  const stored = Object.fromEntries(db.prepare('SELECT k,v FROM meta').all().map((r) => [r.k, r.v]))
  db.close()
  const drift = []
  for (const [k] of COUNT_KEYS) {
    if (String(stored[k] ?? '') !== String(computed[k])) drift.push({ k, stored: stored[k] ?? '<unset>', computed: String(computed[k]) })
  }
  console.log(`reconcile-meta --check: ${COUNT_KEYS.length} count keys recomputed from live tables.`)
  if (drift.length) {
    console.error(`  ✗ ${drift.length} meta key(s) DRIFTED (run \`node tools/capabilities/reconcile-meta.mjs\` to fix):`)
    for (const d of drift) console.error(`    ${d.k}: stored=${d.stored} → actual=${d.computed}`)
    process.exit(1)
  }
  console.log('  ✓ every stored meta count matches the live tables.')
  process.exit(0)
}

// ── write mode ──
const db = new Database(CAPABILITIES_DB)
db.pragma('journal_mode = WAL')
attachCanonicalCapabilityFallback(db)

const before = Object.fromEntries(db.prepare('SELECT k,v FROM meta').all().map((r) => [r.k, r.v]))
const computed = computeAll(db)

// fresh, accurate prose + a current money summary (no `->N` so caps:query scope's live rewrite is a no-op).
const moneyVerified = db.prepare("SELECT count(*) n FROM canonical_capability WHERE moves_money=1 AND status='verified'").get().n
const unreadPending = before.file_census_unread_pending ?? '0'
const note =
  `Capability census recomputed live by reconcile-meta.mjs: ${computed.source_capability_count} source capabilities (74 donors) ` +
  `→ ${computed.canonical_capability_count} canonical Chronica capabilities; ${computed.canonical_verified_count} verified, ` +
  `${computed.money_canonical_count} money-moving (all approval-gated). File-level exhaustiveness is tracked separately in ` +
  `donor_file_census (unread_pending=${unreadPending}).`
const moneyFlagsCorrected =
  `${computed.money_canonical_count} money caps (moves_money=1); all require approval; ${moneyVerified} verified, ` +
  `${computed.money_canonical_count - moneyVerified} unimplemented.`
const reconciledAt = new Date().toISOString()

const setMeta = db.prepare('INSERT INTO meta(k,v) VALUES(?,?) ON CONFLICT(k) DO UPDATE SET v=excluded.v')
const tx = db.transaction(() => {
  for (const [k] of COUNT_KEYS) setMeta.run(k, String(computed[k]))
  setMeta.run('note', note)
  setMeta.run('money_flags_corrected', moneyFlagsCorrected)
  setMeta.run('meta_reconciled_at', reconciledAt)
})
tx()

// ── self-check: every deterministic count key must now equal its recomputed value. ──
const after = Object.fromEntries(db.prepare('SELECT k,v FROM meta').all().map((r) => [r.k, r.v]))
db.close()
let bad = 0
const changes = []
for (const [k] of COUNT_KEYS) {
  if (String(after[k]) !== String(computed[k])) { console.error(`  ✗ self-check: ${k} did not persist (${after[k]} ≠ ${computed[k]})`); bad++ }
  if (String(before[k] ?? '<unset>') !== String(computed[k])) changes.push(`${k}: ${before[k] ?? '<unset>'} → ${computed[k]}`)
}

console.log(`reconcile-meta: recomputed ${COUNT_KEYS.length} count keys + note + money summary from the live tables (meta-only; no capability/money rows touched).`)
if (changes.length) {
  console.log(`  corrected ${changes.length} stale value(s):`)
  for (const c of changes) console.log(`    ${c}`)
} else {
  console.log('  no count drift — meta was already current (idempotent re-run).')
}
console.log(`  preserved event/historical keys (built_at, census_built_at, file_census_*, dedupe_method, donors_*, ai_work_*, company_group_runtime_reconciled_at).`)
console.log(`  meta_reconciled_at = ${reconciledAt}`)
if (bad) { console.error(`  ✗ ${bad} self-check failure(s)`); process.exit(1) }
console.log('  ✓ self-check clean — every stored count matches the live tables.')
