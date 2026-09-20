import Database from 'better-sqlite3'
import { existsSync, mkdirSync } from 'node:fs'
import { dirname, join } from 'node:path'
import { openReadOnlyDatabase } from '../db/sqlite-open.mjs'
import { findTestOnlyModuleFiles } from './cfg-test-module-classifier.mjs'
import { createSchema, resetGeneratedSchema } from './architecture-schema.mjs'
import {
  importCapabilityLinks,
  insertGeneratedArchitectureEvidence,
  openCanonicalCapabilityDb,
} from './capability-coverage.mjs'
import { countPattern, discoverWorkspaceArchitecture, listRustFiles } from './workspace-discovery.mjs'

export { findTestOnlyModuleFiles }
export { discoverWorkspaceArchitecture } from './workspace-discovery.mjs'
export { COVERAGE_STATES, classifyCapabilityCoverage, gapStatusFor } from './capability-coverage.mjs'

export function buildArchitectureDb({ root = process.cwd(), dbPath = join(root, 'docs', 'architecture.db') } = {}) {
  mkdirSync(dirname(dbPath), { recursive: true })
  const arch = discoverWorkspaceArchitecture(root)
  const db = new Database(dbPath)
  resetGeneratedSchema(db)
  createSchema(db)

  const insertMeta = db.prepare('INSERT INTO meta (k,v) VALUES (?,?)')
  insertMeta.run('schema_version', '1')
  insertMeta.run('generated_at', new Date().toISOString())
  insertMeta.run('generator', 'tools/architecture/build-db.mjs')

  const insertNode = db.prepare(`INSERT INTO architecture_node
    (id, kind, name, crate, module_path, file_path, status, evidence_ref)
    VALUES (?, ?, ?, ?, ?, ?, ?, ?)`)
  const insertEdge = db.prepare(`INSERT INTO architecture_edge
    (from_node, to_node, kind, evidence_ref)
    VALUES (?, ?, ?, ?)`)
  const insertInvariant = db.prepare(`INSERT INTO architecture_invariant
    (key, name, description, enforcing_node, verification_command, status)
    VALUES (?, ?, ?, ?, ?, ?)`)
  const insertAuthority = db.prepare(`INSERT INTO authority_rule
    (actor, scope, may_see, may_propose, may_approve, may_execute, may_audit, forbidden_actions, enforcing_node)
    VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)`)
  const insertFlow = db.prepare(`INSERT INTO data_flow
    (key, source_node, target_node, data_kind, scope_rule, gate_required, audit_required)
    VALUES (?, ?, ?, ?, ?, ?, ?)`)

  const tx = db.transaction(() => {
    for (const n of arch.nodes) insertNode.run(n.id, n.kind, n.name, n.crate, n.modulePath, n.filePath, n.status, n.evidenceRef)
    for (const e of arch.edges) insertEdge.run(e.fromNode, e.toNode, e.kind, e.evidenceRef)
    for (const i of arch.invariants) insertInvariant.run(i.key, i.name, i.description, i.enforcingNode, i.verificationCommand, i.status)
    for (const r of arch.authorityRules) insertAuthority.run(r.actor, r.scope, r.maySee, r.mayPropose, r.mayApprove, r.mayExecute, r.mayAudit, r.forbiddenActions, r.enforcingNode)
    for (const f of arch.dataFlows) insertFlow.run(f.key, f.sourceNode, f.targetNode, f.dataKind, f.scopeRule, f.gateRequired, f.auditRequired)
  })
  tx()
  const coverage = importCapabilityLinks(db, root)
  insertGeneratedArchitectureEvidence(db)
  insertMeta.run('architecture_node_count', String(db.prepare('SELECT count(*) n FROM architecture_node').get().n))
  insertMeta.run('architecture_edge_count', String(db.prepare('SELECT count(*) n FROM architecture_edge').get().n))
  insertMeta.run('capability_architecture_link_count', String(coverage.links))
  insertMeta.run('architecture_evidence_count', String(db.prepare('SELECT count(*) n FROM architecture_evidence').get().n))
  insertMeta.run('canonical_capability_count', String(coverage.canonical))
  insertMeta.run('canonical_capability_source', coverage.source)
  insertMeta.run('linked_distinct_capabilities', String(coverage.distinctLinked))
  insertMeta.run('gap_capabilities', String(coverage.distinctGap))
  insertMeta.run('accounted_capabilities', String(coverage.accounted))
  insertMeta.run('unaccounted_capabilities', String(Math.max(0, coverage.canonical - coverage.accounted)))
  insertMeta.run('planned_architecture_node_count', String(coverage.planned))
  insertMeta.run('coverage_percent', coverage.canonical ? ((coverage.accounted / coverage.canonical) * 100).toFixed(2) : '0')
  db.close()
  return { dbPath, ...arch, capabilityLinks: coverage.links, coverage }
}

