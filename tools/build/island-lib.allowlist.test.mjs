import {
  test,
  assert,
  spawnSync,
  join,
  applyAllowlistPolicy,
  classifyWorkspace,
  computeIslandEvidenceFingerprint,
  ISLAND_SCAN_POLICY_VERSION,
  stableStringify,
  validateAllowlistDoc,
  here,
  repoRoot,
  makePkg,
  makeMetadata,
  todayIso,
  fingerprintFor,
  DUMMY_FINGERPRINT,
  validEntry,
  knownFor,
} from './island-lib.test-support.mjs'

test('allowlist: a well-formed entry validates cleanly', () => {
  const { errors, entries } = validateAllowlistDoc(
    { schema_version: 1, exceptions: [validEntry()] },
    { knownCrates: knownFor('chronica-fixture-lib') },
  )
  assert.deepEqual(errors, [])
  assert.equal(entries.length, 1)
})

test('allowlist: missing required field is rejected (fail-closed)', () => {
  const entry = validEntry()
  delete entry.owner
  const { errors, entries } = validateAllowlistDoc(
    { schema_version: 1, exceptions: [entry] },
    { knownCrates: knownFor('chronica-fixture-lib') },
  )
  assert.ok(errors.some((e) => e.includes('owner')))
  assert.equal(entries.length, 0)
})

test('allowlist: missing evidence_fingerprint is rejected (fail-closed)', () => {
  const entry = validEntry()
  delete entry.evidence_fingerprint
  const { errors, entries } = validateAllowlistDoc(
    { schema_version: 1, exceptions: [entry] },
    { knownCrates: knownFor('chronica-fixture-lib') },
  )
  assert.ok(errors.some((e) => e.includes('evidence_fingerprint')))
  assert.equal(entries.length, 0)
})

test('allowlist: a malformed (non-hex, wrong-length) evidence_fingerprint is rejected', () => {
  for (const bad of ['not-a-hash', 'a'.repeat(63), 'a'.repeat(65), 'A'.repeat(64), '']) {
    const { errors } = validateAllowlistDoc(
      { schema_version: 1, exceptions: [validEntry({ evidence_fingerprint: bad })] },
      { knownCrates: knownFor('chronica-fixture-lib') },
    )
    assert.ok(errors.some((e) => e.includes('evidence_fingerprint')), `expected rejection for ${JSON.stringify(bad)}`)
  }
})

test('allowlist: wildcard crate identity is rejected', () => {
  const { errors, entries } = validateAllowlistDoc(
    { schema_version: 1, exceptions: [validEntry({ crate: 'chronica-*' })] },
    { knownCrates: knownFor('chronica-fixture-lib') },
  )
  assert.ok(errors.some((e) => e.includes('wildcard')))
  assert.equal(entries.length, 0)
})

test('allowlist: unknown crate (not a current workspace member) is rejected', () => {
  const { errors, entries } = validateAllowlistDoc(
    { schema_version: 1, exceptions: [validEntry({ crate: 'chronica-does-not-exist' })] },
    { knownCrates: knownFor('chronica-fixture-lib') },
  )
  assert.ok(errors.some((e) => e.includes('unknown crate')))
  assert.equal(entries.length, 0)
})

test('allowlist: duplicate crate entries are rejected', () => {
  const { errors, entries } = validateAllowlistDoc(
    { schema_version: 1, exceptions: [validEntry(), validEntry()] },
    { knownCrates: knownFor('chronica-fixture-lib') },
  )
  assert.ok(errors.some((e) => e.includes('duplicate')))
})

test('allowlist: a grace period beyond the bounded maximum is rejected (no permanent exemption)', () => {
  const { errors, entries } = validateAllowlistDoc(
    { schema_version: 1, exceptions: [validEntry({ created: '2026-01-01', expires: '2027-01-01' })] },
    { knownCrates: knownFor('chronica-fixture-lib') },
  )
  assert.ok(errors.some((e) => e.includes('exceeds the bounded maximum')))
  assert.equal(entries.length, 0)
})

test('allowlist: expires not after created is rejected', () => {
  const { errors } = validateAllowlistDoc(
    { schema_version: 1, exceptions: [validEntry({ created: '2026-07-16', expires: '2026-07-16' })] },
    { knownCrates: knownFor('chronica-fixture-lib') },
  )
  assert.ok(errors.some((e) => e.includes('strictly after')))
})

test('allowlist: malformed dates are rejected', () => {
  const { errors } = validateAllowlistDoc(
    { schema_version: 1, exceptions: [validEntry({ created: 'not-a-date' })] },
    { knownCrates: knownFor('chronica-fixture-lib') },
  )
  assert.ok(errors.some((e) => e.includes('"created" must be an ISO 8601 date')))
})

