#!/usr/bin/env node
// query-cloud-snapshot.mjs - read the generated docs/capabilities-cloud shards.
// This is safe for cloud agents because it opens snapshot shards read-only and does
// not require docs/capabilities.db. Snapshot output is fallback context only; final
// capability truth claims still require local audit against docs/capabilities.db.
//
// Usage:
//   node tools/capabilities/query-cloud-snapshot.mjs summary
//   node tools/capabilities/query-cloud-snapshot.mjs next --limit 20
//   node tools/capabilities/query-cloud-snapshot.mjs get <canonical-key>
import { existsSync, readFileSync } from 'node:fs'
import { join } from 'node:path'
import { DOCS } from '../_paths.mjs'
import { openReadOnlyDatabase } from '../db/sqlite-open.mjs'

const args = process.argv.slice(2)
const dirIdx = args.indexOf('--dir')
const OUT_DIR = dirIdx === -1 ? join(DOCS, 'capabilities-cloud') : args[dirIdx + 1]
const filtered = args.filter((value, idx) => value !== '--dir' && !(dirIdx !== -1 && idx === dirIdx + 1))
const limitIdx = filtered.indexOf('--limit')
const LIMIT = limitIdx === -1 ? 20 : Number(filtered[limitIdx + 1])
const positional = filtered.filter((value, idx) => value !== '--limit' && !(limitIdx !== -1 && idx === limitIdx + 1))
const [cmd = 'summary', arg, arg2] = positional

const SHARDS = {
  core: 'cap-core.db',
  provenance: 'cap-provenance.db',
  census: 'cap-census-summary.db',
  architecture: 'cap-architecture.db',
  workqueue: 'cap-workqueue.db',
}

const print = (value) => console.log(JSON.stringify(value, null, 2))
const open = (name) => {
  const path = join(OUT_DIR, SHARDS[name])
  if (!existsSync(path)) {
    console.error(`missing cloud snapshot shard: ${path}`)
    process.exit(2)
  }
  return openReadOnlyDatabase(path)
}

const readMeta = () => {
  const path = join(OUT_DIR, 'meta.json')
  if (!existsSync(path)) {
    console.error(`missing cloud snapshot meta: ${path}`)
    process.exit(2)
  }
  return JSON.parse(readFileSync(path, 'utf8'))
}

const selectOnly = (sql) => {
  if (!/^\s*select/i.test(sql || '')) {
    console.error('read-only: SQL command must start with SELECT')
    process.exit(2)
  }
  return sql
}

const usage = () => {
  console.log(`cloud capability snapshot query

Authority:
  read-only cloud fallback; local docs/capabilities.db remains final audit authority
  label capability/status claims LOCAL_AUDIT_REQUIRED unless limited to snapshot facts

Commands:
  summary                         snapshot counts and policy
  next [--limit N]                next unverified canonical capabilities
  money [--limit N]               next money-moving canonical capabilities
  domain <domain> [--limit N]     canonical capabilities in one domain
  domains                         per-domain summary
  get <canonical-key>             canonical capability plus provenance leads
  census                          donor file-census summary
  arch <capability-key>           architecture links and gaps for one capability
  notes                           open agent notes copied into the snapshot
  sql <core|provenance|census|architecture|workqueue> "<SELECT ...>"

Options:
  --dir <path>                    snapshot directory (default docs/capabilities-cloud)
  --limit <n>                     result limit for list commands`)
}

