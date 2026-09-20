import test from 'node:test'
import assert from 'node:assert/strict'
import { spawnSync } from 'node:child_process'
import { existsSync, mkdirSync, mkdtempSync, readFileSync, statSync, writeFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join } from 'node:path'
import { fileURLToPath } from 'node:url'
import { computeRoadmapRatio, parseInvariantTable, routeFindingsToOpportunities } from './query-roadmap-ratio.mjs'

const SCRIPT_PATH = fileURLToPath(new URL('./query-roadmap-ratio.mjs', import.meta.url))
const REPO_ROOT = fileURLToPath(new URL('../../', import.meta.url))

function runCli(args) {
  return spawnSync(process.execPath, [SCRIPT_PATH, ...args], { encoding: 'utf8' })
}

const REAL_INVARIANT_KEYS = [
  'one_money_gate',
  'one_merkle_audit',
  'scopepath_isolation',
  'screening_tighten_only',
  'cognition_tighten_only',
  'council_money_authority',
  'browser_execution_armed_only',
  'browser_evidence_required',
  'browser_payment_click_after_approval_only',
]

// ── real-repo tests: pin today's actual, independently-verified state ──────────────────────────
// These numbers were confirmed by hand (grep + python3 json parsing of the real committed
// docs/architecture-canonical/evidence.jsonl and a real crates/ directory listing) before this tool
// was written -- they are not tuned to make the tool look clean. If a future edit to
// docs/doctrines/020-roadmap.md or docs/architecture-canonical/evidence.jsonl genuinely changes this, these
// tests are SUPPOSED to fail and force a look, exactly like the analogous hardcoded-count tests in
// tools/benchmark/query-benchmark-ratio.test.mjs.
test('parseInvariantTable extracts exactly the 9 real named invariants from the live doc, in table order', () => {
  const result = computeRoadmapRatio({ root: REPO_ROOT })
  const keys = result.invariants.rows.map((r) => r.key)
  assert.deepEqual(keys, REAL_INVARIANT_KEYS)
})

test('computeRoadmapRatio(real repo): all 9 invariants are CONFIRMED against the real architecture_invariant registry', () => {
  const result = computeRoadmapRatio({ root: REPO_ROOT })
  assert.equal(result.invariants.total, 9)
  assert.equal(result.invariants.confirmed, 9)
  assert.equal(result.invariants.drifted, 0)
  assert.equal(result.invariants.unverifiable, 0)
  for (const row of result.invariants.rows) {
    assert.equal(row.classification, 'CONFIRMED', `${row.key}: ${JSON.stringify(row.diffs)}`)
    assert.equal(row.evidence.status, 'verified')
    assert.equal(row.evidence.blocker_reason, null)
    assert.equal(row.evidence.verified_by, 'tools/architecture/verify.mjs')
  }
})

test('computeRoadmapRatio(real repo): all 12 crate-existence claims are CONFIRMED against a real crates/ + Cargo.toml scan', () => {
  const result = computeRoadmapRatio({ root: REPO_ROOT })
  assert.equal(result.crate_claims.total, 12)
  assert.equal(result.crate_claims.confirmed, 12)
  assert.equal(result.crate_claims.drifted, 0)
  assert.equal(result.crate_claims.registry_citation_stale, 0)
  const byName = Object.fromEntries(result.crate_claims.rows.map((r) => [r.crate, r]))
  assert.equal(byName['chronica-ai-workforce'].claim, 'exists')
  assert.equal(byName['chronica-ai-workforce'].real_state.directory_exists, true)
  assert.equal(byName['chronica-web-compiler'].claim, 'deleted')
  assert.equal(byName['chronica-web-compiler'].real_state.directory_exists, false)
  assert.equal(byName['chronica-web-types'].claim, 'deleted')
  assert.equal(byName['chronica-web-types'].real_state.directory_exists, false)
})

test('never claims root docs/capabilities.db, docs/architecture.db, or a real cargo test run', () => {
  const result = computeRoadmapRatio({ root: REPO_ROOT })
  assert.equal(result.truth_label, 'LOCAL_AUDIT_REQUIRED')
  assert.ok(result.non_claims.some((c) => c.includes('docs/capabilities.db')))
  assert.ok(result.non_claims.some((c) => c.includes('never invokes cargo')))
  assert.match(result.architecture_invariant_evidence_caveat, /mechanical.*stamp|unconditional/)
})

