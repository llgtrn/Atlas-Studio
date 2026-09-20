import test from 'node:test'
import assert from 'node:assert/strict'
import { spawnSync } from 'node:child_process'
import { existsSync, mkdirSync, mkdtempSync, statSync, writeFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join } from 'node:path'
import { fileURLToPath } from 'node:url'
import { findVerificationCandidates, tokenizeCapabilityKey, tokenizeName } from './find-verification-candidates.mjs'

const SCRIPT_PATH = fileURLToPath(new URL('./find-verification-candidates.mjs', import.meta.url))
const REPO_ROOT = fileURLToPath(new URL('../../', import.meta.url))

function runCli(args) {
  return spawnSync(process.execPath, [SCRIPT_PATH, ...args], { encoding: 'utf8' })
}

test('tokenizeCapabilityKey drops the domain prefix and short/stopword tokens', () => {
  assert.deepEqual(tokenizeCapabilityKey('policy.consent_propagation_outbox'), ['consent', 'propagation', 'outbox'])
  assert.deepEqual(tokenizeCapabilityKey('other.openapi_gateway'), ['openapi', 'gateway'])
  assert.deepEqual(tokenizeCapabilityKey('erp.a_of_to'), []) // all short/stopword after domain strip
})

test('tokenizeName filters stopwords and short words', () => {
  assert.deepEqual(tokenizeName('Consent Withdrawal Propagation-Intent Outbox'), ['consent', 'withdrawal', 'propagation', 'intent', 'outbox'])
})

// ── real-repo tests ──────────────────────────────────────────────────────────────────────────────
// This is a real, known-good match found by hand-reading crates/chronica-consent/src/propagation.rs
// + propagation_tests.rs before this tool was written (capability policy.consent_propagation_outbox,
// status implemented_unverified) -- used as a positive-control anchor so a future regression that
// breaks matching entirely (e.g. an attribute-detection bug) is caught, not just "0 candidates found"
// silently passing as if that were always fine.
test('real repo: the known consent_propagation_outbox capability is found as a HIGH-confidence candidate', () => {
  const result = findVerificationCandidates({ root: REPO_ROOT })
  const row = result.results.find((r) => r.capability_key === 'policy.consent_propagation_outbox')
  assert.ok(row, 'expected policy.consent_propagation_outbox to be in scope')
  assert.equal(row.classification, 'CANDIDATE')
  assert.equal(row.confidence, 'HIGH')
  assert.equal(row.target_crate, 'chronica-consent')
  assert.match(row.candidate_test_file, /propagation_tests\.rs$/)
  assert.ok(row.matched_tokens.primary.includes('propagation') || row.matched_tokens.primary.includes('consent'))
})

test('real repo: never claims root docs/capabilities.db authority, never claims to have called record-verified.mjs', () => {
  const result = findVerificationCandidates({ root: REPO_ROOT })
  assert.equal(result.truth_label, 'HEURISTIC_CANDIDATE_ONLY')
  assert.ok(result.non_claims.some((c) => c.includes('never calls tools/capabilities/record-verified.mjs')))
  assert.ok(result.authority.includes('never writes anything'))
})

test('real repo: scope is limited to implemented_unverified/implemented, never touches the 5000+ unimplemented rows', () => {
  const result = findVerificationCandidates({ root: REPO_ROOT })
  assert.deepEqual(new Set(result.scope.statuses_searched), new Set(['implemented_unverified', 'implemented']))
  assert.ok(result.scope.total_capabilities_in_scope < 200, 'expected a small, bounded scope, not a full-repo sweep')
})

test('real repo: counts are internally consistent (candidates + no_candidate == total)', () => {
  const result = findVerificationCandidates({ root: REPO_ROOT })
  assert.equal(result.counts.candidates + result.counts.no_candidate, result.counts.total)
  assert.equal(result.counts.by_confidence.HIGH + result.counts.by_confidence.MEDIUM + result.counts.by_confidence.LOW, result.counts.candidates)
  assert.equal(result.results.length, result.counts.total)
})

// ── fixture-based tests: fully controlled match/no-match scenarios ─────────────────────────────
function writeJsonl(filePath, records) {
  writeFileSync(filePath, records.map((r) => JSON.stringify(r)).join('\n') + '\n')
}

function buildFixture({ capabilities, rustFiles }) {
  const root = mkdtempSync(join(tmpdir(), 'chronica-find-candidates-fixture-'))
  mkdirSync(join(root, 'docs', 'capabilities-canonical', 'domains'), { recursive: true })
  writeJsonl(join(root, 'docs', 'capabilities-canonical', 'domains', 'fixture.jsonl'), capabilities)
  for (const [relPath, content] of Object.entries(rustFiles)) {
    const full = join(root, relPath)
    mkdirSync(join(full, '..'), { recursive: true })
    writeFileSync(full, content)
  }
  return root
}

