#!/usr/bin/env node
// record-verified.mjs — record a capability as VERIFIED with full Chronica-side implementation
// evidence, and GUARD that the proving test actually exists in the code before allowing it.
//
// This closes the "DB only tracks a test name" gap: a verified cap now records WHAT code proves it
// (impl_file + impl_symbols), the proving TEST (test_file::test_symbol), the DONOR source it was
// modeled against, and a REAL timestamp — all in the survivable `impl_evidence` table, plus the
// status/test in canonical_status_override + canonical_capability.
//
// HARD GUARD: it greps the impl/test file for the test symbol; if the test is NOT present in code,
// it REFUSES to mark verified (status stays as-is). So "verified" can never dangle on a missing test.
// MONEY GUARD: a money cap cannot be verified without a real financial_control_test.
//
// Usage:
//   node tools/capabilities/record-verified.mjs --key <k> --test <test_symbol> \
//     --impl <impl_file> --symbols <a,b,c> --donor <donor_src> [--fin <fin_test_path>] \
//     [--config]   # --config => this cap moves NO money: set moves_money=0, no fin-test required
import Database from 'better-sqlite3'
import { execFileSync } from 'node:child_process'
import { readFileSync, existsSync } from 'node:fs'
import { dirname, join } from 'node:path'
import { fileURLToPath } from 'node:url'
import { ARCHITECTURE_DB, CAPABILITIES_DB } from '../_paths.mjs'
import { isUsableCanonicalCapabilityTable } from './canonical-fallback.mjs'
import { exportCanonicalCapabilityShards } from './export-canonical-shards.mjs'
import { verifyCanonicalCapabilityShards } from './verify-canonical-shards.mjs'
import { exportCanonicalArchitectureShards } from '../architecture/export-canonical-shards.mjs'
import { verifyCanonicalArchitectureShards } from '../architecture/verify-canonical-shards.mjs'
import { loadArchitectureShards, loadCapabilityShards, rel } from '../canonical-shards/jsonl-lib.mjs'

const here = dirname(fileURLToPath(import.meta.url))

function arg(flag) { const i = process.argv.indexOf(flag); return i >= 0 ? process.argv[i + 1] : undefined }
const isConfig = process.argv.includes('--config')
const key = arg('--key')
const testSymbol = arg('--test')
const implFile = arg('--impl')
const symbols = arg('--symbols') || ''
const donorSrc = arg('--donor') || ''
const finTest = arg('--fin')
const movesMoneyArg = arg('--moves-money')
const testFile = arg('--test-file') || implFile
const verifiedBy = arg('--by') || 'loop:verify'
const notes = arg('--notes') || ''

if (!key || !testSymbol || !implFile) {
  console.error('Usage: --key <k> --test <test_symbol> --impl <impl_file> [--symbols a,b] [--donor src] [--fin path] [--config]')
  process.exit(2)
}

const db = new Database(CAPABILITIES_DB)
const hasTable = (name) => db.prepare("SELECT count(*) n FROM sqlite_master WHERE type='table' AND name=?").get(name).n > 0
const requireTable = (name) => {
  if (!hasTable(name)) {
    console.error(`CAPABILITY_DB_SCHEMA_BLOCKER: required table is missing: ${name}; run pnpm caps:schema`)
    process.exit(1)
  }
}
requireTable('canonical_status_override')
requireTable('impl_evidence')
// an EMPTY canonical_capability table (0 rows) is treated the same as a missing one — both fall
// back to the slice_canonical/canonical_status_override side tables instead of wrongly refusing a
// genuinely known key just because the canonical row was lost, not because it never existed
// (issue #713).
const hasCanonical = isUsableCanonicalCapabilityTable(db, 'canonical_capability')
const cap = hasCanonical ? db.prepare('SELECT key, moves_money, target_crate FROM canonical_capability WHERE key=?').get(key) : null
if (hasCanonical && !cap) { console.error(`no such canonical capability: ${key}`); process.exit(1) }
if (!hasCanonical) {
  const knownInBridge = hasTable('slice_canonical')
    ? db.prepare('SELECT count(*) n FROM slice_canonical WHERE canonical_key=?').get(key).n > 0
    : false
  const knownInOverride = db.prepare('SELECT count(*) n FROM canonical_status_override WHERE canonical_key=?').get(key).n > 0
  if (!knownInBridge && !knownInOverride) {
    console.error(`REFUSED: canonical_capability is missing and key is not known in slice_canonical/canonical_status_override: ${key}`)
    process.exit(1)
  }
}

