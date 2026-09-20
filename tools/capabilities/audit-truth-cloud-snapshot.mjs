#!/usr/bin/env node
// audit-truth-cloud-snapshot.mjs - cloud-readable companion to tools/capabilities/audit-truth.mjs's
// source_money_underclaimed check.
//
// tools/capabilities/audit-truth.mjs opens docs/capabilities.db directly (fileMustExist: true) and
// has no fallback: `pnpm caps:audit-truth` hard-fails in any checkout where that DB is absent
// (it is gitignored/local-only per docs/doctrines/023-five-dimension-cloud-shard-contract.md). This script
// reproduces exactly one of its checks -- source_capability rows that move money while their
// canonical_capability is classified moves_money=0 -- against the git-tracked cloud snapshot
// (docs/capabilities-cloud/cap-core.db + cap-provenance.db) instead, using the identical filter and
// dedup logic. The snapshot splits canonical_capability (cap-core.db) and source_capability/
// provenance (cap-provenance.db) across two files, so this script ATTACHes them rather than
// importing audit-truth.mjs's single-connection query.
//
// This is read-only and diagnostic/reporting only. It does not write to any DB and it is not a
// replacement for tools/capabilities/audit-truth.mjs: per docs/023 section 1/3, local aggregate
// audit against the real docs/capabilities.db remains the sole authority for final capability/money
// truth claims. Any finding here must be labeled LOCAL_AUDIT_REQUIRED.
//
// Usage:
//   node tools/capabilities/audit-truth-cloud-snapshot.mjs
//   node tools/capabilities/audit-truth-cloud-snapshot.mjs --dir docs/capabilities-cloud
//   node tools/capabilities/audit-truth-cloud-snapshot.mjs --fail-on-findings
import { join, resolve } from 'node:path'
import { fileURLToPath } from 'node:url'
import { DOCS } from '../_paths.mjs'
import { openReadOnlyDatabase } from '../db/sqlite-open.mjs'

function parseArgs(argv) {
  const out = { dir: join(DOCS, 'capabilities-cloud'), failOnFindings: false, help: false }
  for (let i = 0; i < argv.length; i += 1) {
    const arg = argv[i]
    if (arg === '--help' || arg === '-h') {
      out.help = true
      continue
    }
    if (arg === '--fail-on-findings') {
      out.failOnFindings = true
      continue
    }
    if (arg === '--dir') {
      const value = argv[i + 1]
      if (!value) throw new Error('--dir requires a value')
      out.dir = value
      i += 1
      continue
    }
    throw new Error(`unknown argument: ${arg}`)
  }
  return out
}

function truthyInt(value) {
  return Number(value ?? 0) === 1
}

const QUERY = `
  SELECT
    s.id AS sourceId, s.donor AS donor, s.canonical_name AS sourceName, s.source_files AS sourceFiles,
    COALESCE(s.moves_money, 0) AS sourceMovesMoney, s.requires_approval AS sourceRequiresApproval,
    s.side_effect_class AS sourceSideEffectClass,
    c.key AS canonicalKey, c.canonical_name AS canonicalName,
    COALESCE(o.moves_money, c.moves_money) AS canonicalMovesMoney,
    COALESCE(o.requires_approval, c.requires_approval) AS canonicalRequiresApproval,
    c.status AS canonicalStatus, c.domain AS canonicalDomain, c.target_crate AS canonicalTargetCrate
  FROM source_capability s
  JOIN core.canonical_capability c ON c.id = s.canonical_id
  LEFT JOIN core.canonical_status_override o ON o.canonical_key = c.key
  WHERE (COALESCE(s.moves_money, 0)=1 OR lower(COALESCE(s.side_effect_class, '')) LIKE '%money%')
    AND COALESCE(COALESCE(o.moves_money, c.moves_money), 0)=0

  UNION ALL

  SELECT
    s.id AS sourceId, s.donor AS donor, s.canonical_name AS sourceName, s.source_files AS sourceFiles,
    COALESCE(s.moves_money, 0) AS sourceMovesMoney, s.requires_approval AS sourceRequiresApproval,
    s.side_effect_class AS sourceSideEffectClass,
    c.key AS canonicalKey, c.canonical_name AS canonicalName,
    COALESCE(o.moves_money, c.moves_money) AS canonicalMovesMoney,
    COALESCE(o.requires_approval, c.requires_approval) AS canonicalRequiresApproval,
    c.status AS canonicalStatus, c.domain AS canonicalDomain, c.target_crate AS canonicalTargetCrate
  FROM source_capability s
  JOIN provenance p ON p.source_id = s.id
  JOIN core.canonical_capability c ON c.id = p.canonical_id
  LEFT JOIN core.canonical_status_override o ON o.canonical_key = c.key
  WHERE (COALESCE(s.moves_money, 0)=1 OR lower(COALESCE(s.side_effect_class, '')) LIKE '%money%')
    AND COALESCE(COALESCE(o.moves_money, c.moves_money), 0)=0
`

