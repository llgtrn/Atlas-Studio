import assert from 'node:assert/strict'
import { mkdirSync, rmSync, symlinkSync, writeFileSync } from 'node:fs'
import { dirname, join } from 'node:path'
import { fileURLToPath } from 'node:url'
import test from 'node:test'
import {
  BOUNDS,
  discoverWorkspaceCrates,
  discoverWorkspaceCratesWithFindings,
  escapeMarkdownCell,
  extractParseErrorColumn,
  findDuplicateTopLevelKeys,
  loadCrateDocCapabilityStatus,
  loadCrateShard,
  validateShardRecordShape,
} from './sync-anchor-v2-shard.mjs'
import { buildAdversarialSecretShapes, cap, makeMinimalCrateWorkspace } from './sync-anchor-v2-fixtures.mjs'

const repoRoot = join(dirname(fileURLToPath(import.meta.url)), '..', '..')

// ── escapeMarkdownCell ───────────────────────────────────────────────────────────────────────

test('escapeMarkdownCell: pipe and backtick cannot break a Markdown table cell', () => {
  const escaped = escapeMarkdownCell('a | b `code` \n c')
  assert.ok(!escaped.includes('|') || escaped.includes('\\|'))
  assert.ok(!escaped.includes('`'))
  assert.ok(!escaped.includes('\n'))
})

test('escapeMarkdownCell: bounds length like boundedText', () => {
  const escaped = escapeMarkdownCell('y'.repeat(1000), 20)
  assert.ok(escaped.length < 1000)
})

// ── workspace / crate discovery (traversal- AND symlink-escape-safe) ───────────────────────────

test('discoverWorkspaceCratesWithFindings: a workspace member escaping root is skipped and reported, never read', () => {
  const { root, cratePath } = makeMinimalCrateWorkspace('chronica-sync-anchor-v2-traversal-')
  try {
    mkdirSync(join(root, '..', 'chronica-sync-anchor-v2-outside-root'), { recursive: true })
    writeFileSync(
      join(root, '..', 'chronica-sync-anchor-v2-outside-root', 'Cargo.toml'),
      '[package]\nname = "should-never-be-discovered"\nversion = "0.1.0"\n',
    )
    writeFileSync(
      join(root, 'Cargo.toml'),
      '[workspace]\nmembers = ["crates/chronica-fixture", "../chronica-sync-anchor-v2-outside-root"]\n',
    )

    const { crates, escapedMembers } = discoverWorkspaceCratesWithFindings(root)
    assert.deepEqual(crates.map((c) => c.name), ['chronica-fixture'])
    assert.ok(!crates.some((c) => c.name === 'should-never-be-discovered'))
    assert.deepEqual(escapedMembers, ['../chronica-sync-anchor-v2-outside-root'])

    assert.deepEqual(discoverWorkspaceCrates(root).map((c) => c.name), ['chronica-fixture'])
  } finally {
    rmSync(root, { recursive: true, force: true })
    rmSync(join(root, '..', 'chronica-sync-anchor-v2-outside-root'), { recursive: true, force: true })
  }
})

test('discoverWorkspaceCratesWithFindings: a workspace member whose Cargo.toml is a SYMLINK escaping root is skipped and reported (repair round 5, item 2)', () => {
  const { root, cratePath } = makeMinimalCrateWorkspace('chronica-sync-anchor-v2-symlink-member-')
  const outside = join(root, '..', 'chronica-sync-anchor-v2-symlink-outside')
  try {
    mkdirSync(outside, { recursive: true })
    writeFileSync(join(outside, 'Cargo.toml'), '[package]\nname = "should-never-be-discovered-via-symlink"\nversion = "0.1.0"\n')
    mkdirSync(join(root, 'crates', 'chronica-evil'), { recursive: true })
    symlinkSync(join(outside, 'Cargo.toml'), join(root, 'crates', 'chronica-evil', 'Cargo.toml'))
    writeFileSync(join(root, 'Cargo.toml'), '[workspace]\nmembers = ["crates/chronica-fixture", "crates/chronica-evil"]\n')

    const { crates, escapedMembers } = discoverWorkspaceCratesWithFindings(root)
    assert.deepEqual(crates.map((c) => c.name), ['chronica-fixture'])
    assert.ok(!crates.some((c) => c.name === 'should-never-be-discovered-via-symlink'))
    assert.deepEqual(escapedMembers, ['crates/chronica-evil'])
  } finally {
    rmSync(root, { recursive: true, force: true })
    rmSync(outside, { recursive: true, force: true })
  }
})

