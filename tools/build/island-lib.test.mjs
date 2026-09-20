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

test('current-repo truth: real cargo metadata classifies known entrypoints and known islands', (t) => {
  const probe = spawnSync('cargo', ['--version'], { encoding: 'utf8' })
  if (probe.status !== 0) {
    t.skip('cargo toolchain not available in this environment')
    return
  }
  const result = spawnSync('cargo', ['metadata', '--format-version', '1', '--no-deps'], {
    cwd: repoRoot,
    encoding: 'utf8',
    maxBuffer: 64 * 1024 * 1024,
  })
  assert.equal(result.status, 0, result.stderr)
  const metadata = JSON.parse(result.stdout)
  const scan = classifyWorkspace(metadata)
  assert.deepEqual(scan.errors, [])

  const byName = new Map(scan.crates.map((c) => [c.crate, c]))

  // 2026-09-16 hard refoundation (issue #5324): the pre-reset workspace's
  // chronica-api/chronica-cli entrypoints and chronica-automation/
  // chronica-network-negotiation/chronica-organization/chronica-tempo-tracing
  // islands were deleted wholesale along with the rest of crates/ (see
  // docs/decisions/0016-legacy-backend-retired-to-git-history.md and tools/refoundation/legacy-crates-inventory-2026-09-16.txt). The minimal bootstrap workspace
  // (core/runtime/adapter/organism) has no shipped binary entry point at
  // all yet, so it has zero ENTRYPOINT crates by construction — that is the
  // real, disclosed, tracked gap, not something to work around here.
  const entrypoints = scan.crates.filter((c) => c.status === 'ENTRYPOINT').map((c) => c.crate)
  assert.deepEqual(entrypoints, [])

  // Real, currently-unresolved islands (issue #5324). This assertion is
  // intentionally exact truth, not a permissive subset check: if this list
  // changes, it means a crate was promoted/retired (update the allowlist +
  // this test together) or a NEW island appeared (investigate before
  // touching this test — do not loosen it to make CI green).
  const knownIslands = ['adapter', 'core', 'organism', 'runtime']
  const actualIslands = scan.crates.filter((c) => c.status === 'ISLAND').map((c) => c.crate).sort()
  assert.deepEqual(actualIslands, knownIslands.slice().sort())

  // core's reverse dependents are all production edges from the other three
  // bootstrap crates, none of them transitively reachable from an entry
  // point (there is none yet) — still correctly ISLAND, not LIVE.
  const core = byName.get('core')
  assert.equal(core.reverse_dependents.production.length, 3)
  assert.ok(core.reverse_dependents.production.some((e) => e.from === 'runtime'))
  assert.ok(core.reverse_dependents.production.some((e) => e.from === 'adapter'))
  assert.ok(core.reverse_dependents.production.some((e) => e.from === 'organism'))
  assert.match(core.reason, /none is transitively reachable from a shipped entry point/)
})

// ── fixture orphan failure ──────────────────────────────────────────────────

test('fixture orphan: a lib crate with zero reverse dependents is flagged ISLAND', () => {
  const metadata = makeMetadata([
    makePkg('fx-cli', { bin: true, lib: false }),
    makePkg('fx-used', { deps: [] }),
    makePkg('fx-orphan', { deps: [] }), // nobody depends on this
  ])
  // fx-cli must depend on fx-used to make fx-used LIVE, contrasted against fx-orphan.
  metadata.packages[0].dependencies.push({ name: 'fx-used', kind: null, optional: false, target: null })

  const scan = classifyWorkspace(metadata)
  const byName = new Map(scan.crates.map((c) => [c.crate, c]))
  assert.equal(byName.get('fx-orphan').status, 'ISLAND')
  assert.equal(byName.get('fx-used').status, 'LIVE')
  assert.match(byName.get('fx-orphan').reason, /zero reverse Cargo dependents/)
})

// ── external/binary entrypoint non-false-positive ──────────────────────────

test('binary entry points are never flagged ISLAND even with zero reverse-lib-dependents', () => {
  const metadata = makeMetadata([
    makePkg('fx-api', { bin: true, lib: false }),
    makePkg('fx-worker-cli', { bin: true, lib: false }),
  ])
  const scan = classifyWorkspace(metadata)
  for (const c of scan.crates) {
    assert.equal(c.status, 'ENTRYPOINT')
    assert.equal(c.is_entrypoint, true)
  }
})

