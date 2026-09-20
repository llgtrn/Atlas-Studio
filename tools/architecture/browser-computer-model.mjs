function node(id, kind, name, fields = {}) {
  return {
    id,
    kind,
    name,
    crate: fields.crate ?? null,
    modulePath: fields.modulePath ?? null,
    filePath: fields.filePath ?? null,
    status: fields.status ?? 'generated',
    evidenceRef: fields.evidenceRef ?? fields.filePath ?? null,
  }
}

function edge(fromNode, toNode, kind, evidenceRef = null) {
  return { fromNode, toNode, kind, evidenceRef }
}

const doc = 'docs/design/governed-browser-computer.md'

export const BROWSER_COMPUTER_NODES = [
  node('runtime_entity:GovernedBrowserComputer', 'runtime_entity', 'GovernedBrowserComputer', {
    crate: 'chronica-internet-hand',
    evidenceRef: doc,
  }),
  node('runtime_entity:BrowserProfile', 'runtime_entity', 'BrowserProfile', {
    crate: 'chronica-internet-hand',
    evidenceRef: doc,
  }),
  node('runtime_entity:SiteProfile', 'runtime_entity', 'SiteProfile', {
    crate: 'chronica-internet-hand',
    evidenceRef: doc,
  }),
  node('runtime_entity:BrowserActionPlan', 'runtime_entity', 'BrowserActionPlan', {
    crate: 'chronica-internet-hand',
    evidenceRef: doc,
  }),
  node('gate:browser_armed_execution', 'gate', 'Browser Armed Execution Gate', {
    crate: 'chronica-runtime',
    evidenceRef: doc,
  }),
  node('evidence:browser_run_evidence', 'evidence', 'Browser Run Evidence', {
    crate: 'chronica-observability',
    evidenceRef: doc,
  }),
  node('runtime_entity:BrowserCommerceRun', 'runtime_entity', 'BrowserCommerceRun', {
    crate: 'chronica-internet-hand',
    modulePath: 'commerce',
    filePath: 'crates/chronica-internet-hand/src/commerce.rs',
    status: 'implemented',
    evidenceRef: 'crates/chronica-internet-hand/src/commerce.rs',
  }),
  node('approval:BrowserPaymentApproval', 'approval', 'BrowserPaymentApproval', {
    crate: 'chronica-internet-hand',
    modulePath: 'commerce',
    filePath: 'crates/chronica-internet-hand/src/commerce.rs',
    status: 'implemented',
    evidenceRef: 'crates/chronica-internet-hand/tests/browser_commerce.rs',
  }),
  node('finance:BrowserFinanceRecord', 'finance', 'BrowserFinanceRecord', {
    crate: 'chronica-internet-hand',
    modulePath: 'commerce',
    filePath: 'crates/chronica-internet-hand/src/commerce.rs',
    status: 'implemented',
    evidenceRef: 'crates/chronica-internet-hand/tests/browser_commerce.rs',
  }),
]