// ── fixture-based tests: fully controlled drift/UNVERIFIABLE/stale scenarios ────────────────────
function writeJsonl(filePath, records) {
  writeFileSync(filePath, records.map((r) => JSON.stringify(r)).join('\n') + '\n')
}

function buildFixture({
  invariantDocStatus = 'implemented',
  invariantRegistryStatus = 'implemented',
  includeInvariantEvidence = true,
  blockerReason = null,
  includeEnforcingNode = true,
  docText = null,
  seedOpportunities = null,
} = {}) {
  const root = mkdtempSync(join(tmpdir(), 'chronica-roadmap-ratio-fixture-'))
  mkdirSync(join(root, 'docs', 'architecture-canonical'), { recursive: true })
  mkdirSync(join(root, 'docs', 'doctrines'), { recursive: true })
  mkdirSync(join(root, 'crates', 'chronica-fixture-real'), { recursive: true })
  if (seedOpportunities !== null) {
    mkdirSync(join(root, 'tools', 'capabilities'), { recursive: true })
    writeFileSync(join(root, 'tools', 'capabilities', 'opportunities.json'), JSON.stringify({ opportunities: seedOpportunities }, null, 2) + '\n')
  }

  writeFileSync(
    join(root, 'docs', 'doctrines', '020-roadmap.md'),
    docText ??
      [
        '# Fixture Roadmap',
        '',
        '## Money-safety invariants',
        '',
        '| Invariant | Meaning | Status |',
        '| --- | --- | --- |',
        `| \`fixture_invariant\` | A fixture invariant for testing. | ${invariantDocStatus} |`,
        '',
        '## Where we actually are',
        '',
        '- **`chronica-fixture-real`** — a fixture crate that exists',
        '- **`chronica-fixture-gone`** — a fixture crate that was **DELETED**',
        '',
      ].join('\n'),
  )

  writeFileSync(
    join(root, 'Cargo.toml'),
    ['[workspace]', 'resolver = "2"', 'members = [', '    "crates/chronica-fixture-real",', ']', ''].join('\n'),
  )
  writeFileSync(
    join(root, 'crates', 'chronica-fixture-real', 'Cargo.toml'),
    '[package]\nname = "chronica-fixture-real"\nversion = "0.1.0"\nedition = "2021"\n',
  )

  const nodes = includeEnforcingNode
    ? [{ schema_version: 1, record_type: 'architecture_node', id: 'fixture:node', kind: 'gate', name: 'Fixture Node', crate: 'chronica-fixture-real', module_path: null, file_path: null, status: 'generated', evidence_ref: null }]
    : []
  writeJsonl(join(root, 'docs', 'architecture-canonical', 'nodes.jsonl'), nodes)

  const evidence = [
    {
      schema_version: 1,
      record_type: 'architecture_invariant',
      key: 'fixture_invariant',
      name: 'Fixture invariant',
      description: 'test',
      enforcing_node: 'fixture:node',
      verification_command: 'cargo test -p chronica-fixture fixture',
      status: invariantRegistryStatus,
    },
  ]
  if (includeInvariantEvidence) {
    evidence.push({
      schema_version: 1,
      record_type: 'architecture_evidence',
      architecture_node: 'fixture:node',
      status: 'verified',
      evidence_kind: 'architecture_invariant',
      evidence_ref: 'architecture_invariant:fixture_invariant',
      test_command: 'cargo test -p chronica-fixture fixture',
      test_result: 'verified_by_arch_verify',
      verified_at: '2026-01-01T00:00:00.000Z',
      verified_by: 'tools/architecture/verify.mjs',
      blocker_reason: blockerReason,
    })
  }
  writeJsonl(join(root, 'docs', 'architecture-canonical', 'evidence.jsonl'), evidence)

  return root
}

