#!/usr/bin/env node
// audit-truth.mjs — read-only business/evidence truth audit across capabilities.db and architecture.db.
//
// This is intentionally narrower than the existing dangling-symbol/count verifiers. It looks for
// contradictions that can make a "verified" or "safe" tracking row untrue:
//   - donor/source rows move money but the canonical capability says moves_money=0
//   - verified proving tests are ignored with #[ignore]
//   - verified evidence records the proving test as the implementation symbol
//   - architecture evidence_ref values point at missing local repo files
//
// Usage:
//   node tools/capabilities/audit-truth.mjs
//   node tools/capabilities/audit-truth.mjs --fail-on-findings
//   node tools/capabilities/audit-truth.mjs --root <repo> --capabilities-db <path> --architecture-db <path>
import Database from 'better-sqlite3'
import { existsSync, readFileSync } from 'node:fs'
import { isAbsolute, join, normalize, relative, resolve, sep } from 'node:path'
import { fileURLToPath } from 'node:url'
import { ARCHITECTURE_DB, CAPABILITIES_DB } from '../_paths.mjs'

const FINDING_TYPES = [
  'architecture_missing_file_ref',
  'ignored_verified_test',
  'source_money_underclaimed',
  'test_as_implementation',
]

function usage() {
  return `Usage: node tools/capabilities/audit-truth.mjs [--root <repo>] [--capabilities-db <path>] [--architecture-db <path>] [--fail-on-findings]`
}

function parseArgs(argv) {
  const out = {
    root: process.cwd(),
    capabilitiesDb: null,
    architectureDb: null,
    failOnFindings: false,
  }
  for (let i = 0; i < argv.length; i += 1) {
    const arg = argv[i]
    if (arg === '--help' || arg === '-h') return { help: true }
    if (arg === '--fail-on-findings') {
      out.failOnFindings = true
      continue
    }
    if (arg === '--root' || arg === '--capabilities-db' || arg === '--architecture-db') {
      const value = argv[i + 1]
      if (!value || value.startsWith('--')) throw new Error(`${arg} requires a value`)
      if (arg === '--root') out.root = value
      if (arg === '--capabilities-db') out.capabilitiesDb = value
      if (arg === '--architecture-db') out.architectureDb = value
      i += 1
      continue
    }
    throw new Error(`unknown argument: ${arg}`)
  }
  out.root = resolve(out.root)
  out.capabilitiesDb = resolve(out.capabilitiesDb ?? (out.root === process.cwd() ? CAPABILITIES_DB : join(out.root, 'docs', 'capabilities.db')))
  out.architectureDb = resolve(out.architectureDb ?? (out.root === process.cwd() ? ARCHITECTURE_DB : join(out.root, 'docs', 'architecture.db')))
  return out
}

function openReadonly(dbPath) {
  return new Database(dbPath, { readonly: true, fileMustExist: true })
}

function hasTable(db, name) {
  return db.prepare("SELECT count(*) n FROM sqlite_master WHERE type IN ('table','view') AND name=?").get(name).n > 0
}

function columnNames(db, table) {
  if (!hasTable(db, table)) return new Set()
  return new Set(db.prepare(`PRAGMA table_info(${quoteIdent(table)})`).all().map((c) => c.name))
}

function quoteIdent(name) {
  if (!/^[A-Za-z_][A-Za-z0-9_]*$/.test(name)) throw new Error(`unsafe sqlite identifier: ${name}`)
  return `"${name}"`
}

function sqlCol(cols, alias, name, fallback) {
  return cols.has(name) ? `${alias}.${quoteIdent(name)}` : fallback
}

function truthyInt(value) {
  return Number(value ?? 0) === 1
}

function splitSymbols(raw) {
  return String(raw ?? '')
    .split(',')
    .map((s) => s.trim())
    .filter(Boolean)
}

function slash(path) {
  return String(path).split(sep).join('/')
}

function resolveRepoPath(root, path) {
  if (!path) return null
  return isAbsolute(path) ? normalize(path) : normalize(join(root, path))
}

function escapeRegex(value) {
  return String(value).replace(/[.*+?^${}()|[\]\\]/g, '\\$&')
}

function directlyIgnoredAttribute(line) {
  return /^#\s*\[\s*ignore(?:\s*(?:=|\]|\(|,)|$)/.test(line) || /^#\s*\[\s*cfg_attr\([^,\]]+,\s*ignore\b/.test(line)
}

