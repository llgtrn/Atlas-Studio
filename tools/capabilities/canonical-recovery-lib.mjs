// canonical-recovery-lib.mjs — pure, DB-I/O-free helpers for recover-canonical.mjs.
//
// Kept side-effect-free on purpose: merging/validating recovery rows is exercised by unit tests
// with plain JS arrays (no SQLite file needed), while recover-canonical.mjs owns all the file I/O,
// backup, and transaction/rollback machinery. This split is what makes the malformed-snapshot and
// duplicate-key failure paths cheap to test exhaustively.
//
// The canonical_capability shape mirrors census-schema.mjs's DDL exactly (kept in sync by hand;
// this file must never `import` census-schema.mjs, since that module opens/creates
// docs/capabilities.db as a side effect of being imported — recover-canonical.mjs must only ever
// touch an EXISTING db file it was explicitly pointed at, never the real root DB by accident).
export const CANONICAL_CAPABILITY_DDL = `CREATE TABLE IF NOT EXISTS canonical_capability (
  id              INTEGER PRIMARY KEY,
  key             TEXT UNIQUE NOT NULL,
  canonical_name  TEXT NOT NULL,
  domain          TEXT,
  target_crate    TEXT,
  target_module   TEXT,
  side_effect_class TEXT,
  moves_money     INTEGER DEFAULT 0,
  requires_approval INTEGER DEFAULT 0,
  acceptance_criteria TEXT,
  required_tests  TEXT,
  financial_control_test TEXT,
  acceptance_test TEXT,
  status          TEXT DEFAULT 'unimplemented',
  exclusion_note  TEXT,
  blocker         TEXT,
  slice           INTEGER,
  donor_count     INTEGER DEFAULT 0
)`

export const REQUIRED_CLOUD_CORE_COLUMNS = [
  'id', 'key', 'canonical_name', 'domain', 'target_crate', 'target_module',
  'moves_money', 'requires_approval', 'status',
]

export const REQUIRED_ARCH_LINK_COLUMNS = ['capability_key', 'target_crate', 'target_module', 'status']

const STATUS_RANK = { verified: 3, unimplemented: 2, excluded: 1 }
const MONEY_DOMAIN = new Set(['erp', 'commerce', 'finance', 'billing', 'payments', 'accounting', 'trading-markets'])
const MONEY_CRATE = /payment|account|billing|erp|ledger|money|finance|invoice|payout|settle/i
const moneyish = (domain, crate) => MONEY_DOMAIN.has(domain) || (crate ? MONEY_CRATE.test(crate) : false)
const humanize = (key) => {
  const suffix = key.includes('.') ? key.slice(key.indexOf('.') + 1) : key
  const s = suffix.replace(/_/g, ' ').trim()
  return s.charAt(0).toUpperCase() + s.slice(1)
}

// Finds keys that appear more than once in a row set. A well-formed source can never produce this
// (canonical_capability.key is UNIQUE, capability_architecture_link is collapsed by key below) —
// surfacing it here catches a genuinely malformed/hand-crafted snapshot (e.g. a fixture DB built
// without the UNIQUE constraint, or a hostile/corrupted file) before any row reaches the live DB.
export function findDuplicateKeys(rows) {
  const seen = new Set()
  const dups = new Set()
  for (const r of rows) {
    const key = r.key ?? r.capability_key
    if (!key) continue
    if (seen.has(key)) dups.add(key)
    seen.add(key)
  }
  return [...dups].sort()
}

export function findDuplicateIds(rows) {
  const seen = new Set()
  const dups = new Set()
  for (const row of rows) {
    if (!Number.isSafeInteger(row.id) || row.id <= 0) continue
    if (seen.has(row.id)) dups.add(row.id)
    seen.add(row.id)
  }
  return [...dups].sort((a, b) => a - b)
}

export function missingColumns(columns, required) {
  const present = new Set(columns)
  return required.filter((c) => !present.has(c))
}

// cap-core.db's canonical_capability rows already carry the full target schema — pass them
// through with explicit defaults so every recovered row has every column recover-canonical.mjs's
// INSERT expects, regardless of which optional columns a given snapshot happened to populate.
export function rowsFromCloudCore(cloudCoreRows) {
  return cloudCoreRows.map((r) => ({
    id: r.id,
    key: r.key,
    canonical_name: r.canonical_name || r.key,
    domain: r.domain ?? null,
    target_crate: r.target_crate ?? null,
    target_module: r.target_module ?? null,
    side_effect_class: r.side_effect_class ?? null,
    moves_money: r.moves_money ? 1 : 0,
    requires_approval: r.requires_approval ? 1 : 0,
    acceptance_criteria: r.acceptance_criteria ?? null,
    required_tests: r.required_tests ?? null,
    financial_control_test: r.financial_control_test ?? null,
    acceptance_test: r.acceptance_test ?? null,
    status: r.status || 'unimplemented',
    exclusion_note: r.exclusion_note ?? null,
    blocker: r.blocker ?? null,
    slice: r.slice ?? null,
    donor_count: r.donor_count ?? 0,
    recovery_source: 'docs/capabilities-cloud/cap-core.db',
  }))
}

