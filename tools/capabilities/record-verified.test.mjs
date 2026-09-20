import test from 'node:test'
import assert from 'node:assert/strict'
import Database from 'better-sqlite3'
import { execFileSync } from 'node:child_process'
import { existsSync, mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { dirname, join } from 'node:path'
import { fileURLToPath } from 'node:url'
import { computeCanonicalInputFingerprint } from './build-db-from-canonical-shards.mjs'

const here = dirname(fileURLToPath(import.meta.url))
const script = join(here, 'record-verified.mjs')

// Minimal real Cargo workspace so the DB-REBUILD AUTOMATION step (subdb:gen/verify, which does a
// real Cargo-workspace discovery scan) can run against a synthetic test root. Callers create
// crates/<crateName>/src themselves (the impl/test fixture file usually lives there already).
function writeMinimalCargoWorkspace(root, crateName) {
  writeFileSync(join(root, 'Cargo.toml'), `[workspace]\nmembers = ["crates/${crateName}"]\n`)
  writeFileSync(join(root, 'crates', crateName, 'Cargo.toml'), `[package]\nname = "${crateName}"\nversion = "0.1.0"\n`)
}

// CONV0AR: exportCanonicalCapabilityShards() now requires the DB to carry a canonical_input_sha256
// baseline fingerprint matching the on-disk canonical corpus (see export-canonical-shards.mjs's
// CANONICAL BASELINE FENCE and build-db-from-canonical-shards.mjs's computeCanonicalInputFingerprint)
// before it will patch ANY declared key. These hand-built test DBs predate that concept, so each
// test that exercises the hasCanonical export path must also seed a matching canonical JSONL
// fixture and record its fingerprint in a `meta` table.
//
// CONV0AR2: the export is now also FIELD-bounded, not just key-bounded -- for the key under test,
// only record-verified.mjs's own authorized fields (see its `authorizedFields`) are patched from
// the fresh DB record; every other field is copied straight from this seeded placeholder. So the
// placeholder can no longer be a bare stub: it must already be a complete, schema-valid record
// (matching the test's own canonical_capability INSERT, notably target_crate, which record-verified
// never writes and so must come from here) for the post-promotion result to pass
// verify-canonical-shards.mjs's structural checks.
function seedCanonicalBaselineAndFingerprint(root, dbPathOrDb, keys) {
  // keys: array of { key, domain, ...recordOverrides }. CONV0AR2's field-bounded merge preserves
  // every field NOT in record-verified.mjs's authorized field list straight from this seeded
  // record, so it must already be a complete, schema-valid, pre-promotion record (matching the
  // test's own canonical_capability INSERT) -- not just a bare {capability_key, domain} stub.
  const byDomain = new Map()
  for (const { key, domain, ...overrides } of keys) {
    const list = byDomain.get(domain) ?? []
    list.push({
      schema_version: 1,
      capability_key: key,
      canonical_name: key,
      domain,
      target_crate: null,
      target_module: null,
      status: 'implemented_unverified',
      side_effect_class: 'internal_write',
      moves_money: false,
      requires_approval: false,
      acceptance_criteria: 'test',
      required_tests: [],
      financial_control_test: null,
      acceptance_test: null,
      docs_refs: [],
      code_refs: [],
      test_refs: [],
      architecture_refs: [],
      source_refs: [],
      evidence_refs: [],
      blocker: null,
      ...overrides,
    })
    byDomain.set(domain, list)
  }
  const dir = join(root, 'docs', 'capabilities-canonical', 'domains')
  mkdirSync(dir, { recursive: true })
  const allRecords = []
  for (const [domain, records] of byDomain) {
    const sorted = [...records].sort((a, b) => a.capability_key.localeCompare(b.capability_key))
    writeFileSync(join(dir, `${domain}.jsonl`), sorted.map((r) => JSON.stringify(r)).join('\n') + '\n')
    allRecords.push(...sorted)
  }
  const fingerprint = computeCanonicalInputFingerprint(allRecords)
  const db = typeof dbPathOrDb === 'string' ? new Database(dbPathOrDb) : dbPathOrDb
  db.exec(`CREATE TABLE meta (k TEXT PRIMARY KEY, v TEXT NOT NULL); INSERT INTO meta (k,v) VALUES ('canonical_input_sha256','${fingerprint}');`)
  if (typeof dbPathOrDb === 'string') db.close()
}

test('record-verified writes survivable evidence when canonical_capability is absent', () => {
  const root = mkdtempSync(join(tmpdir(), 'chronica-record-verified-test-'))
  try {
    mkdirSync(join(root, 'docs'), { recursive: true })
    mkdirSync(join(root, 'crates', 'chronica-demo', 'src'), { recursive: true })
    const impl = join(root, 'crates', 'chronica-demo', 'src', 'lib.rs')
    writeFileSync(impl, 'pub fn demo_symbol() {}\n#[test]\nfn proves_demo() {}\n')

    const db = new Database(join(root, 'docs', 'capabilities.db'))
    db.exec(`
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
      CREATE TABLE slice_canonical (
        slice INTEGER,
        canonical_key TEXT,
        match_method TEXT,
        confidence TEXT,
        reviewed INTEGER,
        note TEXT,
        PRIMARY KEY (slice, canonical_key)
      );
      INSERT INTO slice_canonical VALUES (1, 'demo.capability', 'test', 'high', 1, NULL);
    `)
    db.close()

    execFileSync(process.execPath, [
      script,
      '--key', 'demo.capability',
      '--test', 'proves_demo',
      '--impl', impl,
      '--symbols', 'demo_symbol',
      '--donor', 'test',
      '--config',
      '--now', '2026-07-10T00:00:00.000Z',
    ], { cwd: root, encoding: 'utf8' })

    const check = new Database(join(root, 'docs', 'capabilities.db'), { readonly: true })
    assert.deepEqual(
      check.prepare('SELECT status, acceptance_test, moves_money FROM canonical_status_override WHERE canonical_key=?').get('demo.capability'),
      { status: 'verified', acceptance_test: 'proves_demo', moves_money: 0 },
    )
    assert.equal(check.prepare('SELECT test_symbol FROM impl_evidence WHERE canonical_key=?').get('demo.capability').test_symbol, 'proves_demo')
    check.close()
    assert.match(readFileSync(impl, 'utf8'), /demo_symbol/)
  } finally {
    rmSync(root, { recursive: true, force: true })
  }
})

test('record-verified syncs capability_architecture_link.status in architecture.db when present', () => {
  // Regression test for the 2026-08 capability-verification-promotion sync gap: promoting a
  // capability used to update canonical_capability.status (and canonical_status_override) but
  // never touched architecture.db's capability_architecture_link.status, so a subsequent
  // caps:canonical:export/arch:canonical:export still baked the pre-promotion status into
  // architecture_refs and docs/architecture-canonical/links.jsonl.
  const root = mkdtempSync(join(tmpdir(), 'chronica-record-verified-test-'))
  try {
    mkdirSync(join(root, 'docs'), { recursive: true })
    mkdirSync(join(root, 'crates', 'chronica-demo', 'src'), { recursive: true })
    const impl = join(root, 'crates', 'chronica-demo', 'src', 'lib.rs')
    writeFileSync(impl, 'pub fn demo_symbol() {}\n#[test]\nfn proves_demo() {}\n')

    const db = new Database(join(root, 'docs', 'capabilities.db'))
    db.exec(`
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
      CREATE TABLE slice_canonical (
        slice INTEGER,
        canonical_key TEXT,
        match_method TEXT,
        confidence TEXT,
        reviewed INTEGER,
        note TEXT,
        PRIMARY KEY (slice, canonical_key)
      );
      INSERT INTO slice_canonical VALUES (1, 'demo.capability', 'test', 'high', 1, NULL);
    `)
    db.close()

    const archDb = new Database(join(root, 'docs', 'architecture.db'))
    archDb.exec(`
      CREATE TABLE architecture_node (
        id TEXT PRIMARY KEY,
        kind TEXT,
        name TEXT,
        crate TEXT,
        module_path TEXT,
        file_path TEXT,
        status TEXT,
        evidence_ref TEXT
      );
      CREATE TABLE capability_architecture_link (
        capability_key TEXT,
        architecture_node TEXT,
        relationship TEXT,
        status TEXT,
        target_crate TEXT,
        target_module TEXT
      );
      -- read unconditionally (for its own index) by export-cloud-snapshot.mjs's
      -- exportArchitecture(), which the DB-REBUILD AUTOMATION step now runs for real.
      CREATE TABLE architecture_target_gap (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        capability_key TEXT NOT NULL,
        target_crate TEXT,
        target_module TEXT,
        gap_kind TEXT NOT NULL,
        suggested_architecture_node TEXT,
        reason TEXT NOT NULL,
        status TEXT NOT NULL,
        evidence_ref TEXT
      );
      INSERT INTO architecture_node VALUES
        ('crate:chronica-demo', 'crate', 'chronica-demo', 'chronica-demo', NULL, NULL, 'implemented_unverified', NULL),
        ('module:chronica-demo:lib', 'module', 'lib', 'chronica-demo', 'lib', NULL, 'implemented_unverified', NULL),
        ('crate:chronica-other', 'crate', 'chronica-other', 'chronica-other', NULL, NULL, 'implemented_unverified', NULL);
      INSERT INTO capability_architecture_link VALUES
        ('demo.capability', 'crate:chronica-demo', 'targets', 'implemented_unverified', 'chronica-demo', 'lib'),
        ('demo.capability', 'module:chronica-demo:lib', 'implemented_by', 'implemented_unverified', 'chronica-demo', 'lib'),
        ('other.capability', 'crate:chronica-other', 'targets', 'implemented_unverified', 'chronica-other', 'lib');
    `)
    archDb.close()

    execFileSync(process.execPath, [
      script,
      '--key', 'demo.capability',
      '--test', 'proves_demo',
      '--impl', impl,
      '--symbols', 'demo_symbol',
      '--donor', 'test',
      '--config',
      '--now', '2026-07-10T00:00:00.000Z',
    ], { cwd: root, encoding: 'utf8' })

    const checkArch = new Database(join(root, 'docs', 'architecture.db'), { readonly: true })
    const rows = checkArch.prepare('SELECT capability_key, status FROM capability_architecture_link ORDER BY capability_key, architecture_node').all()
    assert.deepEqual(rows, [
      { capability_key: 'demo.capability', status: 'verified' },
      { capability_key: 'demo.capability', status: 'verified' },
      { capability_key: 'other.capability', status: 'implemented_unverified' },
    ])
    checkArch.close()

    // the NEW architecture-side POST-WRITE VERIFY GUARD must also have regenerated
    // docs/architecture-canonical/links.jsonl from the now-updated architecture.db, not just
    // left the DB update as an in-memory fact nothing else ever reads.
    const linksPath = join(root, 'docs', 'architecture-canonical', 'links.jsonl')
    const linkRecords = readFileSync(linksPath, 'utf8').trim().split('\n').map((line) => JSON.parse(line))
    const demoLinks = linkRecords.filter((record) => record.capability_key === 'demo.capability')
    assert.equal(demoLinks.length, 2)
    assert.ok(demoLinks.every((record) => record.status === 'verified'))
  } finally {
    rmSync(root, { recursive: true, force: true })
  }
})

test('record-verified preserves a non-money capability\'s moves_money=0 when --config is omitted', () => {
  // Regression test: the hasCanonical write path used to hardcode moves_money=1 whenever --config
  // wasn't passed, silently mislabeling any non-money capability promoted without --config as an
  // unguarded money capability (moves_money=1, requires_approval=0, no financial_control_test) —
  // exactly the state the "money safety invariants" checks in docs/_generated/026-status.md flag as ❌.
  const root = mkdtempSync(join(tmpdir(), 'chronica-record-verified-test-'))
  try {
    mkdirSync(join(root, 'docs'), { recursive: true })
    mkdirSync(join(root, 'crates', 'chronica-demo', 'src'), { recursive: true })
    const impl = join(root, 'crates', 'chronica-demo', 'src', 'lib.rs')
    writeFileSync(impl, 'pub fn demo_symbol() {}\n#[test]\nfn proves_demo() {}\n')
    // referenced by the POST-WRITE VERIFY GUARD's docs_refs requirement for a verified capability.
    writeFileSync(join(root, 'docs', '900-demo-capability.md'), 'See `demo.capability` for details.\n')
    // a real target_crate on this capability makes the DB-REBUILD AUTOMATION step run for real
    // (caps:cloud-export + subdb:gen/verify), so this needs a minimal real Cargo workspace too.
    writeMinimalCargoWorkspace(root, 'chronica-demo')

    const db = new Database(join(root, 'docs', 'capabilities.db'))
    db.exec(`
      CREATE TABLE canonical_capability (
        id INTEGER PRIMARY KEY,
        key TEXT UNIQUE NOT NULL,
        canonical_name TEXT NOT NULL,
        domain TEXT,
        target_crate TEXT,
        target_module TEXT,
        side_effect_class TEXT,
        moves_money INTEGER DEFAULT 0,
        requires_approval INTEGER DEFAULT 0,
        acceptance_criteria TEXT,
        required_tests TEXT,
        financial_control_test TEXT,
        acceptance_test TEXT,
        status TEXT DEFAULT 'unimplemented',
        exclusion_note TEXT,
        blocker TEXT,
        slice INTEGER,
        donor_count INTEGER DEFAULT 0
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
      -- read unconditionally by export-cloud-snapshot.mjs's exportProvenance(); the DB-REBUILD
      -- AUTOMATION step now runs that script for real, so these must exist even empty.
      CREATE TABLE source_capability (id INTEGER PRIMARY KEY, donor TEXT, canonical_id INTEGER, moves_money INTEGER);
      CREATE TABLE provenance (source_id INTEGER, canonical_id INTEGER);
      CREATE TABLE slice_canonical (
        slice INTEGER,
        canonical_key TEXT,
        match_method TEXT,
        confidence TEXT,
        reviewed INTEGER,
        note TEXT,
        PRIMARY KEY (slice, canonical_key)
      );
      INSERT INTO canonical_capability (id, key, canonical_name, domain, target_crate, side_effect_class, moves_money, acceptance_criteria, status)
      VALUES (1, 'demo.capability', 'Demo capability', 'demo', 'chronica-demo', 'internal_write', 0, 'test', 'unimplemented');
    `)
    seedCanonicalBaselineAndFingerprint(root, join(root, 'docs', 'capabilities.db'), [{ key: 'demo.capability', domain: 'demo', canonical_name: 'Demo capability', target_crate: 'chronica-demo' }])
    db.close()

    const archDb = new Database(join(root, 'docs', 'architecture.db'))
    archDb.exec(`
      CREATE TABLE architecture_node (
        id TEXT PRIMARY KEY,
        kind TEXT,
        name TEXT,
        crate TEXT,
        module_path TEXT,
        file_path TEXT,
        status TEXT,
        evidence_ref TEXT
      );
      CREATE TABLE capability_architecture_link (
        capability_key TEXT,
        architecture_node TEXT,
        relationship TEXT,
        status TEXT,
        target_crate TEXT,
        target_module TEXT
      );
      -- read unconditionally (for its own index) by export-cloud-snapshot.mjs's
      -- exportArchitecture(), which the DB-REBUILD AUTOMATION step now runs for real.
      CREATE TABLE architecture_target_gap (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        capability_key TEXT NOT NULL,
        target_crate TEXT,
        target_module TEXT,
        gap_kind TEXT NOT NULL,
        suggested_architecture_node TEXT,
        reason TEXT NOT NULL,
        status TEXT NOT NULL,
        evidence_ref TEXT
      );
      INSERT INTO architecture_node VALUES
        ('crate:chronica-demo', 'crate', 'chronica-demo', 'chronica-demo', NULL, NULL, 'implemented_unverified', NULL);
      INSERT INTO capability_architecture_link VALUES
        ('demo.capability', 'crate:chronica-demo', 'targets', 'implemented_unverified', 'chronica-demo', 'lib');
    `)
    archDb.close()

    // deliberately NOT passing --config, to exercise the hasCanonical (non-config) write path.
    const output = execFileSync(process.execPath, [
      script,
      '--key', 'demo.capability',
      '--test', 'proves_demo',
      '--impl', impl,
      '--symbols', 'demo_symbol',
      '--donor', 'test',
      '--now', '2026-07-10T00:00:00.000Z',
    ], { cwd: root, encoding: 'utf8' })

    const check = new Database(join(root, 'docs', 'capabilities.db'), { readonly: true })
    assert.deepEqual(
      check.prepare('SELECT status, moves_money FROM canonical_capability WHERE key=?').get('demo.capability'),
      { status: 'verified', moves_money: 0 },
    )
    assert.equal(
      check.prepare('SELECT moves_money FROM canonical_status_override WHERE canonical_key=?').get('demo.capability').moves_money,
      0,
    )
    check.close()

    // the POST-WRITE VERIFY GUARD should have regenerated the canonical shard with a consistent,
    // non-stale architecture_refs entry for this capability's own target_crate.
    const shard = JSON.parse(readFileSync(join(root, 'docs', 'capabilities-canonical', 'domains', 'demo.jsonl'), 'utf8').trim())
    assert.equal(shard.status, 'verified')
    assert.ok(shard.architecture_refs.some((ref) => ref === 'targets:crate:chronica-demo:verified'))

    // DB-REBUILD AUTOMATION success path: caps:cloud-export and subdb:gen/verify must have
    // actually run (not just been printed as a reminder) and produced real artifacts.
    assert.match(output, /DB rebuild: caps:cloud-export \+ subdb:gen\/verify --crate chronica-demo completed/)
    const cloudCore = new Database(join(root, 'docs', 'capabilities-cloud', 'cap-core.db'), { readonly: true })
    assert.equal(cloudCore.prepare("SELECT status FROM canonical_capability WHERE key='demo.capability'").get().status, 'verified')
    cloudCore.close()
    const subdbJsonlPath = join(root, 'crates', 'chronica-demo', '.chronica', 'sub-cap-arch.jsonl')
    assert.ok(existsSync(subdbJsonlPath), 'subdb:gen must have written the crate-local sub-cap-arch.jsonl shard')
    const subdbRecords = readFileSync(subdbJsonlPath, 'utf8').trim().split('\n').map((line) => JSON.parse(line))
    const subdbCapRecord = subdbRecords.find((record) => record.record_type === 'capability' && record.capability_key === 'demo.capability')
    assert.equal(subdbCapRecord.status, 'verified')
  } finally {
    rmSync(root, { recursive: true, force: true })
  }
})

test('record-verified refuses and rolls back a promotion that fails the canonical-shard invariant check', () => {
  // Regression test for the Lane C2 census finding: PR #2654 added a check inside
  // verify-canonical-shards.mjs specifically to catch a promotion that updates `status` without
  // keeping this same capability's architecture linkage in sync (the 38-record gap) -- but
  // nothing ever called that check from the actual promotion path, so it was a "paper tiger"
  // only reachable by an agent manually running `pnpm caps:canonical:verify` and pasting the
  // output into a report. Before this fix, promoting `demo.capability` to `verified` here would
  // succeed silently even though it has never been linked to any architecture node (no
  // docs/architecture.db, no capability_architecture_link row) -- exactly the same "status says
  // verified, the rest of the wiring says otherwise" drift as the original bug, just caught here
  // via the general "verified requires non-empty architecture_refs" rule in the same check
  // function rather than the narrower stale-ref-status rule (which #2654's own DB-sync code in
  // this script now makes unreachable through this script's own writes -- see the sync block
  // above). After this fix, the promotion must fail non-zero and the DB writes must be rolled
  // back instead of landing half-synced.
  const root = mkdtempSync(join(tmpdir(), 'chronica-record-verified-test-'))
  try {
    mkdirSync(join(root, 'docs'), { recursive: true })
    mkdirSync(join(root, 'crates', 'chronica-demo', 'src'), { recursive: true })
    const impl = join(root, 'crates', 'chronica-demo', 'src', 'lib.rs')
    writeFileSync(impl, 'pub fn demo_symbol() {}\n#[test]\nfn proves_demo() {}\n')

    const db = new Database(join(root, 'docs', 'capabilities.db'))
    db.exec(`
      CREATE TABLE canonical_capability (
        id INTEGER PRIMARY KEY,
        key TEXT UNIQUE NOT NULL,
        canonical_name TEXT NOT NULL,
        domain TEXT,
        target_crate TEXT,
        target_module TEXT,
        side_effect_class TEXT,
        moves_money INTEGER DEFAULT 0,
        requires_approval INTEGER DEFAULT 0,
        acceptance_criteria TEXT,
        required_tests TEXT,
        financial_control_test TEXT,
        acceptance_test TEXT,
        status TEXT DEFAULT 'unimplemented',
        exclusion_note TEXT,
        blocker TEXT,
        slice INTEGER,
        donor_count INTEGER DEFAULT 0
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
      INSERT INTO canonical_capability (id, key, canonical_name, domain, target_crate, side_effect_class, moves_money, acceptance_criteria, status)
      VALUES (1, 'demo.capability', 'Demo capability', 'demo', 'chronica-demo', 'internal_write', 0, 'test', 'implemented_unverified');
    `)
    seedCanonicalBaselineAndFingerprint(root, join(root, 'docs', 'capabilities.db'), [{ key: 'demo.capability', domain: 'demo', canonical_name: 'Demo capability', target_crate: 'chronica-demo' }])
    db.close()
    // deliberately no docs/architecture.db and no doc mentioning the key: this capability has
    // never been architecturally linked, so the canonical shard's architecture_refs/docs_refs
    // stay empty across the promotion -- the drift the invariant exists to catch.

    let threw = null
    try {
      execFileSync(process.execPath, [
        script,
        '--key', 'demo.capability',
        '--test', 'proves_demo',
        '--impl', impl,
        '--symbols', 'demo_symbol',
        '--donor', 'test',
        '--now', '2026-07-10T00:00:00.000Z',
      ], { cwd: root, encoding: 'utf8' })
    } catch (error) {
      threw = error
    }

    assert.ok(threw, 'record-verified should have exited non-zero and refused the promotion')
    assert.equal(threw.status, 1)
    assert.match(String(threw.stderr), /REFUSED.*rolled back/)
    assert.match(String(threw.stderr), /architecture_refs is required for verified capability/)

    // the DB writes must be rolled back, not left half-promoted.
    const check = new Database(join(root, 'docs', 'capabilities.db'), { readonly: true })
    assert.equal(
      check.prepare('SELECT status FROM canonical_capability WHERE key=?').get('demo.capability').status,
      'implemented_unverified',
    )
    assert.equal(check.prepare('SELECT count(*) n FROM canonical_status_override WHERE canonical_key=?').get('demo.capability').n, 0)
    assert.equal(check.prepare('SELECT count(*) n FROM impl_evidence WHERE canonical_key=?').get('demo.capability').n, 0)
    check.close()
  } finally {
    rmSync(root, { recursive: true, force: true })
  }
})

test('record-verified scopes the POST-WRITE VERIFY GUARD to the exact promoted line, not a numeric-prefix match', () => {
  // Regression test for the Lane C1 audit finding on PR #2659: the guard's scoping filter used
  // to be a bare `error.startsWith(path)` where `path` is "<file>:<line>" with no boundary after
  // the line number -- so an error on line 50 ("demo.jsonl:50...") string-prefix-matches a scope
  // filter built for line 5 ("demo.jsonl:5"), because "...:50" literally begins with "...:5".
  // A clean promotion at a low line number would be falsely refused and rolled back purely
  // because of an unrelated pre-existing violation elsewhere in the same domain shard whose line
  // number happens to start with the same digits. Real domain shards run 500-1000+ records, so
  // this collision is a real, recurring risk, not a corner case. This test builds a 50-record
  // domain shard so the promoted capability lands on line 5 and a pre-existing, already-verified,
  // already-violating capability lands on line 50 -- the exact digit-prefix collision C1
  // reproduced -- and confirms the line-5 promotion now succeeds instead of being cross-
  // contaminated by the line-50 violation.
  const root = mkdtempSync(join(tmpdir(), 'chronica-record-verified-test-'))
  try {
    mkdirSync(join(root, 'docs'), { recursive: true })
    mkdirSync(join(root, 'crates', 'chronica-demo', 'src'), { recursive: true })
    const impl = join(root, 'crates', 'chronica-demo', 'src', 'lib.rs')
    writeFileSync(impl, 'pub fn demo_symbol() {}\n#[test]\nfn proves_demo() {}\n')
    writeFileSync(join(root, 'docs', '900-demo-capability.md'), 'See `demo.cap05` for details.\n')
    // demo.cap05's target_crate makes the DB-REBUILD AUTOMATION step run for real.
    writeMinimalCargoWorkspace(root, 'chronica-demo')

    const db = new Database(join(root, 'docs', 'capabilities.db'))
    db.exec(`
      CREATE TABLE canonical_capability (
        id INTEGER PRIMARY KEY,
        key TEXT UNIQUE NOT NULL,
        canonical_name TEXT NOT NULL,
        domain TEXT,
        target_crate TEXT,
        target_module TEXT,
        side_effect_class TEXT,
        moves_money INTEGER DEFAULT 0,
        requires_approval INTEGER DEFAULT 0,
        acceptance_criteria TEXT,
        required_tests TEXT,
        financial_control_test TEXT,
        acceptance_test TEXT,
        status TEXT DEFAULT 'unimplemented',
        exclusion_note TEXT,
        blocker TEXT,
        slice INTEGER,
        donor_count INTEGER DEFAULT 0
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
      CREATE TABLE source_capability (id INTEGER PRIMARY KEY, donor TEXT, canonical_id INTEGER, moves_money INTEGER);
      CREATE TABLE provenance (source_id INTEGER, canonical_id INTEGER);
      CREATE TABLE slice_canonical (
        slice INTEGER,
        canonical_key TEXT,
        match_method TEXT,
        confidence TEXT,
        reviewed INTEGER,
        note TEXT,
        PRIMARY KEY (slice, canonical_key)
      );
    `)

    const insertRow = db.prepare(`
      INSERT INTO canonical_capability (key, canonical_name, domain, target_crate, side_effect_class, acceptance_criteria, status)
      VALUES (@key, @name, 'demo', @targetCrate, 'internal_write', 'test', @status)
    `)
    // 50 records in the same domain shard, sorted alphabetically by key -> lands exactly on the
    // matching line number in the exported JSONL (1-indexed). demo.cap05 (line 5) is the clean
    // capability this test promotes; demo.cap50 (line 50) is a PRE-EXISTING, already-verified
    // capability with a real violation (no docs_refs/architecture_refs) that must NOT block the
    // line-5 promotion.
    for (let n = 1; n <= 50; n += 1) {
      const paddedKey = `demo.cap${String(n).padStart(2, '0')}`
      if (n === 50) {
        insertRow.run({ key: paddedKey, name: `Filler ${n}`, targetCrate: 'chronica-demo-unrelated', status: 'verified' })
      } else if (n === 5) {
        insertRow.run({ key: paddedKey, name: `Filler ${n}`, targetCrate: 'chronica-demo', status: 'implemented_unverified' })
      } else {
        insertRow.run({ key: paddedKey, name: `Filler ${n}`, targetCrate: null, status: null })
      }
    }
    seedCanonicalBaselineAndFingerprint(root, db, Array.from({ length: 50 }, (_, i) => {
      const n = i + 1
      const key = `demo.cap${String(n).padStart(2, '0')}`
      return n === 5 ? { key, domain: 'demo', target_crate: 'chronica-demo' } : { key, domain: 'demo' }
    }))
    db.close()

    const archDb = new Database(join(root, 'docs', 'architecture.db'))
    archDb.exec(`
      CREATE TABLE architecture_node (
        id TEXT PRIMARY KEY,
        kind TEXT,
        name TEXT,
        crate TEXT,
        module_path TEXT,
        file_path TEXT,
        status TEXT,
        evidence_ref TEXT
      );
      CREATE TABLE capability_architecture_link (
        capability_key TEXT,
        architecture_node TEXT,
        relationship TEXT,
        status TEXT,
        target_crate TEXT,
        target_module TEXT
      );
      -- read unconditionally (for its own index) by export-cloud-snapshot.mjs's
      -- exportArchitecture(), which the DB-REBUILD AUTOMATION step now runs for real.
      CREATE TABLE architecture_target_gap (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        capability_key TEXT NOT NULL,
        target_crate TEXT,
        target_module TEXT,
        gap_kind TEXT NOT NULL,
        suggested_architecture_node TEXT,
        reason TEXT NOT NULL,
        status TEXT NOT NULL,
        evidence_ref TEXT
      );
      INSERT INTO architecture_node VALUES
        ('crate:chronica-demo', 'crate', 'chronica-demo', 'chronica-demo', NULL, NULL, 'implemented_unverified', NULL);
      INSERT INTO capability_architecture_link VALUES
        ('demo.cap05', 'crate:chronica-demo', 'targets', 'implemented_unverified', 'chronica-demo', 'lib');
    `)
    archDb.close()

    execFileSync(process.execPath, [
      script,
      '--key', 'demo.cap05',
      '--test', 'proves_demo',
      '--impl', impl,
      '--symbols', 'demo_symbol',
      '--donor', 'test',
      '--config',
      '--now', '2026-08-08T00:00:00.000Z',
    ], { cwd: root, encoding: 'utf8' })

    // the line-5 promotion must have succeeded despite the line-50 violation.
    const check = new Database(join(root, 'docs', 'capabilities.db'), { readonly: true })
    assert.equal(check.prepare('SELECT status FROM canonical_capability WHERE key=?').get('demo.cap05').status, 'verified')
    // the unrelated line-50 violation must be untouched (still there, still verified) -- proof
    // this test isn't accidentally passing because the guard stopped checking anything at all.
    assert.equal(check.prepare('SELECT status FROM canonical_capability WHERE key=?').get('demo.cap50').status, 'verified')
    check.close()

    const shardPath = join(root, 'docs', 'capabilities-canonical', 'domains', 'demo.jsonl')
    const lines = readFileSync(shardPath, 'utf8').trim().split('\n')
    assert.equal(JSON.parse(lines[4]).capability_key, 'demo.cap05')
    assert.equal(JSON.parse(lines[4]).status, 'verified')
    assert.equal(JSON.parse(lines[49]).capability_key, 'demo.cap50')
  } finally {
    rmSync(root, { recursive: true, force: true })
  }
})

test('record-verified refuses and rolls back a promotion whose architecture link points at a nonexistent architecture_node', () => {
  // Regression test for the NEW architecture-side POST-WRITE VERIFY GUARD (issue #2688 root-cause
  // fix): a capability_architecture_link row can reference an architecture_node id that was never
  // recorded in architecture_node. The capabilities-side guard cannot see this -- it only reads
  // docs/capabilities-canonical/**, and derives a non-empty, non-stale `architecture_refs` array
  // straight from capability_architecture_link regardless of whether the node it points at
  // actually exists -- so a promotion like this used to sail through the capabilities-side check
  // alone. Only re-exporting and verifying docs/architecture-canonical/links.jsonl (this fix)
  // catches the dangling reference. This is exactly the "links.jsonl not run automatically" gap
  // the census flagged: before this fix nothing ever ran arch:canonical:verify from this script.
  const root = mkdtempSync(join(tmpdir(), 'chronica-record-verified-test-'))
  try {
    mkdirSync(join(root, 'docs'), { recursive: true })
    mkdirSync(join(root, 'crates', 'chronica-demo', 'src'), { recursive: true })
    const impl = join(root, 'crates', 'chronica-demo', 'src', 'lib.rs')
    writeFileSync(impl, 'pub fn demo_symbol() {}\n#[test]\nfn proves_demo() {}\n')
    // satisfies the capabilities-side guard's docs_refs requirement, so THAT guard passes cleanly
    // and this test proves the architecture-side guard is what actually catches the problem.
    writeFileSync(join(root, 'docs', '900-demo-capability.md'), 'See `demo.capability` for details.\n')

    const db = new Database(join(root, 'docs', 'capabilities.db'))
    db.exec(`
      CREATE TABLE canonical_capability (
        id INTEGER PRIMARY KEY,
        key TEXT UNIQUE NOT NULL,
        canonical_name TEXT NOT NULL,
        domain TEXT,
        target_crate TEXT,
        target_module TEXT,
        side_effect_class TEXT,
        moves_money INTEGER DEFAULT 0,
        requires_approval INTEGER DEFAULT 0,
        acceptance_criteria TEXT,
        required_tests TEXT,
        financial_control_test TEXT,
        acceptance_test TEXT,
        status TEXT DEFAULT 'unimplemented',
        exclusion_note TEXT,
        blocker TEXT,
        slice INTEGER,
        donor_count INTEGER DEFAULT 0
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
      INSERT INTO canonical_capability (id, key, canonical_name, domain, target_crate, side_effect_class, moves_money, acceptance_criteria, status)
      VALUES (1, 'demo.capability', 'Demo capability', 'demo', 'chronica-demo', 'internal_write', 0, 'test', 'implemented_unverified');
    `)
    seedCanonicalBaselineAndFingerprint(root, join(root, 'docs', 'capabilities.db'), [{ key: 'demo.capability', domain: 'demo', canonical_name: 'Demo capability', target_crate: 'chronica-demo' }])
    db.close()

    // deliberately NO architecture_node table/rows: capability_architecture_link points at
    // 'crate:chronica-demo', a node that was never recorded -- the dangling reference this guard
    // exists to catch.
    const archDb = new Database(join(root, 'docs', 'architecture.db'))
    archDb.exec(`
      CREATE TABLE capability_architecture_link (
        capability_key TEXT,
        architecture_node TEXT,
        relationship TEXT,
        status TEXT,
        target_crate TEXT,
        target_module TEXT
      );
      INSERT INTO capability_architecture_link VALUES
        ('demo.capability', 'crate:chronica-demo', 'targets', 'implemented_unverified', 'chronica-demo', 'lib');
    `)
    archDb.close()

    let threw = null
    try {
      execFileSync(process.execPath, [
        script,
        '--key', 'demo.capability',
        '--test', 'proves_demo',
        '--impl', impl,
        '--symbols', 'demo_symbol',
        '--donor', 'test',
        '--config',
        '--now', '2026-08-08T00:00:00.000Z',
      ], { cwd: root, encoding: 'utf8' })
    } catch (error) {
      threw = error
    }

    assert.ok(threw, 'record-verified should have exited non-zero and refused the promotion')
    assert.equal(threw.status, 1)
    assert.match(String(threw.stderr), /REFUSED.*rolled back/)
    assert.match(String(threw.stderr), /arch:canonical:verify/)
    assert.match(String(threw.stderr), /architecture_node references missing node crate:chronica-demo/)

    // both DB writes must be rolled back, not left half-promoted.
    const check = new Database(join(root, 'docs', 'capabilities.db'), { readonly: true })
    assert.equal(check.prepare('SELECT status FROM canonical_capability WHERE key=?').get('demo.capability').status, 'implemented_unverified')
    assert.equal(check.prepare('SELECT count(*) n FROM canonical_status_override WHERE canonical_key=?').get('demo.capability').n, 0)
    assert.equal(check.prepare('SELECT count(*) n FROM impl_evidence WHERE canonical_key=?').get('demo.capability').n, 0)
    check.close()

    const checkArch = new Database(join(root, 'docs', 'architecture.db'), { readonly: true })
    assert.equal(
      checkArch.prepare('SELECT status FROM capability_architecture_link WHERE capability_key=?').get('demo.capability').status,
      'implemented_unverified',
    )
    checkArch.close()
  } finally {
    rmSync(root, { recursive: true, force: true })
  }
})

test('record-verified fails loudly (non-zero exit, DB writes NOT rolled back) when the DB-rebuild automation step errors', () => {
  // The DB-REBUILD AUTOMATION step (caps:cloud-export + subdb:gen/verify) runs real subprocesses
  // after the promotion has already passed both in-process invariant guards. If one of those
  // subprocesses fails (here: subdb:gen's real Cargo-workspace discovery scan, because this root
  // deliberately has no Cargo.toml), record-verified.mjs must exit non-zero and print the real
  // command plus its stderr instead of silently reporting success -- exactly the "reminder that's
  // easy to skip" failure mode this fix replaces. The promotion's own DB writes are NOT rolled
  // back here: they already passed correctness verification, so only the downstream cache
  // artifacts are considered stale, not the promotion itself.
  const root = mkdtempSync(join(tmpdir(), 'chronica-record-verified-test-'))
  try {
    mkdirSync(join(root, 'docs'), { recursive: true })
    mkdirSync(join(root, 'crates', 'chronica-demo', 'src'), { recursive: true })
    const impl = join(root, 'crates', 'chronica-demo', 'src', 'lib.rs')
    writeFileSync(impl, 'pub fn demo_symbol() {}\n#[test]\nfn proves_demo() {}\n')
    writeFileSync(join(root, 'docs', '900-demo-capability.md'), 'See `demo.capability` for details.\n')
    // deliberately NOT calling writeMinimalCargoWorkspace(root, 'chronica-demo') -- no Cargo.toml
    // means subdb:gen's discoverCrates() hits a real ENOENT reading it, a genuine real-world
    // failure mode (e.g. record-verified.mjs invoked outside a full workspace checkout).

    const db = new Database(join(root, 'docs', 'capabilities.db'))
    db.exec(`
      CREATE TABLE canonical_capability (
        id INTEGER PRIMARY KEY,
        key TEXT UNIQUE NOT NULL,
        canonical_name TEXT NOT NULL,
        domain TEXT,
        target_crate TEXT,
        target_module TEXT,
        side_effect_class TEXT,
        moves_money INTEGER DEFAULT 0,
        requires_approval INTEGER DEFAULT 0,
        acceptance_criteria TEXT,
        required_tests TEXT,
        financial_control_test TEXT,
        acceptance_test TEXT,
        status TEXT DEFAULT 'unimplemented',
        exclusion_note TEXT,
        blocker TEXT,
        slice INTEGER,
        donor_count INTEGER DEFAULT 0
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
      CREATE TABLE source_capability (id INTEGER PRIMARY KEY, donor TEXT, canonical_id INTEGER, moves_money INTEGER);
      CREATE TABLE provenance (source_id INTEGER, canonical_id INTEGER);
      CREATE TABLE slice_canonical (
        slice INTEGER,
        canonical_key TEXT,
        match_method TEXT,
        confidence TEXT,
        reviewed INTEGER,
        note TEXT,
        PRIMARY KEY (slice, canonical_key)
      );
      INSERT INTO canonical_capability (id, key, canonical_name, domain, target_crate, side_effect_class, moves_money, acceptance_criteria, status)
      VALUES (1, 'demo.capability', 'Demo capability', 'demo', 'chronica-demo', 'internal_write', 0, 'test', 'unimplemented');
    `)
    seedCanonicalBaselineAndFingerprint(root, join(root, 'docs', 'capabilities.db'), [{ key: 'demo.capability', domain: 'demo', canonical_name: 'Demo capability', target_crate: 'chronica-demo' }])
    db.close()

    const archDb = new Database(join(root, 'docs', 'architecture.db'))
    archDb.exec(`
      CREATE TABLE architecture_node (
        id TEXT PRIMARY KEY,
        kind TEXT,
        name TEXT,
        crate TEXT,
        module_path TEXT,
        file_path TEXT,
        status TEXT,
        evidence_ref TEXT
      );
      CREATE TABLE capability_architecture_link (
        capability_key TEXT,
        architecture_node TEXT,
        relationship TEXT,
        status TEXT,
        target_crate TEXT,
        target_module TEXT
      );
      CREATE TABLE architecture_target_gap (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        capability_key TEXT NOT NULL,
        target_crate TEXT,
        target_module TEXT,
        gap_kind TEXT NOT NULL,
        suggested_architecture_node TEXT,
        reason TEXT NOT NULL,
        status TEXT NOT NULL,
        evidence_ref TEXT
      );
      INSERT INTO architecture_node VALUES
        ('crate:chronica-demo', 'crate', 'chronica-demo', 'chronica-demo', NULL, NULL, 'implemented_unverified', NULL);
      INSERT INTO capability_architecture_link VALUES
        ('demo.capability', 'crate:chronica-demo', 'targets', 'implemented_unverified', 'chronica-demo', 'lib');
    `)
    archDb.close()

    let threw = null
    try {
      execFileSync(process.execPath, [
        script,
        '--key', 'demo.capability',
        '--test', 'proves_demo',
        '--impl', impl,
        '--symbols', 'demo_symbol',
        '--donor', 'test',
        '--now', '2026-08-08T00:00:00.000Z',
      ], { cwd: root, encoding: 'utf8' })
    } catch (error) {
      threw = error
    }

    assert.ok(threw, 'record-verified should have exited non-zero when the DB-rebuild automation step fails')
    assert.equal(threw.status, 1)
    assert.match(String(threw.stderr), /REFUSED: required follow-up step 'subdb:gen' failed/)
    assert.match(String(threw.stderr), /gen-crate-subdb\.mjs --crate chronica-demo/)
    assert.match(String(threw.stderr), /STALE/)

    // the promotion itself already passed both invariant guards, so it must NOT be rolled back
    // just because the downstream rebuild step failed.
    const check = new Database(join(root, 'docs', 'capabilities.db'), { readonly: true })
    assert.equal(check.prepare('SELECT status FROM canonical_capability WHERE key=?').get('demo.capability').status, 'verified')
    check.close()
    // caps:cloud-export (the step before the one that failed) must still have run for real.
    assert.ok(existsSync(join(root, 'docs', 'capabilities-cloud', 'cap-core.db')))
  } finally {
    rmSync(root, { recursive: true, force: true })
  }
})
