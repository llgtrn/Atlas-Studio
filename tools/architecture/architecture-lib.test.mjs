import test from 'node:test'
import assert from 'node:assert/strict'
import { mkdtempSync, mkdirSync, writeFileSync } from 'node:fs'
import { join } from 'node:path'
import { tmpdir } from 'node:os'
import Database from 'better-sqlite3'
import {
  buildArchitectureDb,
  classifyCapabilityCoverage,
  discoverWorkspaceArchitecture,
  gapStatusFor,
  verifyArchitectureDb,
} from './architecture-lib.mjs'

function makeMiniRepo() {
  const root = mkdtempSync(join(tmpdir(), 'chronica-arch-'))
  mkdirSync(join(root, 'crates', 'chronica-core'), { recursive: true })
  mkdirSync(join(root, 'crates', 'chronica-engine'), { recursive: true })
  mkdirSync(join(root, 'crates', 'chronica-internet-hand'), { recursive: true })
  mkdirSync(join(root, 'crates', 'chronica-policy'), { recursive: true })
  mkdirSync(join(root, 'crates', 'chronica-runtime'), { recursive: true })
  mkdirSync(join(root, 'crates', 'chronica-strategy', 'src', 'cognition'), { recursive: true })
  mkdirSync(join(root, 'docs'), { recursive: true })

  writeFileSync(join(root, 'Cargo.toml'), `[workspace]
members = [
  "crates/chronica-core",
  "crates/chronica-engine",
  "crates/chronica-internet-hand",
  "crates/chronica-policy",
  "crates/chronica-runtime",
  "crates/chronica-strategy",
]
`)
  writeFileSync(join(root, 'crates', 'chronica-core', 'Cargo.toml'), `[package]
name = "chronica-core"
`)
  writeFileSync(join(root, 'crates', 'chronica-policy', 'Cargo.toml'), `[package]
name = "chronica-policy"

[dependencies]
chronica-core = { workspace = true }
`)
  writeFileSync(join(root, 'crates', 'chronica-engine', 'Cargo.toml'), `[package]
name = "chronica-engine"

[dependencies]
chronica-core = { workspace = true }
chronica-policy = { workspace = true }
`)
  writeFileSync(join(root, 'crates', 'chronica-internet-hand', 'Cargo.toml'), `[package]
name = "chronica-internet-hand"

[dependencies]
chronica-core = { workspace = true }
`)
  writeFileSync(join(root, 'crates', 'chronica-runtime', 'Cargo.toml'), `[package]
name = "chronica-runtime"

[dependencies]
chronica-core = { workspace = true }
chronica-policy = { workspace = true }
`)
  writeFileSync(join(root, 'crates', 'chronica-strategy', 'Cargo.toml'), `[package]
name = "chronica-strategy"
`)
  writeFileSync(join(root, 'crates', 'chronica-strategy', 'src', 'lib.rs'), `pub mod cognition;
`)
  writeFileSync(join(root, 'crates', 'chronica-strategy', 'src', 'cognition', 'mod.rs'), `pub fn advisory_only() -> bool { true }
`)

  const caps = new Database(join(root, 'docs', 'capabilities.db'))
  caps.exec(`CREATE TABLE canonical_capability (
    key TEXT PRIMARY KEY,
    canonical_name TEXT,
    target_crate TEXT,
    target_module TEXT,
    status TEXT
  )`)
  const insertCap = caps.prepare(`INSERT INTO canonical_capability
    (key, canonical_name, target_crate, target_module, status) VALUES (?, ?, ?, ?, ?)`)
  insertCap.run('runtime.invoke_tool', 'Invoke gated runtime tool', 'chronica-runtime', 'invoke_tool', 'verified') // live crate
  insertCap.run('runtime.batch', 'Batch grouping', 'chronica-runtime', null, 'unimplemented') // live crate
  insertCap.run('gov.approvals', 'Governance approvals', 'chronica-governance', null, 'verified') // logical -> live chronica-policy
  insertCap.run('multi.flow', 'Multi-crate flow', 'chronica-core, chronica-policy', null, 'verified') // multi -> primary chronica-core
  insertCap.run('legacy.py', 'Legacy python capability', 'N/A (pure Python)', null, 'unimplemented') // legacy/non-rust
  insertCap.run('missing.meta', 'No target crate', null, null, 'unimplemented') // missing metadata
  insertCap.run('planned.platform', 'Platform domain', 'chronica-platform', null, 'unimplemented') // planned logical domain
  insertCap.run('planned.browser', 'Browser control domain', 'chronica-browser-launch', null, 'unimplemented') // planned logical domain with owner
  caps.close()

  return root
}