// ── GUARD 1: the impl file must exist and CONTAIN the proving test symbol (no dangling verified). ──
if (!existsSync(implFile)) { console.error(`REFUSED: impl file not found: ${implFile}`); process.exit(1) }
const implSrc = readFileSync(implFile, 'utf8')
if (!existsSync(testFile)) { console.error(`REFUSED: test file not found: ${testFile}`); process.exit(1) }
const testSrc = testFile === implFile ? implSrc : readFileSync(testFile, 'utf8')
if (!new RegExp(`\\bfn\\s+${testSymbol}\\b`).test(testSrc)) {
  console.error(`REFUSED: test \`fn ${testSymbol}\` not found in ${testFile} — cannot mark verified on a missing test.`)
  process.exit(1)
}
// every declared impl symbol must actually appear in the impl file.
const declared = symbols.split(',').map(s => s.trim()).filter(Boolean)
const missing = declared.filter(s => !new RegExp(`\\b${s}\\b`).test(implSrc))
if (missing.length) { console.error(`REFUSED: impl symbols not found in ${implFile}: ${missing.join(', ')}`); process.exit(1) }

// ── GUARD 2: money invariant — a money cap needs a real financial-control test. ──
let isMoney
if (isConfig) {
  isMoney = 0
} else if (hasCanonical) {
  isMoney = cap.moves_money === 1 ? 1 : 0
} else if (movesMoneyArg === '0' || movesMoneyArg === '1') {
  isMoney = Number(movesMoneyArg)
} else if (finTest) {
  isMoney = 1
} else {
  console.error('REFUSED: canonical_capability is missing; pass --config or --moves-money <0|1> so the survivable override records money classification explicitly.')
  process.exit(1)
}
if (isMoney && (!finTest || /^REQUIRED/i.test(finTest))) {
  console.error(`REFUSED: money capability ${key} requires a real --fin financial-control test.`)
  process.exit(1)
}

// CONV0AR2: field authority must match this script's ACTUAL mutation semantics, not merely fields
// the fresh DB-derived record happens to carry a value for. Both write paths below (the isConfig
// UPDATE and the normal UPDATE) touch status/acceptance_test/financial_control_test/moves_money;
// only the isConfig path also sets side_effect_class. Both paths upsert impl_evidence
// unconditionally, which feeds code_refs/test_refs/evidence_refs. architecture_refs is authorized
// because the architecture-link sync below (`capability_architecture_link.status=...`) is a real
// write this script performs. docs_refs is authorized because it is the doc-mention evidence this
// promotion's own POST-WRITE VERIFY GUARD requires to be non-empty before a capability may be
// 'verified' -- record-verified.mjs's entire purpose is asserting that evidence now exists, not a
// value it merely happens to see. target_crate and requires_approval are never written or asserted
// by this script and must never be authorized here.
const authorizedFields = [
  'status', 'acceptance_test', 'financial_control_test', 'moves_money',
  'code_refs', 'test_refs', 'evidence_refs', 'docs_refs', 'architecture_refs',
]
if (isConfig) authorizedFields.push('side_effect_class')

const nowIso = arg('--now') || isoNowFromArgs()
function isoNowFromArgs() {
  // Date.now()/new Date() are fine in a normal CLI (only restricted inside Workflow scripts).
  return new Date().toISOString()
}

// ── Pre-write snapshot for POST-WRITE VERIFY GUARD rollback (see below): captured before any
// row is touched so a promotion that fails the canonical-shard invariant check can be undone
// instead of landing half-synced, the same failure mode that produced the 38-record gap. Built
// unconditionally (not just when hasCanonical) because the ARCHITECTURE-SIDE guard below can
// also trigger a rollback for a capability that only ever lived in canonical_status_override
// (no canonical_capability row) but still has stale/broken architecture.db linkage.
const preWriteSnapshot = {
  canonicalCapabilityRow: hasCanonical
    ? db.prepare('SELECT status, acceptance_test, financial_control_test, moves_money, side_effect_class FROM canonical_capability WHERE key=?').get(key)
    : null,
  overrideRow: db.prepare('SELECT status, acceptance_test, financial_control_test, moves_money, set_at, set_by FROM canonical_status_override WHERE canonical_key=?').get(key) ?? null,
  implEvidenceRow: db.prepare('SELECT impl_file, impl_symbols, test_file, test_symbol, donor_source, verified_at, verified_by, notes FROM impl_evidence WHERE canonical_key=?').get(key) ?? null,
}

