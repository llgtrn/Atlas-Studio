#!/usr/bin/env node
import Database from 'better-sqlite3'
import { mkdirSync, rmSync } from 'node:fs'
import { dirname } from 'node:path'
import { fileURLToPath } from 'node:url'
import {
  intFromBool,
  loadArchitectureShards,
  makeTempDbPath,
  parseArgs,
  rel,
} from '../canonical-shards/jsonl-lib.mjs'
import { verifyCanonicalArchitectureShards } from './verify-canonical-shards.mjs'

function createSchema(db) {
  db.exec(`
    PRAGMA journal_mode = DELETE;
    CREATE TABLE meta (
      k TEXT PRIMARY KEY,
      v TEXT NOT NULL
    );
    CREATE TABLE architecture_node (
      id TEXT PRIMARY KEY,
      kind TEXT NOT NULL,
      name TEXT NOT NULL,
      crate TEXT,
      module_path TEXT,
      file_path TEXT,
      status TEXT NOT NULL,
      evidence_ref TEXT
    );
    CREATE TABLE architecture_edge (
      id INTEGER PRIMARY KEY AUTOINCREMENT,
      from_node TEXT NOT NULL,
      to_node TEXT NOT NULL,
      kind TEXT NOT NULL,
      evidence_ref TEXT
    );
    CREATE TABLE architecture_invariant (
      key TEXT PRIMARY KEY,
      name TEXT NOT NULL,
      description TEXT NOT NULL,
      enforcing_node TEXT NOT NULL,
      verification_command TEXT NOT NULL,
      status TEXT NOT NULL
    );
    CREATE TABLE authority_rule (
      id INTEGER PRIMARY KEY AUTOINCREMENT,
      actor TEXT NOT NULL,
      scope TEXT NOT NULL,
      may_see TEXT NOT NULL,
      may_propose TEXT NOT NULL,
      may_approve TEXT NOT NULL,
      may_execute TEXT NOT NULL,
      may_audit TEXT NOT NULL,
      forbidden_actions TEXT NOT NULL,
      enforcing_node TEXT NOT NULL
    );
    CREATE TABLE data_flow (
      key TEXT PRIMARY KEY,
      source_node TEXT NOT NULL,
      target_node TEXT NOT NULL,
      data_kind TEXT NOT NULL,
      scope_rule TEXT NOT NULL,
      gate_required INTEGER NOT NULL,
      audit_required INTEGER NOT NULL
    );
    CREATE TABLE capability_architecture_link (
      capability_key TEXT NOT NULL,
      architecture_node TEXT NOT NULL,
      relationship TEXT NOT NULL,
      status TEXT,
      target_crate TEXT,
      target_module TEXT,
      PRIMARY KEY (capability_key, architecture_node, relationship)
    );
    CREATE TABLE architecture_evidence (
      id INTEGER PRIMARY KEY AUTOINCREMENT,
      architecture_node TEXT NOT NULL,
      status TEXT NOT NULL,
      evidence_kind TEXT NOT NULL,
      evidence_ref TEXT NOT NULL,
      test_command TEXT,
      test_result TEXT,
      verified_at TEXT,
      verified_by TEXT,
      blocker_reason TEXT
    );
    CREATE TABLE architecture_status_override (
      architecture_node TEXT PRIMARY KEY,
      status TEXT NOT NULL,
      reason TEXT NOT NULL,
      updated_at TEXT NOT NULL,
      updated_by TEXT NOT NULL
    );
    CREATE TABLE architecture_target_gap (
      capability_key TEXT NOT NULL,
      target_crate TEXT,
      target_module TEXT,
      gap_kind TEXT NOT NULL,
      suggested_architecture_node TEXT,
      reason TEXT NOT NULL,
      status TEXT NOT NULL,
      evidence_ref TEXT,
      PRIMARY KEY (capability_key, gap_kind)
    );
    CREATE TABLE planned_architecture_node (
      id TEXT PRIMARY KEY,
      kind TEXT NOT NULL,
      name TEXT NOT NULL,
      logical_domain TEXT,
      intended_owner_crate TEXT,
      status TEXT NOT NULL,
      reason TEXT NOT NULL,
      evidence_ref TEXT
    );
    CREATE INDEX idx_arch_node_kind ON architecture_node(kind);
    CREATE INDEX idx_arch_edge_from ON architecture_edge(from_node);
    CREATE INDEX idx_arch_edge_to ON architecture_edge(to_node);
    CREATE INDEX idx_cap_arch_key ON capability_architecture_link(capability_key);
    CREATE INDEX idx_arch_evidence_node ON architecture_evidence(architecture_node);
    CREATE INDEX idx_arch_evidence_status ON architecture_evidence(status);
    CREATE INDEX idx_arch_gap_kind ON architecture_target_gap(gap_kind);
    CREATE INDEX idx_arch_gap_target ON architecture_target_gap(target_crate);
  `)
}

