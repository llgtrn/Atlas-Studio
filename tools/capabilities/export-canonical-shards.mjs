#!/usr/bin/env node
import Database from 'better-sqlite3'
import { existsSync, readdirSync, readFileSync } from 'node:fs'
import { join, relative } from 'node:path'
import { fileURLToPath } from 'node:url'
import { openReadOnlyDatabase } from '../db/sqlite-open.mjs'
import { computeCanonicalInputFingerprint } from './build-db-from-canonical-shards.mjs'
import {
  addUnique,
  boolFromDb,
  CAPABILITY_CANONICAL_DIR,
  cleanRequiredString,
  cleanString,
  emptyDirectory,
  loadCapabilityShards,
  normalizeArray,
  parseArgs,
  rel,
  sanitizeShardName,
  slash,
  uniqueStrings,
  writeJsonFile,
  writeJsonl,
} from '../canonical-shards/jsonl-lib.mjs'

// ── CANONICAL BASELINE FENCE + FIELD-BOUNDED PATCH (CONV0AR/CONV0AR2) ────────────────────────────
// Canonical JSONL (docs/capabilities-canonical/domains/*.jsonl) is truth; docs/capabilities.db is a
// derived staging cache that can lossily compress some fields even at build time (e.g. impl_evidence
// keeps only one code_ref/test_ref per capability -- see build-db-from-canonical-shards.mjs).
// Rewriting the WHOLE canonical tree -- or even a whole RECORD -- from that cache was never safe:
// it silently regenerated, and could corrupt, fields a promotion never touched (a real, repeated
// incident on `blocker` alone; see docs/_machine/agent-notes.md #301/#303 and record-blocker.mjs's own header
// comment, which named the mechanism without enforcing against it).
//
// The structural fix has two independent parts, both required (they solve different failure
// classes): (1) BASELINE FENCE -- the DB must prove it was built from EXACTLY the canonical corpus
// currently on disk, via build-db-from-canonical-shards.mjs's `canonical_input_sha256` meta
// fingerprint, recomputed here and required to match byte-for-byte; any canonical edit made after
// the DB was built, to any key or field, refuses before the first write. This protects against
// canonical drift AFTER the DB was built. (2) FIELD-BOUNDED PATCH -- `expectedChanges` is
// `{ "capability.key": ["field", ...] }`, the exact fields a call may take a new value for on that
// key; every other field on that key, and every field on every other key, is copied through from
// the current on-disk record VERBATIM. This protects against the DB's own lossy representation of
// fields a promotion never intended to touch, a defect that exists even AT build time. There is no
// "trust the whole DB" escape hatch: a deliberate bulk change must rebuild the DB from canonical
// JSONL, mutate its finite known key set, and export with exactly that key/field set declared.

// The canonical record's own field names (see makeRecord() below). capability_key/schema_version
// identify the record rather than describe mutable state, so they are never independently
// patchable -- a caller doesn't "authorize" identity, only real fields.
const CANONICAL_RECORD_FIELDS = new Set([
  'domain', 'canonical_name', 'target_crate', 'target_module', 'status', 'side_effect_class',
  'moves_money', 'requires_approval', 'acceptance_criteria', 'required_tests',
  'financial_control_test', 'acceptance_test', 'docs_refs', 'code_refs', 'test_refs',
  'architecture_refs', 'source_refs', 'evidence_refs', 'blocker',
])