test('discoverWorkspaceCratesWithFindings: an oversized member Cargo.toml is refused and reported, never partially read (repair round 5, item 4)', () => {
  const { root, cratePath } = makeMinimalCrateWorkspace('chronica-sync-anchor-v2-manifest-oversized-')
  try {
    mkdirSync(join(root, 'crates', 'chronica-huge'), { recursive: true })
    writeFileSync(join(root, 'crates', 'chronica-huge', 'Cargo.toml'), `[package]\nname = "chronica-huge"\n# ${'x'.repeat(BOUNDS.MAX_MANIFEST_BYTES + 10)}\n`)
    writeFileSync(join(root, 'Cargo.toml'), '[workspace]\nmembers = ["crates/chronica-fixture", "crates/chronica-huge"]\n')

    const { crates, manifestFindings } = discoverWorkspaceCratesWithFindings(root)
    assert.ok(!crates.some((c) => c.name === 'chronica-huge'))
    assert.equal(manifestFindings.length, 1)
    assert.equal(manifestFindings[0].reason, 'MANIFEST_OVERSIZED')
    assert.equal(manifestFindings[0].member, 'crates/chronica-huge')
  } finally {
    rmSync(root, { recursive: true, force: true })
  }
})

test('discoverWorkspaceCratesWithFindings: an oversized ROOT Cargo.toml is refused and reported, never a thrown exception', () => {
  const { root, cratePath } = makeMinimalCrateWorkspace('chronica-sync-anchor-v2-root-manifest-oversized-')
  try {
    writeFileSync(join(root, 'Cargo.toml'), `[workspace]\nmembers = ["crates/chronica-fixture"]\n# ${'x'.repeat(BOUNDS.MAX_MANIFEST_BYTES + 10)}\n`)
    const result = discoverWorkspaceCratesWithFindings(root)
    assert.deepEqual(result.crates, [])
    assert.equal(result.manifestFindings.length, 1)
    assert.equal(result.manifestFindings[0].reason, 'MANIFEST_OVERSIZED')
    assert.equal(result.manifestFindings[0].member, '(root)')
  } finally {
    rmSync(root, { recursive: true, force: true })
  }
})

// ── repair round 6, item (a): a workspace member whose Cargo.toml cannot even be RESOLVED (not ──
// ── found, or a real stat/realpath failure) is a structured finding, never a silent skip ────────

test('discoverWorkspaceCratesWithFindings: a workspace member with no directory/Cargo.toml on disk at all is a WORKSPACE_MEMBER_UNRESOLVED finding, not a silent skip', () => {
  const { root, cratePath } = makeMinimalCrateWorkspace('chronica-sync-anchor-v2-member-not-found-')
  try {
    writeFileSync(join(root, 'Cargo.toml'), '[workspace]\nmembers = ["crates/chronica-fixture", "crates/chronica-ghost"]\n')
    // crates/chronica-ghost is declared but never created -- previously this silently vanished
    // from both `crates` and every finding bucket with zero trace.
    const { crates, manifestFindings } = discoverWorkspaceCratesWithFindings(root)
    assert.deepEqual(crates.map((c) => c.name), ['chronica-fixture'])
    assert.equal(manifestFindings.length, 1)
    assert.equal(manifestFindings[0].reason, 'WORKSPACE_MEMBER_UNRESOLVED')
    assert.equal(manifestFindings[0].member, 'crates/chronica-ghost')
    assert.equal(manifestFindings[0].code, 'ENOENT')
  } finally {
    rmSync(root, { recursive: true, force: true })
  }
})