function ignoredNearTestSymbol(source, symbol) {
  if (!symbol) return false
  const match = new RegExp(`\\bfn\\s+${escapeRegex(symbol)}\\b`).exec(source)
  if (!match) return false
  const lines = source.split(/\r?\n/)
  const lineIndex = source.slice(0, match.index).split(/\r?\n/).length - 1
  const attrs = []
  for (let i = lineIndex - 1; i >= Math.max(0, lineIndex - 8); i -= 1) {
    const line = lines[i].trim()
    if (!line || line.startsWith('//')) continue
    if (line.startsWith('#[')) {
      attrs.unshift(line)
      continue
    }
    break
  }
  return attrs.some(directlyIgnoredAttribute)
}

function capStatusExpr(db) {
  if (hasTable(db, 'canonical_status_override')) {
    return {
      join: 'LEFT JOIN canonical_status_override o ON o.canonical_key=c.key',
      expr: 'COALESCE(o.status, c.status)',
    }
  }
  return { join: '', expr: 'c.status' }
}

function auditSourceMoneyUnderclaims(db) {
  if (!hasTable(db, 'canonical_capability') || !hasTable(db, 'source_capability')) return []

  const canonicalCols = columnNames(db, 'canonical_capability')
  const sourceCols = columnNames(db, 'source_capability')
  const canJoinDirect = canonicalCols.has('id') && sourceCols.has('canonical_id')
  const canJoinProvenance = canonicalCols.has('id') && sourceCols.has('id') && hasTable(db, 'provenance')
  if (!canonicalCols.has('key') || !canonicalCols.has('moves_money') || (!canJoinDirect && !canJoinProvenance)) return []

  const hasOverrides = hasTable(db, 'canonical_status_override')
  const overrideJoin = hasOverrides ? 'LEFT JOIN canonical_status_override o ON o.canonical_key=c.key' : ''
  const canonicalMoves = hasOverrides ? 'COALESCE(o.moves_money, c.moves_money)' : 'c.moves_money'
  const canonicalRequiresBase = sqlCol(canonicalCols, 'c', 'requires_approval', 'NULL')
  const canonicalRequires = hasOverrides ? `COALESCE(o.requires_approval, ${canonicalRequiresBase})` : canonicalRequiresBase
  const canonicalName = sqlCol(canonicalCols, 'c', 'canonical_name', 'NULL')
  const sourceDonor = sqlCol(sourceCols, 's', 'donor', 'NULL')
  const sourceName = sqlCol(sourceCols, 's', 'canonical_name', 'NULL')
  const sourceFiles = sqlCol(sourceCols, 's', 'source_files', 'NULL')
  const sourceMoves = sqlCol(sourceCols, 's', 'moves_money', '0')
  const sourceRequires = sqlCol(sourceCols, 's', 'requires_approval', 'NULL')
  const sourceSideEffect = sqlCol(sourceCols, 's', 'side_effect_class', 'NULL')
  const sourceId = sqlCol(sourceCols, 's', 'id', 'NULL')

  const select = (joinSql) => `
    SELECT
      ${sourceId} AS sourceId,
      ${sourceDonor} AS donor,
      ${sourceName} AS sourceName,
      ${sourceFiles} AS sourceFiles,
      ${sourceMoves} AS sourceMovesMoney,
      ${sourceRequires} AS sourceRequiresApproval,
      ${sourceSideEffect} AS sourceSideEffectClass,
      c.key AS canonicalKey,
      ${canonicalName} AS canonicalName,
      ${canonicalMoves} AS canonicalMovesMoney,
      ${canonicalRequires} AS canonicalRequiresApproval
    FROM source_capability s
    ${joinSql}
    ${overrideJoin}
    WHERE (COALESCE(${sourceMoves}, 0)=1 OR lower(COALESCE(${sourceSideEffect}, '')) LIKE '%money%')
      AND COALESCE(${canonicalMoves}, 0)=0
  `

  const rows = []
  if (canJoinDirect) rows.push(...db.prepare(select('JOIN canonical_capability c ON c.id=s.canonical_id')).all())
  if (canJoinProvenance) {
    rows.push(...db.prepare(select(`
      JOIN provenance p ON p.source_id=s.id
      JOIN canonical_capability c ON c.id=p.canonical_id
    `)).all())
  }

  const seen = new Set()
  return rows
    .filter((row) => {
      const key = `${row.sourceId ?? 'unknown'}:${row.canonicalKey}`
      if (seen.has(key)) return false
      seen.add(key)
      return true
    })
    .map((row) => ({
      type: 'source_money_underclaimed',
      severity: truthyInt(row.canonicalRequiresApproval) ? 'medium' : 'high',
      canonicalKey: row.canonicalKey,
      canonicalName: row.canonicalName,
      canonicalMovesMoney: Number(row.canonicalMovesMoney ?? 0),
      canonicalRequiresApproval: row.canonicalRequiresApproval == null ? null : Number(row.canonicalRequiresApproval),
      sourceId: row.sourceId,
      donor: row.donor,
      sourceName: row.sourceName,
      sourceFiles: row.sourceFiles,
      sourceMovesMoney: Number(row.sourceMovesMoney ?? 0),
      sourceRequiresApproval: row.sourceRequiresApproval == null ? null : Number(row.sourceRequiresApproval),
      sourceSideEffectClass: row.sourceSideEffectClass,
    }))
    .sort((a, b) => String(a.canonicalKey).localeCompare(String(b.canonicalKey)) || Number(a.sourceId ?? 0) - Number(b.sourceId ?? 0))
}