// Pure and unit-testable: validates `expectedChanges` and returns { key -> Set<field> }. Fails
// closed on anything not a plain, known, once-declared field name -- no wildcard key, no wildcard
// field, no nested path, no blank/unknown field ever reaches the merge step.
export function parseExpectedChanges(expectedChanges) {
  const changesByKey = new Map()
  for (const [key, fields] of Object.entries(expectedChanges ?? {})) {
    if (typeof key !== 'string' || key.trim() === '') {
      throw new Error(`expectedChanges contains a blank/non-string key: ${JSON.stringify(key)}`)
    }
    if (!Array.isArray(fields) || fields.length === 0) {
      throw new Error(`expectedChanges["${key}"] must be a non-empty array of field names`)
    }
    const fieldSet = new Set()
    for (const field of fields) {
      if (typeof field !== 'string' || field.trim() === '') {
        throw new Error(`expectedChanges["${key}"] contains a blank/non-string field name: ${JSON.stringify(field)}`)
      }
      if (field === '*' || field.includes('.') || field.includes('[')) {
        throw new Error(`expectedChanges["${key}"] contains an unsupported field name (no wildcard, no nested path): ${JSON.stringify(field)}`)
      }
      if (!CANONICAL_RECORD_FIELDS.has(field)) {
        throw new Error(`expectedChanges["${key}"] declares an unknown canonical field: ${JSON.stringify(field)}`)
      }
      fieldSet.add(field) // an exact duplicate field name in the same list is harmlessly de-duplicated.
    }
    changesByKey.set(key, fieldSet)
  }
  return changesByKey
}

function tableExists(db, table) {
  return db.prepare("SELECT count(*) n FROM sqlite_master WHERE type='table' AND name=?").get(table).n > 0
}

function columns(db, table) {
  if (!tableExists(db, table)) return []
  return db.prepare(`PRAGMA table_info(${table})`).all().map((row) => row.name)
}

function tryAll(db, sql, params = []) {
  try {
    return db.prepare(sql).all(...params)
  } catch {
    return []
  }
}

function dbSource(root, requestedPath) {
  if (requestedPath) return requestedPath
  const local = join(root, 'docs', 'capabilities.db')
  if (existsSync(local)) return local
  const cloud = join(root, 'docs', 'capabilities-cloud', 'cap-core.db')
  if (existsSync(cloud)) return cloud
  throw new Error('No capability DB source found. Pass --db <path> or provide docs/capabilities-cloud/cap-core.db.')
}

function labelPath(root, path) {
  const normalized = slash(path)
  if (normalized.endsWith('/docs/capabilities.db') || normalized === 'docs/capabilities.db') return 'docs/capabilities.db'
  if (normalized.endsWith('/docs/architecture.db') || normalized === 'docs/architecture.db') return 'docs/architecture.db'
  const relativePath = slash(relative(root, path))
  return relativePath.startsWith('..') ? slash(path) : relativePath
}

function loadDocRefs(root, keys) {
  const refs = new Map()
  // Numbered docs live in up to five places: hand-authored prose in docs/,
  // docs/doctrines/ (the doctrine/invariant/operating corpus consolidated out of
  // docs/ root), or docs/benchmarks/ (benchmark methodology/doctrine), and every
  // machine-generated/hybrid doc (the ones capability keys are actually most likely
  // to be mentioned in — the cap-roadmap and crate-atlas docs, plus the hybrid
  // benchmark-gate docs) in docs/_generated/ or docs/crates/. Scan all five so
  // evidence_ref coverage doesn't silently drop when a doc migrates between them.
  const scanDirs = [join(root, 'docs'), join(root, 'docs', 'doctrines'), join(root, 'docs', '_generated'), join(root, 'docs', 'crates'), join(root, 'docs', 'benchmarks')]
  for (const docsDir of scanDirs) {
    if (!existsSync(docsDir)) continue
    for (const fileName of readdirSync(docsDir)) {
      if (!/^\d{3}-.*\.md$/.test(fileName)) continue
      const raw = readFileSync(join(docsDir, fileName), 'utf8')
      const relDir = relative(root, docsDir)
      for (const match of raw.matchAll(/`([a-z0-9][a-z0-9._-]*\.[a-z0-9][a-z0-9._-]*)`/gi)) {
        const key = match[1]
        if (!keys.has(key)) continue
        const list = refs.get(key) ?? []
        if (!list.includes(join(relDir, fileName).replaceAll('\\', '/'))) {
          list.push(join(relDir, fileName).replaceAll('\\', '/'))
        }
        refs.set(key, list)
      }
    }
  }
  return refs
}