test('discoverWorkspaceCratesWithFindings: a workspace member path resolving through a symlink cycle (ELOOP) is a WORKSPACE_MEMBER_UNRESOLVED finding with the real OS code, not a silent skip', () => {
  const { root, cratePath } = makeMinimalCrateWorkspace('chronica-sync-anchor-v2-member-eloop-')
  try {
    mkdirSync(join(root, 'crates'), { recursive: true })
    // A two-hop symlink cycle: crates/loopy-a -> crates/loopy-b -> crates/loopy-a. realpathSync
    // throws ELOOP walking this regardless of user privilege (unlike EACCES, which root bypasses),
    // so this is a deterministic, portable way to exercise the genuine-UNREADABLE branch.
    symlinkSync(join(root, 'crates', 'loopy-b'), join(root, 'crates', 'loopy-a'))
    symlinkSync(join(root, 'crates', 'loopy-a'), join(root, 'crates', 'loopy-b'))
    writeFileSync(join(root, 'Cargo.toml'), '[workspace]\nmembers = ["crates/chronica-fixture", "crates/loopy-a"]\n')

    const { crates, manifestFindings } = discoverWorkspaceCratesWithFindings(root)
    assert.deepEqual(crates.map((c) => c.name), ['chronica-fixture'])
    assert.equal(manifestFindings.length, 1)
    assert.equal(manifestFindings[0].reason, 'WORKSPACE_MEMBER_UNRESOLVED')
    assert.equal(manifestFindings[0].member, 'crates/loopy-a')
    assert.equal(manifestFindings[0].code, 'ELOOP')
  } finally {
    rmSync(root, { recursive: true, force: true })
  }
})

// ── repair round 6, item (b): opendir/readdir failures are structured findings, never a thrown ──
// ── native exception ─────────────────────────────────────────────────────────────────────────

test('loadCrateDocCapabilityStatus: a docs/ path that exists but is a FILE, not a directory, is a DIR_ENUMERATION_UNREADABLE docFinding, never a thrown exception', () => {
  const { root, cratePath } = makeMinimalCrateWorkspace('chronica-sync-anchor-v2-docs-not-a-dir-')
  try {
    mkdirSync(join(root, 'docs'), { recursive: true })
    writeFileSync(join(root, 'docs', 'crates'), 'not actually a directory\n')
    const doc = loadCrateDocCapabilityStatus(root, 'chronica-fixture')
    assert.ok(doc.docFinding, 'a docs/ path that cannot be opendir-ed must produce a docFinding, never throw and never silently report zero statuses')
    assert.equal(doc.docFinding.reason, 'DIR_ENUMERATION_UNREADABLE')
    assert.equal(doc.docFinding.code, 'ENOTDIR')
    assert.equal(doc.statuses.size, 0)
  } finally {
    rmSync(root, { recursive: true, force: true })
  }
})

// ── duplicate top-level JSON key detection (fail-closed on last-write-wins ambiguity) ───────────

test('findDuplicateTopLevelKeys: no duplicates in a normal flat object', () => {
  const line = JSON.stringify({ a: 1, b: 2, c: 3 })
  assert.deepEqual(findDuplicateTopLevelKeys(line), [])
})

test('findDuplicateTopLevelKeys: detects a duplicated top-level key', () => {
  const line = '{"capability_key": "a.b", "status": "implemented", "capability_key": "a.c"}'
  assert.deepEqual(findDuplicateTopLevelKeys(line), ['capability_key'])
})

test('findDuplicateTopLevelKeys: a key repeated inside a NESTED object is not a top-level duplicate', () => {
  const line = '{"capability_key": "a.b", "nested": {"capability_key": "inner"}}'
  assert.deepEqual(findDuplicateTopLevelKeys(line), [])
})

test('findDuplicateTopLevelKeys: a key name appearing only as a STRING VALUE is not mistaken for a duplicate key', () => {
  const line = '{"capability_key": "capability_key", "status": "capability_key"}'
  assert.deepEqual(findDuplicateTopLevelKeys(line), [])
})

// ── repair round 7, item 2: a duplicate key smuggled past raw-text comparison via a Unicode ─────
// ── escape must still be detected AFTER JSON string-escape decoding ─────────────────────────────