// architecture.db's capability_architecture_link only carries key/target/status — this is a
// backfill source for keys cap-core.db does not know about, never authoritative over cap-core.db.
// Mirrors restore-canonical-from-arch.mjs's collapse-by-key + conservative money-flagging
// (under-flagging money is dangerous; over-flagging only costs an extra financial-control-test
// requirement before a cap can be marked verified).
export function rowsFromArchitectureLinks(archLinkRows) {
  const byKey = new Map()
  for (const r of archLinkRows) {
    if (!r.capability_key) continue
    const cur = byKey.get(r.capability_key)
    if (!cur) { byKey.set(r.capability_key, r); continue }
    if ((STATUS_RANK[r.status] || 0) > (STATUS_RANK[cur.status] || 0)) {
      byKey.set(r.capability_key, { ...r, target_crate: r.target_crate || cur.target_crate, target_module: r.target_module || cur.target_module })
    } else {
      cur.target_crate ||= r.target_crate
      cur.target_module ||= r.target_module
    }
  }
  const out = []
  for (const [key, r] of byKey) {
    const domain = key.includes('.') ? key.split('.')[0] : 'unclustered'
    const isMoney = moneyish(domain, r.target_crate) ? 1 : 0
    out.push({
      key,
      canonical_name: humanize(key),
      domain,
      target_crate: r.target_crate || null,
      target_module: r.target_module || null,
      side_effect_class: isMoney ? 'money' : 'internal_write',
      moves_money: isMoney,
      requires_approval: 0,
      acceptance_criteria: 'recovered from architecture.db (evidence pending re-extraction)',
      required_tests: null,
      financial_control_test: isMoney ? 'REQUIRED: policy->approval->CostRecord->SHA256 audit; no external charge when the gate denies' : null,
      acceptance_test: null,
      status: r.status === 'verified' ? 'unimplemented' : (r.status || 'unimplemented'), // never claim verified without present evidence
      exclusion_note: null,
      blocker: null,
      slice: null,
      donor_count: 0,
      recovery_source: 'docs/architecture.db',
    })
  }
  return out
}

// Merges the two sources. FAILS CLOSED (returns rows: [], with the problem in `issues`) on ANY
// malformed-snapshot or duplicate-key signal — a recovery tool must never guess its way past
// corrupt input by silently dropping or overwriting rows.
export function mergeRecoverySources({ cloudCoreRows = [], archLinkRows = [] } = {}) {
  const issues = []

  const cloudDupKeys = findDuplicateKeys(cloudCoreRows)
  if (cloudDupKeys.length) {
    issues.push({ type: 'MALFORMED_SNAPSHOT_DUPLICATE_KEY', source: 'docs/capabilities-cloud/cap-core.db', keys: cloudDupKeys })
  }
  const cloudDupIds = findDuplicateIds(cloudCoreRows)
  if (cloudDupIds.length) {
    issues.push({ type: 'MALFORMED_SNAPSHOT_DUPLICATE_ID', source: 'docs/capabilities-cloud/cap-core.db', ids: cloudDupIds })
  }
  for (const r of cloudCoreRows) {
    if (!r.key || typeof r.key !== 'string' || !r.key.trim()) {
      issues.push({ type: 'MALFORMED_SNAPSHOT_EMPTY_KEY', source: 'docs/capabilities-cloud/cap-core.db', row: r })
    }
    if (!Number.isSafeInteger(r.id) || r.id <= 0) {
      issues.push({ type: 'MALFORMED_SNAPSHOT_INVALID_ID', source: 'docs/capabilities-cloud/cap-core.db', row: r })
    }
  }
  if (issues.length) return { rows: [], issues }

  const primary = rowsFromCloudCore(cloudCoreRows)
  const primaryKeys = new Set(primary.map((r) => r.key))
  const maxCloudId = primary.reduce((max, row) => Math.max(max, row.id), 0)
  const secondary = rowsFromArchitectureLinks(archLinkRows)
    .filter((r) => !primaryKeys.has(r.key))
    .sort((a, b) => a.key.localeCompare(b.key))
    .map((row, index) => ({ ...row, id: maxCloudId + index + 1 }))
  return { rows: [...primary, ...secondary], issues: [] }
}

// Rows synthesized from architecture.db alone (the backfill path, never authoritative) must never
// claim 'verified' — rowsFromArchitectureLinks() already downgrades verified->unimplemented, so
// this only fires if that invariant regresses. cap-core.db-sourced rows are exempt: a 'verified'
// status there is PRE-EXISTING recorded truth from before the local canonical table was lost, not
// a claim recovery invents.
export function assertNoInventedVerified(rows) {
  const offenders = rows.filter((r) => r.recovery_source === 'docs/architecture.db' && r.status === 'verified').map((r) => r.key)
  if (offenders.length) {
    throw new Error(`recovery would insert 'verified' status for ${offenders.length} architecture.db-backfilled row(s) with no live evidence: ${offenders.slice(0, 10).join(', ')}`)
  }
}