test('allowlist: an issue URL outside this repo is rejected', () => {
  const { errors } = validateAllowlistDoc(
    { schema_version: 1, exceptions: [validEntry({ issue: 'https://example.com/not-github' })] },
    { knownCrates: knownFor('chronica-fixture-lib') },
  )
  assert.ok(errors.some((e) => e.includes('"issue"')))
})

test('allowlist: wrong schema_version is rejected', () => {
  const { errors } = validateAllowlistDoc(
    { schema_version: 2, exceptions: [validEntry()] },
    { knownCrates: knownFor('chronica-fixture-lib') },
  )
  assert.ok(errors.some((e) => e.includes('ALLOWLIST_SCHEMA_VERSION_MISMATCH')))
})

test('allowlist: non-array exceptions field is rejected', () => {
  const { errors } = validateAllowlistDoc({ schema_version: 1, exceptions: 'nope' }, { knownCrates: [] })
  assert.ok(errors.some((e) => e.includes('must be an array')))
})

test('allowlist: non-object root is rejected', () => {
  const { errors } = validateAllowlistDoc(null, { knownCrates: [] })
  assert.ok(errors.some((e) => e.includes('ALLOWLIST_MALFORMED')))
})

// ── allowlist policy: fingerprint is authority, prose is documentation ─────

test('policy: an ISLAND crate with no allowlist entry is BLOCKED', () => {
  const metadata = makeMetadata([
    makePkg('fx-api', { bin: true, lib: false }),
    makePkg('fx-unallowlisted-island', { deps: [] }),
  ])
  const scan = classifyWorkspace(metadata)
  const policy = applyAllowlistPolicy(scan, [], { today: todayIso(0) })
  assert.equal(policy.pass, false)
  assert.deepEqual(policy.blocked_crates, ['fx-unallowlisted-island'])
})

test('policy: an ISLAND crate with a valid, unexpired, fingerprint-matching entry is ALLOWLISTED_GRACE_PERIOD and the gate passes', () => {
  const metadata = makeMetadata([
    makePkg('fx-api', { bin: true, lib: false }),
    makePkg('chronica-fixture-lib', { deps: [] }),
  ])
  const scan = classifyWorkspace(metadata)
  const entry = validEntry({ evidence_fingerprint: fingerprintFor(scan, 'chronica-fixture-lib') })
  const policy = applyAllowlistPolicy(scan, [entry], { today: todayIso(0) })
  assert.equal(policy.pass, true)
  assert.deepEqual(policy.allowlisted_crates, ['chronica-fixture-lib'])
})

test('policy: an allowlist entry past its expiry date re-blocks the crate (fail-closed after grace period)', () => {
  const metadata = makeMetadata([
    makePkg('fx-api', { bin: true, lib: false }),
    makePkg('chronica-fixture-lib', { deps: [] }),
  ])
  const scan = classifyWorkspace(metadata)
  const entry = validEntry({
    evidence_fingerprint: fingerprintFor(scan, 'chronica-fixture-lib'),
    created: todayIso(-20),
    expires: todayIso(-1), // expired yesterday
  })
  const policy = applyAllowlistPolicy(scan, [entry], { today: todayIso(0) })
  assert.equal(policy.pass, false)
  assert.deepEqual(policy.blocked_crates, ['chronica-fixture-lib'])
  const c = policy.crates.find((c) => c.crate === 'chronica-fixture-lib')
  assert.equal(c.final_status, 'BLOCKED')
  assert.equal(c.exception.expired, true)
})

test('policy: LIVE and ENTRYPOINT crates with NO allowlist entry pass through untouched', () => {
  const metadata = makeMetadata([
    makePkg('fx-api', { bin: true, lib: false, deps: [{ name: 'fx-live' }] }),
    makePkg('fx-live', { deps: [] }),
  ])
  const scan = classifyWorkspace(metadata)
  const policy = applyAllowlistPolicy(scan, [], { today: todayIso(0) })
  assert.equal(policy.pass, true)
  const live = policy.crates.find((c) => c.crate === 'fx-live')
  assert.equal(live.final_status, 'LIVE')
  assert.equal(live.exception, null)
})

// ── P2 mutation: evidence tamper — the fingerprint is authority, prose is not ──