test('findDuplicateTopLevelKeys: a key spelled once literally and once with EVERY character \\uXXXX-escaped is still detected as a duplicate', () => {
  // "capability_key" fully re-spelled using \uXXXX for every character -- byte-for-byte different
  // from the literal spelling in the raw line, but JSON.parse (and this function, post-fix) both
  // decode it to the exact same string.
  const escaped = [...'capability_key'].map((c) => `\\u${c.codePointAt(0).toString(16).padStart(4, '0')}`).join('')
  const line = `{"capability_key": "a.b", "status": "implemented", "${escaped}": "a.c"}`
  assert.deepEqual(JSON.parse(line).capability_key, 'a.c', 'sanity check: JSON.parse itself already resolves this to one last-value-wins key')
  assert.deepEqual(findDuplicateTopLevelKeys(line), ['capability_key'])
})

test('findDuplicateTopLevelKeys: a key with only ONE character \\u-escaped (mixed literal+escaped spelling) is still detected as a duplicate', () => {
  // Only the underscore is escaped (_); every other character is a literal byte -- the
  // "mixed literal/escaped" adversarial shape named explicitly in the round-7 task.
  const line = '{"capability_key": "a.b", "status": "implemented", "capability\\u005fkey": "a.c"}'
  assert.deepEqual(JSON.parse(line).capability_key, 'a.c', 'sanity check: JSON.parse resolves both spellings to the same key')
  assert.deepEqual(findDuplicateTopLevelKeys(line), ['capability_key'])
})

test('findDuplicateTopLevelKeys: two DIFFERENT keys that happen to use different escape styles for otherwise-identical characters are still distinguished (no false positive)', () => {
  const line = '{"capability_key": "a.b", "capability_kex": "a.c"}'
  assert.deepEqual(findDuplicateTopLevelKeys(line), [], 'capability_key and capability_kex are genuinely different keys -- decoding must not blur them together')
})

test('findDuplicateTopLevelKeys: common single-character escapes (\\n, \\t, \\", \\\\) are decoded consistently, not just \\uXXXX', () => {
  // A key containing a literal escaped quote character, spelled two different ways: once via \"
  // and once via the equivalent " -- both decode to the same key text (`a"b`).
  const line = '{"a\\"b": 1, "a\\u0022b": 2}'
  assert.deepEqual(findDuplicateTopLevelKeys(line), ['a"b'])
})

// ── shard record schema validation ──────────────────────────────────────────────────────────

test('validateShardRecordShape: null/array/scalar rows are NOT_AN_OBJECT', () => {
  assert.equal(validateShardRecordShape(null, 'null').valid, false)
  assert.equal(validateShardRecordShape(null, 'null').detail, 'NOT_AN_OBJECT')
  assert.equal(validateShardRecordShape([1, 2], '[1,2]').detail, 'NOT_AN_OBJECT')
  assert.equal(validateShardRecordShape('a string', '"a string"').detail, 'NOT_AN_OBJECT')
  assert.equal(validateShardRecordShape(42, '42').detail, 'NOT_AN_OBJECT')
})

test('validateShardRecordShape: missing/unrecognized record_type is UNRECOGNIZED_RECORD_TYPE', () => {
  assert.equal(validateShardRecordShape({}, '{}').detail, 'UNRECOGNIZED_RECORD_TYPE')
  assert.equal(validateShardRecordShape({ record_type: 'not_a_real_type' }, '{}').detail, 'UNRECOGNIZED_RECORD_TYPE')
  assert.equal(validateShardRecordShape({ record_type: 7 }, '{}').detail, 'UNRECOGNIZED_RECORD_TYPE')
})

test('validateShardRecordShape: "meta" rows need no further fields', () => {
  assert.equal(validateShardRecordShape({ record_type: 'meta', crate: 'x' }, '{}').valid, true)
})

test('validateShardRecordShape: every real non-capability record_type from tools/subdb/subdb-lib.mjs is recognized, not just meta', () => {
  for (const recordType of ['meta', 'architecture_node', 'architecture_edge', 'capability_architecture_link', 'architecture_target_gap']) {
    const row = { record_type: recordType }
    const result = validateShardRecordShape(row, JSON.stringify(row))
    assert.equal(result.valid, true, `record_type "${recordType}" must be recognized`)
  }
  assert.equal(validateShardRecordShape({ record_type: 'capability' }, '{}').detail, 'MISSING_OR_NON_STRING_FIELD:capability_key')
})

