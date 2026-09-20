#!/usr/bin/env node
// reconcile-w9-target-keys.mjs — reconcile the five Wave 9 target capability names named in
// issue #713 against canonical_capability, from the committed durable seed
// tools/capabilities/w9-target-reconciliation.json.
//
// Each seed entry is one of:
//   insert_unverified — the key does not deterministically match an existing canonical row; import
//                        it as a new canonical_capability row with status='implemented_unverified'
//                        (real code + tests landed on main, but root-DB promotion is local-audit
//                        work, per AGENTS.md) and explicit acceptance_criteria/evidence_ref sourced
//                        from the tracked reconcile report. NEVER 'verified'.
//   map_existing       — the key maps deterministically to one or more PRE-EXISTING canonical keys;
//                         no new row is minted, the existing rows' status is left untouched, and an
//                         informational agent_note records the mapping for local audit.
//
// Idempotent: insert_unverified uses INSERT OR IGNORE (a key that already exists — because a real
// census extraction independently discovered it, or a prior run of this script already inserted it
// — is never overwritten). map_existing's agent_note is only posted once (deduped by body).
//
// Run: node tools/capabilities/reconcile-w9-target-keys.mjs [--db <path>]
import Database from 'better-sqlite3'
import { readFileSync } from 'node:fs'
import { join } from 'node:path'
import { CAPABILITIES_DB } from '../_paths.mjs'

function arg(flag) {
  const i = process.argv.indexOf(flag)
  return i >= 0 ? process.argv[i + 1] : undefined
}
const DB_PATH = arg('--db') || CAPABILITIES_DB
const SRC = join(process.cwd(), 'tools', 'capabilities', 'w9-target-reconciliation.json')

const seed = JSON.parse(readFileSync(SRC, 'utf8'))
const targets = Array.isArray(seed.targets) ? seed.targets : []

const bad = []
for (const t of targets) {
  if (!t.issue_key) { bad.push('missing issue_key'); continue }
  if (t.action === 'insert_unverified') {
    if (!t.canonical_name || !t.acceptance_criteria || !t.evidence_ref) bad.push(`${t.issue_key}: insert_unverified requires canonical_name/acceptance_criteria/evidence_ref`)
  } else if (t.action === 'map_existing') {
    if (!Array.isArray(t.maps_to) || t.maps_to.length === 0) bad.push(`${t.issue_key}: map_existing requires a non-empty maps_to array`)
  } else {
    bad.push(`${t.issue_key}: unknown action ${t.action}`)
  }
}
if (bad.length) {
  console.error(JSON.stringify({ error: 'REFUSED: invalid w9-target-reconciliation.json', problems: bad }, null, 2))
  process.exit(1)
}

const db = new Database(DB_PATH)
db.pragma('journal_mode = WAL')
const hasTable = (name) => db.prepare("SELECT count(*) n FROM sqlite_master WHERE type='table' AND name=?").get(name).n > 0
if (!hasTable('canonical_capability')) {
  console.error(JSON.stringify({
    error: 'CAPABILITY_DB_SCHEMA_BLOCKER: canonical_capability is missing.',
    hint: 'Run `node tools/capabilities/census-schema.mjs` (and, if canonical_capability was lost while ' +
      'side tables survived, `node tools/capabilities/recover-canonical.mjs --apply` first) before reconciling W9 target keys.',
  }, null, 2))
  db.close()
  process.exit(1)
}