test('a lib+bin crate is an entrypoint regardless of reverse-lib-dependents', () => {
  const metadata = makeMetadata([makePkg('fx-dual', { bin: true, lib: true })])
  const scan = classifyWorkspace(metadata)
  assert.equal(scan.crates[0].status, 'ENTRYPOINT')
  assert.equal(scan.crates[0].crate_kind, 'lib+bin')
})

// ── feature-gated / target-specific edge handling ───────────────────────────

test('an optional (feature-gated) production edge still counts as a real caller, tagged distinctly', () => {
  const metadata = makeMetadata([
    makePkg('fx-api', { bin: true, lib: false, deps: [{ name: 'fx-optional-feature', optional: true }] }),
    makePkg('fx-optional-feature', { deps: [] }),
  ])
  const scan = classifyWorkspace(metadata)
  const c = scan.crates.find((c) => c.crate === 'fx-optional-feature')
  assert.equal(c.status, 'LIVE')
  assert.equal(c.reachable_from_entrypoint, true)
  assert.equal(c.reverse_dependents.production[0].feature_gated, true)
})

test('a target-specific (cfg-gated) production edge still counts as a real caller, tagged distinctly', () => {
  const metadata = makeMetadata([
    makePkg('fx-api', { bin: true, lib: false, deps: [{ name: 'fx-target-gated', target: 'cfg(windows)' }] }),
    makePkg('fx-target-gated', { deps: [] }),
  ])
  const scan = classifyWorkspace(metadata)
  const c = scan.crates.find((c) => c.crate === 'fx-target-gated')
  assert.equal(c.status, 'LIVE')
  assert.equal(c.reverse_dependents.production[0].target, 'cfg(windows)')
})

// ── test-only fake caller rejection ─────────────────────────────────────────

test('a crate whose only reverse dependent is a dev-dependency FROM A DIFFERENT CRATE is ISLAND, not LIVE, and never called a self-reference', () => {
  const metadata = makeMetadata([
    makePkg('fx-api', { bin: true, lib: false }),
    makePkg('fx-has-test-caller-only', { deps: [] }),
    makePkg('fx-test-caller', { deps: [{ name: 'fx-has-test-caller-only', kind: 'dev' }] }),
  ])
  const scan = classifyWorkspace(metadata)
  const c = scan.crates.find((c) => c.crate === 'fx-has-test-caller-only')
  assert.equal(c.status, 'ISLAND')
  assert.equal(c.reverse_dependents.production.length, 0)
  assert.equal(c.reverse_dependents.dev_only.length, 1)
  assert.match(c.reason, /dev-only \(test\/bench\/example\) reverse dependent\(s\) from a different crate: fx-test-caller/)
  assert.doesNotMatch(c.reason, /self-reference/)
})

test('P1 regression: a dev-only edge whose source is genuinely the SAME crate is (correctly) called a self-reference', () => {
  const metadata = makeMetadata([
    makePkg('fx-api', { bin: true, lib: false }),
    makePkg('fx-self', { deps: [{ name: 'fx-self', kind: 'dev' }] }),
  ])
  const scan = classifyWorkspace(metadata)
  const c = scan.crates.find((c) => c.crate === 'fx-self')
  assert.equal(c.status, 'ISLAND')
  assert.match(c.reason, /genuine dev-only self-reference/)
})

test('P1 regression: a crate with BOTH a genuine self-reference and a cross-crate dev-only edge describes both accurately', () => {
  const metadata = makeMetadata([
    makePkg('fx-api', { bin: true, lib: false }),
    makePkg('fx-mixed', { deps: [{ name: 'fx-mixed', kind: 'dev' }] }),
    makePkg('fx-other-tester', { deps: [{ name: 'fx-mixed', kind: 'dev' }] }),
  ])
  const scan = classifyWorkspace(metadata)
  const c = scan.crates.find((c) => c.crate === 'fx-mixed')
  assert.equal(c.status, 'ISLAND')
  assert.match(c.reason, /from a different crate: fx-other-tester/)
  assert.match(c.reason, /genuine dev-only self-reference/)
})

test('a crate whose only reverse dependent is a build-dependency is still ISLAND, not LIVE', () => {
  const metadata = makeMetadata([
    makePkg('fx-api', { bin: true, lib: false }),
    makePkg('fx-build-tool-only', { deps: [] }),
    makePkg('fx-build-consumer', { deps: [{ name: 'fx-build-tool-only', kind: 'build' }] }),
  ])
  const scan = classifyWorkspace(metadata)
  const c = scan.crates.find((c) => c.crate === 'fx-build-tool-only')
  assert.equal(c.status, 'ISLAND')
  assert.equal(c.reverse_dependents.build_only.length, 1)
  assert.match(c.reason, /build-time tooling is not a shipped production caller/)
})