test('validateShardRecordShape: capability_status_correction rows are recognized and require their overlay fields', () => {
  const row = {
    record_type: 'capability_status_correction',
    capability_key: 'fixture.real',
    previous_status: 'unimplemented',
    new_status: 'implemented',
    reason_code: 'SHARD_CLAIMS_UNIMPLEMENTED_BUT_CODE_IS_REAL',
    match_confidence: 'exact',
  }
  assert.equal(validateShardRecordShape(row, JSON.stringify(row)).valid, true)
  const missing = { ...row }
  delete missing.new_status
  assert.equal(validateShardRecordShape(missing, JSON.stringify(missing)).detail, 'MISSING_OR_NON_STRING_FIELD:new_status')
})

for (const [label, value] of [
  ['null', null],
  ['array', ['x']],
  ['number', 42],
  ['boolean', true],
  ['missing', undefined],
]) {
  test(`validateShardRecordShape: capability_key=${label} (non-string) is MISSING_OR_NON_STRING_FIELD, never coerced`, () => {
    const row = { record_type: 'capability', target_module: 'm', status: 'unimplemented' }
    if (value !== undefined) row.capability_key = value
    const result = validateShardRecordShape(row, JSON.stringify(row))
    assert.equal(result.valid, false)
    assert.equal(result.detail, 'MISSING_OR_NON_STRING_FIELD:capability_key')
  })

  test(`validateShardRecordShape: target_module=${label} (non-string) is MISSING_OR_NON_STRING_FIELD, never coerced`, () => {
    const row = { record_type: 'capability', capability_key: 'a.b', status: 'unimplemented' }
    if (value !== undefined) row.target_module = value
    const result = validateShardRecordShape(row, JSON.stringify(row))
    assert.equal(result.valid, false)
    assert.equal(result.detail, 'MISSING_OR_NON_STRING_FIELD:target_module')
  })

  test(`validateShardRecordShape: status=${label} (non-string) is MISSING_OR_NON_STRING_FIELD, never coerced`, () => {
    const row = { record_type: 'capability', capability_key: 'a.b', target_module: 'm' }
    if (value !== undefined) row.status = value
    const result = validateShardRecordShape(row, JSON.stringify(row))
    assert.equal(result.valid, false)
    assert.equal(result.detail, 'MISSING_OR_NON_STRING_FIELD:status')
  })
}

test('validateShardRecordShape: an empty-string status is a TYPE pass (semantic enum check happens elsewhere, unchanged)', () => {
  const row = { record_type: 'capability', capability_key: 'a.b', target_module: 'm', status: '' }
  assert.equal(validateShardRecordShape(row, JSON.stringify(row)).valid, true)
})

test('validateShardRecordShape: a duplicated top-level JSON key is DUPLICATE_JSON_OBJECT_KEY (fail closed, never last-write-wins)', () => {
  const raw = '{"record_type":"capability","capability_key":"a.b","capability_key":"a.c","target_module":"m","status":"unimplemented"}'
  const parsed = JSON.parse(raw)
  assert.equal(parsed.capability_key, 'a.c', 'sanity: confirms the last-write-wins ambiguity this check exists to catch')
  const result = validateShardRecordShape(parsed, raw)
  assert.equal(result.valid, false)
  assert.equal(result.detail, 'DUPLICATE_JSON_OBJECT_KEY')
})

test('validateShardRecordShape: a duplicated top-level key smuggled past a raw-text scan via a Unicode escape (round 7, item 2) is still DUPLICATE_JSON_OBJECT_KEY', () => {
  const raw = '{"record_type":"capability","capability_key":"a.b","capability\\u005fkey":"a.c","target_module":"m","status":"unimplemented"}'
  const parsed = JSON.parse(raw)
  assert.equal(parsed.capability_key, 'a.c', 'sanity: JSON.parse resolves the literal and escaped spellings to the same key, last-write-wins')
  const result = validateShardRecordShape(parsed, raw)
  assert.equal(result.valid, false, 'a duplicate encoded with a Unicode escape must be caught exactly like a literal duplicate, not silently accepted')
  assert.equal(result.detail, 'DUPLICATE_JSON_OBJECT_KEY')
})

// ── parse-error diagnostics: bounded column only, never the raw V8 message/content ─────────────

test('extractParseErrorColumn: extracts only a bounded number from a "column N" message', () => {
  assert.equal(extractParseErrorColumn("Expected property name or '}' in JSON at position 2 (line 1 column 3)", 100), 3)
})