test('fixture: matching doc status and registry status classifies CONFIRMED', () => {
  const root = buildFixture({ invariantDocStatus: 'implemented', invariantRegistryStatus: 'implemented' })
  const result = computeRoadmapRatio({ root })
  const row = result.invariants.rows[0]
  assert.equal(row.classification, 'CONFIRMED')
  assert.deepEqual(row.diffs, [])
})

// Regression analogue of query-benchmark-ratio.test.mjs's "Open Design vs OpenAI" false-positive
// test: the architecture_evidence row's status is ALWAYS "verified" in real data (it's an
// unconditional stamp, not a real test-execution result -- see the tool's file header). A naive
// implementation that classified CONFIRMED whenever evidence.status === 'verified' would call this
// DRIFTED case a false CONFIRMED. This fixture forces evidence.status to stay "verified" while the
// doc and registry statuses genuinely disagree, and asserts the tool still reports DRIFTED --
// proving classification depends on the registry status comparison, not the always-'verified' stamp.
test('fixture (anti-gaming): doc status vs registry status mismatch is DRIFTED even though evidence.status stays "verified"', () => {
  const root = buildFixture({ invariantDocStatus: 'implemented', invariantRegistryStatus: 'generated' })
  const result = computeRoadmapRatio({ root })
  const row = result.invariants.rows[0]
  assert.equal(row.classification, 'DRIFTED')
  assert.equal(row.evidence.status, 'verified', 'evidence stamp is verified regardless -- this is exactly the trap')
  assert.ok(row.diffs.some((d) => d.includes('status="implemented"') && d.includes('status="generated"')))
})

test('fixture: a non-null blocker_reason on the evidence row is DRIFTED', () => {
  const root = buildFixture({ blockerReason: 'invariant enforcement removed pending review' })
  const result = computeRoadmapRatio({ root })
  const row = result.invariants.rows[0]
  assert.equal(row.classification, 'DRIFTED')
  assert.ok(row.diffs.some((d) => d.includes('blocker_reason')))
})

test('fixture: missing architecture_evidence row for a registered invariant is DRIFTED, not silently CONFIRMED', () => {
  const root = buildFixture({ includeInvariantEvidence: false })
  const result = computeRoadmapRatio({ root })
  const row = result.invariants.rows[0]
  assert.equal(row.classification, 'DRIFTED')
  assert.ok(row.diffs.some((d) => d.includes('no architecture_evidence row found')))
})

test('fixture: enforcing_node missing from nodes.jsonl is DRIFTED', () => {
  const root = buildFixture({ includeEnforcingNode: false })
  const result = computeRoadmapRatio({ root })
  const row = result.invariants.rows[0]
  assert.equal(row.classification, 'DRIFTED')
  assert.ok(row.diffs.some((d) => d.includes('does not exist in docs/architecture-canonical/nodes.jsonl')))
})

test('fixture: an invariant named in the doc with no architecture_invariant registry row at all is UNVERIFIABLE', () => {
  const root = buildFixture({
    docText: [
      '## Money-safety invariants',
      '',
      '| Invariant | Meaning | Status |',
      '| --- | --- | --- |',
      '| `never_registered` | Named only in the doc. | implemented |',
      '',
    ].join('\n'),
  })
  const result = computeRoadmapRatio({ root })
  assert.equal(result.invariants.rows.length, 1)
  const row = result.invariants.rows[0]
  assert.equal(row.key, 'never_registered')
  assert.equal(row.classification, 'UNVERIFIABLE')
  assert.match(row.reason, /no architecture_invariant record with key="never_registered"/)
})

test('parseInvariantTable never picks up the header or separator row as a claimed invariant', () => {
  const rows = parseInvariantTable(['## Money-safety invariants', '', '| Invariant | Meaning | Status |', '| --- | --- | --- |', '| `real_one` | x | implemented |', '', '## Next section'].join('\n'))
  assert.equal(rows.length, 1)
  assert.equal(rows[0].key, 'real_one')
})

test('CLI smoke: summary on the real repo, run as a real child process', () => {
  const { status, stdout, stderr } = runCli(['summary'])
  assert.equal(status, 0, `expected exit 0, got ${status}; stderr: ${stderr}`)
  assert.match(stdout, /roadmap-ratio: docs\/doctrines\/020-roadmap\.md/)
  assert.match(stdout, /invariants: 9 total/)
})