const tx = db.transaction(() => {
  if (hasCanonical && isConfig) {
    db.prepare("UPDATE canonical_capability SET moves_money=0, side_effect_class='internal_write', status='verified', acceptance_test=?, financial_control_test=NULL WHERE key=?").run(testSymbol, key)
    // moves_money=0 pinned in the survivable override so a census re-extraction cannot re-inflate it.
    db.prepare(`INSERT INTO canonical_status_override (canonical_key,status,acceptance_test,financial_control_test,moves_money,set_at,set_by)
      VALUES (?,?,?,NULL,0,?,?)
      ON CONFLICT(canonical_key) DO UPDATE SET status='verified', acceptance_test=excluded.acceptance_test, financial_control_test=NULL, moves_money=0, set_at=excluded.set_at, set_by=excluded.set_by`)
      .run(key, 'verified', testSymbol, nowIso, verifiedBy)
  } else if (hasCanonical) {
    // preserve the capability's REAL moves_money classification (isMoney, computed above from
    // cap.moves_money) — pinning it in the override too, so a census re-extraction (which defaults
    // moves_money) cannot silently revert a real money cap to "no money". Money caps still require the
    // GUARD above to have passed (a real --fin test); non-money caps keep moves_money=0 even without
    // --config, instead of being silently mislabeled as money (the bug this comment replaces).
    db.prepare("UPDATE canonical_capability SET status=?, acceptance_test=?, financial_control_test=?, moves_money=? WHERE key=?")
      .run('verified', testSymbol, finTest || null, isMoney, key)
    db.prepare(`INSERT INTO canonical_status_override (canonical_key,status,acceptance_test,financial_control_test,moves_money,set_at,set_by)
      VALUES (?,?,?,?,?,?,?)
      ON CONFLICT(canonical_key) DO UPDATE SET status='verified', acceptance_test=excluded.acceptance_test, financial_control_test=excluded.financial_control_test, moves_money=excluded.moves_money, set_at=excluded.set_at, set_by=excluded.set_by`)
      .run(key, 'verified', testSymbol, finTest || null, isMoney, nowIso, verifiedBy)
  } else {
    db.prepare(`INSERT INTO canonical_status_override (canonical_key,status,acceptance_test,financial_control_test,moves_money,set_at,set_by)
      VALUES (?,?,?,?,?,?,?)
      ON CONFLICT(canonical_key) DO UPDATE SET status='verified', acceptance_test=excluded.acceptance_test, financial_control_test=excluded.financial_control_test, moves_money=excluded.moves_money, set_at=excluded.set_at, set_by=excluded.set_by`)
      .run(key, 'verified', testSymbol, finTest || null, isMoney, nowIso, verifiedBy)
  }
  // the Chronica-side proof — the thing that was missing.
  db.prepare(`INSERT INTO impl_evidence (canonical_key,impl_file,impl_symbols,test_file,test_symbol,donor_source,verified_at,verified_by,notes)
    VALUES (@key,@impl,@symbols,@testFile,@test,@donor,@now,@by,@notes)
    ON CONFLICT(canonical_key) DO UPDATE SET impl_file=@impl, impl_symbols=@symbols, test_file=@testFile, test_symbol=@test, donor_source=@donor, verified_at=@now, verified_by=@by, notes=@notes`)
    .run({ key, impl: implFile, symbols, testFile, test: testSymbol, donor: donorSrc, now: nowIso, by: verifiedBy, notes })
})
tx()

console.log(`recorded verified${isConfig ? ' (config, moves_money=0)' : ''}: ${key}`)
console.log(`  impl: ${implFile} [${symbols}]  test: ${testSymbol}  donor: ${donorSrc}  at: ${nowIso}`)
db.close()