export function buildArchitectureDbFromCanonicalShards({
  root = process.cwd(),
  outPath = null,
} = {}) {
  const verify = verifyCanonicalArchitectureShards({ root })
  if (!verify.ok) {
    const error = new Error('canonical architecture shard verification failed')
    error.details = verify
    throw error
  }

  const temp = outPath ? null : makeTempDbPath('chronica-architecture-cache-', 'architecture.db')
  const dbPath = outPath ?? temp.path
  mkdirSync(dirname(dbPath), { recursive: true })
  rmSync(dbPath, { force: true })
  const db = new Database(dbPath)
  createSchema(db)

  const { entries, meta } = loadArchitectureShards(root)
  const records = entries.map((entry) => entry.record)
  const insertMeta = db.prepare('INSERT INTO meta (k, v) VALUES (?, ?)')
  const insertNode = db.prepare(`INSERT INTO architecture_node
    (id, kind, name, crate, module_path, file_path, status, evidence_ref)
    VALUES (?, ?, ?, ?, ?, ?, ?, ?)`)
  const insertEdge = db.prepare(`INSERT INTO architecture_edge
    (id, from_node, to_node, kind, evidence_ref)
    VALUES (?, ?, ?, ?, ?)`)
  const insertInvariant = db.prepare(`INSERT INTO architecture_invariant
    (key, name, description, enforcing_node, verification_command, status)
    VALUES (?, ?, ?, ?, ?, ?)`)
  const insertAuthority = db.prepare(`INSERT INTO authority_rule
    (id, actor, scope, may_see, may_propose, may_approve, may_execute, may_audit, forbidden_actions, enforcing_node)
    VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)`)
  const insertFlow = db.prepare(`INSERT INTO data_flow
    (key, source_node, target_node, data_kind, scope_rule, gate_required, audit_required)
    VALUES (?, ?, ?, ?, ?, ?, ?)`)
  const insertCapLink = db.prepare(`INSERT INTO capability_architecture_link
    (capability_key, architecture_node, relationship, status, target_crate, target_module)
    VALUES (?, ?, ?, ?, ?, ?)`)
  const insertEvidence = db.prepare(`INSERT INTO architecture_evidence
    (id, architecture_node, status, evidence_kind, evidence_ref, test_command, test_result, verified_at, verified_by, blocker_reason)
    VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)`)
  const insertStatusOverride = db.prepare(`INSERT INTO architecture_status_override
    (architecture_node, status, reason, updated_at, updated_by)
    VALUES (?, ?, ?, ?, ?)`)
  const insertGap = db.prepare(`INSERT INTO architecture_target_gap
    (capability_key, target_crate, target_module, gap_kind, suggested_architecture_node, reason, status, evidence_ref)
    VALUES (?, ?, ?, ?, ?, ?, ?, ?)`)
  const insertPlanned = db.prepare(`INSERT INTO planned_architecture_node
    (id, kind, name, logical_domain, intended_owner_crate, status, reason, evidence_ref)
    VALUES (?, ?, ?, ?, ?, ?, ?, ?)`)

  const tx = db.transaction(() => {
    insertMeta.run('schema_version', '1')
    insertMeta.run('authority', 'generated_cache_from_canonical_jsonl')
    insertMeta.run('generator', 'tools/architecture/build-db-from-canonical-shards.mjs')
    for (const [key, value] of Object.entries(meta?.source_meta ?? {})) {
      insertMeta.run(`source.${key}`, String(value))
    }

    for (const record of records) {
      if (record.record_type === 'architecture_node') {
        insertNode.run(record.id, record.kind, record.name, record.crate, record.module_path, record.file_path, record.status, record.evidence_ref)
      } else if (record.record_type === 'architecture_edge') {
        insertEdge.run(record.id, record.from_node, record.to_node, record.relationship, record.evidence_ref)
      } else if (record.record_type === 'architecture_invariant') {
        insertInvariant.run(record.key, record.name, record.description, record.enforcing_node, record.verification_command, record.status)
      } else if (record.record_type === 'authority_rule') {
        insertAuthority.run(record.id, record.actor, record.scope, record.may_see, record.may_propose, record.may_approve, record.may_execute, record.may_audit, record.forbidden_actions, record.enforcing_node)
      } else if (record.record_type === 'data_flow') {
        insertFlow.run(record.key, record.source_node, record.target_node, record.data_kind, record.scope_rule, intFromBool(record.gate_required), intFromBool(record.audit_required))
      } else if (record.record_type === 'capability_architecture_link') {
        insertCapLink.run(record.capability_key, record.architecture_node, record.relationship, record.status, record.target_crate, record.target_module)
      } else if (record.record_type === 'architecture_evidence') {
        insertEvidence.run(record.id, record.architecture_node, record.status, record.evidence_kind, record.evidence_ref, record.test_command, record.test_result, record.verified_at, record.verified_by, record.blocker_reason)
      } else if (record.record_type === 'architecture_status_override') {
        insertStatusOverride.run(record.architecture_node, record.status, record.reason, record.updated_at, record.updated_by)
      } else if (record.record_type === 'architecture_target_gap') {
        insertGap.run(record.capability_key, record.target_crate, record.target_module, record.gap_kind, record.suggested_architecture_node, record.reason, record.status, record.evidence_ref)
      } else if (record.record_type === 'planned_architecture_node') {
        insertPlanned.run(record.id, record.kind, record.name, record.logical_domain, record.intended_owner_crate, record.status, record.reason, record.evidence_ref)
      }
    }

    for (const [table, count] of Object.entries(meta?.counts ?? {})) {
      insertMeta.run(`${table}_count`, String(count))
    }
  })
  tx()

  const counts = Object.fromEntries([
    'architecture_node',
    'architecture_edge',
    'capability_architecture_link',
    'architecture_target_gap',
    'planned_architecture_node',
    'architecture_evidence',
    'architecture_invariant',
    'authority_rule',
    'data_flow',
    'architecture_status_override',
  ].map((table) => [table, db.prepare(`SELECT count(*) n FROM ${table}`).get().n]))
  db.close()
  return { dbPath, tempDir: temp?.dir ?? null, counts }
}

function main() {
  const args = parseArgs(process.argv.slice(2))
  try {
    const result = buildArchitectureDbFromCanonicalShards({
      root: process.cwd(),
      outPath: args.out ? String(args.out) : null,
    })
    console.log(JSON.stringify({
      ok: true,
      db_path: result.tempDir ? result.dbPath : rel(process.cwd(), result.dbPath),
      temp_cache: Boolean(result.tempDir),
      counts: result.counts,
    }, null, 2))
  } catch (error) {
    console.error(JSON.stringify({ ok: false, error: error.message, details: error.details ?? null }, null, 2))
    process.exitCode = 1
  }
}

if (process.argv[1] && fileURLToPath(import.meta.url) === process.argv[1]) {
  main()
}