test('CLI smoke: full prints parseable JSON with 9 invariants and 12 crate claims', () => {
  const { status, stdout, stderr } = runCli(['full'])
  assert.equal(status, 0, `expected exit 0, got ${status}; stderr: ${stderr}`)
  const parsed = JSON.parse(stdout)
  assert.equal(parsed.invariants.rows.length, 9)
  assert.equal(parsed.crate_claims.rows.length, 12)
})

test('CLI smoke: --root points at a fixture repo instead of the real one', () => {
  const root = buildFixture({})
  const { status, stdout, stderr } = runCli(['full', '--root', root])
  assert.equal(status, 0, `expected exit 0, got ${status}; stderr: ${stderr}`)
  const parsed = JSON.parse(stdout)
  assert.equal(parsed.invariants.rows.length, 1)
  assert.equal(parsed.invariants.rows[0].key, 'fixture_invariant')
})

// The CRATE_CLAIMS registry's citations are lifted from the REAL docs/doctrines/020-roadmap.md and are not
// present in a fixture doc at all -- this proves the registry-citation-staleness guard actually
// fires (REGISTRY_CITATION_STALE) instead of silently reusing a registry entry that can no longer be
// matched against whatever doc text is actually loaded, rather than e.g. defaulting to CONFIRMED.
test('fixture: a doc lacking every real registry citation reports REGISTRY_CITATION_STALE for all 12 crate claims, never a silent CONFIRMED', () => {
  const root = buildFixture({})
  const result = computeRoadmapRatio({ root })
  assert.equal(result.crate_claims.total, 12)
  assert.equal(result.crate_claims.registry_citation_stale, 12)
  assert.equal(result.crate_claims.confirmed, 0)
  assert.equal(result.crate_claims.drifted, 0)
  for (const row of result.crate_claims.rows) {
    assert.equal(row.classification, 'REGISTRY_CITATION_STALE')
  }
})

// ── routing: --route-findings / routeFindingsToOpportunities ───────────────────────────────────
// Real-repo run finds 0 DRIFTED/UNVERIFIABLE today (confirmed above), so exercising the actual
// routing WRITE path requires a synthetic fixture with a genuine drift -- the same approach the task
// brief calls out explicitly, mirroring how the REGISTRY_CITATION_STALE anti-gaming test above is
// fixture-driven rather than waiting for a real citation to go stale.
const OPP_CATEGORIES = new Set(['capability', 'workflow', 'integration', 'dedup', 'architecture', 'product', 'revenue', 'automation', 'ux', 'observability', 'security', 'testing', 'synergy'])
const OPP_STATUSES = new Set(['discovered', 'validated', 'planned', 'executing', 'implemented', 'rejected'])

// NOTE: buildFixture()'s default docText never contains the CRATE_CLAIMS registry's real citations
// (those are lifted from the real docs/doctrines/020-roadmap.md), so every fixture run also produces 12
// REGISTRY_CITATION_STALE crate-claim findings that get routed alongside whatever invariant finding
// the test is targeting -- these tests assert on the SPECIFIC entry they care about, not on the
// opportunities array being length 1, so that real (and correct) side effect doesn't make them flaky.
test('routeFindingsToOpportunities: a DRIFTED invariant finding is written as a new, schema-valid opportunity', () => {
  const root = buildFixture({ invariantDocStatus: 'implemented', invariantRegistryStatus: 'generated', seedOpportunities: [] })
  const result = computeRoadmapRatio({ root })
  assert.equal(result.invariants.drifted, 1)

  const routing = routeFindingsToOpportunities({ result, root })
  assert.equal(routing.attempted, true)
  assert.ok(routing.added.includes('roadmap-invariant-drift-fixture_invariant'))
  assert.deepEqual(routing.skipped_existing, [])

  const written = JSON.parse(readFileSync(join(root, 'tools', 'capabilities', 'opportunities.json'), 'utf8'))
  const entry = written.opportunities.find((o) => o.id === 'roadmap-invariant-drift-fixture_invariant')
  assert.ok(entry, 'expected the routed invariant-drift opportunity to be present')
  assert.ok(OPP_CATEGORIES.has(entry.category), `category "${entry.category}" must be in reconcile-opportunities.mjs's enum`)
  assert.ok(OPP_STATUSES.has(entry.status), `status "${entry.status}" must be in reconcile-opportunities.mjs's enum`)
  for (const scoreField of ['impact', 'reach', 'leverage', 'effort', 'confidence']) {
    assert.ok(entry[scoreField] >= 1 && entry[scoreField] <= 5, `${scoreField} must be 1..5`)
  }
  assert.match(entry.description, /fixture_invariant/)
})

