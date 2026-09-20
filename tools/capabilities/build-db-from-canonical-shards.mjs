#!/usr/bin/env node
import Database from 'better-sqlite3'
import { createHash } from 'node:crypto'
import { mkdirSync, rmSync } from 'node:fs'
import { dirname, join } from 'node:path'
import { fileURLToPath } from 'node:url'
import {
  intFromBool,
  loadCapabilityShards,
  makeTempDbPath,
  parseArgs,
  rel,
} from '../canonical-shards/jsonl-lib.mjs'
import { verifyCanonicalCapabilityShards } from './verify-canonical-shards.mjs'

// ── CANONICAL BASELINE FINGERPRINT (CONV0AR) ─────────────────────────────────────────────────────
// A deterministic, content-only digest of the semantic canonical capability corpus this DB was
// built from: sort records by capability_key, recursively sort object keys (arrays keep their
// original order -- array order is semantic, e.g. required_tests/code_refs ordering), serialize,
// SHA-256. No filesystem timestamps, SQLite row ids, temp paths, or wall clock enter it, so two
// builds from byte-identical canonical JSONL always agree. export-canonical-shards.mjs recomputes
// this over whatever is CURRENTLY on disk and requires exact equality with the value stored here
// before it is allowed to treat this DB as a valid source for any targeted patch -- see its
// STALE_CAPABILITIES_DB_BASELINE_REFUSED guard.
function canonicalStringify(value) {
  if (Array.isArray(value)) return `[${value.map(canonicalStringify).join(',')}]`
  if (value && typeof value === 'object') {
    return `{${Object.keys(value).sort().map((k) => `${JSON.stringify(k)}:${canonicalStringify(value[k])}`).join(',')}}`
  }
  return JSON.stringify(value ?? null)
}

export function computeCanonicalInputFingerprint(records) {
  const sorted = [...records].sort((a, b) => a.capability_key.localeCompare(b.capability_key))
  const hash = createHash('sha256')
  for (const record of sorted) hash.update(canonicalStringify(record)).update('\n')
  return hash.digest('hex')
}

function createSchema(db) {
  db.exec(`
    PRAGMA journal_mode = DELETE;
    CREATE TABLE meta (
      k TEXT PRIMARY KEY,
      v TEXT NOT NULL
    );
    CREATE TABLE canonical_capability (
      id INTEGER PRIMARY KEY,
      key TEXT NOT NULL UNIQUE,
      canonical_name TEXT NOT NULL,
      domain TEXT NOT NULL,
      target_crate TEXT,
      target_module TEXT,
      side_effect_class TEXT NOT NULL,
      moves_money INTEGER NOT NULL DEFAULT 0,
      requires_approval INTEGER NOT NULL DEFAULT 0,
      acceptance_criteria TEXT NOT NULL,
      required_tests TEXT,
      financial_control_test TEXT,
      acceptance_test TEXT,
      status TEXT NOT NULL,
      exclusion_note TEXT,
      blocker TEXT,
      slice INTEGER,
      donor_count INTEGER NOT NULL DEFAULT 0
    );
    CREATE TABLE canonical_status_override (
      canonical_key TEXT PRIMARY KEY,
      status TEXT,
      acceptance_test TEXT,
      financial_control_test TEXT,
      moves_money INTEGER,
      requires_approval INTEGER,
      blocker TEXT,
      set_at TEXT,
      set_by TEXT
    );
    CREATE TABLE impl_evidence (
      canonical_key TEXT PRIMARY KEY,
      impl_file TEXT,
      impl_symbols TEXT,
      test_file TEXT,
      test_symbol TEXT,
      donor_source TEXT,
      verified_at TEXT,
      verified_by TEXT,
      notes TEXT
    );
    CREATE TABLE source_capability (
      id INTEGER PRIMARY KEY,
      donor TEXT,
      module TEXT,
      canonical_name TEXT,
      source_files TEXT,
      source_symbols TEXT,
      business_behavior TEXT,
      technical_behavior TEXT,
      inputs TEXT,
      outputs TEXT,
      persistence TEXT,
      surface TEXT,
      side_effect_class TEXT,
      moves_money INTEGER,
      requires_approval INTEGER,
      external_services TEXT,
      target_crate TEXT,
      target_module TEXT,
      canonical_id INTEGER,
      created_pass TEXT,
      raw_ref TEXT
    );
    CREATE TABLE provenance (
      source_id INTEGER NOT NULL,
      canonical_id INTEGER NOT NULL,
      PRIMARY KEY (source_id, canonical_id)
    );
    CREATE INDEX idx_canonical_capability_key ON canonical_capability(key);
    CREATE INDEX idx_canonical_capability_domain ON canonical_capability(domain);
    CREATE INDEX idx_canonical_capability_status ON canonical_capability(status);
    CREATE INDEX idx_source_capability_canonical_id ON source_capability(canonical_id);
  `)
}