function loadArchitectureRefs(root, keys, archDbPath) {
  const refs = new Map()
  if (!archDbPath || !existsSync(archDbPath)) return refs
  const arch = openReadOnlyDatabase(archDbPath)
  try {
    if (tableExists(arch, 'capability_architecture_link')) {
      for (const row of tryAll(arch, `SELECT capability_key, architecture_node, relationship, status
        FROM capability_architecture_link ORDER BY capability_key, relationship, architecture_node`)) {
        if (!keys.has(row.capability_key)) continue
        const list = refs.get(row.capability_key) ?? []
        addUnique(list, [`${row.relationship}:${row.architecture_node}${row.status ? `:${row.status}` : ''}`])
        refs.set(row.capability_key, list)
      }
    }
    if (tableExists(arch, 'architecture_target_gap')) {
      for (const row of tryAll(arch, `SELECT capability_key, gap_kind, suggested_architecture_node, target_crate, target_module, status
        FROM architecture_target_gap ORDER BY capability_key, gap_kind`)) {
        if (!keys.has(row.capability_key)) continue
        const suggested = row.suggested_architecture_node || row.target_crate || row.target_module || 'unowned'
        const list = refs.get(row.capability_key) ?? []
        addUnique(list, [`gap:${row.gap_kind}:${suggested}:${row.status || 'open'}`])
        refs.set(row.capability_key, list)
      }
    }
  } finally {
    arch.close()
  }
  return refs
}

function loadOverrides(db) {
  const overrides = new Map()
  if (!tableExists(db, 'canonical_status_override')) return overrides
  for (const row of tryAll(db, 'SELECT * FROM canonical_status_override ORDER BY canonical_key')) {
    overrides.set(row.canonical_key, row)
  }
  return overrides
}

function loadImplEvidence(db) {
  const evidence = new Map()
  if (!tableExists(db, 'impl_evidence')) return evidence
  for (const row of tryAll(db, 'SELECT * FROM impl_evidence ORDER BY canonical_key, impl_file, test_file, test_symbol')) {
    const list = evidence.get(row.canonical_key) ?? []
    list.push(row)
    evidence.set(row.canonical_key, list)
  }
  return evidence
}

function loadSourceRefs(db) {
  const refs = new Map()
  const canonicalColumns = columns(db, 'canonical_capability')
  if (!canonicalColumns.includes('id') || !tableExists(db, 'provenance') || !tableExists(db, 'source_capability')) {
    return refs
  }
  const sourceColumns = columns(db, 'source_capability')
  const hasRawRef = sourceColumns.includes('raw_ref')
  const rows = tryAll(db, `SELECT cc.key capability_key, s.id source_id, s.donor, s.module, s.canonical_name,
      s.source_files, s.source_symbols${hasRawRef ? ', s.raw_ref' : ''}
    FROM canonical_capability cc
    JOIN provenance p ON p.canonical_id = cc.id
    JOIN source_capability s ON s.id = p.source_id
    ORDER BY cc.key, s.donor, s.id`)
  for (const row of rows) {
    const list = refs.get(row.capability_key) ?? []
    // Prefer the verbatim ref captured on ingest (issue #2869): a large fraction of real
    // source_refs are freeform prose citations that parseRef() can't fully decompose, and
    // reconstructing from its (silently incomplete) parsed fields was overwriting real donor
    // citation text with a lossy `donor=unknown;...` synthetic stub. Only reconstruct when no
    // raw ref was captured (older DBs built before this column existed).
    const raw = cleanString(row.raw_ref)
    if (raw) {
      list.push(raw)
      refs.set(row.capability_key, list)
      continue
    }
    const parts = [
      `source_id=${row.source_id}`,
      `donor=${cleanRequiredString(row.donor, 'unknown')}`,
      `module=${cleanRequiredString(row.module, 'unknown')}`,
      `name=${cleanRequiredString(row.canonical_name, 'unknown')}`,
      `files=${cleanRequiredString(row.source_files, '')}`,
      `symbols=${cleanRequiredString(row.source_symbols, '')}`,
    ]
    list.push(parts.join(';'))
    refs.set(row.capability_key, list)
  }
  return refs
}