test('discovers crates, dependency edges, invariants, and capability architecture links', () => {
  const root = makeMiniRepo()
  const arch = discoverWorkspaceArchitecture(root)

  assert.equal(arch.nodes.some((n) => n.kind === 'crate' && n.name === 'chronica-runtime'), true)
  assert.equal(
    arch.edges.some((e) => e.fromNode === 'crate:chronica-runtime' && e.toNode === 'crate:chronica-policy' && e.kind === 'depends_on'),
    true,
  )
  assert.equal(arch.invariants.some((i) => i.key === 'one_money_gate'), true)
  assert.equal(arch.invariants.some((i) => i.key === 'cognition_tighten_only'), true)
  assert.equal(arch.authorityRules.some((r) => r.actor === 'CompanyCeo'), true)
  assert.equal(arch.authorityRules.some((r) => r.actor === 'ProjectOwner'), true)
  assert.equal(arch.nodes.some((n) => n.id === 'runtime_entity:ProjectOwner'), true)

  const dbPath = join(root, 'docs', 'architecture.db')
  buildArchitectureDb({ root, dbPath })

  const db = new Database(dbPath, { readonly: true })
  assert.equal(db.prepare(`SELECT count(*) n FROM architecture_node WHERE id='crate:chronica-runtime'`).get().n, 1)
  assert.equal(db.prepare(`SELECT count(*) n FROM architecture_edge WHERE kind='depends_on'`).get().n, 6)
  assert.equal(db.prepare(`SELECT count(*) n FROM capability_architecture_link WHERE capability_key='runtime.invoke_tool'`).get().n, 1)
  assert.equal(db.prepare(`SELECT count(*) n FROM architecture_evidence WHERE architecture_node='crate:chronica-runtime' AND status='implemented'`).get().n, 1)
  assert.equal(db.prepare(`SELECT count(*) n FROM architecture_evidence WHERE architecture_node='crate:chronica-runtime' AND status='verified'`).get().n, 1)
  assert.equal(db.prepare(`SELECT count(*) n FROM architecture_status_override`).get().n, 0)
  db.close()

  const report = verifyArchitectureDb({ root, dbPath })
  assert.equal(report.ok, true)
  assert.equal(report.counts.duplicateMoneyGates, 0)
  assert.equal(report.counts.evidenceRows > 0, true)
})

test('money gate architecture evidence points at the live invariant doc', () => {
  const root = makeMiniRepo()
  const dbPath = join(root, 'docs', 'architecture.db')
  buildArchitectureDb({ root, dbPath })

  const db = new Database(dbPath, { readonly: true })
  const moneyGate = db.prepare("SELECT evidence_ref FROM architecture_node WHERE id='gate:money'").get()
  assert.equal(moneyGate.evidence_ref, 'docs/010-invariant-one-money-gate.md')

  const runtimeAgent = db.prepare("SELECT crate, evidence_ref FROM architecture_node WHERE id='runtime_entity:Agent'").get()
  assert.deepEqual(runtimeAgent, {
    crate: 'chronica-ai-workforce',
    evidence_ref: 'crates/chronica-ai-workforce',
  })

  const staleRefs = db.prepare(`
    SELECT evidence_ref FROM architecture_edge
    WHERE (from_node='gate:money' OR to_node='gate:money')
      AND evidence_ref IN ('docs/006-money-safety-approval-gate.md', 'docs/10-invariant-one-money-gate.md')
  `).all()
  assert.deepEqual(staleRefs, [])
  db.close()
})