function auditImplEvidence(db, root) {
  const ignored = []
  const testAsImplementation = []
  if (!hasTable(db, 'impl_evidence') || !hasTable(db, 'canonical_capability')) {
    return { ignored, testAsImplementation }
  }
  const status = capStatusExpr(db)
  const rows = db.prepare(`
    SELECT
      e.canonical_key AS canonicalKey,
      e.impl_file AS implFile,
      e.impl_symbols AS implSymbols,
      e.test_file AS testFile,
      e.test_symbol AS testSymbol,
      ${status.expr} AS effectiveStatus
    FROM impl_evidence e
    JOIN canonical_capability c ON c.key=e.canonical_key
    ${status.join}
    WHERE ${status.expr}='verified'
    ORDER BY e.canonical_key
  `).all()

  const fileCache = new Map()
  const read = (path) => {
    const full = resolveRepoPath(root, path)
    if (!full || !existsSync(full)) return null
    if (!fileCache.has(full)) fileCache.set(full, readFileSync(full, 'utf8'))
    return fileCache.get(full)
  }

  for (const row of rows) {
    const symbols = splitSymbols(row.implSymbols)
    if (symbols.length === 1 && symbols[0] === row.testSymbol) {
      testAsImplementation.push({
        type: 'test_as_implementation',
        severity: 'high',
        canonicalKey: row.canonicalKey,
        implFile: row.implFile,
        implSymbols: row.implSymbols,
        testFile: row.testFile,
        testSymbol: row.testSymbol,
      })
    }

    const source = read(row.testFile)
    if (source && ignoredNearTestSymbol(source, row.testSymbol)) {
      ignored.push({
        type: 'ignored_verified_test',
        severity: 'high',
        canonicalKey: row.canonicalKey,
        testFile: row.testFile,
        testSymbol: row.testSymbol,
      })
    }
  }
  return { ignored, testAsImplementation }
}

function splitEvidenceRefs(raw) {
  const value = String(raw ?? '').trim()
  if (!value) return []
  return value
    .split(/[;,]\s*/)
    .map((v) => v.trim())
    .filter(Boolean)
}