function makeRecord(row, override, implRows, refs) {
  const acceptanceTest = cleanString(override?.acceptance_test) ?? cleanString(row.acceptance_test)
  const financialControlTest = cleanString(override?.financial_control_test) ?? cleanString(row.financial_control_test)
  const requiredTests = normalizeArray(row.required_tests)
  if (acceptanceTest) addUnique(requiredTests, [acceptanceTest])
  if (financialControlTest && !/^required/i.test(financialControlTest)) addUnique(requiredTests, [financialControlTest])

  const codeRefs = []
  const testRefs = []
  const evidenceRefs = []
  for (const evidence of implRows) {
    if (evidence.impl_file) addUnique(codeRefs, [evidence.impl_symbols ? `${evidence.impl_file}#${evidence.impl_symbols}` : evidence.impl_file])
    if (evidence.test_file) addUnique(testRefs, [evidence.test_symbol ? `${evidence.test_file}#${evidence.test_symbol}` : evidence.test_file])
    addUnique(evidenceRefs, [
      evidence.donor_source ? `donor_source:${evidence.donor_source}` : null,
      evidence.verified_at ? `verified_at:${evidence.verified_at}` : null,
      evidence.verified_by ? `verified_by:${evidence.verified_by}` : null,
      evidence.notes ? `notes:${evidence.notes}` : null,
    ])
  }

  return {
    schema_version: 1,
    capability_key: cleanRequiredString(row.key),
    canonical_name: cleanRequiredString(row.canonical_name, row.key),
    domain: cleanRequiredString(row.domain, 'unknown'),
    target_crate: cleanString(row.target_crate),
    target_module: cleanString(row.target_module),
    status: cleanRequiredString(override?.status, cleanRequiredString(row.status, 'unimplemented')),
    side_effect_class: cleanRequiredString(row.side_effect_class, 'unknown'),
    moves_money: override?.moves_money == null ? boolFromDb(row.moves_money) : boolFromDb(override.moves_money),
    requires_approval: override?.requires_approval == null ? boolFromDb(row.requires_approval) : boolFromDb(override.requires_approval),
    acceptance_criteria: cleanRequiredString(row.acceptance_criteria, 'not yet specified'),
    required_tests: requiredTests,
    financial_control_test: financialControlTest,
    acceptance_test: acceptanceTest,
    docs_refs: refs.docs,
    code_refs: codeRefs,
    test_refs: testRefs,
    architecture_refs: refs.architecture,
    source_refs: refs.source,
    evidence_refs: evidenceRefs,
    blocker: cleanString(override?.blocker) ?? cleanString(row.blocker),
  }
}