test('accounts every capability in exactly one honest coverage state', () => {
  const root = makeMiniRepo()
  const dbPath = join(root, 'docs', 'architecture.db')
  const { coverage } = buildArchitectureDb({ root, dbPath })

  // Every canonical capability lands in exactly one coverage bucket — linked XOR gapped.
  assert.equal(coverage.canonical, 8)
  assert.equal(coverage.accounted, 8)
  assert.equal(coverage.distinctLinked, 4) // invoke_tool, batch, gov.approvals, multi.flow
  assert.equal(coverage.distinctGap, 4) // legacy.py, missing.meta, planned.platform, planned.browser
  assert.equal(coverage.distinctLinked + coverage.distinctGap, coverage.canonical)

  const db = new Database(dbPath, { readonly: true })
  const meta = (k) => Number(db.prepare('SELECT v FROM meta WHERE k=?').get(k)?.v)
  assert.equal(meta('canonical_capability_count'), 8)
  assert.equal(meta('accounted_capabilities'), 8)
  assert.equal(meta('unaccounted_capabilities'), 0)
  assert.equal(meta('coverage_percent'), 100)

  const links = (key) =>
    db.prepare('SELECT relationship, architecture_node FROM capability_architecture_link WHERE capability_key=? ORDER BY relationship').all(key)
  const gapKind = (key) => db.prepare('SELECT gap_kind FROM architecture_target_gap WHERE capability_key=?').get(key)?.gap_kind

  // live crate -> linked via 'targets' to the real crate node
  assert.deepEqual(links('runtime.batch'), [{ relationship: 'targets', architecture_node: 'crate:chronica-runtime' }])
  // logical crate homed in a live crate -> 'mapped_to' that live crate, and NEVER a gap
  assert.deepEqual(links('gov.approvals'), [{ relationship: 'mapped_to', architecture_node: 'crate:chronica-policy' }])
  assert.equal(gapKind('gov.approvals'), undefined)
  // multi-crate target -> first live crate is the deterministic primary owner, linked via 'targets'
  assert.deepEqual(links('multi.flow'), [{ relationship: 'targets', architecture_node: 'crate:chronica-core' }])

  // honest gaps — classified, never linked, never claimed implemented
  assert.equal(gapKind('legacy.py'), 'legacy_or_non_rust_target')
  assert.equal(gapKind('missing.meta'), 'missing_target_metadata')
  assert.equal(gapKind('planned.platform'), 'planned_logical_domain')
  assert.equal(links('legacy.py').length, 0)
  assert.equal(links('planned.platform').length, 0)

  // gap rows carry an explicit deterministic label: deferred (non-actionable here) vs open
  // (actionable metadata bug). Labels are derived at build time — they survive the rebuild wipe.
  const gapStatus = (key) => db.prepare('SELECT status FROM architecture_target_gap WHERE capability_key=?').get(key)?.status
  assert.equal(gapStatus('legacy.py'), 'deferred:python_bound')
  assert.equal(gapStatus('planned.platform'), 'deferred:planned_unbuilt')
  assert.equal(gapStatus('planned.browser'), 'deferred:planned_unbuilt')
  assert.equal(gapStatus('missing.meta'), 'open')

  // a planned domain becomes a planned_architecture_node — NOT a fake crate node in architecture_node.
  // It may carry an intended live owner crate, but that is an owner for backlog execution,
  // not an implementation claim for the capability.
  const planned = db.prepare('SELECT * FROM planned_architecture_node WHERE id=?').get('planned:chronica-platform')
  assert.equal(planned.kind, 'logical_domain')
  assert.equal(planned.logical_domain, 'chronica-platform') // the domain is recorded
  assert.equal(planned.intended_owner_crate, 'chronica-engine')
  assert.equal(planned.status, 'planned')
  assert.equal(db.prepare("SELECT count(*) n FROM architecture_node WHERE id='crate:chronica-platform'").get().n, 0)
  assert.deepEqual(links('planned.platform'), []) // intended owner does NOT create a capability link

  const browserPlan = db.prepare('SELECT * FROM planned_architecture_node WHERE id=?').get('planned:chronica-browser-launch')
  assert.equal(browserPlan.intended_owner_crate, 'chronica-internet-hand')
  assert.deepEqual(links('planned.browser'), [])

  // no gap is ever marked implemented/verified, and no capability is both linked and gapped
  assert.equal(db.prepare("SELECT count(*) n FROM architecture_target_gap WHERE status IN ('implemented','verified')").get().n, 0)
  assert.equal(
    db.prepare(`SELECT count(*) n FROM (
      SELECT capability_key FROM capability_architecture_link
      INTERSECT SELECT capability_key FROM architecture_target_gap)`).get().n,
    0,
  )
  db.close()

  const report = verifyArchitectureDb({ root, dbPath })
  assert.equal(report.ok, true, `verify errors: ${report.errors.join('; ')}`)
  assert.equal(report.counts.canonicalCapabilities, 8)
  assert.equal(report.counts.accountedCapabilities, 8)
  assert.equal(report.counts.unaccountedCapabilities, 0)
})