test('routeFindingsToOpportunities: a REGISTRY_CITATION_STALE crate claim is routed too', () => {
  const root = buildFixture({ seedOpportunities: [] }) // default fixture doc lacks every real citation -> all 12 stale
  const result = computeRoadmapRatio({ root })
  assert.equal(result.crate_claims.registry_citation_stale, 12)

  const routing = routeFindingsToOpportunities({ result, root })
  assert.equal(routing.added.length, 12)
  assert.ok(routing.added.includes('roadmap-crate-claim-stale-citation-chronica-ai-workforce'))

  const written = JSON.parse(readFileSync(join(root, 'tools', 'capabilities', 'opportunities.json'), 'utf8'))
  assert.equal(written.opportunities.length, 12)
})

test('routeFindingsToOpportunities: idempotent -- running twice never duplicates or edits the first entry', () => {
  const root = buildFixture({ invariantDocStatus: 'implemented', invariantRegistryStatus: 'generated', seedOpportunities: [] })
  const result = computeRoadmapRatio({ root })

  const first = routeFindingsToOpportunities({ result, root })
  const firstAddedCount = first.added.length
  assert.ok(first.added.includes('roadmap-invariant-drift-fixture_invariant'))
  const afterFirst = JSON.parse(readFileSync(join(root, 'tools', 'capabilities', 'opportunities.json'), 'utf8'))

  const second = routeFindingsToOpportunities({ result, root })
  assert.equal(second.added.length, 0)
  assert.equal(second.skipped_existing.length, firstAddedCount)
  assert.ok(second.skipped_existing.includes('roadmap-invariant-drift-fixture_invariant'))
  const afterSecond = JSON.parse(readFileSync(join(root, 'tools', 'capabilities', 'opportunities.json'), 'utf8'))
  assert.deepEqual(afterSecond, afterFirst, 're-routing the same findings must not change the file at all')
})

test('routeFindingsToOpportunities: never edits or removes a pre-existing, unrelated opportunity', () => {
  const preexisting = { id: 'unrelated-existing-opportunity', title: 'Something else entirely', category: 'product', impact: 3, reach: 3, leverage: 3, effort: 3, confidence: 3, status: 'planned', created_at: '2020-01-01' }
  const root = buildFixture({ invariantDocStatus: 'implemented', invariantRegistryStatus: 'generated', seedOpportunities: [preexisting] })
  const result = computeRoadmapRatio({ root })
  const routing = routeFindingsToOpportunities({ result, root })

  const written = JSON.parse(readFileSync(join(root, 'tools', 'capabilities', 'opportunities.json'), 'utf8'))
  assert.equal(written.opportunities.length, 1 + routing.added.length)
  assert.deepEqual(written.opportunities.find((o) => o.id === 'unrelated-existing-opportunity'), preexisting)
})

// Uses a hand-built synthetic result (rather than a full fixture repo) so this test is decoupled from
// the CRATE_CLAIMS registry's always-stale-in-a-fixture-doc behavior above -- it isolates exactly the
// "genuinely nothing to route" case: every invariant CONFIRMED, every crate claim CONFIRMED.
test('routeFindingsToOpportunities: no findings to route writes nothing at all (not even a no-op rewrite)', () => {
  const root = mkdtempSync(join(tmpdir(), 'chronica-roadmap-ratio-routing-noop-'))
  mkdirSync(join(root, 'tools', 'capabilities'), { recursive: true })
  writeFileSync(join(root, 'tools', 'capabilities', 'opportunities.json'), JSON.stringify({ opportunities: [] }, null, 2) + '\n')
  const oppPath = join(root, 'tools', 'capabilities', 'opportunities.json')
  const before = statSync(oppPath).mtimeMs

  const cleanResult = {
    invariants: { rows: [{ key: 'x', classification: 'CONFIRMED', diffs: [] }] },
    crate_claims: { rows: [{ crate: 'y', claim: 'exists', classification: 'CONFIRMED', diffs: [] }] },
  }
  const routing = routeFindingsToOpportunities({ result: cleanResult, root })
  assert.equal(routing.added.length, 0)
  assert.equal(statSync(oppPath).mtimeMs, before)
})