test('fixture: a strong two-token match in a real #[test] function is HIGH confidence', () => {
  const root = buildFixture({
    capabilities: [
      { capability_key: 'erp.widget_export_flow', canonical_name: 'Widget export flow', domain: 'erp', target_crate: 'chronica-fixture', target_module: 'widgets', status: 'implemented_unverified' },
    ],
    rustFiles: {
      'crates/chronica-fixture/src/widgets.rs': `
pub fn export_widget() {}

#[test]
fn widget_export_flow_succeeds_for_a_real_widget() {
    assert!(true);
}
`,
    },
  })
  const result = findVerificationCandidates({ root })
  assert.equal(result.results.length, 1)
  const row = result.results[0]
  assert.equal(row.classification, 'CANDIDATE')
  assert.equal(row.confidence, 'HIGH')
  assert.equal(row.candidate_test_symbol, 'widget_export_flow_succeeds_for_a_real_widget')
  assert.deepEqual(row.matched_tokens.primary.sort(), ['export', 'flow', 'widget'])
})

test('fixture: a non-#[test] function with matching name is NOT picked up as a candidate (regression: must not treat every fn as a test)', () => {
  const root = buildFixture({
    capabilities: [
      { capability_key: 'erp.widget_export_flow', canonical_name: 'Widget export flow', domain: 'erp', target_crate: 'chronica-fixture', target_module: 'widgets', status: 'implemented_unverified' },
    ],
    rustFiles: {
      // widget_export_flow appears only as a plain fn name, never behind a #[test]-family attribute.
      'crates/chronica-fixture/src/widgets.rs': `
pub fn widget_export_flow() {
    // real production code, not a test
}
`,
    },
  })
  const result = findVerificationCandidates({ root })
  assert.equal(result.results[0].classification, 'NO_CANDIDATE')
})

test('fixture: no token overlap at all is NO_CANDIDATE, not a forced low-confidence guess', () => {
  const root = buildFixture({
    capabilities: [
      { capability_key: 'erp.completely_unrelated_capability', canonical_name: 'Something totally different', domain: 'erp', target_crate: 'chronica-fixture', target_module: null, status: 'implemented_unverified' },
    ],
    rustFiles: {
      'crates/chronica-fixture/src/lib.rs': `
#[test]
fn some_other_thing_entirely() {
    assert!(true);
}
`,
    },
  })
  const result = findVerificationCandidates({ root })
  assert.equal(result.results[0].classification, 'NO_CANDIDATE')
})

test('fixture: a crate that does not exist on disk is reported as NO_CANDIDATE with a clear reason, never a crash', () => {
  const root = buildFixture({
    capabilities: [
      { capability_key: 'erp.ghost_capability', canonical_name: 'Ghost capability', domain: 'erp', target_crate: 'chronica-does-not-exist', target_module: null, status: 'implemented_unverified' },
    ],
    rustFiles: {},
  })
  const result = findVerificationCandidates({ root })
  assert.equal(result.results[0].classification, 'NO_CANDIDATE')
  assert.match(result.results[0].reason, /does not exist on disk/)
})

test('fixture: a capability with no target_crate at all is reported, never silently dropped', () => {
  const root = buildFixture({
    capabilities: [
      { capability_key: 'erp.unassigned_capability', canonical_name: 'Unassigned capability', domain: 'erp', target_crate: null, target_module: null, status: 'implemented_unverified' },
    ],
    rustFiles: {},
  })
  const result = findVerificationCandidates({ root })
  assert.equal(result.results.length, 1)
  assert.equal(result.results[0].classification, 'NO_CANDIDATE')
  assert.match(result.results[0].reason, /no target_crate assigned/)
})

// Anti-gaming style test (same shape as query-benchmark-ratio.test.mjs / query-roadmap-ratio.test.mjs's
// regression tests): a single generic one-token overlap must never be reported as HIGH confidence,
// only a two-primary-token (or module-hint-backed) match earns that.
test('fixture (anti-gaming): a single weak generic token match is never reported HIGH confidence', () => {
  const root = buildFixture({
    capabilities: [
      { capability_key: 'erp.model_export', canonical_name: 'Model export', domain: 'erp', target_crate: 'chronica-fixture', target_module: null, status: 'implemented_unverified' },
    ],
    rustFiles: {
      'crates/chronica-fixture/src/lib.rs': `
#[test]
fn totally_unrelated_model_behavior() {
    assert!(true);
}
`,
    },
  })
  const result = findVerificationCandidates({ root })
  const row = result.results[0]
  assert.equal(row.classification, 'CANDIDATE')
  assert.notEqual(row.confidence, 'HIGH')
  assert.equal(row.matched_tokens.primary.length, 1)
})