switch (cmd) {
  case 'summary': {
    const meta = readMeta()
    const workqueue = open('workqueue')
    const policy = workqueue.prepare('SELECT key, value FROM cloud_policy ORDER BY key').all()
    const domainSummary = workqueue.prepare('SELECT * FROM domain_summary ORDER BY canonical_capabilities DESC LIMIT ?').all(10)
    workqueue.close()
    print({
      generated_at: meta.generated_at,
      authority: meta.authority,
      local_audit_required: meta.local_audit_required,
      truth_label: meta.local_audit_required ? 'LOCAL_AUDIT_REQUIRED' : 'SNAPSHOT_ONLY',
      source: meta.source,
      architecture: meta.architecture,
      files: meta.files,
      policy,
      top_domains: domainSummary,
    })
    break
  }
  case 'next': {
    const db = open('workqueue')
    print(db.prepare('SELECT * FROM next_unverified_canonical LIMIT ?').all(LIMIT))
    db.close()
    break
  }
  case 'money': {
    const db = open('workqueue')
    print(db.prepare('SELECT * FROM next_money_canonical LIMIT ?').all(LIMIT))
    db.close()
    break
  }
  case 'domains': {
    const db = open('workqueue')
    print(db.prepare('SELECT * FROM domain_summary ORDER BY canonical_capabilities DESC').all())
    db.close()
    break
  }
  case 'domain': {
    const db = open('core')
    print(
      db
        .prepare(
          `SELECT key, canonical_name, domain, target_crate, target_module, moves_money, requires_approval, status, donor_count
           FROM canonical_capability WHERE domain=? ORDER BY moves_money DESC, donor_count DESC, key LIMIT ?`,
        )
        .all(arg ?? '', LIMIT),
    )
    db.close()
    break
  }
  case 'get': {
    const meta = readMeta()
    const core = open('core')
    const cap = core.prepare('SELECT * FROM canonical_capability WHERE key=?').get(arg ?? '')
    core.close()
    if (!cap) {
      print({ error: `not found: ${arg ?? ''}` })
      break
    }
    if (meta.source?.canonical_source !== 'docs/capabilities.db') {
      print({
        capability: cap,
        provenance: [],
        provenance_warning:
          'canonical rows were recovered from generated docs, so canonical IDs are not safe to join to provenance; use source_capability/workbank as leads and require local audit',
      })
      break
    }
    const provenance = open('provenance')
    const sources = provenance
      .prepare(
        `SELECT s.id, s.donor, s.module, s.canonical_name, s.source_files, s.source_symbols,
                s.business_behavior, s.technical_behavior, s.moves_money, s.requires_approval,
                s.target_crate, s.target_module
         FROM provenance p
         JOIN source_capability s ON s.id=p.source_id
         WHERE p.canonical_id=?
         ORDER BY s.donor, s.id
         LIMIT ?`,
      )
      .all(cap.id, LIMIT)
    provenance.close()
    print({ capability: cap, provenance: sources })
    break
  }
  case 'census': {
    const db = open('census')
    print({
      totals: db.prepare('SELECT * FROM donor_file_census_totals').get(),
      by_donor: db.prepare('SELECT * FROM donor_file_census_by_donor LIMIT ?').all(LIMIT),
      by_classification: db.prepare('SELECT * FROM donor_file_census_by_classification LIMIT ?').all(LIMIT),
    })
    db.close()
    break
  }
  case 'arch': {
    const db = open('architecture')
    print({
      links: db.prepare('SELECT * FROM capability_architecture_link WHERE capability_key=? ORDER BY relationship, architecture_node').all(arg ?? ''),
      gaps: db.prepare('SELECT * FROM architecture_target_gap WHERE capability_key=? ORDER BY gap_kind, target_crate').all(arg ?? ''),
    })
    db.close()
    break
  }
  case 'notes': {
    const db = open('workqueue')
    print(db.prepare('SELECT * FROM open_agent_note ORDER BY id').all())
    db.close()
    break
  }
  case 'sql': {
    const shard = arg
    const sql = selectOnly(arg2)
    if (!Object.hasOwn(SHARDS, shard)) {
      console.error(`unknown shard: ${shard}; expected ${Object.keys(SHARDS).join(', ')}`)
      process.exit(2)
    }
    const db = open(shard)
    print(db.prepare(sql).all())
    db.close()
    break
  }
  case 'help':
  case '--help':
    usage()
    break
  default:
    usage()
    process.exit(2)
}