// ── Sync the mirrored status in architecture.db's capability_architecture_link table. ──
// This table carries its OWN copy of `status` for every architecture_node this capability links
// to (see tools/architecture/*), and nothing else in this script (or its callers) updates it.
// Left alone, `caps:canonical:export` faithfully re-embeds this now-stale status into the
// capability's own `architecture_refs` field, and a subsequent `arch:canonical:export` bakes the
// same staleness into docs/architecture-canonical/links.jsonl — a promotion that "worked" (canonical
// row and doc surfaces show `verified`) while architecture_refs/links.jsonl/crate shards silently
// keep reporting the pre-promotion status. This is a same-process fix for the one identifiable
// code path that performs a status promotion; it does not (and cannot, from here) replace running
// `caps:canonical:export && arch:canonical:export && pnpm subdb:gen -- --crate <crate>` afterward
// to actually regenerate the git-tracked canonical JSONL and crate shards from the now-consistent
// DB pair — see docs/doctrines/023-five-dimension-cloud-shard-contract.md.
const root = process.cwd()
let archLinkSnapshot = []
let archLinkTablePresent = false
if (existsSync(ARCHITECTURE_DB)) {
  const archDb = new Database(ARCHITECTURE_DB)
  try {
    const hasLinkTable = archDb.prepare("SELECT count(*) n FROM sqlite_master WHERE type='table' AND name='capability_architecture_link'").get().n > 0
    if (hasLinkTable) {
      archLinkTablePresent = true
      // snapshot for the POST-WRITE VERIFY GUARD rollback below.
      archLinkSnapshot = archDb.prepare('SELECT rowid, status FROM capability_architecture_link WHERE capability_key=?').all(key)
      const result = archDb.prepare('UPDATE capability_architecture_link SET status=? WHERE capability_key=?').run('verified', key)
      if (result.changes > 0) console.log(`  architecture.db: synced status=verified on ${result.changes} capability_architecture_link row(s) for ${key}`)
    }
  } finally {
    archDb.close()
  }
}

// ── POST-WRITE VERIFY GUARD ──────────────────────────────────────────────────────────────────
// The two syncs above make the DB pair self-consistent, but nothing regenerates or checks the
// git-tracked canonical JSONL shards (docs/capabilities-canonical/domains/*.jsonl) that
// `caps:canonical:verify` actually inspects -- that requires a separate `caps:canonical:export`
// run this script never triggered. This left the invariant added in PR #2654 specifically to
// catch this class of drift (verify-canonical-shards.mjs's architecture_refs-staleness check)
// reachable only by an agent manually running `pnpm caps:canonical:verify` and pasting the
// output into a report -- a "paper tiger" that never actually blocks a bad promotion. Close the
// gap by re-exporting from the now-updated DB pair and running the SAME check function used by
// `caps:canonical:verify`, scoped to just this capability's own shard record so a pre-existing,
// unrelated violation on a different capability elsewhere in the shard file never blocks this
// promotion. A NEW violation on THIS record fails the promotion (non-zero exit) and rolls the
// DB writes back so a refused promotion never lands half-synced.
if (hasCanonical) {
  exportCanonicalCapabilityShards({ root, expectedChanges: { [key]: authorizedFields } })
  const verifyResult = verifyCanonicalCapabilityShards({ root })
  if (!verifyResult.ok) {
    const { entries } = loadCapabilityShards(root)
    const entry = entries.find((candidate) => candidate.record?.capability_key === key)
    const path = entry ? `${rel(root, entry.file)}:${entry.line}` : null
    // anchored on a boundary right after the line number ('.' or ':', the only two characters
    // every error format in verify-canonical-shards.mjs puts immediately after `path`) -- a bare
    // `error.startsWith(path)` would let an error on line 50 ("file:50...") false-match a scope
    // filter built for line 5 ("file:5"), since "...:50" is a string-prefix of "...:5" followed by
    // more digits. Real domain shards run 500-1000+ records, so this line-number collision is a
    // real, recurring risk, not a corner case (Lane C1 audit on PR #2659).
    const scoped = path ? verifyResult.errors.filter((error) => error.startsWith(`${path}:`) || error.startsWith(`${path}.`)) : []
    if (scoped.length) {
      rollbackCapabilitiesDb(CAPABILITIES_DB, key, preWriteSnapshot)
      rollbackArchitectureDb(ARCHITECTURE_DB, archLinkSnapshot)
      // re-sync the JSONL export to the now-rolled-back DB pair so the refused promotion leaves
      // no trace in the git-tracked canonical shards either.
      if (existsSync(CAPABILITIES_DB)) exportCanonicalCapabilityShards({ root, expectedChanges: { [key]: authorizedFields } })
      console.error(`REFUSED: promotion of ${key} rolled back -- caps:canonical:verify found new violation(s) in its own canonical-shard record:`)
      for (const error of scoped) console.error(`  ${error}`)
      process.exit(1)
    }
  }
}