test('fixture: status outside implemented_unverified/implemented (e.g. unimplemented, verified) is excluded from scope entirely', () => {
  const root = buildFixture({
    capabilities: [
      { capability_key: 'erp.already_verified', canonical_name: 'Already verified', domain: 'erp', target_crate: 'chronica-fixture', target_module: null, status: 'verified' },
      { capability_key: 'erp.not_built_yet', canonical_name: 'Not built yet', domain: 'erp', target_crate: 'chronica-fixture', target_module: null, status: 'unimplemented' },
    ],
    rustFiles: {},
  })
  const result = findVerificationCandidates({ root })
  assert.equal(result.results.length, 0)
  assert.equal(result.scope.total_capabilities_in_scope, 0)
})

test('this tool never writes any file (read-only guarantee, checked by mtimes of all fixture inputs)', () => {
  const root = buildFixture({
    capabilities: [
      { capability_key: 'erp.widget_export_flow', canonical_name: 'Widget export flow', domain: 'erp', target_crate: 'chronica-fixture', target_module: 'widgets', status: 'implemented_unverified' },
    ],
    rustFiles: {
      'crates/chronica-fixture/src/widgets.rs': '#[test]\nfn widget_export_flow_case() { assert!(true); }\n',
    },
  })
  const capPath = join(root, 'docs', 'capabilities-canonical', 'domains', 'fixture.jsonl')
  const rsPath = join(root, 'crates', 'chronica-fixture', 'src', 'widgets.rs')
  const readMtime = (p) => statSync(p).mtimeMs
  const before = { cap: readMtime(capPath), rs: readMtime(rsPath) }
  findVerificationCandidates({ root })
  const after = { cap: readMtime(capPath), rs: readMtime(rsPath) }
  assert.deepEqual(before, after)
})

test('CLI smoke: summary on the real repo, run as a real child process', () => {
  const { status, stdout, stderr } = runCli(['summary'])
  assert.equal(status, 0, `expected exit 0, got ${status}; stderr: ${stderr}`)
  assert.match(stdout, /find-verification-candidates: \d+ capability\(ies\) in scope/)
})

test('CLI smoke: full prints parseable JSON', () => {
  const { status, stdout, stderr } = runCli(['full'])
  assert.equal(status, 0, `expected exit 0, got ${status}; stderr: ${stderr}`)
  const parsed = JSON.parse(stdout)
  assert.ok(Array.isArray(parsed.results))
})

test('CLI smoke: --domain filters scope, run as a real child process', () => {
  const { status, stdout } = runCli(['full', '--domain', 'policy-approval-audit'])
  assert.equal(status, 0)
  const parsed = JSON.parse(stdout)
  assert.equal(parsed.scope.domain_filter, 'policy-approval-audit')
  for (const r of parsed.results) {
    // every result must trace back to a capability actually in that domain — spot check via key prefix
    // is not reliable (domain != key prefix always), so just assert the filter was applied at all.
    assert.ok(r.capability_key)
  }
})

test('CLI smoke: --root points at a fixture repo instead of the real one', () => {
  const root = buildFixture({
    capabilities: [
      { capability_key: 'erp.widget_export_flow', canonical_name: 'Widget export flow', domain: 'erp', target_crate: 'chronica-fixture', target_module: 'widgets', status: 'implemented_unverified' },
    ],
    rustFiles: {
      'crates/chronica-fixture/src/widgets.rs': '#[test]\nfn widget_export_flow_case() { assert!(true); }\n',
    },
  })
  const { status, stdout, stderr } = runCli(['full', '--root', root])
  assert.equal(status, 0, `expected exit 0, got ${status}; stderr: ${stderr}`)
  const parsed = JSON.parse(stdout)
  assert.equal(parsed.results.length, 1)
  assert.equal(parsed.results[0].capability_key, 'erp.widget_export_flow')
})

test('CLI smoke: missing docs/capabilities-canonical/domains exits non-zero with a clear message, never a stack trace', () => {
  const root = mkdtempSync(join(tmpdir(), 'chronica-find-candidates-empty-'))
  const { status, stdout, stderr } = runCli(['summary', '--root', root])
  assert.notEqual(status, 0)
  assert.equal(stdout, '')
  assert.match(stderr, /docs\/capabilities-canonical\/domains not found/)
})