test('accounts capabilities from the cloud core shard when the local canonical table is absent', () => {
  const root = makeMiniRepo()
  const local = new Database(join(root, 'docs', 'capabilities.db'))
  local.exec('DROP TABLE canonical_capability')
  local.close()

  mkdirSync(join(root, 'docs', 'capabilities-cloud'), { recursive: true })
  const core = new Database(join(root, 'docs', 'capabilities-cloud', 'cap-core.db'))
  core.exec(`CREATE TABLE canonical_capability (
    key TEXT PRIMARY KEY,
    canonical_name TEXT,
    target_crate TEXT,
    target_module TEXT,
    status TEXT,
    moves_money INTEGER DEFAULT 0,
    requires_approval INTEGER DEFAULT 0
  )`)
  core
    .prepare(`INSERT INTO canonical_capability
      (key, canonical_name, target_crate, target_module, status, moves_money, requires_approval)
      VALUES (?, ?, ?, ?, ?, ?, ?)`)
    .run('runtime.cloud_fallback', 'Runtime cloud fallback', 'chronica-runtime', null, 'unimplemented', 0, 0)
  core.close()

  const dbPath = join(root, 'docs', 'architecture.db')
  const { coverage } = buildArchitectureDb({ root, dbPath })

  assert.equal(coverage.canonical, 1)
  assert.equal(coverage.accounted, 1)
  assert.equal(coverage.distinctLinked, 1)
  assert.equal(coverage.distinctGap, 0)

  const report = verifyArchitectureDb({ root, dbPath })
  assert.equal(report.ok, true, `verify errors: ${report.errors.join('; ')}`)
  assert.equal(report.counts.canonicalCapabilities, 1)
  assert.equal(report.counts.accountedCapabilities, 1)
  assert.equal(report.counts.unaccountedCapabilities, 0)
})