test('P2 mutation (fingerprint tamper): a tampered/incorrect evidence_fingerprint is detected and blocks the gate, even though the crate is still genuinely ISLAND', () => {
  const metadata = makeMetadata([
    makePkg('fx-api', { bin: true, lib: false }),
    makePkg('chronica-fixture-lib', { deps: [] }),
  ])
  const scan = classifyWorkspace(metadata)
  const realFp = fingerprintFor(scan, 'chronica-fixture-lib')
  const tamperedFp = realFp.replace(/^./, realFp[0] === '0' ? '1' : '0') // flip one hex digit
  const entry = validEntry({ evidence_fingerprint: tamperedFp })
  const policy = applyAllowlistPolicy(scan, [entry], { today: todayIso(0) })
  assert.equal(policy.pass, false)
  const c = policy.crates.find((c) => c.crate === 'chronica-fixture-lib')
  assert.equal(c.final_status, 'BLOCKED')
  assert.equal(c.exception.stale_reason, 'FINGERPRINT_MISMATCH')
  assert.equal(c.exception.current_fingerprint, realFp)
  assert.equal(policy.stale_exceptions.length, 1)
  assert.equal(policy.stale_exceptions[0].reason, 'FINGERPRINT_MISMATCH')
})

test('P2 (fingerprint is authority, prose evidence is documentation only): a wrong/misleading "evidence" prose string does NOT block the gate as long as evidence_fingerprint matches reality', () => {
  const metadata = makeMetadata([
    makePkg('fx-api', { bin: true, lib: false }),
    makePkg('chronica-fixture-lib', { deps: [] }),
  ])
  const scan = classifyWorkspace(metadata)
  const entry = validEntry({
    evidence_fingerprint: fingerprintFor(scan, 'chronica-fixture-lib'),
    evidence: 'this text is completely wrong and describes a different crate entirely',
  })
  const policy = applyAllowlistPolicy(scan, [entry], { today: todayIso(0) })
  // Passes on the strength of the correct fingerprint alone — proving prose
  // is never consulted as authority, exactly as documented in the allowlist
  // file's _comment and tools/build/island-lib.mjs's fingerprint doc comment.
  assert.equal(policy.pass, true)
})

// ── P2 mutation: a newly added production caller makes an exception stale ──

test('P2 mutation (chronica-automation-shaped: real new caller): an ISLAND that becomes LIVE makes its allowlist entry stale and fails the gate until removed — never a silent pass', () => {
  const beforeMetadata = makeMetadata([
    makePkg('fx-cli-like', { bin: true, lib: false }),
    makePkg('chronica-fixture-lib', { deps: [] }),
  ])
  const beforeScan = classifyWorkspace(beforeMetadata)
  const entry = validEntry({ evidence_fingerprint: fingerprintFor(beforeScan, 'chronica-fixture-lib') })
  const beforePolicy = applyAllowlistPolicy(beforeScan, [entry], { today: todayIso(0) })
  assert.equal(beforePolicy.pass, true)
  assert.deepEqual(beforePolicy.allowlisted_crates, ['chronica-fixture-lib'])

  // A real production caller lands (mirrors the reviewer's live experiment:
  // wiring chronica-automation into chronica-cli) — the allowlist entry is
  // left untouched, exactly as a developer might forget to clean it up.
  const afterMetadata = makeMetadata([
    makePkg('fx-cli-like', { bin: true, lib: false, deps: [{ name: 'chronica-fixture-lib' }] }),
    makePkg('chronica-fixture-lib', { deps: [] }),
  ])
  const afterScan = classifyWorkspace(afterMetadata)
  assert.equal(afterScan.crates.find((c) => c.crate === 'chronica-fixture-lib').status, 'LIVE')

  const afterPolicy = applyAllowlistPolicy(afterScan, [entry], { today: todayIso(0) })
  assert.equal(afterPolicy.pass, false, 'a stale now-LIVE allowlist entry must fail the gate, not pass silently')
  assert.equal(afterPolicy.stale_exceptions.length, 1)
  assert.equal(afterPolicy.stale_exceptions[0].crate, 'chronica-fixture-lib')
  assert.equal(afterPolicy.stale_exceptions[0].reason, 'NOW_LIVE_OR_ENTRYPOINT')
  const finalEntry = afterPolicy.crates.find((c) => c.crate === 'chronica-fixture-lib')
  assert.equal(finalEntry.status, 'LIVE')
  assert.equal(finalEntry.final_status, 'LIVE')
  assert.equal(finalEntry.exception.stale, true)

  // Removing the stale entry (the intended remediation) restores a clean pass.
  const cleanedPolicy = applyAllowlistPolicy(afterScan, [], { today: todayIso(0) })
  assert.equal(cleanedPolicy.pass, true)
})

// ── P2 mutation: dev-only edge SOURCE rename invalidates the fingerprint ───