function normalizeEvidenceRef(raw) {
  let ref = String(raw ?? '').trim()
  ref = ref.replace(/^['"`]+|['"`]+$/g, '')
  ref = ref.replace(/#.*$/, '')
  ref = ref.replace(/:(\d+)(?::\d+)?$/, '')
  if (ref.startsWith('./')) ref = ref.slice(2)
  return ref
}

function looksLikeLocalRef(ref) {
  if (!ref) return false
  if (/^[a-z][a-z0-9+.-]*:\/\//i.test(ref)) return false
  if (/^(architecture_invariant|capabilities\.db|canonical_capability|capability):/i.test(ref)) return false
  if (/^[A-Za-z]:[\\/]/.test(ref)) return true
  if (ref.startsWith('.') || ref.startsWith('/') || ref.includes('\\')) return true
  if (/^(docs|crates|tools|ui|specs|Temporary|legacy)\//.test(ref)) return true
  return ref.includes('/') && !/^[a-z][a-z0-9+.-]*:/i.test(ref)
}

function auditArchitectureMissingFileRefs(db, root) {
  const sources = [
    { table: 'architecture_evidence', id: 'id', ref: 'evidence_ref', owner: 'architecture_node' },
    { table: 'architecture_node', id: 'id', ref: 'evidence_ref', owner: 'id' },
    { table: 'architecture_node', id: 'id', ref: 'file_path', owner: 'id' },
    { table: 'architecture_edge', id: 'id', ref: 'evidence_ref', owner: 'kind' },
    { table: 'architecture_target_gap', id: 'capability_key', ref: 'evidence_ref', owner: 'capability_key' },
    { table: 'planned_architecture_node', id: 'id', ref: 'evidence_ref', owner: 'id' },
  ]
  const findings = []
  const seen = new Set()

  for (const source of sources) {
    if (!hasTable(db, source.table)) continue
    const cols = columnNames(db, source.table)
    if (!cols.has(source.ref)) continue
    const idExpr = cols.has(source.id) ? quoteIdent(source.id) : 'NULL'
    const ownerExpr = cols.has(source.owner) ? quoteIdent(source.owner) : 'NULL'
    const rows = db.prepare(`SELECT ${idExpr} AS rowId, ${ownerExpr} AS owner, ${quoteIdent(source.ref)} AS evidenceRef FROM ${quoteIdent(source.table)} ORDER BY rowId`).all()
    for (const row of rows) {
      for (const rawRef of splitEvidenceRefs(row.evidenceRef)) {
        const ref = normalizeEvidenceRef(rawRef)
        if (!looksLikeLocalRef(ref)) continue
        const full = resolveRepoPath(root, ref)
        if (!full || existsSync(full)) continue
        const key = `${source.table}:${row.rowId}:${source.ref}:${ref}`
        if (seen.has(key)) continue
        seen.add(key)
        findings.push({
          type: 'architecture_missing_file_ref',
          severity: ref.startsWith('docs/') ? 'high' : 'medium',
          table: source.table,
          rowId: row.rowId,
          owner: row.owner,
          column: source.ref,
          evidenceRef: ref,
          resolvedPath: slash(relative(root, full)),
        })
      }
    }
  }
  return findings.sort((a, b) =>
    String(a.evidenceRef).localeCompare(String(b.evidenceRef)) ||
    String(a.table).localeCompare(String(b.table)) ||
    String(a.rowId).localeCompare(String(b.rowId)),
  )
}

export function auditTruth({
  root = process.cwd(),
  capabilitiesDb = join(root, 'docs', 'capabilities.db'),
  architectureDb = join(root, 'docs', 'architecture.db'),
} = {}) {
  const absRoot = resolve(root)
  const findings = {
    architecture_missing_file_ref: [],
    ignored_verified_test: [],
    source_money_underclaimed: [],
    test_as_implementation: [],
  }

  const caps = openReadonly(resolve(capabilitiesDb))
  try {
    findings.source_money_underclaimed = auditSourceMoneyUnderclaims(caps)
    const impl = auditImplEvidence(caps, absRoot)
    findings.ignored_verified_test = impl.ignored
    findings.test_as_implementation = impl.testAsImplementation
  } finally {
    caps.close()
  }

  const arch = openReadonly(resolve(architectureDb))
  try {
    findings.architecture_missing_file_ref = auditArchitectureMissingFileRefs(arch, absRoot)
  } finally {
    arch.close()
  }

  const byType = {}
  let totalFindings = 0
  for (const type of FINDING_TYPES) {
    byType[type] = findings[type].length
    totalFindings += findings[type].length
  }

  return {
    ok: totalFindings === 0,
    root: absRoot,
    capabilitiesDb: resolve(capabilitiesDb),
    architectureDb: resolve(architectureDb),
    counts: {
      totalFindings,
      byType,
    },
    findings,
  }
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
    const report = auditTruth(args)
    console.log(JSON.stringify(report, null, 2))
    if (args.failOnFindings && !report.ok) process.exit(1)
  } catch (error) {
    console.error(`audit-truth failed: ${error.message}`)
    process.exit(2)
  }
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  main()
}
