import test from 'node:test'
import assert from 'node:assert/strict'
import { spawnSync } from 'node:child_process'
import { mkdtempSync, rmSync, writeFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { dirname, join } from 'node:path'
import { fileURLToPath } from 'node:url'

const here = dirname(fileURLToPath(import.meta.url))
const script = join(here, 'validate-four-way-report.mjs')

function writeReport(report) {
  const root = mkdtempSync(join(tmpdir(), 'chronica-four-way-report-'))
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
    truth_label: 'IMPLEMENTED_UNDER_TRACKED',
    meaning: {
      purpose: 'Redact sensitive business event debug payloads before they can appear in operator logs.',
      doc_claim: 'The events crate stores append-only business event properties without claiming safe debug output.',
      code_reality: 'crates/chronica-events/src/lib.rs::BusinessEvent now uses a custom Debug formatter and tests cover redaction.',
      judgment: 'Code needed a narrower debug boundary; docs were accurate about storage but silent on operator log redaction.',
      action_taken: 'Implemented custom Debug redaction and added positive and negative tests for payload visibility.',
    },
    four_way: {
      code_tests: {
        status: 'CONFIRMED',
        evidence: ['cargo test -p chronica-events business_event_debug_redacts_payload'],
      },
      docs_meaning: {
        status: 'CONFIRMED',
        evidence: ['docs/crates/312-crate-chronica-events.md rationale updated with runtime non-claim boundary'],
      },
      capability_tracking: {
        status: 'LOCAL_AUDIT_REQUIRED',
        capability_keys: ['events.business_event_redaction'],
        reason: 'Cloud did not have full docs/capabilities.db; local Brain must record impl_evidence after audit.',
      },
      architecture_tracking: {
        status: 'CLOUD_CONFIRMED',
        evidence: ['node tools/architecture/query.mjs coverage reported 100.00 accounted coverage'],
      },
    },
    blockers: [],
    non_claims: ['No production readiness or money authorization claimed.'],
    ...overrides,
  }
}

function completeEcosystemConnections() {
  return {
    identity_scope: {
      status: 'CONNECTED',
      evidence: ['tenant/site principal is identified in the site deployment contract'],
    },
    policy_approval: {
      status: 'CONNECTED',
      evidence: ['deploy and side-effect actions flow through policy/approval gates'],
    },
    vault_secrets: {
      status: 'CONNECTED',
      evidence: ['provider credentials are referenced by secret handles only'],
    },
    template_artifact: {
      status: 'CONNECTED',
      evidence: ['design/template artifact provenance is listed in the report'],
    },
    commerce: {
      status: 'CONNECTED',
      evidence: ['storefront/catalog/checkout relationship is declared'],
    },
    erp: {
      status: 'CONNECTED',
      evidence: ['inventory/order/invoice join keys are declared'],
    },
    crm: {
      status: 'CONNECTED',
      evidence: ['lead/account/contact capture relationship is declared'],
    },
    helpdesk: {
      status: 'CONNECTED',
      evidence: ['support form/chat/ticket handoff relationship is declared'],
    },
    legal_privacy: {
      status: 'CONNECTED',
      evidence: ['consent/privacy/rights checks are declared'],
    },
    events_projection: {
      status: 'CONNECTED',
      evidence: ['events/projection/audit stream relationship is declared'],
    },
    analytics_observability: {
      status: 'CONNECTED',
      evidence: ['funnel/traffic/runtime/build log metrics are declared'],
    },
    workflow_automation: {
      status: 'CONNECTED',
      evidence: ['scheduler/workflow follow-up relationship is declared'],
    },
    strategy_memory: {
      status: 'CONNECTED',
      evidence: ['strategy/memory/simulation feedback relationship is declared'],
    },
    deployment_runtime: {
      status: 'CONNECTED',
      evidence: ['domain/DNS/TLS/preview/prod/rollback runtime relationship is declared'],
    },
  }
}

test('accepts a meaning-driven four-way report with local capability audit required', () => {
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

test('rejects web hosting reports that omit ecosystem connection coverage', () => {
  const { root, path } = writeReport(
    validReport({
      domain: 'web hosting',
      scope: 'crates/chronica-web-runtime/src/hosting.rs',
    }),
  )
  try {
    const result = spawnSync(process.execPath, [script, path], { encoding: 'utf8' })
    assert.equal(result.status, 1)
    const out = JSON.parse(result.stdout)
    assert.ok(out.errors.includes('ecosystem_connections is required for web/commerce/design/hosting reports'))
  } finally {
    rmSync(root, { recursive: true, force: true })
  }
})

test('accepts web hosting reports with complete no-white-zone ecosystem coverage', () => {
  const { root, path } = writeReport(
    validReport({
      domain: 'web hosting',
      scope: 'crates/chronica-web-runtime/src/hosting.rs',
      ecosystem_connections: completeEcosystemConnections(),
    }),
  )
  try {
    const result = spawnSync(process.execPath, [script, path], { encoding: 'utf8' })
    assert.equal(result.status, 0, result.stderr || result.stdout)
    const out = JSON.parse(result.stdout)
    assert.equal(out.ok, true)
  } finally {
    rmSync(root, { recursive: true, force: true })
  }
})

test('rejects cloud reports that omit meaning-driven fields', () => {
  const report = validReport()
  delete report.meaning.doc_claim
  report.meaning.judgment = 'n/a'
  const { root, path } = writeReport(report)
  try {
    const result = spawnSync(process.execPath, [script, path], { encoding: 'utf8' })
    assert.equal(result.status, 1)
    const out = JSON.parse(result.stdout)
    assert.equal(out.ok, false)
    assert.ok(out.errors.includes('meaning.doc_claim is required'))
    assert.ok(out.errors.includes('meaning.judgment must not be filler'))
  } finally {
    rmSync(root, { recursive: true, force: true })
  }
})

test('rejects reports missing any four-way dimension', () => {
  const report = validReport()
  delete report.four_way.architecture_tracking
  const { root, path } = writeReport(report)
  try {
    const result = spawnSync(process.execPath, [script, path], { encoding: 'utf8' })
    assert.equal(result.status, 1)
    const out = JSON.parse(result.stdout)
    assert.ok(out.errors.includes('four_way.architecture_tracking is required'))
  } finally {
    rmSync(root, { recursive: true, force: true })
  }
})

test('rejects confirmed dimensions without evidence and production-ready claims', () => {
  const report = validReport({
    truth_label: 'PRODUCTION_READY',
  })
  report.four_way.code_tests.evidence = []
  const { root, path } = writeReport(report)
  try {
    const result = spawnSync(process.execPath, [script, path], { encoding: 'utf8' })
    assert.equal(result.status, 1)
    const out = JSON.parse(result.stdout)
    assert.ok(out.errors.includes('truth_label must not claim PRODUCTION_READY from a cloud/harvest report'))
    assert.ok(out.errors.includes('four_way.code_tests.evidence is required when status is CONFIRMED'))
  } finally {
    rmSync(root, { recursive: true, force: true })
  }
})
