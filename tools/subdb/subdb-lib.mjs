import crypto from 'node:crypto'
import Database from 'better-sqlite3'
import {
  existsSync,
  mkdirSync,
  readFileSync,
  rmSync,
  writeFileSync,
} from 'node:fs'
import { dirname, join, relative, sep } from 'node:path'
import { discoverWorkspaceArchitecture } from '../architecture/architecture-lib.mjs'
import { ARCHITECTURE_CANONICAL_DIR, readJsonlFile } from '../canonical-shards/jsonl-lib.mjs'
import { openReadOnlyDatabase } from '../db/sqlite-open.mjs'

export const SUBDB_DIR = '.chronica'
export const SUBDB_JSONL = 'sub-cap-arch.jsonl'
export const SUBDB_SQLITE = 'sub-cap-arch.db'
export const SUBDB_SCHEMA_VERSION = 'chronica-subdb-v1'
const GENESIS_HASH = 'sha256:GENESIS'

const slash = (path) => path.split(sep).join('/')

function readText(path) {
  return readFileSync(path, 'utf8')
}

function parseWorkspaceMembers(toml) {
  const match = toml.match(/members\s*=\s*\[([\s\S]*?)\]/m)
  if (!match) return []
  return [...match[1].matchAll(/"([^"]+)"/g)].map((item) => item[1])
}

function parsePackageName(toml) {
  return toml.match(/^\s*name\s*=\s*"([^"]+)"/m)?.[1] ?? null
}

function sortedObject(value) {
  if (Array.isArray(value)) return value.map(sortedObject)
  if (!value || typeof value !== 'object') return value
  return Object.fromEntries(Object.keys(value).sort().map((key) => [key, sortedObject(value[key])]))
}

function stableJson(value) {
  return JSON.stringify(sortedObject(value))
}

export function hashRecord(record) {
  return `sha256:${crypto.createHash('sha256').update(stableJson(record)).digest('hex')}`
}

export const SUBDB_GENESIS_HASH = GENESIS_HASH

function subdbPaths(cratePath) {
  const dir = join(cratePath, SUBDB_DIR)
  return {
    dir,
    jsonl: join(dir, SUBDB_JSONL),
    sqlite: join(dir, SUBDB_SQLITE),
  }
}

function openCloudDb(root, name) {
  const path = join(root, 'docs', 'capabilities-cloud', name)
  return existsSync(path) ? openReadOnlyDatabase(path) : null
}

function tableExists(db, table) {
  return Boolean(db.prepare("SELECT name FROM sqlite_master WHERE type='table' AND name=?").get(table))
}

export function discoverCrates(root = process.cwd()) {
  const members = parseWorkspaceMembers(readText(join(root, 'Cargo.toml')))
  return members
    .map((member) => {
      const cargoPath = join(root, member, 'Cargo.toml')
      if (!existsSync(cargoPath)) return null
      const name = parsePackageName(readText(cargoPath))
      if (!name) return null
      return {
        name,
        member: slash(member),
        crate_path: slash(member),
        abs_path: join(root, member),
        cargo_path: slash(relative(root, cargoPath)),
      }
    })
    .filter(Boolean)
    .sort((a, b) => a.name.localeCompare(b.name))
}

function capabilityRows(root, crateName) {
  const db = openCloudDb(root, 'cap-core.db')
  if (!db) return []
  try {
    return db
      .prepare(
        `SELECT key, canonical_name, domain, target_crate, target_module, side_effect_class,
                moves_money, requires_approval, status, donor_count
         FROM canonical_capability
         WHERE target_crate=?
         ORDER BY key`,
      )
      .all(crateName)
  } finally {
    db.close()
  }
}

function architectureTrackingRows(root, crateName) {
  const db = openCloudDb(root, 'cap-architecture.db')
  if (!db) return { links: [], gaps: [] }
  try {
    const modulePrefix = `module:${crateName}:%`
    const links = tableExists(db, 'capability_architecture_link')
      ? db
        .prepare(
          `SELECT capability_key, architecture_node, relationship, status, target_crate, target_module
           FROM capability_architecture_link
           WHERE target_crate=? OR architecture_node=? OR architecture_node LIKE ?
           ORDER BY capability_key, architecture_node, relationship`,
        )
        .all(crateName, `crate:${crateName}`, modulePrefix)
      : []
    const gaps = tableExists(db, 'architecture_target_gap')
      ? db
        .prepare(
          `SELECT capability_key, target_crate, target_module, gap_kind, suggested_architecture_node,
                  reason, status, evidence_ref
           FROM architecture_target_gap
           WHERE target_crate=?
           ORDER BY capability_key, gap_kind`,
        )
        .all(crateName)
      : []
    return {
      links,
      gaps,
    }
  } finally {
    db.close()
  }
}