// ── ARCHITECTURE-SIDE POST-WRITE VERIFY GUARD ──────────────────────────────────────────────────
// Mirrors the capabilities-side guard above, for the architecture-canonical surface: the
// capability_architecture_link sync above makes architecture.db self-consistent, but nothing
// regenerated docs/architecture-canonical/links.jsonl+meta.json from it, or ran the same check
// `arch:canonical:verify` uses -- so a promotion could pass the capabilities-side guard while
// leaving links.jsonl reporting stale or structurally-broken architecture linkage (e.g. a link
// pointing at an architecture_node id that was never recorded, which the capabilities-side guard
// cannot see since it only reads docs/capabilities-canonical/**). Close the gap the same way:
// re-export from the now-updated architecture.db and run verifyCanonicalArchitectureShards,
// scoped to just this capability's own link records so an unrelated pre-existing violation
// elsewhere in links.jsonl never blocks this promotion. A NEW violation on THIS capability's own
// links fails the promotion (non-zero exit) and rolls BOTH DB writes back, same as above.
if (archLinkTablePresent) {
  exportCanonicalArchitectureShards({ root })
  const archVerify = verifyCanonicalArchitectureShards({ root })
  if (!archVerify.ok) {
    const { entries } = loadArchitectureShards(root)
    const linkPaths = entries
      .filter((entry) => entry.record?.record_type === 'capability_architecture_link' && entry.record.capability_key === key)
      .map((entry) => `${rel(root, entry.file)}:${entry.line}`)
    const scoped = archVerify.errors.filter((error) =>
      linkPaths.some((path) => error.startsWith(`${path}:`) || error.startsWith(`${path}.`))
      || error === `capability ${key} is both linked and gapped`,
    )
    if (scoped.length) {
      rollbackCapabilitiesDb(CAPABILITIES_DB, key, preWriteSnapshot)
      rollbackArchitectureDb(ARCHITECTURE_DB, archLinkSnapshot)
      // re-sync BOTH exports to the now-rolled-back DB pair so the refused promotion leaves no
      // trace in either git-tracked canonical shard tree.
      if (existsSync(CAPABILITIES_DB)) exportCanonicalCapabilityShards({ root, expectedChanges: { [key]: authorizedFields } })
      if (existsSync(ARCHITECTURE_DB)) exportCanonicalArchitectureShards({ root })
      console.error(`REFUSED: promotion of ${key} rolled back -- arch:canonical:verify found new violation(s) in its own canonical-shard record:`)
      for (const error of scoped) console.error(`  ${error}`)
      process.exit(1)
    }
  }
}

// ── DB-REBUILD AUTOMATION (issue #2688 root-cause fix) ──────────────────────────────────────────
// The two guards above (caps:canonical:*, arch:canonical:*) are cheap, git-tracked JSONL exports
// safe to run in-process. These next two are NOT: export-cloud-snapshot.mjs runs its whole export
// at module-load time and calls process.exit() directly (importing it would kill this process),
// and subdb:gen/verify do a full Cargo-workspace scan and rewrite files scattered across every
// crate directory -- both are real, separate CLI programs, not internal library calls. This is
// the actual gap that produced the propagation-drift bug hand-patched three times this session
// (PR #2691: 3 capabilities, PR #2694: 2 more): caps:cloud-export and subdb:gen never ran after a
// promotion, so docs/capabilities-cloud/*.db and the promoted capability's own crate-local
// .chronica/sub-cap-arch.jsonl kept reporting the pre-promotion status even though every
// in-process-checked surface was already correct. Shell out to both and FAIL LOUDLY (non-zero
// exit, printing the real command plus its stderr) instead of only printing a reminder that's
// easy to skip. The promotion's DB writes already passed both invariant guards above, so a
// rebuild failure here does NOT roll them back -- only the generated cache artifacts are stale
// until the operator fixes the failure and re-runs the printed command.
const followUpCrate = cap?.target_crate ? String(cap.target_crate).split(',')[0].trim() : null