test('extractParseErrorColumn: falls back to position+1 when no column is present', () => {
  assert.equal(extractParseErrorColumn('Unexpected token o in JSON at position 5', 100), 6)
})

test('extractParseErrorColumn: returns null (never a raw string) when no number is present', () => {
  assert.equal(extractParseErrorColumn('totally unstructured message', 100), null)
})

test('extractParseErrorColumn: clamps to line length, never a wild/negative number', () => {
  assert.equal(extractParseErrorColumn('at position 99999', 10), 10)
})

// ── loadCrateShard: bounds, structural validation, duplicate keys — never throw, never AGREE silently

test('loadCrateShard: a malformed JSONL line does not throw and is recorded with its 1-based line number and a bounded column, never the raw message', () => {
  const { root, cratePath } = makeMinimalCrateWorkspace('chronica-sync-anchor-v2-malformed-')
  try {
    const lines = [
      JSON.stringify({ record_type: 'meta', crate: 'chronica-fixture' }),
      JSON.stringify(cap('fixture.real', 'real_mod', 'unimplemented')),
      '{ this line is not valid JSON',
    ]
    writeFileSync(join(cratePath, '.chronica', 'sub-cap-arch.jsonl'), lines.join('\n') + '\n')

    const crates = discoverWorkspaceCrates(root)
    assert.doesNotThrow(() => loadCrateShard(root, crates[0]))
    const shard = loadCrateShard(root, crates[0])
    assert.equal(shard.exists, true)
    assert.equal(shard.capabilities.length, 1)
    assert.equal(shard.capabilities[0].capability_key, 'fixture.real')
    assert.equal(shard.parseErrors.length, 1)
    assert.equal(shard.parseErrors[0].line, 3)
    assert.equal(typeof shard.parseErrors[0].column, 'number')
    assert.equal(Object.keys(shard.parseErrors[0]).sort().join(','), 'column,line', 'must never carry a raw message field')
  } finally {
    rmSync(root, { recursive: true, force: true })
  }
})

test('loadCrateShard: a secret-shaped string inside a malformed line never appears in the parse-error output', () => {
  const { root, cratePath } = makeMinimalCrateWorkspace('chronica-sync-anchor-v2-secret-')
  try {
    const secret = buildAdversarialSecretShapes().genericSkLiveMedium
    writeFileSync(join(cratePath, '.chronica', 'sub-cap-arch.jsonl'), `{"capability_key": "${secret}, broken\n`)
    const crates = discoverWorkspaceCrates(root)
    const shard = loadCrateShard(root, crates[0])
    assert.equal(shard.parseErrors.length, 1)
    const serialized = JSON.stringify(shard)
    assert.ok(!serialized.includes(secret), 'secret-shaped content must never appear in the shard-loading result')
  } finally {
    rmSync(root, { recursive: true, force: true })
  }
})

test('loadCrateShard: a structurally-invalid record (non-string capability_key) is SHARD_RECORD_INVALID, excluded from capabilities, never coerced', () => {
  const { root, cratePath } = makeMinimalCrateWorkspace('chronica-sync-anchor-v2-invalid-record-')
  try {
    const lines = [
      JSON.stringify(cap('fixture.real', 'real_mod', 'unimplemented')),
      JSON.stringify({ record_type: 'capability', capability_key: null, target_module: 'real_mod', status: 'unimplemented' }),
      JSON.stringify({ record_type: 'capability', capability_key: ['array'], target_module: 'real_mod', status: 'unimplemented' }),
      JSON.stringify(42),
    ]
    writeFileSync(join(cratePath, '.chronica', 'sub-cap-arch.jsonl'), lines.join('\n') + '\n')
    const crates = discoverWorkspaceCrates(root)
    const shard = loadCrateShard(root, crates[0])
    assert.equal(shard.capabilities.length, 1)
    assert.equal(shard.capabilities[0].capability_key, 'fixture.real')
    assert.equal(shard.recordErrors.length, 3)
    assert.ok(shard.recordErrors.every((e) => typeof e.line === 'number'))
    assert.equal(shard.recordErrors[0].detail, 'MISSING_OR_NON_STRING_FIELD:capability_key')
    assert.equal(shard.recordErrors[2].detail, 'NOT_AN_OBJECT')
  } finally {
    rmSync(root, { recursive: true, force: true })
  }
})

