#!/usr/bin/env node
import { join } from 'node:path'
import { architectureSummary } from './architecture-lib.mjs'
import { openReadOnlyDatabase } from '../db/sqlite-open.mjs'
import { loadArchitectureShards } from '../canonical-shards/jsonl-lib.mjs'
import { verifyCanonicalArchitectureShards } from './verify-canonical-shards.mjs'

const dbPath = join(process.cwd(), 'docs', 'architecture.db')
const [cmd, ...rest] = process.argv.slice(2)
const arg = rest[0]
const print = (value) => console.log(JSON.stringify(value, null, 2))

// docs/architecture.db is a local aggregate audit target, not a cloud-promotable
// source of final truth (docs/doctrines/023-five-dimension-cloud-shard-contract.md section 1,
// 3; docs/benchmarks/116-operating-world-class-benchmark-gates.md section 8). A live read of
// this DB is a real, reproducible number -- it is not to be nulled out -- but it
// must never be reported as final/promoted truth without this label, so a
// disputed prior figure is never silently overwritten by a bare number.
const COVERAGE_TRUTH_LABEL = 'LOCAL_AUDIT_REQUIRED'
const COVERAGE_LOCAL_AUDIT_NOTE =
  'docs/architecture.db is a local aggregate audit target (docs/023 section 1, 3; docs/116 section 8). ' +
  'This coverage output is a live, reproducible read of the committed DB, not a local-audit-promoted final ' +
  'verification. Any report built from these numbers must carry truth_label LOCAL_AUDIT_REQUIRED and must ' +
  'record -- not silently overwrite -- disagreement with any prior reported figure.'

function groupedShardRecords() {
  try {
    const shard = loadArchitectureShards(process.cwd())
    if (!shard.meta || shard.entries.length === 0) return null
    const verification = verifyCanonicalArchitectureShards({ root: process.cwd() })
    if (!verification.ok) return { verification }
    const records = shard.entries.map((entry) => entry.record)
    const byType = new Map()
    for (const record of records) {
      const list = byType.get(record.record_type) ?? []
      list.push(record)
      byType.set(record.record_type, list)
    }
    return { meta: shard.meta, records, byType, verification }
  } catch {
    return null
  }
}