export function verifyArchitectureDb({ root = process.cwd(), dbPath = join(root, 'docs', 'architecture.db') } = {}) {
  const errors = []
  if (!existsSync(dbPath)) {
    return { ok: false, errors: ['docs/architecture.db missing; run pnpm arch:build'], counts: {} }
  }
  const db = openReadOnlyDatabase(dbPath)
  const requiredTables = [
    'architecture_node',
    'architecture_edge',
    'architecture_invariant',
    'authority_rule',
    'data_flow',
    'capability_architecture_link',
    'architecture_evidence',
    'architecture_status_override',
  ]
  for (const table of requiredTables) {
    const exists = db.prepare("SELECT count(*) n FROM sqlite_master WHERE type='table' AND name=?").get(table).n
    if (!exists) errors.push(`missing table: ${table}`)
  }
  if (errors.length) {
    db.close()
    return { ok: false, errors, counts: {} }
  }

  const workspace = discoverWorkspaceArchitecture(root)
  const workspaceCrates = workspace.nodes.filter((n) => n.kind === 'crate')
  const dbCrates = db.prepare("SELECT id FROM architecture_node WHERE kind='crate'").all().map((r) => r.id)
  for (const crate of workspaceCrates) {
    if (!dbCrates.includes(crate.id)) errors.push(`missing crate node: ${crate.id}`)
  }

  const missingEdgeNodes = db.prepare(`SELECT e.from_node, e.to_node, e.kind
    FROM architecture_edge e
    LEFT JOIN architecture_node f ON f.id=e.from_node
    LEFT JOIN architecture_node t ON t.id=e.to_node
    WHERE f.id IS NULL OR t.id IS NULL`).all()
  for (const e of missingEdgeNodes) errors.push(`edge references missing node: ${e.from_node} -${e.kind}-> ${e.to_node}`)

  const missingInvariantNodes = db.prepare(`SELECT i.key, i.enforcing_node
    FROM architecture_invariant i
    LEFT JOIN architecture_node n ON n.id=i.enforcing_node
    WHERE n.id IS NULL`).all()
  for (const i of missingInvariantNodes) errors.push(`invariant ${i.key} enforces missing node ${i.enforcing_node}`)

  const missingEvidenceNodes = db.prepare(`SELECT e.architecture_node, e.status
    FROM architecture_evidence e
    LEFT JOIN architecture_node n ON n.id=e.architecture_node
    WHERE n.id IS NULL`).all()
  for (const e of missingEvidenceNodes) errors.push(`evidence references missing node: ${e.architecture_node} status=${e.status}`)

  const nodesWithoutImplementationEvidence = db.prepare(`SELECT n.id
    FROM architecture_node n
    LEFT JOIN architecture_evidence e ON e.architecture_node=n.id AND e.status IN ('implemented','verified')
    WHERE e.id IS NULL
    LIMIT 20`).all()
  for (const n of nodesWithoutImplementationEvidence) errors.push(`architecture node lacks implementation evidence: ${n.id}`)

  const rsFiles = listRustFiles(join(root, 'crates'))
  const moneyGateHits = countPattern(rsFiles, /evaluate_gate|ApprovalDecision|CostRecord|money gate/i)
  const invokeBypassHits = countPattern(rsFiles, /direct adapter bypass|bypass.*invoke_tool/i)
  const missingCoreInvariants = ['one_money_gate', 'one_merkle_audit', 'scopepath_isolation', 'council_money_authority']
    .filter((key) => db.prepare('SELECT count(*) n FROM architecture_invariant WHERE key=?').get(key).n === 0)
  for (const key of missingCoreInvariants) errors.push(`missing core invariant: ${key}`)

  // ── coverage accounting: every canonical capability must land in exactly one honest state ──
  const allowedGapKinds = "('planned_logical_domain','legacy_or_non_rust_target','missing_target_crate','invalid_target','missing_target_metadata')"
  for (const g of db.prepare(`SELECT DISTINCT gap_kind FROM architecture_target_gap WHERE gap_kind NOT IN ${allowedGapKinds}`).all()) {
    errors.push(`unknown gap_kind in architecture_target_gap: ${g.gap_kind}`)
  }
  const fakeImplementedGaps = db.prepare("SELECT count(*) n FROM architecture_target_gap WHERE status IN ('implemented','verified')").get().n
  if (fakeImplementedGaps > 0) {
    errors.push(`${fakeImplementedGaps} architecture_target_gap row(s) claim implemented/verified — a gap must never be read as implemented architecture`)
  }
  const mappedGapWithoutNode = db.prepare(`SELECT count(*) n FROM architecture_target_gap g
    WHERE COALESCE(g.suggested_architecture_node, '') <> ''
      AND NOT EXISTS (SELECT 1 FROM architecture_node n WHERE n.id=g.suggested_architecture_node)`).get().n
  if (mappedGapWithoutNode > 0) errors.push(`${mappedGapWithoutNode} gap row(s) suggest a non-existent architecture node`)
  const plannedOwnerWithoutCrate = db.prepare(`SELECT id, intended_owner_crate FROM planned_architecture_node p
    WHERE COALESCE(p.intended_owner_crate, '') <> ''
      AND NOT EXISTS (SELECT 1 FROM architecture_node n WHERE n.id='crate:' || p.intended_owner_crate)`).all()
  for (const p of plannedOwnerWithoutCrate) {
    errors.push(`planned node ${p.id} names missing owner crate ${p.intended_owner_crate}`)
  }
  const gapRowVsDistinct = db.prepare('SELECT count(*) rows, count(DISTINCT capability_key) caps FROM architecture_target_gap').get()
  if (gapRowVsDistinct.rows !== gapRowVsDistinct.caps) {
    errors.push(`architecture_target_gap has ${gapRowVsDistinct.rows} rows for ${gapRowVsDistinct.caps} distinct capabilities — each capability must have exactly one gap row`)
  }
  const linkMissingNode = db.prepare(`SELECT count(*) n FROM capability_architecture_link l
    LEFT JOIN architecture_node n ON n.id=l.architecture_node WHERE n.id IS NULL`).get().n
  if (linkMissingNode > 0) errors.push(`${linkMissingNode} capability_architecture_link row(s) reference a missing architecture node`)
  const bothLinkedAndGap = db.prepare(`SELECT count(*) n FROM (
    SELECT capability_key FROM capability_architecture_link
    INTERSECT SELECT capability_key FROM architecture_target_gap)`).get().n
  if (bothLinkedAndGap > 0) errors.push(`${bothLinkedAndGap} capability(ies) are BOTH linked and gapped — coverage state must be exactly one`)
  const accountedCapabilities = db.prepare(`SELECT count(*) n FROM (
    SELECT capability_key FROM capability_architecture_link
    UNION SELECT capability_key FROM architecture_target_gap)`).get().n
  let canonicalCapabilities = null
  let canonicalSource = 'unavailable'
  const canonical = openCanonicalCapabilityDb(root)
  if (canonical) {
    canonicalCapabilities = canonical.database.prepare('SELECT count(*) n FROM canonical_capability').get().n
    canonicalSource = canonical.source
    canonical.database.close()
  }
  if (canonicalCapabilities != null) {
    const unaccounted = canonicalCapabilities - accountedCapabilities
    if (unaccounted !== 0) {
      errors.push(`coverage gap: ${accountedCapabilities} accounted vs ${canonicalCapabilities} canonical capabilities (unaccounted=${unaccounted})`)
    }
    const metaCanonical = Number(db.prepare("SELECT v FROM meta WHERE k='canonical_capability_count'").get()?.v ?? -1)
    if (metaCanonical !== canonicalCapabilities) {
      errors.push(`meta canonical_capability_count=${metaCanonical} disagrees with ${canonicalSource}=${canonicalCapabilities}`)
    }
  }

  const counts = {
    nodes: db.prepare('SELECT count(*) n FROM architecture_node').get().n,
    canonicalCapabilitySource: canonicalSource,
    canonicalCapabilities,
    accountedCapabilities,
    unaccountedCapabilities: canonicalCapabilities == null ? null : canonicalCapabilities - accountedCapabilities,
    linkedDistinctCapabilities: db.prepare('SELECT count(DISTINCT capability_key) n FROM capability_architecture_link').get().n,
    gapCapabilities: db.prepare('SELECT count(DISTINCT capability_key) n FROM architecture_target_gap').get().n,
    plannedNodes: db.prepare('SELECT count(*) n FROM planned_architecture_node').get().n,
    crates: db.prepare("SELECT count(*) n FROM architecture_node WHERE kind='crate'").get().n,
    modules: db.prepare("SELECT count(*) n FROM architecture_node WHERE kind='module'").get().n,
    edges: db.prepare('SELECT count(*) n FROM architecture_edge').get().n,
    invariants: db.prepare('SELECT count(*) n FROM architecture_invariant').get().n,
    authorityRules: db.prepare('SELECT count(*) n FROM authority_rule').get().n,
    dataFlows: db.prepare('SELECT count(*) n FROM data_flow').get().n,
    capabilityLinks: db.prepare('SELECT count(*) n FROM capability_architecture_link').get().n,
    evidenceRows: db.prepare('SELECT count(*) n FROM architecture_evidence').get().n,
    verifiedEvidenceRows: db.prepare("SELECT count(*) n FROM architecture_evidence WHERE status='verified'").get().n,
    statusOverrides: db.prepare('SELECT count(*) n FROM architecture_status_override').get().n,
    duplicateMoneyGates: Math.max(0, db.prepare("SELECT count(*) n FROM architecture_node WHERE kind='gate' AND name LIKE '%Money%'").get().n - 1),
    moneyGateEvidenceFiles: moneyGateHits.count,
    bypassEvidenceFiles: invokeBypassHits.count,
  }
  if (counts.duplicateMoneyGates > 0) errors.push(`duplicate money gate nodes: ${counts.duplicateMoneyGates + 1}`)
  if (counts.bypassEvidenceFiles > 0) errors.push(`possible invoke_tool bypass evidence files: ${counts.bypassEvidenceFiles}`)

  db.close()
  return { ok: errors.length === 0, errors, counts }
}

export function architectureSummary(dbPath) {
  const db = openReadOnlyDatabase(dbPath)
  const summary = {
    meta: Object.fromEntries(db.prepare('SELECT k,v FROM meta ORDER BY k').all().map((r) => [r.k, r.v])),
    nodeKinds: db.prepare('SELECT kind, count(*) n FROM architecture_node GROUP BY kind ORDER BY n DESC').all(),
    edgeKinds: db.prepare('SELECT kind, count(*) n FROM architecture_edge GROUP BY kind ORDER BY n DESC').all(),
    invariants: db.prepare('SELECT key, name, status FROM architecture_invariant ORDER BY key').all(),
    authorityRules: db.prepare('SELECT actor, scope, enforcing_node FROM authority_rule ORDER BY actor').all(),
    evidence: db.prepare('SELECT status, evidence_kind, count(*) n FROM architecture_evidence GROUP BY status, evidence_kind ORDER BY status, evidence_kind').all(),
  }
  db.close()
  return summary
}