test('loadCrateShard: duplicate capability_key across two otherwise-valid rows is reported and BOTH rows are excluded from capabilities (fail-closed, never one arbitrary AGREE)', () => {
  const { root, cratePath } = makeMinimalCrateWorkspace('chronica-sync-anchor-v2-dup-key-')
  try {
    const lines = [
      JSON.stringify(cap('fixture.dup', 'real_mod', 'unimplemented')),
      JSON.stringify(cap('fixture.dup', 'real_mod', 'verified')),
      JSON.stringify(cap('fixture.unique', 'real_mod', 'unimplemented')),
    ]
    writeFileSync(join(cratePath, '.chronica', 'sub-cap-arch.jsonl'), lines.join('\n') + '\n')
    const crates = discoverWorkspaceCrates(root)
    const shard = loadCrateShard(root, crates[0])
    assert.equal(shard.capabilities.length, 1)
    assert.equal(shard.capabilities[0].capability_key, 'fixture.unique')
    assert.equal(shard.duplicateCapabilityKeys.length, 1)
    assert.deepEqual(shard.duplicateCapabilityKeys[0], { capability_key: 'fixture.dup', lines: [1, 2] })
  } finally {
    rmSync(root, { recursive: true, force: true })
  }
})

test('loadCrateShard: capability_status_correction rows overlay the effective capability status', () => {
  const { root, cratePath } = makeMinimalCrateWorkspace('chronica-sync-anchor-v2-correction-overlay-')
  try {
    const lines = [
      JSON.stringify(cap('fixture.real', 'real_mod', 'unimplemented')),
      JSON.stringify({
        record_type: 'capability_status_correction',
        capability_key: 'fixture.real',
        previous_status: 'unimplemented',
        new_status: 'implemented',
        reason_code: 'SHARD_CLAIMS_UNIMPLEMENTED_BUT_CODE_IS_REAL',
        match_confidence: 'exact',
      }),
    ]
    writeFileSync(join(cratePath, '.chronica', 'sub-cap-arch.jsonl'), lines.join('\n') + '\n')
    const crates = discoverWorkspaceCrates(root)
    const shard = loadCrateShard(root, crates[0])
    assert.equal(shard.recordErrors.length, 0)
    assert.equal(shard.capabilities.length, 1)
    assert.equal(shard.capabilities[0].capability_key, 'fixture.real')
    assert.equal(shard.capabilities[0].status, 'implemented')
    assert.equal(cap('fixture.real', 'real_mod', 'unimplemented').status, 'unimplemented')
  } finally {
    rmSync(root, { recursive: true, force: true })
  }
})

test('loadCrateShard: a within-record duplicate key smuggled via a Unicode escape (round 7, item 2) is excluded as SHARD_RECORD_INVALID, never silently accepted with the last-write-wins value', () => {
  const { root, cratePath } = makeMinimalCrateWorkspace('chronica-sync-anchor-v2-dup-key-escaped-')
  try {
    // capability_key appears once literally ("a.b") and once with the underscore \u-escaped ("a.c")
    // -- JSON.parse resolves this to "a.c" (last-write-wins), but the record must be excluded.
    const smuggled = '{"record_type":"capability","capability_key":"a.b","capability\\u005fkey":"a.c","target_module":"real_mod","status":"unimplemented"}'
    const lines = [smuggled, JSON.stringify(cap('fixture.unique', 'real_mod', 'unimplemented'))]
    writeFileSync(join(cratePath, '.chronica', 'sub-cap-arch.jsonl'), lines.join('\n') + '\n')
    const crates = discoverWorkspaceCrates(root)
    const shard = loadCrateShard(root, crates[0])
    assert.equal(shard.capabilities.length, 1, 'only the unrelated, unambiguous row is accepted')
    assert.equal(shard.capabilities[0].capability_key, 'fixture.unique')
    assert.ok(!shard.capabilities.some((c) => c.capability_key === 'a.c'), 'the smuggled-duplicate record must never be silently accepted with its last-write-wins value')
    assert.equal(shard.recordErrors.length, 1)
    assert.equal(shard.recordErrors[0].detail, 'DUPLICATE_JSON_OBJECT_KEY')
  } finally {
    rmSync(root, { recursive: true, force: true })
  }
})