test('P2 mutation (dev-edge source rename): renaming the crate on the other end of a dev-only edge changes the fingerprint and makes the old exception stale', () => {
  const beforeMetadata = makeMetadata([
    makePkg('fx-api', { bin: true, lib: false }),
    makePkg('chronica-fixture-lib', { deps: [] }),
    makePkg('chronica-old-tester-name', { deps: [{ name: 'chronica-fixture-lib', kind: 'dev' }] }),
  ])
  const beforeScan = classifyWorkspace(beforeMetadata)
  const before = beforeScan.crates.find((c) => c.crate === 'chronica-fixture-lib')
  assert.equal(before.status, 'ISLAND')
  const entry = validEntry({ evidence_fingerprint: fingerprintFor(beforeScan, 'chronica-fixture-lib') })

  // Mutate: the dev-dependent crate is renamed (e.g. a crate rename PR) —
  // same relationship, same "shape" of evidence, different source identity.
  const afterMetadata = makeMetadata([
    makePkg('fx-api', { bin: true, lib: false }),
    makePkg('chronica-fixture-lib', { deps: [] }),
    makePkg('chronica-renamed-tester', { deps: [{ name: 'chronica-fixture-lib', kind: 'dev' }] }),
  ])
  const afterScan = classifyWorkspace(afterMetadata)
  const after = afterScan.crates.find((c) => c.crate === 'chronica-fixture-lib')
  assert.equal(after.status, 'ISLAND') // still an island in shape...
  assert.match(after.reason, /chronica-renamed-tester/)

  const policy = applyAllowlistPolicy(afterScan, [entry], { today: todayIso(0) })
  assert.equal(policy.pass, false, 'a rename on the dev-only source must invalidate the fingerprint, not silently keep passing')
  assert.equal(policy.crates.find((c) => c.crate === 'chronica-fixture-lib').exception.stale_reason, 'FINGERPRINT_MISMATCH')
})

// ── P2 mutation: feature/target edge change invalidates the fingerprint ────

test('P2 mutation (feature/target edge change): toggling a production edge from feature-gated to unconditional (or vice versa) changes the fingerprint', () => {
  const metadataA = makeMetadata([
    makePkg('fx-api', { bin: true, lib: false, deps: [{ name: 'chronica-fixture-lib', optional: true }] }),
    makePkg('chronica-fixture-lib', { deps: [] }),
  ])
  const scanA = classifyWorkspace(metadataA)
  const cA = scanA.crates.find((c) => c.crate === 'chronica-fixture-lib')
  assert.equal(cA.status, 'LIVE')
  const fpA = computeIslandEvidenceFingerprint(cA)

  const metadataB = makeMetadata([
    makePkg('fx-api', { bin: true, lib: false, deps: [{ name: 'chronica-fixture-lib', optional: false }] }),
    makePkg('chronica-fixture-lib', { deps: [] }),
  ])
  const scanB = classifyWorkspace(metadataB)
  const cB = scanB.crates.find((c) => c.crate === 'chronica-fixture-lib')
  const fpB = computeIslandEvidenceFingerprint(cB)

  assert.notEqual(fpA, fpB, 'feature-gated vs. unconditional must produce different fingerprints even though both are LIVE')

  // Same principle proven on the ISLAND side, where it actually gates the build:
  const islandMetadataA = makeMetadata([
    makePkg('fx-api', { bin: true, lib: false }),
    makePkg('fx-other', { deps: [{ name: 'chronica-fixture-lib', target: 'cfg(windows)' }] }),
    makePkg('chronica-fixture-lib', { deps: [] }),
  ])
  // fx-other is itself unreachable from fx-api, so chronica-fixture-lib stays ISLAND
  // (production edge exists but not from a reachable crate), letting us fingerprint
  // an ISLAND whose evidence includes a target-specific edge.
  const islandScanA = classifyWorkspace(islandMetadataA)
  const islandCA = islandScanA.crates.find((c) => c.crate === 'chronica-fixture-lib')
  assert.equal(islandCA.status, 'ISLAND')
  const entry = validEntry({ evidence_fingerprint: computeIslandEvidenceFingerprint(islandCA) })

  const islandMetadataB = makeMetadata([
    makePkg('fx-api', { bin: true, lib: false }),
    makePkg('fx-other', { deps: [{ name: 'chronica-fixture-lib', target: 'cfg(unix)' }] }), // target changed
    makePkg('chronica-fixture-lib', { deps: [] }),
  ])
  const islandScanB = classifyWorkspace(islandMetadataB)
  const policy = applyAllowlistPolicy(islandScanB, [entry], { today: todayIso(0) })
  assert.equal(policy.pass, false, 'a target-cfg change on the underlying edge must invalidate the fingerprint')
  assert.equal(policy.crates.find((c) => c.crate === 'chronica-fixture-lib').exception.stale_reason, 'FINGERPRINT_MISMATCH')
})