export function auditSourceMoneyUnderclaimsCloudSnapshot({ dir = join(DOCS, 'capabilities-cloud') } = {}) {
  const provenanceDb = join(dir, 'cap-provenance.db')
  const coreDb = join(dir, 'cap-core.db')
  const db = openReadOnlyDatabase(provenanceDb)
  db.exec(`ATTACH DATABASE '${coreDb.replace(/'/g, "''")}' AS core`)

  let rows
  try {
    rows = db.prepare(QUERY).all()
  } finally {
    db.close()
  }

  const seen = new Set()
  const findings = []
  for (const row of rows) {
    const key = `${row.sourceId ?? 'unknown'}:${row.canonicalKey}`
    if (seen.has(key)) continue
    seen.add(key)
    findings.push({
      type: 'source_money_underclaimed',
      severity: truthyInt(row.canonicalRequiresApproval) ? 'medium' : 'high',
      canonicalKey: row.canonicalKey,
      canonicalName: row.canonicalName,
      canonicalMovesMoney: Number(row.canonicalMovesMoney ?? 0),
      canonicalRequiresApproval: row.canonicalRequiresApproval == null ? null : Number(row.canonicalRequiresApproval),
      canonicalStatus: row.canonicalStatus,
      canonicalDomain: row.canonicalDomain,
      canonicalTargetCrate: row.canonicalTargetCrate,
      sourceId: row.sourceId,
      donor: row.donor,
      sourceName: row.sourceName,
      sourceFiles: row.sourceFiles,
      sourceMovesMoney: Number(row.sourceMovesMoney ?? 0),
      sourceRequiresApproval: row.sourceRequiresApproval == null ? null : Number(row.sourceRequiresApproval),
      sourceSideEffectClass: row.sourceSideEffectClass,
    })
  }

  findings.sort(
    (a, b) => String(a.canonicalKey).localeCompare(String(b.canonicalKey)) || Number(a.sourceId ?? 0) - Number(b.sourceId ?? 0),
  )

  return {
    ok: findings.length === 0,
    truth_label: 'LOCAL_AUDIT_REQUIRED',
    authority: 'Cloud snapshot re-derivation of audit-truth.mjs::auditSourceMoneyUnderclaims; final capability/money truth requires local audit against docs/capabilities.db.',
    count: findings.length,
    findings,
  }
}

function usage() {
  return 'Usage: node tools/capabilities/audit-truth-cloud-snapshot.mjs [--dir <docs/capabilities-cloud>] [--fail-on-findings]'
}

function main() {
  let args
  try {
    args = parseArgs(process.argv.slice(2))
  } catch (error) {
    console.error(`${error.message}\n${usage()}`)
    process.exit(2)
  }
  if (args.help) {
    console.log(usage())
    return
  }
  try {
    const report = auditSourceMoneyUnderclaimsCloudSnapshot({ dir: args.dir })
    console.log(JSON.stringify(report, null, 2))
    if (args.failOnFindings && !report.ok) process.exit(1)
  } catch (error) {
    console.error(`audit-truth-cloud-snapshot failed: ${error.message}`)
    process.exit(2)
  }
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  main()
}