function runRequiredStep(label, args) {
  try {
    execFileSync(process.execPath, args, { cwd: root, encoding: 'utf8' })
  } catch (error) {
    console.error(`REFUSED: required follow-up step '${label}' failed for ${key} -- the promotion itself already landed and passed its own canonical-shard invariant checks, but this rebuild step did not complete, so its generated artifacts are now STALE until you fix and re-run:`)
    console.error(`  node ${args.join(' ')}`)
    if (error.stdout) console.error(String(error.stdout).trim())
    if (error.stderr) console.error(String(error.stderr).trim())
    process.exit(1)
  }
}

if (followUpCrate) {
  runRequiredStep('caps:cloud-export', [join(here, 'export-cloud-snapshot.mjs')])
  runRequiredStep('subdb:gen', [join(here, '..', 'subdb', 'gen-crate-subdb.mjs'), '--crate', followUpCrate])
  runRequiredStep('subdb:verify', [join(here, '..', 'subdb', 'verify-crate-subdb.mjs'), '--crate', followUpCrate])
  console.log(`  DB rebuild: caps:cloud-export + subdb:gen/verify --crate ${followUpCrate} completed`)
} else {
  // no canonical target_crate to scope a rebuild to (config-only cap, or canonical_capability
  // unavailable) -- caps:cloud-export/subdb:gen cannot be run safely/usefully here, so this stays
  // a manual reminder, same as before this fix.
  console.log('NEXT STEPS to fully propagate this promotion (target_crate unknown -- caps:cloud-export/subdb:gen could not be scoped and run automatically):')
  console.log('  pnpm caps:cloud-export && pnpm subdb:gen')
}

function rollbackCapabilitiesDb(dbPath, capabilityKey, snapshot) {
  const rollbackDb = new Database(dbPath)
  try {
    rollbackDb.transaction(() => {
      if (snapshot.canonicalCapabilityRow) {
        const row = snapshot.canonicalCapabilityRow
        rollbackDb.prepare('UPDATE canonical_capability SET status=?, acceptance_test=?, financial_control_test=?, moves_money=?, side_effect_class=? WHERE key=?')
          .run(row.status, row.acceptance_test, row.financial_control_test, row.moves_money, row.side_effect_class, capabilityKey)
      }

      if (snapshot.overrideRow) {
        const override = snapshot.overrideRow
        rollbackDb.prepare('UPDATE canonical_status_override SET status=?, acceptance_test=?, financial_control_test=?, moves_money=?, set_at=?, set_by=? WHERE canonical_key=?')
          .run(override.status, override.acceptance_test, override.financial_control_test, override.moves_money, override.set_at, override.set_by, capabilityKey)
      } else {
        rollbackDb.prepare('DELETE FROM canonical_status_override WHERE canonical_key=?').run(capabilityKey)
      }

      if (snapshot.implEvidenceRow) {
        const evidence = snapshot.implEvidenceRow
        rollbackDb.prepare(`UPDATE impl_evidence SET impl_file=?, impl_symbols=?, test_file=?, test_symbol=?, donor_source=?, verified_at=?, verified_by=?, notes=? WHERE canonical_key=?`)
          .run(evidence.impl_file, evidence.impl_symbols, evidence.test_file, evidence.test_symbol, evidence.donor_source, evidence.verified_at, evidence.verified_by, evidence.notes, capabilityKey)
      } else {
        rollbackDb.prepare('DELETE FROM impl_evidence WHERE canonical_key=?').run(capabilityKey)
      }
    })()
  } finally {
    rollbackDb.close()
  }
}

function rollbackArchitectureDb(dbPath, linkSnapshot) {
  if (!existsSync(dbPath) || linkSnapshot.length === 0) return
  const rollbackDb = new Database(dbPath)
  try {
    rollbackDb.transaction(() => {
      for (const row of linkSnapshot) {
        rollbackDb.prepare('UPDATE capability_architecture_link SET status=? WHERE rowid=?').run(row.status, row.rowid)
      }
    })()
  } finally {
    rollbackDb.close()
  }
}