test('classifyCapabilityCoverage assigns deterministic honest states', () => {
  const live = new Set(['chronica-core', 'chronica-policy', 'chronica-erp'])
  // exact live workspace crate
  assert.equal(classifyCapabilityCoverage('chronica-core', live).state, 'linked_to_live_crate')
  // logical domain with a code-verified live home -> mapped to that live crate
  const mapped = classifyCapabilityCoverage('chronica-accounting', live)
  assert.equal(mapped.state, 'mapped_to_existing_architecture_node')
  assert.equal(mapped.suggestedNode, 'crate:chronica-erp')
  // non-Rust / legacy donor target
  assert.equal(classifyCapabilityCoverage('N/A (pure Python)', live).state, 'legacy_or_non_rust_target')
  // missing metadata
  assert.equal(classifyCapabilityCoverage('', live).state, 'missing_target_metadata')
  assert.equal(classifyCapabilityCoverage(null, live).state, 'missing_target_metadata')
  // coherent logical domain with NO live home -> planned, never a fabricated mapping
  assert.equal(classifyCapabilityCoverage('chronica-platform', live).state, 'planned_logical_domain')
  // a logical name whose home crate is NOT live downgrades to planned, never fake-mapped
  assert.equal(classifyCapabilityCoverage('chronica-accounting', new Set(['chronica-core'])).state, 'planned_logical_domain')
  // multi-crate -> first live crate is the deterministic primary owner
  const multi = classifyCapabilityCoverage('chronica-core, chronica-policy', live)
  assert.equal(multi.state, 'linked_to_live_crate')
  assert.equal(multi.suggestedNode, 'crate:chronica-core')

  // regression guard for the under-coverage fix: domains whose BUILT caps have code-verified
  // homes must map to the owning live crate, never fall through to a planned/unbuilt gap.
  const live2 = new Set(['chronica-erp', 'chronica-workflows', 'chronica-traces'])
  assert.equal(classifyCapabilityCoverage('chronica-finance', live2).suggestedNode, 'crate:chronica-erp')
  assert.equal(classifyCapabilityCoverage('chronica-algo', live2).suggestedNode, 'crate:chronica-erp')
  assert.equal(classifyCapabilityCoverage('chronica-operations', live2).suggestedNode, 'crate:chronica-workflows')
  assert.equal(classifyCapabilityCoverage('chronica-tempo-backend', live2).suggestedNode, 'crate:chronica-traces')
  assert.equal(classifyCapabilityCoverage('chronica-tempo-generator', live2).suggestedNode, 'crate:chronica-traces')

  // gap-closure round: code-verified logical homes map (never gap); unbuilt tempo domains stay planned
  const live3 = new Set(['chronica-olap', 'chronica-integrations', 'chronica-traces'])
  const storageEngine = classifyCapabilityCoverage('chronica-storage-engine', live3)
  assert.equal(storageEngine.state, 'mapped_to_existing_architecture_node')
  assert.equal(storageEngine.suggestedNode, 'crate:chronica-olap') // src/mergetree/ owns MergeTree storage
  assert.equal(classifyCapabilityCoverage('chronica-integration', live3).suggestedNode, 'crate:chronica-integrations')
  assert.equal(classifyCapabilityCoverage('chronica-tempo-querier', live3).suggestedNode, 'crate:chronica-traces') // src/query.rs
  assert.equal(classifyCapabilityCoverage('chronica-tempo-pusher', live3).suggestedNode, 'crate:chronica-traces') // src/ingest.rs
  assert.equal(classifyCapabilityCoverage('chronica-tempo-frontend', live3).state, 'planned_logical_domain') // no owning module yet
})

test('gapStatusFor derives explicit deterministic deferral labels', () => {
  assert.equal(gapStatusFor('legacy_or_non_rust_target', 'N/A (pure Python)'), 'deferred:python_bound')
  assert.equal(gapStatusFor('legacy_or_non_rust_target', 'tradingagents'), 'deferred:python_bound')
  assert.equal(gapStatusFor('legacy_or_non_rust_target', 'chronica-python'), 'deferred:python_bound')
  assert.equal(gapStatusFor('legacy_or_non_rust_target', 'openfang-runtime,openfang-wire'), 'deferred:external_donor_crate')
  assert.equal(gapStatusFor('legacy_or_non_rust_target', 'alertmanager-dispatch'), 'deferred:external_donor_crate')
  assert.equal(gapStatusFor('legacy_or_non_rust_target', 'N/A'), 'deferred:non_rust_external')
  assert.equal(gapStatusFor('legacy_or_non_rust_target', 'Not applicable (Electron TypeScript)'), 'deferred:non_rust_external')
  assert.equal(gapStatusFor('planned_logical_domain', 'chronica-platform'), 'deferred:planned_unbuilt')
  assert.equal(gapStatusFor('missing_target_metadata', null), 'open')
  assert.equal(gapStatusFor('invalid_target', '???'), 'open')
})