export const BROWSER_COMPUTER_EDGES = [
  edge('runtime_entity:Agent', 'runtime_entity:GovernedBrowserComputer', 'proposes', doc),
  edge('runtime_entity:GovernedBrowserComputer', 'scope:ScopePath', 'requires', doc),
  edge('runtime_entity:GovernedBrowserComputer', 'runtime_entity:BrowserProfile', 'uses', doc),
  edge('runtime_entity:GovernedBrowserComputer', 'runtime_entity:SiteProfile', 'uses', doc),
  edge('runtime_entity:GovernedBrowserComputer', 'runtime_entity:BrowserActionPlan', 'plans', doc),
  edge('runtime_entity:BrowserActionPlan', 'gate:money', 'routes_money_actions_to', doc),
  edge('runtime_entity:BrowserActionPlan', 'gate:browser_armed_execution', 'requires_before_live_run', doc),
  edge('gate:browser_armed_execution', 'runtime_entity:Tool', 'arms_runtime_tool', doc),
  edge('runtime_entity:Tool', 'evidence:browser_run_evidence', 'emits', doc),
  edge('evidence:browser_run_evidence', 'audit:merkle', 'writes', doc),
  edge('runtime_entity:GovernedBrowserComputer', 'runtime_entity:BrowserCommerceRun', 'specializes_to_payment', 'crates/chronica-internet-hand/src/commerce.rs'),
  edge('runtime_entity:BrowserCommerceRun', 'approval:BrowserPaymentApproval', 'requires_before_payment_click', 'crates/chronica-internet-hand/tests/browser_commerce.rs'),
  edge('runtime_entity:BrowserCommerceRun', 'gate:money', 'routes_payment_to', 'crates/chronica-internet-hand/src/commerce.rs'),
  edge('runtime_entity:BrowserCommerceRun', 'finance:BrowserFinanceRecord', 'writes_after_approved_click', 'crates/chronica-internet-hand/src/commerce.rs'),
  edge('finance:BrowserFinanceRecord', 'audit:merkle', 'anchors_evidence_hash', 'crates/chronica-internet-hand/tests/browser_commerce.rs'),
]

export const BROWSER_COMPUTER_INVARIANTS = [
  {
    key: 'browser_execution_armed_only',
    name: 'Browser execution is armed-only',
    description: 'Live browser execution must be scoped, policy-cleared, previewed, approved when irreversible or money-touching, and explicitly armed before adapter execution.',
    enforcingNode: 'gate:browser_armed_execution',
    verificationCommand: 'cargo test -p chronica-internet-hand --test browser_commerce unapproved_or_unarmed_checkout_cannot_click',
    status: 'implemented',
  },
  {
    key: 'browser_evidence_required',
    name: 'Browser evidence required',
    description: 'Live browser runs must emit trace/evidence/audit records; learning updates never bypass future policy or approvals.',
    enforcingNode: 'evidence:browser_run_evidence',
    verificationCommand: 'cargo test -p chronica-internet-hand --test browser_commerce approved_payment_click_records_cost_finance_evidence_and_blocks_replay',
    status: 'implemented',
  },
  {
    key: 'browser_payment_click_after_approval_only',
    name: 'Browser payment click after approval only',
    description: 'Browser payment completion must require board approval, explicit arming, approved cart/domain/payment-scope snapshot, evidence redaction, CostRecord, finance handoff, and audit chain evidence.',
    enforcingNode: 'runtime_entity:BrowserCommerceRun',
    verificationCommand: 'cargo test -p chronica-internet-hand --test browser_commerce',
    status: 'implemented',
  },
]

export const BROWSER_COMPUTER_DATA_FLOWS = [
  {
    key: 'governed_browser_computer',
    sourceNode: 'runtime_entity:Agent',
    targetNode: 'runtime_entity:GovernedBrowserComputer',
    dataKind: 'ScopedBrowserActionPlan',
    scopeRule: 'VerifiedPrincipal and ScopePath are required before profile, site, secret, or live browser access',
    gateRequired: 1,
    auditRequired: 1,
  },
  {
    key: 'browser_evidence_to_audit',
    sourceNode: 'runtime_entity:GovernedBrowserComputer',
    targetNode: 'audit:merkle',
    dataKind: 'RuntimeTraceScreenshotEvidenceAuditEvent',
    scopeRule: 'evidence is scoped and secrets are redacted before audit/learning',
    gateRequired: 0,
    auditRequired: 1,
  },
  {
    key: 'browser_commerce_to_finance',
    sourceNode: 'runtime_entity:BrowserCommerceRun',
    targetNode: 'finance:BrowserFinanceRecord',
    dataKind: 'ApprovedPaymentEvidenceCostRecord',
    scopeRule: 'payment finance handoff is written only after approved and armed browser payment execution at the run ScopePath',
    gateRequired: 1,
    auditRequired: 1,
  },
]