function architectureRows(root, crateName) {
  const arch = discoverWorkspaceArchitecture(root)
  const nodeIds = new Set()
  const prefix = `module:${crateName}:`
  const nodes = arch.nodes
    .filter((node) => node.id === `crate:${crateName}` || node.crate === crateName || node.id.startsWith(prefix))
    .sort((a, b) => a.id.localeCompare(b.id))
  for (const node of nodes) nodeIds.add(node.id)

  const edges = arch.edges
    .filter((edge) => nodeIds.has(edge.fromNode) || nodeIds.has(edge.toNode))
    .sort((a, b) => `${a.fromNode}|${a.toNode}|${a.kind}`.localeCompare(`${b.fromNode}|${b.toNode}|${b.kind}`))
  return { nodes, edges, nodeIds }
}

// docs/architecture-canonical/evidence.jsonl is a mixed file: alongside
// architecture_evidence rows it also carries architecture_invariant,
// authority_rule, and data_flow rows, which legitimately stay root-only/global
// since they span crates. architecture_evidence rows are sharded straight from
// this committed canonical file (not re-derived from a DB) — a row is kept for
// a crate iff it evidences one of that crate's own architecture_node ids (same
// ownership test as architectureRows).
function architectureEvidenceRows(root, nodeIds) {
  const path = join(root, ARCHITECTURE_CANONICAL_DIR, 'evidence.jsonl')
  if (!existsSync(path)) return []
  return readJsonlFile(path)
    .map((entry) => entry.record)
    .filter((record) => record.record_type === 'architecture_evidence' && nodeIds.has(record.architecture_node))
    .sort((a, b) => `${a.architecture_node}|${a.evidence_kind}|${a.evidence_ref}|${a.id}`
      .localeCompare(`${b.architecture_node}|${b.evidence_kind}|${b.evidence_ref}|${b.id}`))
}

function recordId(record) {
  if (record.record_type === 'meta') return `meta:${record.crate}`
  if (record.record_type === 'capability') return `capability:${record.capability_key}`
  if (record.record_type === 'architecture_node') return `architecture_node:${record.node_id}`
  if (record.record_type === 'architecture_edge') return `architecture_edge:${record.from_node}:${record.to_node}:${record.kind}`
  if (record.record_type === 'architecture_evidence') return `architecture_evidence:${record.evidence_id}:${record.architecture_node}`
  if (record.record_type === 'capability_architecture_link') {
    return `capability_architecture_link:${record.capability_key}:${record.architecture_node}:${record.relationship}`
  }
  if (record.record_type === 'architecture_target_gap') return `architecture_target_gap:${record.capability_key}:${record.gap_kind}`
  return `${record.record_type}:${stableJson(record)}`
}

function buildPlainRecords({ root, crate }) {
  const caps = capabilityRows(root, crate.name)
  const arch = architectureRows(root, crate.name)
  const evidence = architectureEvidenceRows(root, arch.nodeIds)
  const tracking = architectureTrackingRows(root, crate.name)
  const records = [
    {
      record_type: 'meta',
      schema_version: SUBDB_SCHEMA_VERSION,
      crate: crate.name,
      crate_path: crate.crate_path,
      cargo_path: crate.cargo_path,
      authority: 'crate-local cloud shard; root docs/capabilities.db and docs/architecture.db remain local aggregate audit targets',
      runtime_boundary: 'build_time_audit_time_only',
    },
    ...caps.map((capability) => ({
      record_type: 'capability',
      capability_key: capability.key,
      canonical_name: capability.canonical_name,
      domain: capability.domain,
      target_crate: capability.target_crate,
      target_module: capability.target_module,
      side_effect_class: capability.side_effect_class,
      moves_money: Number(capability.moves_money ?? 0),
      requires_approval: Number(capability.requires_approval ?? 0),
      status: capability.status,
      donor_count: Number(capability.donor_count ?? 0),
    })),
    ...arch.nodes.map((node) => ({
      record_type: 'architecture_node',
      node_id: node.id,
      kind: node.kind,
      name: node.name,
      crate: node.crate,
      module_path: node.modulePath,
      file_path: node.filePath,
      status: node.status,
      evidence_ref: node.evidenceRef,
    })),
    ...arch.edges.map((edge) => ({
      record_type: 'architecture_edge',
      from_node: edge.fromNode,
      to_node: edge.toNode,
      kind: edge.kind,
      evidence_ref: edge.evidenceRef,
    })),
    ...evidence.map((row) => ({
      record_type: 'architecture_evidence',
      evidence_id: row.id,
      architecture_node: row.architecture_node,
      status: row.status,
      evidence_kind: row.evidence_kind,
      evidence_ref: row.evidence_ref,
      test_command: row.test_command ?? null,
      test_result: row.test_result ?? null,
      verified_at: row.verified_at ?? null,
      verified_by: row.verified_by ?? null,
      blocker_reason: row.blocker_reason ?? null,
    })),
    ...tracking.links.map((link) => ({
      record_type: 'capability_architecture_link',
      capability_key: link.capability_key,
      architecture_node: link.architecture_node,
      relationship: link.relationship,
      status: link.status,
      target_crate: link.target_crate,
      target_module: link.target_module,
    })),
    ...tracking.gaps.map((gap) => ({
      record_type: 'architecture_target_gap',
      capability_key: gap.capability_key,
      target_crate: gap.target_crate,
      target_module: gap.target_module,
      gap_kind: gap.gap_kind,
      suggested_architecture_node: gap.suggested_architecture_node,
      reason: gap.reason,
      status: gap.status,
      evidence_ref: gap.evidence_ref,
    })),
  ]

  return records.map((record) => ({ record_id: recordId(record), ...record }))
}