export function exportCanonicalCapabilityShards({
  root = process.cwd(),
  dbPath = null,
  architectureDbPath = join(root, 'docs', 'architecture.db'),
  outDir = join(root, CAPABILITY_CANONICAL_DIR),
  // { "capability.key": ["field", ...] } -- the exact fields this export call may patch, per key,
  // from the DB. Every other field on a declared key, and every field on every undeclared key, is
  // preserved from the current on-disk canonical record. See parseExpectedChanges() above for the
  // validation rules (no wildcard key/field, no nested path, no blank/unknown field).
  expectedChanges = {},
} = {}) {
  const changesByKey = parseExpectedChanges(expectedChanges)
  const allowedKeys = new Set(changesByKey.keys())

  const { entries: existingEntries } = loadCapabilityShards(root)
  const existingByKey = new Map(existingEntries.map((entry) => [entry.record.capability_key, entry.record]))
  const currentFingerprint = computeCanonicalInputFingerprint([...existingByKey.values()])

  const sourceDbPath = dbSource(root, dbPath)
  const db = openReadOnlyDatabase(sourceDbPath)
  if (!tableExists(db, 'canonical_capability')) {
    db.close()
    throw new Error(`${labelPath(root, sourceDbPath)} has no canonical_capability table`)
  }
  const baselineRow = tableExists(db, 'meta') ? db.prepare("SELECT v FROM meta WHERE k='canonical_input_sha256'").get() : null
  if (!baselineRow || baselineRow.v !== currentFingerprint) {
    db.close()
    const error = new Error(
      `STALE_CAPABILITIES_DB_BASELINE_REFUSED: ${labelPath(root, sourceDbPath)}'s recorded canonical_input_sha256 ` +
      `(${baselineRow?.v ?? 'MISSING'}) does not match the current on-disk canonical corpus ` +
      `(${currentFingerprint}). The canonical shards changed (or this DB predates the baseline fingerprint) since ` +
      'this DB was built. Run tools/capabilities/build-db-from-canonical-shards.mjs to rebuild it from the ' +
      'current canonical shards, re-apply any intended DB mutation, and retry.'
    )
    error.code = 'STALE_CAPABILITIES_DB_BASELINE'
    throw error
  }

  // expectedChanges is a write AUTHORIZATION, not merely a staleness exception: a declared key must
  // already exist both in the current canonical corpus and in this DB -- no capability is created
  // through this path. (Every real caller already enforces "key exists in canonical_capability"
  // before calling this function; this is a second, structural check that does not trust the
  // caller.)
  for (const key of allowedKeys) {
    if (!existingByKey.has(key)) {
      db.close()
      throw new Error(`REFUSED: expectedChanges declares "${key}", which does not exist in the current canonical corpus -- capability creation is not authorized through this export path.`)
    }
  }
  const keyPlaceholders = [...allowedKeys].map(() => '?').join(',') || 'NULL'
  const changedRows = db.prepare(`SELECT * FROM canonical_capability WHERE key IN (${keyPlaceholders})`).all(...allowedKeys)
  if (changedRows.length !== allowedKeys.size) {
    const missing = [...allowedKeys].filter((key) => !changedRows.some((row) => row.key === key))
    db.close()
    throw new Error(`REFUSED: expectedChanges declares key(s) not found in ${labelPath(root, sourceDbPath)}'s canonical_capability table: ${missing.join(', ')}`)
  }

  // Fresh records are computed ONLY for declared keys -- an undeclared key's docs_refs/
  // architecture_refs/code_refs/etc. are never recomputed from the DB, docs/, or architecture.db,
  // so nothing about them can drift on an export that was never supposed to touch them.
  const overrides = loadOverrides(db)
  const implEvidence = loadImplEvidence(db)
  const sourceRefs = loadSourceRefs(db)
  const docsRefs = loadDocRefs(root, allowedKeys)
  const architectureRefs = loadArchitectureRefs(root, allowedKeys, architectureDbPath)
  const freshRecords = changedRows.map((row) => makeRecord(row, overrides.get(row.key), implEvidence.get(row.key) ?? [], {
    docs: docsRefs.get(row.key) ?? [],
    architecture: architectureRefs.get(row.key) ?? [],
    source: sourceRefs.get(row.key) ?? [],
  }))
  db.close()

  // The targeted patch itself: for a declared key, start from its EXISTING canonical record and
  // replace ONLY the explicitly authorized fields with the fresh DB-derived value -- every other
  // field on that same record, including any the DB represents lossily, is untouched. An undeclared
  // key's on-disk record is reused verbatim, whole.
  const mergedByKey = new Map(existingByKey)
  for (const fresh of freshRecords) {
    const key = fresh.capability_key
    const patched = { ...existingByKey.get(key) }
    for (const field of changesByKey.get(key)) patched[field] = fresh[field]
    mergedByKey.set(key, patched)
  }
  const records = [...mergedByKey.values()]

  const byDomain = new Map()
  for (const record of records) {
    const list = byDomain.get(record.domain) ?? []
    list.push(record)
    byDomain.set(record.domain, list)
  }

  // Only clear domains/ (what this function actually regenerates) -- outDir itself
  // (docs/capabilities-canonical/) also holds local-authority/, a separately-maintained,
  // manually-curated raw-table export (PR #2608, ~431k donor_file_census rows across 94
  // shards) that this function neither reads nor rewrites. Emptying the whole outDir here
  // used to silently delete that data on every `pnpm caps:canonical:export` run.
  //
  // Note (CONV0AR section 19, disclosed rather than silently claimed): this is still a
  // delete-then-rewrite of domains/, not a temp-directory-then-atomic-replace. Every validation
  // above (baseline fingerprint, key existence, merge construction) has already passed by this
  // point, so a crash here can only ever produce a partially-written domains/ tree, never a
  // silently-wrong one -- and the very next export attempt fails closed (the on-disk corpus no
  // longer matches any DB's recorded baseline). Full filesystem-level atomicity was not added in
  // this repair to stay within its authorized scope.
  const domainsDir = join(outDir, 'domains')
  emptyDirectory(domainsDir)
  const files = []
  for (const domain of [...byDomain.keys()].sort()) {
    const domainRecords = byDomain.get(domain).sort((a, b) => a.capability_key.localeCompare(b.capability_key))
    const file = join(domainsDir, `${sanitizeShardName(domain)}.jsonl`)
    writeJsonl(file, domainRecords)
    files.push(rel(root, file))
  }
  const meta = {
    schema_version: 1,
    authority: 'canonical_jsonl',
    generated_by: 'tools/capabilities/export-canonical-shards.mjs',
    source_db: labelPath(root, sourceDbPath),
    architecture_source_db: architectureDbPath && existsSync(architectureDbPath) ? labelPath(root, architectureDbPath) : null,
    record_count: records.length,
    domain_count: byDomain.size,
    verified_count: records.filter((record) => record.status === 'verified').length,
    implemented_unverified_count: records.filter((record) => record.status === 'implemented_unverified').length,
    money_count: records.filter((record) => record.moves_money).length,
    files,
  }
  writeJsonFile(join(outDir, 'meta.json'), meta)

  // Advance the DB's recorded baseline to the NEW canonical corpus only now that the canonical
  // write above has actually succeeded (CONV0AR section 6). This is what lets a later rollback
  // re-export of the same declared key see a matching baseline and patch it back; a crash between
  // the write above and this update instead fails the NEXT export closed, never silently.
  if (existsSync(sourceDbPath)) {
    const writableDb = new Database(sourceDbPath)
    try {
      if (tableExists(writableDb, 'meta')) {
        writableDb.prepare("INSERT INTO meta (k, v) VALUES ('canonical_input_sha256', ?) ON CONFLICT(k) DO UPDATE SET v=excluded.v")
          .run(computeCanonicalInputFingerprint(records))
      }
    } finally {
      writableDb.close()
    }
  }

  return meta
}

function main() {
  const args = parseArgs(process.argv.slice(2))
  const root = process.cwd()
  const result = exportCanonicalCapabilityShards({
    root,
    dbPath: args.db ? String(args.db) : null,
    architectureDbPath: args['architecture-db'] ? String(args['architecture-db']) : join(root, 'docs', 'architecture.db'),
    outDir: args.out ? String(args.out) : join(root, CAPABILITY_CANONICAL_DIR),
    // --expected-changes '{"capability.key": ["status", "blocker"]}'
    expectedChanges: args['expected-changes'] ? JSON.parse(String(args['expected-changes'])) : {},
  })
  console.log(JSON.stringify(result, null, 2))
}

if (process.argv[1] && fileURLToPath(import.meta.url) === process.argv[1]) {
  main()
}