// ── transitive reachability ─────────────────────────────────────────────────

test('a crate reachable only transitively (A -> B -> C) is LIVE, not ISLAND', () => {
  const metadata = makeMetadata([
    makePkg('fx-api', { bin: true, lib: false, deps: [{ name: 'fx-mid' }] }),
    makePkg('fx-mid', { deps: [{ name: 'fx-leaf' }] }),
    makePkg('fx-leaf', { deps: [] }),
  ])
  const scan = classifyWorkspace(metadata)
  const leaf = scan.crates.find((c) => c.crate === 'fx-leaf')
  assert.equal(leaf.status, 'LIVE')
  assert.equal(leaf.reachable_from_entrypoint, true)
})

test('a crate depended on only by an already-unreachable crate is still ISLAND (disconnected subgraph)', () => {
  const metadata = makeMetadata([
    makePkg('fx-api', { bin: true, lib: false }), // does NOT depend on fx-orphan-mid
    makePkg('fx-orphan-mid', { deps: [{ name: 'fx-orphan-leaf' }] }),
    makePkg('fx-orphan-leaf', { deps: [] }),
  ])
  const scan = classifyWorkspace(metadata)
  const byName = new Map(scan.crates.map((c) => [c.crate, c]))
  assert.equal(byName.get('fx-orphan-mid').status, 'ISLAND')
  assert.equal(byName.get('fx-orphan-leaf').status, 'ISLAND')
})

// ── unknown additional entrypoint ───────────────────────────────────────────

test('an unknown additional-entrypoint override is a scan-level error, not silently ignored', () => {
  const metadata = makeMetadata([makePkg('fx-lonely', { deps: [] })])
  const scan = classifyWorkspace(metadata, { additionalEntrypoints: ['fx-does-not-exist'] })
  assert.ok(scan.errors.some((e) => e.includes('fx-does-not-exist')))
})

// ── deterministic replay ────────────────────────────────────────────────────

test('replay: classifying the same metadata twice yields byte-identical stableStringify output', () => {
  const metadata = makeMetadata([
    makePkg('fx-api', { bin: true, lib: false, deps: [{ name: 'fx-a' }, { name: 'fx-b', optional: true }] }),
    makePkg('fx-a', { deps: [] }),
    makePkg('fx-b', { deps: [] }),
    makePkg('fx-c', { deps: [] }), // island
  ])
  const first = stableStringify(classifyWorkspace(metadata))
  const second = stableStringify(classifyWorkspace(metadata))
  assert.equal(first, second)
})

test('stableStringify sorts object keys regardless of insertion order', () => {
  const a = stableStringify({ z: 1, a: 2, m: { y: 1, b: 2 } })
  const b = stableStringify({ a: 2, m: { b: 2, y: 1 }, z: 1 })
  assert.equal(a, b)
})

test('replay: computeIslandEvidenceFingerprint is deterministic for the same scan entry', () => {
  const metadata = makeMetadata([
    makePkg('fx-api', { bin: true, lib: false }),
    makePkg('fx-island', { deps: [] }),
  ])
  const scan = classifyWorkspace(metadata)
  const c = scan.crates.find((c) => c.crate === 'fx-island')
  const fp1 = computeIslandEvidenceFingerprint(c)
  const fp2 = computeIslandEvidenceFingerprint(c)
  assert.equal(fp1, fp2)
  assert.match(fp1, /^[0-9a-f]{64}$/)
})

// ── mutation: classifier is evidence-driven, not hardcoded ─────────────────

test('mutation: a previously LIVE crate flips to ISLAND when its only caller edge is removed', () => {
  const before = makeMetadata([
    makePkg('fx-api', { bin: true, lib: false, deps: [{ name: 'fx-mutating' }] }),
    makePkg('fx-mutating', { deps: [] }),
  ])
  const beforeScan = classifyWorkspace(before)
  assert.equal(beforeScan.crates.find((c) => c.crate === 'fx-mutating').status, 'LIVE')

  // Mutate: fx-api no longer depends on fx-mutating (its only production edge removed).
  const after = makeMetadata([
    makePkg('fx-api', { bin: true, lib: false, deps: [] }),
    makePkg('fx-mutating', { deps: [] }),
  ])
  const afterScan = classifyWorkspace(after)
  assert.equal(afterScan.crates.find((c) => c.crate === 'fx-mutating').status, 'ISLAND')
})