export function chainRecords(records) {
  let previous = GENESIS_HASH
  return records.map((record, index) => {
    const withChain = { sequence: index, prev_hash: previous, ...record }
    const record_hash = hashRecord(withChain)
    previous = record_hash
    return { ...withChain, record_hash }
  })
}

function writeJsonl(path, rows) {
  mkdirSync(dirname(path), { recursive: true })
  writeFileSync(path, `${rows.map((row) => JSON.stringify(row)).join('\n')}\n`)
}

export function writeSqliteDb(path, rows) {
  rmSync(path, { force: true })
  const db = new Database(path)
  db.exec(`
    PRAGMA journal_mode = DELETE;
    CREATE TABLE subdb_record (
      sequence INTEGER PRIMARY KEY,
      record_type TEXT NOT NULL,
      record_id TEXT NOT NULL,
      prev_hash TEXT NOT NULL,
      record_hash TEXT NOT NULL,
      payload_json TEXT NOT NULL
    );
    CREATE INDEX idx_subdb_record_type ON subdb_record(record_type);
    CREATE INDEX idx_subdb_record_id ON subdb_record(record_id);
    CREATE TABLE subdb_capability (
      capability_key TEXT PRIMARY KEY,
      status TEXT,
      moves_money INTEGER NOT NULL,
      requires_approval INTEGER NOT NULL,
      target_module TEXT,
      record_hash TEXT NOT NULL
    );
    CREATE TABLE subdb_architecture_node (
      node_id TEXT PRIMARY KEY,
      kind TEXT NOT NULL,
      file_path TEXT,
      record_hash TEXT NOT NULL
    );
    CREATE TABLE subdb_capability_architecture_link (
      capability_key TEXT NOT NULL,
      architecture_node TEXT NOT NULL,
      relationship TEXT NOT NULL,
      status TEXT,
      record_hash TEXT NOT NULL
    );
  `)
  const insertRecord = db.prepare('INSERT INTO subdb_record VALUES (?, ?, ?, ?, ?, ?)')
  const insertCapability = db.prepare('INSERT INTO subdb_capability VALUES (?, ?, ?, ?, ?, ?)')
  const insertNode = db.prepare('INSERT INTO subdb_architecture_node VALUES (?, ?, ?, ?)')
  const insertLink = db.prepare('INSERT INTO subdb_capability_architecture_link VALUES (?, ?, ?, ?, ?)')
  const tx = db.transaction(() => {
    for (const row of rows) {
      insertRecord.run(row.sequence, row.record_type, row.record_id, row.prev_hash, row.record_hash, JSON.stringify(row))
      if (row.record_type === 'capability') {
        insertCapability.run(row.capability_key, row.status, row.moves_money, row.requires_approval, row.target_module, row.record_hash)
      } else if (row.record_type === 'architecture_node') {
        insertNode.run(row.node_id, row.kind, row.file_path, row.record_hash)
      } else if (row.record_type === 'capability_architecture_link') {
        insertLink.run(row.capability_key, row.architecture_node, row.relationship, row.status, row.record_hash)
      }
    }
  })
  tx()
  db.close()
}

function selectedCrates(root, requested) {
  const crates = discoverCrates(root)
  if (!requested?.length) return crates
  const wanted = new Set(requested)
  return crates.filter((crate) => wanted.has(crate.name))
}