function tryCanonicalShardCommand() {
  if (cmd === 'sql') return false
  const shard = groupedShardRecords()
  if (!shard) return false
  if (shard.verification && !shard.verification.ok) {
    print({
      truth_label: 'CLOUD_FINAL_BLOCKED',
      source: 'docs/architecture-canonical/**/*.jsonl',
      errors: shard.verification.errors,
      warnings: shard.verification.warnings,
    })
    process.exitCode = 2
    return true
  }
  const rows = (type) => shard.byType.get(type) ?? []
  const capLinks = rows('capability_architecture_link')
  const gaps = rows('architecture_target_gap')
  const linkedCapabilities = new Set(capLinks.map((row) => row.capability_key))
  const gappedCapabilities = new Set(gaps.map((row) => row.capability_key))
  const metaCanonical = Number(shard.meta?.source_meta?.canonical_capability_count ?? shard.meta?.counts?.canonical_capability_count ?? 0)
  switch (cmd || 'summary') {
    case 'summary':
      print({
        truth_label: 'CLOUD_FINAL_VERIFIED',
        source: 'docs/architecture-canonical/**/*.jsonl',
        meta: shard.meta,
        nodeKinds: Object.values(rows('architecture_node').reduce((acc, row) => {
          acc[row.kind] ??= { kind: row.kind, n: 0 }
          acc[row.kind].n += 1
          return acc
        }, {})).sort((a, b) => b.n - a.n || a.kind.localeCompare(b.kind)),
        edgeKinds: Object.values(rows('architecture_edge').reduce((acc, row) => {
          acc[row.relationship] ??= { kind: row.relationship, n: 0 }
          acc[row.relationship].n += 1
          return acc
        }, {})).sort((a, b) => b.n - a.n || a.kind.localeCompare(b.kind)),
      })
      return true
    case 'nodes':
      print(rows('architecture_node').filter((row) => !arg || row.kind === arg).slice(0, 200))
      return true
    case 'edges':
      print(rows('architecture_edge').filter((row) => !arg || row.relationship === arg).slice(0, 200))
      return true
    case 'authority':
      print(rows('authority_rule').sort((a, b) => a.actor.localeCompare(b.actor)))
      return true
    case 'flows':
      print(rows('data_flow').sort((a, b) => a.key.localeCompare(b.key)))
      return true
    case 'cap':
      print(capLinks.filter((row) => row.capability_key.includes(arg ?? '')).slice(0, 200))
      return true
    case 'coverage':
      print({
        truth_label: 'CLOUD_FINAL_VERIFIED',
        source: 'docs/architecture-canonical/**/*.jsonl',
        meta: shard.meta,
        live: {
          architecture_node_count: rows('architecture_node').length,
          architecture_edge_count: rows('architecture_edge').length,
          capability_architecture_link_count: capLinks.length,
          linked_distinct_capabilities: linkedCapabilities.size,
          gap_capabilities: gappedCapabilities.size,
          accounted_capabilities: new Set([...linkedCapabilities, ...gappedCapabilities]).size,
          unaccounted_capabilities: metaCanonical ? metaCanonical - new Set([...linkedCapabilities, ...gappedCapabilities]).size : null,
        },
      })
      return true
    case 'gaps':
      print(Object.values(gaps.reduce((acc, row) => {
        const key = `${row.gap_kind}|${row.target_crate ?? '<none>'}|${row.suggested_architecture_node ?? ''}`
        acc[key] ??= {
          gap_kind: row.gap_kind,
          target_crate: row.target_crate ?? '<none>',
          suggested_architecture_node: row.suggested_architecture_node ?? '',
          caps: 0,
        }
        acc[key].caps += 1
        return acc
      }, {})).sort((a, b) => a.gap_kind.localeCompare(b.gap_kind) || b.caps - a.caps).slice(0, 300))
      return true
    case 'planned':
      print(rows('planned_architecture_node').sort((a, b) => a.name.localeCompare(b.name)).slice(0, 400))
      return true
    case 'evidence':
      print(rows('architecture_evidence').filter((row) => {
        const needle = arg ?? ''
        return row.architecture_node.includes(needle) || row.status.includes(needle) || row.evidence_kind.includes(needle)
      }).slice(0, 200))
      return true
    case 'overrides':
      print(rows('architecture_status_override').sort((a, b) => a.architecture_node.localeCompare(b.architecture_node)))
      return true
    default:
      return false
  }
}

if (tryCanonicalShardCommand()) {
  process.exit(0)
}