test('loadCrateShard: an oversized shard (line count) is refused deterministically instead of unbounded work', () => {
  const { root, cratePath } = makeMinimalCrateWorkspace('chronica-sync-anchor-v2-oversized-')
  try {
    const rows = Array.from({ length: BOUNDS.MAX_SHARD_LINES + 5 }, () => '')
    writeFileSync(join(cratePath, '.chronica', 'sub-cap-arch.jsonl'), rows.join('\n'))
    const crates = discoverWorkspaceCrates(root)
    const shard = loadCrateShard(root, crates[0])
    assert.ok(shard.oversized)
    assert.equal(shard.capabilities.length, 0)
  } finally {
    rmSync(root, { recursive: true, force: true })
  }
})

test('loadCrateShard: a single over-length line is LINE_TOO_LONG, not an unbounded scan', () => {
  const { root, cratePath } = makeMinimalCrateWorkspace('chronica-sync-anchor-v2-longline-')
  try {
    const hugeLine = `{"record_type":"capability","capability_key":"a.b","target_module":"${'x'.repeat(BOUNDS.MAX_LINE_LENGTH + 10)}","status":"unimplemented"}`
    writeFileSync(join(cratePath, '.chronica', 'sub-cap-arch.jsonl'), hugeLine)
    const crates = discoverWorkspaceCrates(root)
    const shard = loadCrateShard(root, crates[0])
    assert.equal(shard.capabilities.length, 0)
    assert.equal(shard.recordErrors.length, 1)
    assert.equal(shard.recordErrors[0].detail, 'LINE_TOO_LONG')
  } finally {
    rmSync(root, { recursive: true, force: true })
  }
})

test('loadCrateShard: a shard file that is a SYMLINK escaping root is refused, never read (repair round 5, item 2)', () => {
  const { root, cratePath } = makeMinimalCrateWorkspace('chronica-sync-anchor-v2-shard-symlink-')
  const outside = join(root, '..', 'chronica-sync-anchor-v2-shard-symlink-outside')
  try {
    mkdirSync(outside, { recursive: true })
    const marker = 'OUTSIDE_ROOT_SHARD_CONTENT'
    writeFileSync(join(outside, 'sub-cap-arch.jsonl'), `${JSON.stringify(cap(marker, 'real_mod', 'unimplemented'))}\n`)
    rmSync(join(cratePath, '.chronica', 'sub-cap-arch.jsonl'), { force: true })
    symlinkSync(join(outside, 'sub-cap-arch.jsonl'), join(cratePath, '.chronica', 'sub-cap-arch.jsonl'))

    const crates = discoverWorkspaceCrates(root)
    const shard = loadCrateShard(root, crates[0])
    assert.equal(shard.exists, true)
    assert.ok(shard.unreadable)
    assert.equal(shard.unreadable.reason, 'ESCAPES_ROOT')
    assert.equal(shard.capabilities.length, 0)
    assert.ok(!JSON.stringify(shard).includes(marker), 'the outside-root shard content must never be read/exposed')
  } finally {
    rmSync(root, { recursive: true, force: true })
    rmSync(outside, { recursive: true, force: true })
  }
})

// ── real-repo regression guard: every live crate's own shard must validate cleanly ─────────────

test('loadCrateShard: every live crate shard in this repo has zero SHARD_RECORD_INVALID / parse-error findings against the real record-type vocabulary', () => {
  const crates = discoverWorkspaceCrates(repoRoot)
  assert.ok(crates.length > 0, 'sanity: the real workspace must resolve at least one crate')
  const offenders = []
  for (const crate of crates) {
    const shard = loadCrateShard(repoRoot, crate)
    if (!shard.exists || shard.unreadable || shard.oversized) continue
    for (const e of shard.recordErrors) offenders.push({ crate: crate.name, line: e.line, detail: e.detail })
    for (const e of shard.parseErrors) offenders.push({ crate: crate.name, line: e.line, detail: 'PARSE_ERROR' })
  }
  assert.deepEqual(offenders, [], 'no real crate shard should trip structural validation; a hit here likely means RECOGNIZED_RECORD_TYPES is missing a real record_type')
})
