#!/usr/bin/env node
import { existsSync, readFileSync, readdirSync } from 'node:fs'
import { join } from 'node:path'
import { fileURLToPath } from 'node:url'
import { openReadOnlyDatabase } from '../db/sqlite-open.mjs'
import {
  ARCHITECTURE_CANONICAL_DIR,
  CAPABILITY_CANONICAL_DIR,
  cleanString,
  emptyDirectory,
  parseArgs,
  rel,
  slash,
  writeJsonFile,
  writeJsonl,
} from '../canonical-shards/jsonl-lib.mjs'

function tableExists(db, table) {
  return db.prepare("SELECT count(*) n FROM sqlite_master WHERE type='table' AND name=?").get(table).n > 0
}

function all(db, table, orderBy) {
  if (!tableExists(db, table)) return []
  return db.prepare(`SELECT * FROM ${table} ORDER BY ${orderBy}`).all()
}

function metaRows(db) {
  if (!tableExists(db, 'meta')) return {}
  return Object.fromEntries(db.prepare('SELECT k, v FROM meta ORDER BY k').all().map((row) => [row.k, row.v]))
}

function architectureDbSource(root, requestedPath) {
  const path = requestedPath ?? join(root, 'docs', 'architecture.db')
  if (!existsSync(path)) throw new Error(`Architecture DB not found: ${path}`)
  return path
}

function nodeRecord(row) {
  return {
    schema_version: 1,
    record_type: 'architecture_node',
    id: row.id,
    kind: row.kind,
    name: row.name,
    crate: cleanString(row.crate),
    module_path: cleanString(row.module_path),
    file_path: cleanString(row.file_path),
    status: row.status,
    evidence_ref: cleanString(row.evidence_ref),
  }
}

function edgeRecord(row) {
  return {
    schema_version: 1,
    record_type: 'architecture_edge',
    id: row.id,
    from_node: row.from_node,
    to_node: row.to_node,
    relationship: row.kind,
    evidence_ref: cleanString(row.evidence_ref),
  }
}

function capLinkRecord(row) {
  return {
    schema_version: 1,
    record_type: 'capability_architecture_link',
    capability_key: row.capability_key,
    architecture_node: row.architecture_node,
    relationship: row.relationship,
    status: cleanString(row.status),
    target_crate: cleanString(row.target_crate),
    target_module: cleanString(row.target_module),
  }
}

function targetGapRecord(row) {
  return {
    schema_version: 1,
    record_type: 'architecture_target_gap',
    capability_key: row.capability_key,
    target_crate: cleanString(row.target_crate),
    target_module: cleanString(row.target_module),
    gap_kind: row.gap_kind,
    suggested_architecture_node: cleanString(row.suggested_architecture_node),
    reason: row.reason,
    status: row.status,
    evidence_ref: cleanString(row.evidence_ref),
  }
}

function plannedNodeRecord(row) {
  return {
    schema_version: 1,
    record_type: 'planned_architecture_node',
    id: row.id,
    kind: row.kind,
    name: row.name,
    logical_domain: cleanString(row.logical_domain),
    intended_owner_crate: cleanString(row.intended_owner_crate),
    status: row.status,
    reason: row.reason,
    evidence_ref: cleanString(row.evidence_ref),
  }
}

function invariantRecord(row) {
  return {
    schema_version: 1,
    record_type: 'architecture_invariant',
    key: row.key,
    name: row.name,
    description: row.description,
    enforcing_node: row.enforcing_node,
    verification_command: row.verification_command,
    status: row.status,
  }
}

function authorityRuleRecord(row) {
  return {
    schema_version: 1,
    record_type: 'authority_rule',
    id: row.id,
    actor: row.actor,
    scope: row.scope,
    may_see: row.may_see,
    may_propose: row.may_propose,
    may_approve: row.may_approve,
    may_execute: row.may_execute,
    may_audit: row.may_audit,
    forbidden_actions: row.forbidden_actions,
    enforcing_node: row.enforcing_node,
  }
}

function dataFlowRecord(row) {
  return {
    schema_version: 1,
    record_type: 'data_flow',
    key: row.key,
    source_node: row.source_node,
    target_node: row.target_node,
    data_kind: row.data_kind,
    scope_rule: row.scope_rule,
    gate_required: Boolean(row.gate_required),
    audit_required: Boolean(row.audit_required),
  }
}

// Maps capability_key -> "docs/capabilities-canonical/domains/<shard>.jsonl:<line>" by
// scanning the canonical capability shards once per export run. This is what
// normalizeEvidenceRef() resolves `capabilities.db:<key>` refs against instead of
// emitting the literal, never-resolvable glob `docs/capabilities-canonical/**/*.jsonl::<key>`.
function buildCapabilityKeyIndex(root) {
  const index = new Map()
  const domainsDir = join(root, CAPABILITY_CANONICAL_DIR, 'domains')
  if (!existsSync(domainsDir)) return index
  for (const fileName of readdirSync(domainsDir)) {
    if (!fileName.endsWith('.jsonl')) continue
    const filePath = join(domainsDir, fileName)
    const lines = readFileSync(filePath, 'utf8').split('\n')
    lines.forEach((line, i) => {
      if (!line.trim()) return
      let record
      try {
        record = JSON.parse(line)
      } catch {
        return
      }
      const key = record?.capability_key
      if (key && !index.has(key)) {
        index.set(key, `${slash(join('docs', 'capabilities-canonical', 'domains', fileName))}:${i + 1}`)
      }
    })
  }
  return index
}