test('routeFindingsToOpportunities: real repo, real findings (0 today) writes nothing (mtime unchanged)', () => {
  const oppPath = join(REPO_ROOT, 'tools', 'capabilities', 'opportunities.json')
  const before = statSync(oppPath).mtimeMs
  const result = computeRoadmapRatio({ root: REPO_ROOT })
  const routing = routeFindingsToOpportunities({ result, root: REPO_ROOT })
  assert.equal(routing.added.length, 0)
  assert.equal(statSync(oppPath).mtimeMs, before)
})

test('routeFindingsToOpportunities: db_reconcile is honestly reported as skipped when reconcile-opportunities.mjs is not present (fixture root)', () => {
  const root = buildFixture({ invariantDocStatus: 'implemented', invariantRegistryStatus: 'generated', seedOpportunities: [] })
  const result = computeRoadmapRatio({ root })
  const routing = routeFindingsToOpportunities({ result, root })
  assert.equal(routing.db_reconcile.attempted, false)
  assert.match(routing.db_reconcile.reason, /reconcile-opportunities\.mjs not found/)
})

test('routeFindingsToOpportunities: missing opportunities.json is reported, never crashes or fabricates a write', () => {
  const root = buildFixture({ invariantDocStatus: 'implemented', invariantRegistryStatus: 'generated' }) // seedOpportunities omitted -> no opportunities.json at all
  const result = computeRoadmapRatio({ root })
  const routing = routeFindingsToOpportunities({ result, root })
  assert.equal(routing.attempted, false)
  assert.match(routing.reason, /opportunities\.json not found/)
})

test('CLI smoke: --route-findings against a fixture repo actually writes the new opportunity', () => {
  const root = buildFixture({ invariantDocStatus: 'implemented', invariantRegistryStatus: 'generated', seedOpportunities: [] })
  const { status, stdout, stderr } = runCli(['summary', '--root', root, '--route-findings'])
  assert.equal(status, 0, `expected exit 0, got ${status}; stderr: ${stderr}`)
  assert.match(stdout, /route-findings: added \d+ new opportunity/)
  assert.match(stdout, /\+ roadmap-invariant-drift-fixture_invariant/)
  const written = JSON.parse(readFileSync(join(root, 'tools', 'capabilities', 'opportunities.json'), 'utf8'))
  assert.ok(written.opportunities.some((o) => o.id === 'roadmap-invariant-drift-fixture_invariant'))
})

test('CLI smoke: without --route-findings, no routing happens at all (opportunities.json untouched)', () => {
  const root = buildFixture({ invariantDocStatus: 'implemented', invariantRegistryStatus: 'generated', seedOpportunities: [] })
  const oppPath = join(root, 'tools', 'capabilities', 'opportunities.json')
  const before = statSync(oppPath).mtimeMs
  const { status, stdout } = runCli(['summary', '--root', root])
  assert.equal(status, 0)
  assert.doesNotMatch(stdout, /route-findings/)
  assert.equal(statSync(oppPath).mtimeMs, before)
})

test('CLI smoke: missing docs/doctrines/020-roadmap.md exits non-zero with a clear message, never a stack trace', () => {
  const root = mkdtempSync(join(tmpdir(), 'chronica-roadmap-ratio-empty-'))
  const { status, stdout, stderr } = runCli(['summary', '--root', root])
  assert.notEqual(status, 0)
  assert.equal(stdout, '')
  assert.match(stderr, /docs\/doctrines\/020-roadmap\.md not found/)
})