function parseRef(ref) {
  const out = {}
  for (const part of String(ref).split(';')) {
    const index = part.indexOf('=')
    if (index < 0) continue
    out[part.slice(0, index)] = part.slice(index + 1)
  }
  return out
}

function splitSymbolRef(ref) {
  const [file, ...rest] = String(ref).split('#')
  return { file: file || null, symbol: rest.join('#') || null }
}

function evidenceValue(refs, prefix) {
  const match = refs.find((ref) => String(ref).startsWith(prefix))
  return match ? String(match).slice(prefix.length) : null
}

export function buildCapabilityDbFromCanonicalShards({
  root = process.cwd(),
  outPath = null,
} = {}) {
  const verify = verifyCanonicalCapabilityShards({ root })
  if (!verify.ok) {
    const error = new Error('canonical capability shard verification failed')
    error.details = verify
    throw error
  }

  const temp = outPath ? null : makeTempDbPath('chronica-capabilities-cache-', 'capabilities.db')
  const dbPath = outPath ?? temp.path
  mkdirSync(dirname(dbPath), { recursive: true })
  rmSync(dbPath, { force: true })
  const db = new Database(dbPath)
  createSchema(db)

  const { entries, meta } = loadCapabilityShards(root)
  const records = entries.map((entry) => entry.record).sort((a, b) => a.capability_key.localeCompare(b.capability_key))

  const insertMeta = db.prepare('INSERT INTO meta (k, v) VALUES (?, ?)')
  const insertCapability = db.prepare(`INSERT INTO canonical_capability
    (id, key, canonical_name, domain, target_crate, target_module, side_effect_class, moves_money,
      requires_approval, acceptance_criteria, required_tests, financial_control_test, acceptance_test,
      status, exclusion_note, blocker, slice, donor_count)
    VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)`)
  const insertImpl = db.prepare(`INSERT INTO impl_evidence
    (canonical_key, impl_file, impl_symbols, test_file, test_symbol, donor_source, verified_at, verified_by, notes)
    VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)`)
  const insertSource = db.prepare(`INSERT OR IGNORE INTO source_capability
    (id, donor, module, canonical_name, source_files, source_symbols, business_behavior, technical_behavior,
      inputs, outputs, persistence, surface, side_effect_class, moves_money, requires_approval,
      external_services, target_crate, target_module, canonical_id, created_pass, raw_ref)
    VALUES (?, ?, ?, ?, ?, ?, NULL, NULL, NULL, NULL, NULL, NULL, ?, ?, ?, NULL, ?, ?, ?, ?, ?)`)
  const insertProvenance = db.prepare('INSERT OR IGNORE INTO provenance (source_id, canonical_id) VALUES (?, ?)')

  const sourceIds = new Map()
  let nextSourceId = 1
  const sourceIdFor = (rawRef) => {
    const parsed = parseRef(rawRef)
    const requested = Number(parsed.source_id)
    if (Number.isInteger(requested) && requested > 0 && ![...sourceIds.values()].includes(requested)) {
      sourceIds.set(rawRef, requested)
      nextSourceId = Math.max(nextSourceId, requested + 1)
      return requested
    }
    if (!sourceIds.has(rawRef)) sourceIds.set(rawRef, nextSourceId++)
    return sourceIds.get(rawRef)
  }

  const tx = db.transaction(() => {
    insertMeta.run('schema_version', '1')
    insertMeta.run('authority', 'generated_cache_from_canonical_jsonl')
    insertMeta.run('generator', 'tools/capabilities/build-db-from-canonical-shards.mjs')
    insertMeta.run('canonical_record_count', String(records.length))
    insertMeta.run('canonical_input_sha256', computeCanonicalInputFingerprint(records))
    insertMeta.run('source_meta_record_count', String(meta?.record_count ?? records.length))

    records.forEach((record, index) => {
      const canonicalId = index + 1
      insertCapability.run(
        canonicalId,
        record.capability_key,
        record.canonical_name,
        record.domain,
        record.target_crate,
        record.target_module,
        record.side_effect_class,
        intFromBool(record.moves_money),
        intFromBool(record.requires_approval),
        record.acceptance_criteria,
        JSON.stringify(record.required_tests ?? []),
        record.financial_control_test,
        record.acceptance_test,
        record.status,
        record.status === 'excluded' ? record.blocker : null,
        record.blocker,
        null,
        Array.isArray(record.source_refs) ? record.source_refs.length : 0,
      )

      if ((record.code_refs?.length ?? 0) || (record.test_refs?.length ?? 0) || (record.evidence_refs?.length ?? 0)) {
        const code = splitSymbolRef(record.code_refs?.[0] ?? '')
        const test = splitSymbolRef(record.test_refs?.[0] ?? '')
        const donorSource = evidenceValue(record.evidence_refs ?? [], 'donor_source:')
        const verifiedAt = evidenceValue(record.evidence_refs ?? [], 'verified_at:')
        const verifiedBy = evidenceValue(record.evidence_refs ?? [], 'verified_by:')
        const notesValue = evidenceValue(record.evidence_refs ?? [], 'notes:')
        const otherRefs = (record.evidence_refs ?? []).filter((ref) => !/^(donor_source|verified_at|verified_by|notes):/.test(ref))
        const notes = [notesValue, ...otherRefs].filter(Boolean).join(' | ') || null
        insertImpl.run(record.capability_key, code.file, code.symbol, test.file, test.symbol, donorSource, verifiedAt, verifiedBy, notes)
      }

      for (const sourceRef of record.source_refs ?? []) {
        const parsed = parseRef(sourceRef)
        const sourceId = sourceIdFor(sourceRef)
        insertSource.run(
          sourceId,
          parsed.donor || null,
          parsed.module || null,
          parsed.name || record.canonical_name,
          parsed.files || null,
          parsed.symbols || null,
          record.side_effect_class,
          intFromBool(record.moves_money),
          intFromBool(record.requires_approval),
          record.target_crate,
          record.target_module,
          canonicalId,
          'canonical_jsonl',
          // Preserve the exact source-shard string verbatim (issue #2869): a large fraction of
          // real source_refs are freeform prose citations, not the strict `key=value;...`
          // micro-format parseRef() understands. Reconstructing from the (silently incomplete)
          // parsed fields on export was destroying real citation text. The exporter now prefers
          // this raw column and only falls back to reconstruction when it's absent.
          String(sourceRef),
        )
        insertProvenance.run(sourceId, canonicalId)
      }
    })
  })
  tx()
  const counts = {
    canonical_capability: db.prepare('SELECT count(*) n FROM canonical_capability').get().n,
    impl_evidence: db.prepare('SELECT count(*) n FROM impl_evidence').get().n,
    source_capability: db.prepare('SELECT count(*) n FROM source_capability').get().n,
    provenance: db.prepare('SELECT count(*) n FROM provenance').get().n,
  }
  db.close()
  return { dbPath, tempDir: temp?.dir ?? null, counts }
}

function main() {
  const args = parseArgs(process.argv.slice(2))
  try {
    const result = buildCapabilityDbFromCanonicalShards({
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