function normalizeEvidenceRef(value, capabilityKeyIndex) {
  const ref = cleanString(value)
  if (!ref) return ref
  const capabilityMatch = ref.match(/^capabilities\.db:(.+)$/)
  if (!capabilityMatch) return ref
  const key = capabilityMatch[1]
  const location = capabilityKeyIndex.get(key)
  if (!location) {
    throw new Error(
      `export-canonical-shards: cannot resolve evidence_ref "${ref}" — capability_key "${key}" not found in any docs/capabilities-canonical/domains/*.jsonl shard.`,
    )
  }
  return `${location}#${key}`
}

function architectureEvidenceRecord(row, capabilityKeyIndex) {
  return {
    schema_version: 1,
    record_type: 'architecture_evidence',
    id: row.id,
    architecture_node: row.architecture_node,
    status: row.status,
    evidence_kind: row.evidence_kind,
    evidence_ref: normalizeEvidenceRef(row.evidence_ref, capabilityKeyIndex),
    test_command: cleanString(row.test_command),
    test_result: cleanString(row.test_result),
    verified_at: cleanString(row.verified_at),
    verified_by: cleanString(row.verified_by),
    blocker_reason: cleanString(row.blocker_reason),
  }
}

function statusOverrideRecord(row) {
  return {
    schema_version: 1,
    record_type: 'architecture_status_override',
    architecture_node: row.architecture_node,
    status: row.status,
    reason: row.reason,
    updated_at: row.updated_at,
    updated_by: row.updated_by,
  }
}

function recordSortKey(record) {
  return [
    record.record_type,
    record.id ?? record.key ?? record.capability_key ?? record.architecture_node ?? record.actor ?? '',
    record.relationship ?? record.kind ?? record.from_node ?? '',
    record.to_node ?? record.target_crate ?? '',
  ].join('|')
}

function sortRecords(records) {
  return records.sort((a, b) => recordSortKey(a).localeCompare(recordSortKey(b)))
}

export function exportCanonicalArchitectureShards({
  root = process.cwd(),
  dbPath = null,
  outDir = join(root, ARCHITECTURE_CANONICAL_DIR),
} = {}) {
  const sourceDbPath = architectureDbSource(root, dbPath)
  const db = openReadOnlyDatabase(sourceDbPath)
  const capabilityKeyIndex = buildCapabilityKeyIndex(root)

  const nodes = all(db, 'architecture_node', 'id').map(nodeRecord)
  const edgeRows = all(db, 'architecture_edge', 'from_node, to_node, kind, id').map(edgeRecord)
  const capLinks = all(db, 'capability_architecture_link', 'capability_key, architecture_node, relationship').map(capLinkRecord)
  const gaps = [
    ...all(db, 'architecture_target_gap', 'capability_key, gap_kind').map(targetGapRecord),
    ...all(db, 'planned_architecture_node', 'id').map(plannedNodeRecord),
  ]
  const evidence = [
    ...all(db, 'architecture_evidence', 'architecture_node, evidence_kind, evidence_ref, id').map((row) =>
      architectureEvidenceRecord(row, capabilityKeyIndex),
    ),
    ...all(db, 'architecture_invariant', 'key').map(invariantRecord),
    ...all(db, 'authority_rule', 'actor, scope, id').map(authorityRuleRecord),
    ...all(db, 'data_flow', 'key').map(dataFlowRecord),
    ...all(db, 'architecture_status_override', 'architecture_node').map(statusOverrideRecord),
  ]

  emptyDirectory(outDir)
  writeJsonl(join(outDir, 'nodes.jsonl'), sortRecords(nodes))
  writeJsonl(join(outDir, 'links.jsonl'), sortRecords([...edgeRows, ...capLinks]))
  writeJsonl(join(outDir, 'gaps.jsonl'), sortRecords(gaps))
  writeJsonl(join(outDir, 'evidence.jsonl'), sortRecords(evidence))

  const meta = {
    schema_version: 1,
    authority: 'canonical_jsonl',
    generated_by: 'tools/architecture/export-canonical-shards.mjs',
    source_db: rel(root, sourceDbPath),
    source_meta: metaRows(db),
    counts: {
      architecture_node: nodes.length,
      architecture_edge: edgeRows.length,
      capability_architecture_link: capLinks.length,
      architecture_target_gap: gaps.filter((record) => record.record_type === 'architecture_target_gap').length,
      planned_architecture_node: gaps.filter((record) => record.record_type === 'planned_architecture_node').length,
      architecture_evidence: evidence.filter((record) => record.record_type === 'architecture_evidence').length,
      architecture_invariant: evidence.filter((record) => record.record_type === 'architecture_invariant').length,
      authority_rule: evidence.filter((record) => record.record_type === 'authority_rule').length,
      data_flow: evidence.filter((record) => record.record_type === 'data_flow').length,
      architecture_status_override: evidence.filter((record) => record.record_type === 'architecture_status_override').length,
    },
  }
  writeJsonFile(join(outDir, 'meta.json'), meta)
  db.close()
  return meta
}

function main() {
  const args = parseArgs(process.argv.slice(2))
  const result = exportCanonicalArchitectureShards({
    root: process.cwd(),
    dbPath: args.db ? String(args.db) : null,
    outDir: args.out ? String(args.out) : join(process.cwd(), ARCHITECTURE_CANONICAL_DIR),
  })
  console.log(JSON.stringify(result, null, 2))
}

if (process.argv[1] && fileURLToPath(import.meta.url) === process.argv[1]) {
  main()
}