export function generateCrateSubdbs({ root = process.cwd(), crates = [], writeSqlite = true } = {}) {
  const generated = []
  for (const crate of selectedCrates(root, crates)) {
    const records = chainRecords(buildPlainRecords({ root, crate }))
    const paths = subdbPaths(crate.abs_path)
    mkdirSync(paths.dir, { recursive: true })
    writeJsonl(paths.jsonl, records)
    if (writeSqlite) writeSqliteDb(paths.sqlite, records)
    generated.push({
      crate: crate.name,
      path: slash(relative(root, paths.jsonl)),
      sqlite: slash(relative(root, paths.sqlite)),
      records: records.length,
      capabilities: records.filter((row) => row.record_type === 'capability').length,
      architecture_nodes: records.filter((row) => row.record_type === 'architecture_node').length,
      architecture_evidence: records.filter((row) => row.record_type === 'architecture_evidence').length,
    })
  }
  return { ok: true, crates: generated }
}

function readJsonl(path) {
  return readText(path).split(/\r?\n/).filter(Boolean).map((line) => JSON.parse(line))
}

function verifyRows(rows, crateName, jsonlPath) {
  const errors = []
  let previous = GENESIS_HASH
  rows.forEach((row, index) => {
    if (row.sequence !== index) errors.push(`${jsonlPath}: sequence mismatch at ${index}`)
    if (row.prev_hash !== previous) errors.push(`${jsonlPath}: prev_hash mismatch at sequence ${index}`)
    const { record_hash, ...withoutHash } = row
    const expected = hashRecord(withoutHash)
    if (record_hash !== expected) errors.push(`${jsonlPath}: hash mismatch at sequence ${index}`)
    previous = record_hash
  })
  const meta = rows[0]
  if (meta?.record_type !== 'meta') errors.push(`${jsonlPath}: first record must be meta`)
  if (meta?.schema_version !== SUBDB_SCHEMA_VERSION) errors.push(`${jsonlPath}: schema_version must be ${SUBDB_SCHEMA_VERSION}`)
  if (meta?.crate !== crateName) errors.push(`${jsonlPath}: meta crate must be ${crateName}`)
  return errors
}

export function verifyCrateSubdbs({ root = process.cwd(), crates = [] } = {}) {
  const errors = []
  const verified = []
  for (const crate of selectedCrates(root, crates)) {
    const paths = subdbPaths(crate.abs_path)
    if (!existsSync(paths.jsonl)) {
      errors.push(`missing shard for crate ${crate.name}: ${slash(relative(root, paths.jsonl))}`)
      continue
    }
    if (!existsSync(paths.sqlite)) {
      errors.push(`missing sqlite subdb for crate ${crate.name}: ${slash(relative(root, paths.sqlite))}`)
    }
    let rows = []
    try {
      rows = readJsonl(paths.jsonl)
      errors.push(...verifyRows(rows, crate.name, slash(relative(root, paths.jsonl))))
    } catch (error) {
      errors.push(`${slash(relative(root, paths.jsonl))}: ${error.message}`)
    }
    verified.push({ crate: crate.name, records: rows.length, path: slash(relative(root, paths.jsonl)) })
  }
  return { ok: errors.length === 0, errors, crates: verified }
}

export function collectCrateSubdbs({ root = process.cwd(), crates = [], write = false, outPath = join(root, 'docs', '_machine', 'subdb-collection.json') } = {}) {
  const records = []
  const missing = []
  const summaries = []
  for (const crate of selectedCrates(root, crates)) {
    const paths = subdbPaths(crate.abs_path)
    if (!existsSync(paths.jsonl)) {
      missing.push(crate.name)
      continue
    }
    const rows = readJsonl(paths.jsonl)
    records.push(...rows.map((row) => ({ crate: crate.name, ...row })))
    summaries.push({ crate: crate.name, path: slash(relative(root, paths.jsonl)), records: rows.length })
  }
  const result = {
    schema_version: SUBDB_SCHEMA_VERSION,
    crate_count: summaries.length,
    record_count: records.length,
    missing_crates: missing,
    crates: summaries,
    records,
  }
  if (write) {
    mkdirSync(dirname(outPath), { recursive: true })
    writeFileSync(outPath, `${JSON.stringify(result, null, 2)}\n`)
  }
  return result
}

export function parseCli(argv) {
  const args = [...argv]
  const crates = []
  let write = false
  let json = false
  for (let i = 0; i < args.length; i += 1) {
    if (args[i] === '--crate') crates.push(args[++i])
    else if (args[i] === '--write') write = true
    else if (args[i] === '--json') json = true
  }
  return { crates, write, json }
}

export function printResult(result, { json = false } = {}) {
  if (json) console.log(JSON.stringify(result, null, 2))
  else console.log(`${result.ok === false ? 'subdb: FAILED' : 'subdb: OK'} ${JSON.stringify(result)}`)
}