if (cmd === 'summary' || !cmd) {
  print(architectureSummary(dbPath))
} else if (cmd === 'nodes') {
  const db = openReadOnlyDatabase(dbPath)
  print(db.prepare('SELECT * FROM architecture_node WHERE kind=COALESCE(?, kind) ORDER BY kind, id LIMIT 200').all(arg ?? null))
  db.close()
} else if (cmd === 'edges') {
  const db = openReadOnlyDatabase(dbPath)
  print(db.prepare('SELECT * FROM architecture_edge WHERE kind=COALESCE(?, kind) ORDER BY kind, from_node, to_node LIMIT 200').all(arg ?? null))
  db.close()
} else if (cmd === 'authority') {
  const db = openReadOnlyDatabase(dbPath)
  print(db.prepare('SELECT * FROM authority_rule ORDER BY actor').all())
  db.close()
} else if (cmd === 'flows') {
  const db = openReadOnlyDatabase(dbPath)
  print(db.prepare('SELECT * FROM data_flow ORDER BY key').all())
  db.close()
} else if (cmd === 'cap') {
  const db = openReadOnlyDatabase(dbPath)
  print(db.prepare('SELECT * FROM capability_architecture_link WHERE capability_key LIKE ? ORDER BY capability_key, architecture_node LIMIT 200').all(`%${arg ?? ''}%`))
  db.close()
} else if (cmd === 'coverage') {
  const db = openReadOnlyDatabase(dbPath)
  const meta = Object.fromEntries(
    db.prepare(`SELECT k,v FROM meta WHERE k IN
      ('canonical_capability_count','linked_distinct_capabilities','gap_capabilities',
       'accounted_capabilities','unaccounted_capabilities','planned_architecture_node_count',
       'coverage_percent','capability_architecture_link_count')`).all().map((r) => [r.k, r.v]),
  )
  const byRelationship = db.prepare(
    'SELECT relationship, count(DISTINCT capability_key) caps FROM capability_architecture_link GROUP BY relationship ORDER BY caps DESC',
  ).all()
  const byGapKind = db.prepare(
    'SELECT gap_kind, count(*) caps FROM architecture_target_gap GROUP BY gap_kind ORDER BY caps DESC',
  ).all()
  const live = {
    architecture_node_count: db.prepare('SELECT count(*) n FROM architecture_node').get().n,
    architecture_edge_count: db.prepare('SELECT count(*) n FROM architecture_edge').get().n,
    capability_architecture_link_count: db.prepare('SELECT count(*) n FROM capability_architecture_link').get().n,
    linked_distinct_capabilities: db.prepare('SELECT count(DISTINCT capability_key) n FROM capability_architecture_link').get().n,
    gap_capabilities: db.prepare('SELECT count(DISTINCT capability_key) n FROM architecture_target_gap').get().n,
    accounted_capabilities: db.prepare(`SELECT count(*) n FROM (
      SELECT capability_key FROM capability_architecture_link
      UNION
      SELECT capability_key FROM architecture_target_gap
    )`).get().n,
  }
  live.local_extra_accounted_over_canonical = Math.max(
    0,
    live.accounted_capabilities - Number(meta.canonical_capability_count || 0),
  )
  print({
    truth_label: COVERAGE_TRUTH_LABEL,
    local_audit_note: COVERAGE_LOCAL_AUDIT_NOTE,
    meta,
    live,
    linkedByRelationship: byRelationship,
    gapsByKind: byGapKind,
  })
  db.close()
} else if (cmd === 'gaps') {
  const db = openReadOnlyDatabase(dbPath)
  print(db.prepare(`SELECT gap_kind, COALESCE(target_crate,'<none>') AS target_crate,
    COALESCE(suggested_architecture_node,'') AS suggested_architecture_node, count(*) caps
    FROM architecture_target_gap GROUP BY gap_kind, target_crate ORDER BY gap_kind, caps DESC LIMIT 300`).all())
  db.close()
} else if (cmd === 'planned') {
  const db = openReadOnlyDatabase(dbPath)
  print(db.prepare('SELECT id, name, logical_domain, intended_owner_crate, status, reason FROM planned_architecture_node ORDER BY name LIMIT 400').all())
  db.close()
} else if (cmd === 'evidence') {
  const db = openReadOnlyDatabase(dbPath)
  print(db.prepare(`SELECT * FROM architecture_evidence
    WHERE architecture_node LIKE ? OR status LIKE ? OR evidence_kind LIKE ?
    ORDER BY architecture_node, status, evidence_kind LIMIT 200`).all(`%${arg ?? ''}%`, `%${arg ?? ''}%`, `%${arg ?? ''}%`))
  db.close()
} else if (cmd === 'overrides') {
  const db = openReadOnlyDatabase(dbPath)
  print(db.prepare('SELECT * FROM architecture_status_override ORDER BY architecture_node').all())
  db.close()
} else if (cmd === 'sql') {
  if (!/^\s*select/i.test(arg || '')) {
    console.error('read-only: SELECT only')
    process.exit(2)
  }
  const db = openReadOnlyDatabase(dbPath)
  print(db.prepare(arg).all())
  db.close()
} else {
  console.log(`architecture DB query — docs/architecture.db
Commands:
  summary
  nodes [kind]
  edges [kind]
  authority
  flows
  coverage
  gaps
  planned
  cap <capability-key-fragment>
  evidence [node/status/kind fragment]
  overrides
  sql "<SELECT ...>"`)
}
