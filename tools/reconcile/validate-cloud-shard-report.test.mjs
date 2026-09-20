import test from 'node:test'
import assert from 'node:assert/strict'
import { execFileSync, spawnSync } from 'node:child_process'
import { mkdtempSync, rmSync, writeFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { dirname, join } from 'node:path'
import { fileURLToPath } from 'node:url'
import { validateCloudShardReport } from './validate-cloud-shard-report.mjs'

const here = dirname(fileURLToPath(import.meta.url))
const script = join(here, 'validate-cloud-shard-report.mjs')

function writeReport(report) {
  const root = mkdtempSync(join(tmpdir(), 'chronica-cloud-shard-report-'))
  const path = join(root, 'report.json')
  writeFileSync(path, JSON.stringify(report, null, 2))
  return { root, path }
}

function validReport(overrides = {}) {
  return {
    task_id: 'task-demo',
    branch: 'codex/security-benchmark-refactor/demo-cloud-001',
    commit: 'abcdef1',
    domain: 'events',
    scope: 'crates/chronica-events/src/lib.rs',
    truth_label: 'CLOUD_EVIDENCE_READY',
    meaning: {
      purpose: 'Redact sensitive business event debug payloads before they can appear in operator logs.',
      doc_claim: 'The events crate stores append-only business event properties without claiming safe debug output.',
      code_reality: 'crates/chronica-events/src/lib.rs::BusinessEvent uses a custom Debug formatter and tests cover redaction.',
      judgment: 'Code and docs are aligned, but root DB final verification remains local aggregate audit work.',
      action_taken: 'Implemented and validated custom Debug redaction, then prepared crate-local shard evidence for local harvest.',
    },
    five_way: {
      code: {
        status: 'CONFIRMED',
        evidence: ['crates/chronica-events/src/lib.rs::BusinessEvent'],
      },
      tests_evidence: {
        status: 'CONFIRMED',
        evidence: ['cargo test -p chronica-events business_event_debug_redacts_payload'],
      },
      docs: {
        status: 'CONFIRMED',
        evidence: ['docs/crates/312-crate-chronica-events.md records the redaction boundary'],
      },
      crate_sub_cap_db: {
        status: 'CLOUD_CONFIRMED',
        shard: 'crates/chronica-events/.chronica/sub-cap-arch.jsonl',
        capability_keys: ['events.business_event_redaction'],
        evidence: ['crate-local sub-cap shard row prepared with status CLOUD_EVIDENCE_READY'],
      },
      crate_sub_arch_db: {
        status: 'CLOUD_CONFIRMED',
        shard: 'crates/chronica-events/.chronica/sub-cap-arch.jsonl',
        architecture_nodes: ['crate:chronica-events'],
        invariant_keys: ['tracking_dbs_not_runtime'],
        evidence: ['crate-local sub-arch shard row prepared for local aggregate audit'],
      },
    },
    local_aggregate: {
      caps_db: {
        status: 'LOCAL_AUDIT_ONLY',
        targets: ['docs/capabilities.db::events.business_event_redaction'],
        reason: 'Root capabilities DB is a local aggregate audit target, not a cloud source or dimension.',
      },
      arch_db: {
        status: 'LOCAL_AUDIT_ONLY',
        targets: ['docs/architecture.db::crate:chronica-events/tracking_dbs_not_runtime'],
        reason: 'Root architecture DB is a local aggregate audit target, not a cloud source or dimension.',
      },
    },
    conflicts: [],
    blockers: [],
    non_claims: ['No production readiness, money authorization, or root DB write is claimed.'],
    ...overrides,
  }
}

function validCloudFinalReport(overrides = {}) {
  return {
    task_id: 'task-canonical',
    branch: 'codex/canonical-shards-cloud-final-demo',
    commit: 'abcdef1',
    domain: 'capability-tracking',
    scope: 'docs/capabilities-canonical domains plus docs/architecture-canonical',
    truth_label: 'CLOUD_FINAL_VERIFIED',
    meaning: {
      purpose: 'Make canonical JSONL shards the reviewable authority for capability and architecture tracking.',
      doc_claim: 'Cloud-final reports validate text shards and rebuild generated caches without committing DB binaries.',
      code_reality: 'Canonical export, verify, and build-cache scripts exist for capability and architecture surfaces.',
      judgment: 'Canonical text authority is confirmed; SQLite files are generated caches.',
      action_taken: 'Validated all six cloud-final dimensions and left binary DB artifacts out of the PR.',
    },
    six_way: {
      code: {
        status: 'CONFIRMED',
        evidence: ['tools/capabilities/verify-canonical-shards.mjs'],
      },
      tests_evidence: {
        status: 'CONFIRMED',
        evidence: ['pnpm caps:canonical:verify', 'pnpm arch:canonical:verify'],
      },
      docs_specs_doctrine: {
        status: 'CONFIRMED',
        evidence: ['docs/doctrines/023-five-dimension-cloud-shard-contract.md'],
      },
      canonical_capability_shards: {
        status: 'CONFIRMED',
        shards: ['docs/capabilities-canonical/domains/meta.jsonl'],
        capability_keys: ['meta.analysis_dependency_contract'],
        evidence: ['pnpm caps:canonical:verify'],
      },
      canonical_architecture_shards: {
        status: 'CONFIRMED',
        shards: ['docs/architecture-canonical/nodes.jsonl', 'docs/architecture-canonical/links.jsonl'],
        architecture_nodes: ['crate:chronica-meta-plane'],
        evidence: ['pnpm arch:canonical:verify'],
      },
      generated_cache_reproducibility: {
        status: 'CONFIRMED',
        commands: ['pnpm caps:canonical:build-db', 'pnpm arch:canonical:build-db'],
        evidence: ['temporary DB caches rebuilt from canonical JSONL'],
        pr_artifacts: [],
      },
    },
    conflicts: [],
    blockers: [],
    non_claims: ['No binary DB artifact, production readiness, or runtime DB dependency is claimed.'],
    ...overrides,
  }
}

function completeEcosystemConnections() {
  return Object.fromEntries([
    'identity_scope',
    'policy_approval',
    'vault_secrets',
    'template_artifact',
    'commerce',
    'erp',
    'crm',
    'helpdesk',
    'legal_privacy',
    'events_projection',
    'analytics_observability',
    'workflow_automation',
    'strategy_memory',
    'deployment_runtime',
  ].map((key) => [key, { status: 'CONNECTED', evidence: [`${key} connection is declared`] }]))
}

test('accepts a meaning-driven five-dimension cloud shard report', () => {
  const { root, path } = writeReport(validReport())
  try {
    const result = spawnSync(process.execPath, [script, path], { encoding: 'utf8' })
    assert.equal(result.status, 0, result.stderr || result.stdout)
    const out = JSON.parse(result.stdout)
    assert.equal(out.ok, true)
    assert.deepEqual(out.errors, [])
  } finally {
    rmSync(root, { recursive: true, force: true })
  }
})

test('accepts a cloud-final six-way report without local aggregate DB authority', () => {
  const { root, path } = writeReport(validCloudFinalReport())
  try {
    const result = spawnSync(process.execPath, [script, path], { encoding: 'utf8' })
    assert.equal(result.status, 0, result.stderr || result.stdout)
    const out = JSON.parse(result.stdout)
    assert.equal(out.ok, true)
    assert.deepEqual(out.errors, [])
  } finally {
    rmSync(root, { recursive: true, force: true })
  }
})

test('rejects binary DB artifacts in cloud-final generated cache evidence', () => {
  const report = validCloudFinalReport()
  report.six_way.generated_cache_reproducibility.pr_artifacts = ['docs/architecture.db']
  const { root, path } = writeReport(report)
  try {
    const result = spawnSync(process.execPath, [script, path], { encoding: 'utf8' })
    assert.equal(result.status, 1)
    const out = JSON.parse(result.stdout)
    assert.ok(out.errors.includes('six_way.generated_cache_reproducibility.pr_artifacts must not list binary DB artifact for a cloud PR: docs/architecture.db'))
  } finally {
    rmSync(root, { recursive: true, force: true })
  }
})

test('rejects binary DB artifacts in top-level cloud-final file lists', () => {
  const report = validCloudFinalReport({ changed_files: ['docs/architecture.db'] })
  const { root, path } = writeReport(report)
  try {
    const result = spawnSync(process.execPath, [script, path], { encoding: 'utf8' })
    assert.equal(result.status, 1)
    const out = JSON.parse(result.stdout)
    assert.ok(out.errors.includes('report.changed_files must not list binary DB artifact for a cloud PR: docs/architecture.db'))
  } finally {
    rmSync(root, { recursive: true, force: true })
  }
})

test('requires canonical DB rebuild commands for confirmed generated-cache reproducibility', () => {
  const report = validCloudFinalReport()
  delete report.six_way.generated_cache_reproducibility.commands
  const { root, path } = writeReport(report)
  try {
    const result = spawnSync(process.execPath, [script, path], { encoding: 'utf8' })
    assert.equal(result.status, 1)
    const out = JSON.parse(result.stdout)
    assert.ok(out.errors.includes('six_way.generated_cache_reproducibility.commands is required for generated cache reproducibility'))
    assert.ok(out.errors.includes('six_way.generated_cache_reproducibility.commands must include pnpm caps:canonical:build-db'))
    assert.ok(out.errors.includes('six_way.generated_cache_reproducibility.commands must include pnpm arch:canonical:build-db'))
  } finally {
    rmSync(root, { recursive: true, force: true })
  }
})

test('rejects cloud reports that still use only the legacy four_way dimensions', () => {
  const report = validReport()
  report.four_way = {
    code_tests: { status: 'CONFIRMED', evidence: ['cargo test -p chronica-events'] },
    docs_meaning: { status: 'CONFIRMED', evidence: ['docs/crates/312-crate-chronica-events.md'] },
    capability_tracking: { status: 'LOCAL_AUDIT_REQUIRED', reason: 'root DB local only' },
    architecture_tracking: { status: 'CLOUD_CONFIRMED', evidence: ['arch query'] },
  }
  delete report.five_way
  const { root, path } = writeReport(report)
  try {
    const result = spawnSync(process.execPath, [script, path], { encoding: 'utf8' })
    assert.equal(result.status, 1)
    const out = JSON.parse(result.stdout)
    assert.ok(out.errors.includes('five_way is required'))
  } finally {
    rmSync(root, { recursive: true, force: true })
  }
})

test('rejects cloud reports that treat root DBs as cloud-confirmed dimensions', () => {
  const report = validReport()
  report.local_aggregate.caps_db.status = 'CLOUD_CONFIRMED'
  const { root, path } = writeReport(report)
  try {
    const result = spawnSync(process.execPath, [script, path], { encoding: 'utf8' })
    assert.equal(result.status, 1)
    const out = JSON.parse(result.stdout)
    assert.ok(out.errors.includes('local_aggregate.caps_db.status is invalid for a root aggregate DB: CLOUD_CONFIRMED'))
  } finally {
    rmSync(root, { recursive: true, force: true })
  }
})

test('rejects shard evidence outside the owning crate folder', () => {
  const report = validReport()
  report.five_way.crate_sub_cap_db.shard = 'docs/capabilities-cloud/cap-core.db'
  report.five_way.crate_sub_arch_db.shard = 'tmp/sub-cap-arch.jsonl'
  const { root, path } = writeReport(report)
  try {
    const result = spawnSync(process.execPath, [script, path], { encoding: 'utf8' })
    assert.equal(result.status, 1)
    const out = JSON.parse(result.stdout)
    assert.ok(out.errors.includes('five_way.crate_sub_cap_db.shard must be crates/<crate>/.chronica/sub-cap-arch.jsonl'))
    assert.ok(out.errors.includes('five_way.crate_sub_arch_db.shard must be crates/<crate>/.chronica/sub-cap-arch.jsonl'))
  } finally {
    rmSync(root, { recursive: true, force: true })
  }
})

test('requires fail-closed severity for money and security conflict classes', () => {
  const report = validReport({
    truth_label: 'BLOCKED_WITH_EVIDENCE',
    conflicts: [
      {
        class: 'MONEY_CLASSIFICATION_MISMATCH',
        severity: 'BLOCKED_WITH_EVIDENCE',
        evidence: ['cloud shard says money-free but root target says money-moving'],
      },
      {
        class: 'CROSS_CRATE_AUTHORITY_CONFLICT',
        severity: 'BLOCKED_WITH_EVIDENCE',
        evidence: ['two crates claim the same authority node'],
      },
    ],
  })
  const { root, path } = writeReport(report)
  try {
    const result = spawnSync(process.execPath, [script, path], { encoding: 'utf8' })
    assert.equal(result.status, 1)
    const out = JSON.parse(result.stdout)
    assert.ok(out.errors.includes('conflicts[0].severity must be MONEY_BLOCKED for MONEY_CLASSIFICATION_MISMATCH'))
    assert.ok(out.errors.includes('conflicts[1].severity must be SECURITY_BLOCKED for CROSS_CRATE_AUTHORITY_CONFLICT'))
  } finally {
    rmSync(root, { recursive: true, force: true })
  }
})

test('keeps no-white-zone ecosystem coverage mandatory for web hosting reports', () => {
  const missing = writeReport(validReport({
    domain: 'web hosting',
    scope: 'crates/chronica-web-runtime/src/hosting.rs',
  }))
  try {
    const result = spawnSync(process.execPath, [script, missing.path], { encoding: 'utf8' })
    assert.equal(result.status, 1)
    const out = JSON.parse(result.stdout)
    assert.ok(out.errors.includes('ecosystem_connections is required for web/commerce/design/hosting reports'))
  } finally {
    rmSync(missing.root, { recursive: true, force: true })
  }

  const present = writeReport(validReport({
    domain: 'web hosting',
    scope: 'crates/chronica-web-runtime/src/hosting.rs',
    ecosystem_connections: completeEcosystemConnections(),
  }))
  try {
    const result = spawnSync(process.execPath, [script, present.path], { encoding: 'utf8' })
    assert.equal(result.status, 0, result.stderr || result.stdout)
  } finally {
    rmSync(present.root, { recursive: true, force: true })
  }
})

test('accepts a full-length commit SHA that resolves to a real git object', () => {
  const head = execFileSync('git', ['rev-parse', 'HEAD'], { cwd: here, encoding: 'utf8' }).trim()
  const { root, path } = writeReport(validReport({ commit: head }))
  try {
    const result = spawnSync(process.execPath, [script, path], { encoding: 'utf8', cwd: here })
    assert.equal(result.status, 0, result.stderr || result.stdout)
    const out = JSON.parse(result.stdout)
    assert.equal(out.ok, true)
    assert.deepEqual(out.errors, [])
  } finally {
    rmSync(root, { recursive: true, force: true })
  }
})

test('rejects a current-branch report whose commit binding is stale', () => {
  const branch = execFileSync('git', ['rev-parse', '--abbrev-ref', 'HEAD'], { cwd: here, encoding: 'utf8' }).trim()
  const commits = execFileSync('git', ['rev-list', '--max-count=2', 'HEAD'], { cwd: here, encoding: 'utf8' })
    .trim()
    .split(/\r?\n/)
  assert.ok(commits.length >= 2, 'test requires a branch with at least two commits')
  const { root, path } = writeReport(validReport({ branch, commit: commits[1] }))
  try {
    const result = spawnSync(process.execPath, [script, path], { encoding: 'utf8', cwd: here })
    assert.equal(result.status, 1)
    const out = JSON.parse(result.stdout)
    assert.ok(out.errors.some((error) => error.includes('matches the current checkout')))
  } finally {
    rmSync(root, { recursive: true, force: true })
  }
})

test('accepts a current-branch report refreshed at current head', () => {
  const branch = execFileSync('git', ['rev-parse', '--abbrev-ref', 'HEAD'], { cwd: here, encoding: 'utf8' }).trim()
  const commits = execFileSync('git', ['rev-list', '--max-count=2', 'HEAD'], { cwd: here, encoding: 'utf8' })
    .trim()
    .split(/\r?\n/)
  assert.ok(commits.length >= 2, 'test requires a branch with at least two commits')
  const { root, path } = writeReport(validReport({
    branch,
    commit: commits[1],
    last_report_refresh_commit: commits[0],
  }))
  try {
    const result = spawnSync(process.execPath, [script, path], { encoding: 'utf8', cwd: here })
    assert.equal(result.status, 0, result.stderr || result.stdout)
    const out = JSON.parse(result.stdout)
    assert.equal(out.ok, true)
    assert.deepEqual(out.errors, [])
  } finally {
    rmSync(root, { recursive: true, force: true })
  }
})

test('accepts a self-updating current-branch report whose refresh commit is a head parent', () => {
  const commits = execFileSync('git', ['rev-list', '--max-count=2', 'HEAD'], { cwd: here, encoding: 'utf8' })
    .trim()
    .split(/\r?\n/)
  assert.ok(commits.length >= 2, 'test requires a branch with at least two commits')

  const result = validateCloudShardReport(validReport({
    branch: 'codex/test-report-self-refresh',
    commit: commits[1],
    last_report_refresh_commit: commits[1],
  }), {
    currentBranch: 'codex/test-report-self-refresh',
    currentHead: commits[0],
    currentParents: [commits[1]],
    reportLastCommit: commits[0],
  })

  assert.equal(result.ok, true, result.errors.join('\n'))
  assert.deepEqual(result.errors, [])
})

test('rejects a self-refresh claim when current head did not touch the report path', () => {
  const commits = execFileSync('git', ['rev-list', '--max-count=2', 'HEAD'], { cwd: here, encoding: 'utf8' })
    .trim()
    .split(/\r?\n/)
  assert.ok(commits.length >= 2, 'test requires a branch with at least two commits')

  const result = validateCloudShardReport(validReport({
    branch: 'codex/test-report-self-refresh',
    commit: commits[1],
    last_report_refresh_commit: commits[1],
  }), {
    currentBranch: 'codex/test-report-self-refresh',
    currentHead: commits[0],
    currentParents: [commits[1]],
    reportLastCommit: commits[1],
  })

  assert.equal(result.ok, false)
  assert.ok(result.errors.some((error) => error.includes('report path must be last touched by current HEAD')))
})

test('rejects a full-length commit SHA that does not resolve to a real git object', () => {
  const fakeSha = '0'.repeat(40)
  const { root, path } = writeReport(validReport({ commit: fakeSha }))
  try {
    const result = spawnSync(process.execPath, [script, path], { encoding: 'utf8', cwd: here })
    assert.equal(result.status, 1)
    const out = JSON.parse(result.stdout)
    assert.ok(out.errors.some((error) => error.startsWith(`commit is a full-length SHA (${fakeSha})`)))
  } finally {
    rmSync(root, { recursive: true, force: true })
  }
})

test('warns instead of failing when no git repository context is available', () => {
  const fakeSha = '0'.repeat(40)
  const { root, path } = writeReport(validReport({ commit: fakeSha }))
  const outsideRepo = mkdtempSync(join(tmpdir(), 'chronica-no-git-context-'))
  try {
    const result = spawnSync(process.execPath, [script, path], { encoding: 'utf8', cwd: outsideRepo })
    assert.equal(result.status, 0, result.stderr || result.stdout)
    const out = JSON.parse(result.stdout)
    assert.equal(out.ok, true)
    assert.ok(out.warnings.some((warning) => warning.includes('LOCAL_AUDIT_REQUIRED')))
  } finally {
    rmSync(root, { recursive: true, force: true })
    rmSync(outsideRepo, { recursive: true, force: true })
  }
})