// ── row-count-aware guard, mirroring census-schema.mjs: a canonical_capability table that EXISTS
// but has 0 rows carries the exact same "no real truth here" meaning as a MISSING table (issue
// #713). The `hasTable` check above is a no-op the moment ANY canonical_capability table exists —
// including an empty shell left by census-schema.mjs's `CREATE TABLE IF NOT EXISTS` (reached via
// its own `--allow-empty-canonical` escape hatch). Inserting the four W9 rows into that empty shell
// would push canonical_capability's row count above zero and permanently trip
// recover-canonical.mjs's `ALREADY_RECOVERED_NOOP` guard — silently orphaning the surviving
// side-table data (canonical_status_override/impl_evidence/slice_canonical) with no tool left able
// to complete the recovery. This is wired into every `pnpm caps:rebuild`, so it must fail closed
// here rather than trust standard rebuild ordering to run recover-canonical.mjs first. A genuinely
// fresh bootstrap (no side-table evidence) is unaffected — it has nothing to orphan.
const canonicalRowsPreInsert = db.prepare('SELECT count(*) n FROM canonical_capability').get().n
if (canonicalRowsPreInsert === 0) {
  const overrideRows = hasTable('canonical_status_override') ? db.prepare('SELECT count(*) n FROM canonical_status_override').get().n : 0
  const evidenceRows = hasTable('impl_evidence') ? db.prepare('SELECT count(*) n FROM impl_evidence').get().n : 0
  const bridgeRows = hasTable('slice_canonical') ? db.prepare('SELECT count(*) n FROM slice_canonical').get().n : 0
  if (overrideRows > 0 || evidenceRows > 0 || bridgeRows > 0) {
    console.error(JSON.stringify({
      error: 'CANONICAL_CAPABILITY_EMPTY_WITH_SURVIVING_SIDE_TABLES',
      message: 'canonical_capability has 0 rows in this db file, but canonical_status_override/' +
        'impl_evidence/slice_canonical carry real prior work. Inserting the Wave 9 target keys now ' +
        'would make canonical_capability non-empty and permanently block recover-canonical.mjs ' +
        '(its ALREADY_RECOVERED_NOOP guard fires on ANY row present), orphaning the surviving ' +
        'side-table data with no tool left able to finish the recovery. Run ' +
        '`node tools/capabilities/recover-canonical.mjs` (dry-run, then --apply) to restore the ' +
        'canonical namespace from docs/capabilities-cloud/cap-core.db + docs/architecture.db ' +
        'BEFORE reconciling W9 target keys.',
      canonical_status_override_rows: overrideRows,
      impl_evidence_rows: evidenceRows,
      slice_canonical_rows: bridgeRows,
    }, null, 2))
    db.close()
    process.exit(1)
  }
}

const existingKeys = new Set(db.prepare('SELECT key FROM canonical_capability').all().map((r) => r.key))
const ins = db.prepare(`INSERT OR IGNORE INTO canonical_capability
  (key, canonical_name, domain, target_crate, target_module, side_effect_class, moves_money,
   requires_approval, acceptance_criteria, required_tests, financial_control_test, acceptance_test,
   status, exclusion_note, blocker, slice, donor_count)
  VALUES (@key, @canonical_name, @domain, @target_crate, @target_module, @side_effect_class, @moves_money,
   @requires_approval, @acceptance_criteria, NULL, @financial_control_test, NULL,
   'implemented_unverified', NULL, NULL, NULL, 0)`)

const inserted = []
const alreadyPresent = []
const mapped = []
const unresolvedMappings = []
let notesPosted = 0

const tx = db.transaction(() => {
  for (const t of targets) {
    if (t.action === 'insert_unverified') {
      if (existingKeys.has(t.issue_key)) { alreadyPresent.push(t.issue_key); continue }
      const money = t.moves_money ? 1 : 0
      ins.run({
        key: t.issue_key,
        canonical_name: t.canonical_name,
        domain: t.issue_key.includes('.') ? t.issue_key.split('.')[0] : 'unclustered',
        target_crate: t.target_crate || null,
        target_module: t.target_module || null,
        side_effect_class: money ? 'money' : 'internal_write',
        moves_money: money,
        requires_approval: t.requires_approval ? 1 : 0,
        acceptance_criteria: t.acceptance_criteria,
        financial_control_test: money ? (t.financial_control_test || 'REQUIRED: policy->approval->CostRecord->SHA256 audit; no external charge when the gate denies') : null,
      })
      existingKeys.add(t.issue_key)
      inserted.push(t.issue_key)
    } else if (t.action === 'map_existing') {
      const missing = t.maps_to.filter((k) => !existingKeys.has(k))
      if (missing.length) { unresolvedMappings.push({ issue_key: t.issue_key, missing }); continue }
      mapped.push({ issue_key: t.issue_key, maps_to: t.maps_to })
      if (hasTable('agent_note')) {
        const body = `W9 target key reconciliation (issue #713): '${t.issue_key}' maps to existing canonical key(s) ${t.maps_to.join(', ')}. ${t.notes || ''} Evidence: ${t.evidence_ref}`.trim()
        const already = db.prepare("SELECT count(*) n FROM agent_note WHERE kind='handoff' AND body=?").get(body).n > 0
        if (!already) {
          db.prepare(`INSERT INTO agent_note (author, kind, status, capability_key, donor, body, created_at)
            VALUES ('claude', 'handoff', 'resolved', @cap, NULL, @body, @now)`).run({
            cap: t.maps_to[0],
            body,
            now: new Date().toISOString(),
          })
          notesPosted++
        }
      }
    }
  }
})
tx()
db.close()

console.log(JSON.stringify({
  seed: 'tools/capabilities/w9-target-reconciliation.json',
  inserted_unverified: inserted,
  already_present: alreadyPresent,
  mapped_to_existing: mapped,
  unresolved_mappings: unresolvedMappings,
  agent_notes_posted: notesPosted,
  invariant: 'no row written or updated by this script ever has status=verified',
}, null, 2))
