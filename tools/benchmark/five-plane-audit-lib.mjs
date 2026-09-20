#!/usr/bin/env node
// five-plane-audit-lib.mjs — fail-closed architecture audit for plane separation.
// Extends the existing FIC/benchmark controller. Does not invent a 56th FIC domain,
// a language bake-off, or a second job framework.
import { readFileSync, existsSync } from 'node:fs'
import { join } from 'node:path'

export const SCHEMA = 'chronica.five_plane_architecture_audit.v1'

export const REQUIRED_NO_AI_DOMAINS = Object.freeze([
  'company-organization',
  'identity-authorization',
  'helpdesk',
  'crm',
  'commerce',
  'erp-finance',
  'workflow',
  'scheduling',
  'approvals',
  'feed-activity',
  'backup-restore',
  'external-provider-execution',
  'browser-external-action',
  'audit-observability',
])

export const NO_AI_VERDICTS = Object.freeze([
  'STILL_WORKS_WITHOUT_AI',
  'OPTIONAL_INTELLIGENCE',
  'REQUIRED_INFERENCE',
  'REQUIRES_AI_TO_PROGRESS_BUSINESS_STATE',
])

export const PLANE_STATES = Object.freeze(['MISSING', 'WEAK', 'PARTIAL', 'STRONG'])

export const PLANE_KEYS = Object.freeze([
  'intelligence',
  'control',
  'kernel',
  'effect_data',
  'evidence_ops',
])

export const ANTI_PATTERN_TAGS = Object.freeze([
  'AGENT_AS_KERNEL',
  'HARNESS_WITHOUT_ENGINE',
  'AI_IN_HOT_PATH',
  'CONTROL_PLANE_OWNS_BUSINESS_STATE',
  'PROMPT_AS_SOURCE_OF_TRUTH',
  'LLM_SELF_AUTHORIZATION',
  'EXECUTION_WITHOUT_RECEIPT',
  'STATE_TRANSITION_WITHOUT_STATE_MACHINE',
  'EFFECT_WITHOUT_IDEMPOTENCY',
  'WORK_WITHOUT_DURABILITY',
  'RECOVERY_REQUIRES_AGENT_REASONING',
  'LANGUAGE_REWRITE_THEATER',
])

export const TAG_FINDINGS = Object.freeze(['FOUND', 'NOT_FOUND', 'PARTIAL', 'REJECTED'])

export const ARCHITECTURE_VERDICTS = Object.freeze({
  A: 'PRIMARILY AGENT HARNESS',
  B: 'HARNESS-HEAVY HYBRID',
  C: 'BALANCED COMPANY OS',
  D: 'DETERMINISTIC COMPANY OS WITH AI LAYER',
})

export const SPINE_VERDICTS = Object.freeze(['REQUIRED', 'EXISTING_SPINE_SUFFICIENT'])

export const FROZEN_CONTRACT_IDS = Object.freeze([
  'AI-DURABLE-001',
  'BACKUP-RESTORE-004',
  'CORE-EXECUTION-SPINE-001',
  'FEED-PUBLISH-002',
  'HD-CRM-LINK-005',
  'PROVIDER-RECEIPT-003',
  'TELEMETRY-SLO-006',
])

export const P_ROLES = Object.freeze([
  'P1 INTELLIGENCE',
  'P2 CONTROL / AUTHORITY',
  'P3 DETERMINISTIC BUSINESS KERNEL',
  'P4 EFFECT / DATA EXECUTION',
  'P5 EVIDENCE / OPERATIONS',
])

export const SYSTEM_SHAPES = Object.freeze([
  'durable command execution',
  'state transition',
  'approval wait/resume',
  'external effect with replay',
  'crash recovery',
  'scheduled execution',
  'tenant isolation',
  'receipt/audit lineage',
])

const DETERMINISTIC_NO_AI_DOMAINS = Object.freeze([
  'company-organization',
  'identity-authorization',
  'helpdesk',
  'approvals',
])

const LANGUAGE_BAKEOFF = /\b(go vs rust|rust vs go|java vs python|rewrite in go|rewrite in rust|rewrite in java|rewrite in python)\b/i
const NUMERIC_SCORE_KEY = /(^|_)(score|pct|percent|rating|maturity_index)$/i
const LAW = 'AI proposes. Control authorizes. Deterministic execution commits. State remembers. Evidence proves.'
const ACTOR_LAW = 'AI is an actor in the Company OS. AI is not the Company OS kernel.'

function text(value) {
  return typeof value === 'string' ? value.trim() : ''
}

function isObject(value) {
  return Boolean(value) && typeof value === 'object' && !Array.isArray(value)
}

function walkForbiddenNumbers(node, path, errors, allow) {
  if (Array.isArray(node)) {
    node.forEach((item, i) => walkForbiddenNumbers(item, `${path}[${i}]`, errors, allow))
    return
  }
  if (!isObject(node)) return
  for (const [key, value] of Object.entries(node)) {
    const next = `${path}.${key}`
    if (allow.has(next)) continue
    if (NUMERIC_SCORE_KEY.test(key) && typeof value === 'number') {
      errors.push(`numeric_precision_forbidden:${next}`)
    }
    walkForbiddenNumbers(value, next, errors, allow)
  }
}

function requireEvidence(errors, row, label) {
  const files = Array.isArray(row.evidence_files) ? row.evidence_files.map(text).filter(Boolean) : []
  const symbols = Array.isArray(row.evidence_symbols) ? row.evidence_symbols.map(text).filter(Boolean) : []
  const notes = text(row.evidence)
  if (files.length === 0 && symbols.length === 0 && !notes) {
    errors.push(`${label}: missing evidence`)
  }
}

export function validateFivePlaneAudit(doc, { contracts = [] } = {}) {
  const errors = []
  if (!isObject(doc)) return { ok: false, errors: ['audit must be an object'] }
  if (doc.schema !== SCHEMA) errors.push(`invalid schema ${doc.schema || '(missing)'}`)
  if (doc.not_a_denominator !== true) errors.push('not_a_denominator must be true')
  if (doc.denominator_change) errors.push('must not declare a denominator_change')
  if (text(doc.law) !== LAW) errors.push('law must be the exact five-plane audit law')
  if (text(doc.actor_law) !== ACTOR_LAW) errors.push('actor_law must state AI is an actor, not the kernel')

  const clone = { ...doc, anti_claims: undefined }
  if (Array.isArray(clone.anti_patterns)) {
    clone.anti_patterns = clone.anti_patterns.map((row) => {
      if (row?.tag === 'LANGUAGE_REWRITE_THEATER' && row?.finding === 'REJECTED') {
        return { ...row, evidence: undefined }
      }
      return row
    })
  }
  const blob = JSON.stringify(clone)
  if (LANGUAGE_BAKEOFF.test(blob)) errors.push('language_bakeoff_forbidden')

  const verdict = text(doc.verdict)
  if (!Object.hasOwn(ARCHITECTURE_VERDICTS, verdict)) {
    errors.push(`verdict must be one of A|B|C|D, got ${verdict || '(missing)'}`)
  } else if (text(doc.verdict_label) !== ARCHITECTURE_VERDICTS[verdict]) {
    errors.push(`verdict_label must be ${ARCHITECTURE_VERDICTS[verdict]}`)
  }

  walkForbiddenNumbers(doc, 'audit', errors, new Set())

  const domains = Array.isArray(doc.no_ai_matrix) ? doc.no_ai_matrix : []
  const seenDomains = new Set()
  for (const row of domains) {
    if (!isObject(row)) {
      errors.push('no_ai_matrix row must be an object')
      continue
    }
    const key = text(row.domain)
    if (!key) errors.push('no_ai_matrix row missing domain')
    if (seenDomains.has(key)) errors.push(`duplicate_no_ai_domain:${key}`)
    seenDomains.add(key)
    if (!NO_AI_VERDICTS.includes(row.no_ai)) errors.push(`${key}: invalid no_ai ${row.no_ai}`)
    if (!PLANE_STATES.includes(row.intelligence)) errors.push(`${key}: invalid intelligence`)
    if (!PLANE_STATES.includes(row.control)) errors.push(`${key}: invalid control`)
    if (!PLANE_STATES.includes(row.kernel)) errors.push(`${key}: invalid kernel`)
    if (!PLANE_STATES.includes(row.effect_data)) errors.push(`${key}: invalid effect_data`)
    if (!PLANE_STATES.includes(row.evidence_ops)) errors.push(`${key}: invalid evidence_ops`)
    requireEvidence(errors, row, key)
    const ops = Array.isArray(row.operations) ? row.operations : []
    if (ops.length === 0) errors.push(`${key}: operations required`)
    for (const op of ops) {
      const name = text(op?.name) || '(unnamed operation)'
      if (!NO_AI_VERDICTS.includes(op?.no_ai)) errors.push(`${key}.${name}: invalid no_ai`)
      if (
        op?.no_ai === 'REQUIRES_AI_TO_PROGRESS_BUSINESS_STATE' &&
        op?.inherent_inference !== true
      ) {
        errors.push(`${key}.${name}: REQUIRES_AI_TO_PROGRESS_BUSINESS_STATE is architectural debt unless inherent_inference is true`)
      }
      requireEvidence(errors, op, `${key}.${name}`)
    }
    if (
      DETERMINISTIC_NO_AI_DOMAINS.includes(key) &&
      (row.no_ai === 'REQUIRES_AI_TO_PROGRESS_BUSINESS_STATE' ||
        ops.some((op) => op?.no_ai === 'REQUIRES_AI_TO_PROGRESS_BUSINESS_STATE' && op?.inherent_inference !== true))
    ) {
      errors.push(`${key}: deterministic company-os path must not require AI to progress business state`)
    }
  }
  for (const key of REQUIRED_NO_AI_DOMAINS) {
    if (!seenDomains.has(key)) errors.push(`missing_no_ai_domain:${key}`)
  }

  const tags = Array.isArray(doc.anti_patterns) ? doc.anti_patterns : []
  const seenTags = new Set()
  for (const row of tags) {
    const tag = text(row?.tag)
    if (!ANTI_PATTERN_TAGS.includes(tag)) errors.push(`unknown_anti_pattern:${tag || '(missing)'}`)
    if (seenTags.has(tag)) errors.push(`duplicate_anti_pattern:${tag}`)
    seenTags.add(tag)
    if (!TAG_FINDINGS.includes(row?.finding)) errors.push(`${tag}: invalid finding`)
    requireEvidence(errors, row || {}, tag || 'tag')
    if (tag === 'LANGUAGE_REWRITE_THEATER' && row?.finding !== 'REJECTED') {
      errors.push('LANGUAGE_REWRITE_THEATER must be REJECTED unless a named runtime boundary fails its requirements')
    }
    if (tag === 'AGENT_AS_KERNEL' && row?.finding === 'FOUND') {
      if (row.business_state_progresses_only_because_agent_reasons !== true) {
        errors.push('AGENT_AS_KERNEL FOUND requires business_state_progresses_only_because_agent_reasons')
      }
    }
  }
  for (const tag of ANTI_PATTERN_TAGS) {
    if (!seenTags.has(tag)) errors.push(`missing_anti_pattern:${tag}`)
  }

  const goods = Array.isArray(doc.kernel_good_paths) ? doc.kernel_good_paths : []
  if (goods.length < 2) errors.push('kernel_good_paths: need at least two exemplars')
  for (const row of goods) {
    if (text(row?.id).length === 0) errors.push('kernel_good_path missing id')
    requireEvidence(errors, row || {}, `kernel_good_path:${text(row?.id) || '?'}`)
  }

  const roles = Array.isArray(doc.subsystem_roles) ? doc.subsystem_roles : []
  if (roles.length < 6) errors.push('subsystem_roles: classify at least six subsystems')
  for (const row of roles) {
    if (!P_ROLES.includes(row?.dominant_role)) {
      errors.push(`${text(row?.subsystem) || 'subsystem'}: invalid dominant_role`)
    }
    requireEvidence(errors, row || {}, `role:${text(row?.subsystem) || '?'}`)
  }

  if (!['HARNESS_HEAVY', 'BALANCED', 'KERNEL_HEAVY'].includes(text(doc.imbalance))) {
    errors.push('imbalance must be HARNESS_HEAVY|BALANCED|KERNEL_HEAVY')
  }
  if (verdict === 'B' && doc.imbalance !== 'HARNESS_HEAVY') {
    errors.push('verdict B requires imbalance HARNESS_HEAVY')
  }
  if (verdict === 'B') {
    const stillWorks = domains.filter((d) => d.no_ai === 'STILL_WORKS_WITHOUT_AI').length
    if (stillWorks < 4) errors.push('verdict B requires several STILL_WORKS_WITHOUT_AI domains')
    const harness = tags.find((t) => t.tag === 'HARNESS_WITHOUT_ENGINE')
    if (!harness || !['FOUND', 'PARTIAL'].includes(harness.finding)) {
      errors.push('verdict B requires HARNESS_WITHOUT_ENGINE FOUND or PARTIAL')
    }
  }
  if (verdict === 'A') {
    const aiRequired = domains.filter((d) => d.no_ai === 'REQUIRES_AI_TO_PROGRESS_BUSINESS_STATE').length
    if (aiRequired < 8) errors.push('verdict A requires most domains REQUIRES_AI_TO_PROGRESS_BUSINESS_STATE')
  }
  if (verdict === 'D') {
    const strongKernel = domains.filter((d) => d.kernel === 'STRONG').length
    if (strongKernel < 8) errors.push('verdict D requires STRONG kernel across most audited domains')
  }

  const spine = doc.spine_decision
  if (!isObject(spine) || !SPINE_VERDICTS.includes(spine.verdict)) {
    errors.push('spine_decision.verdict must be REQUIRED or EXISTING_SPINE_SUFFICIENT')
  } else {
    requireEvidence(errors, spine, 'spine_decision')
    if (spine.verdict === 'EXISTING_SPINE_SUFFICIENT') {
      const files = Array.isArray(spine.evidence_files) ? spine.evidence_files : []
      if (files.length < 3) errors.push('EXISTING_SPINE_SUFFICIENT requires exact runtime evidence files')
      if (!/WorkRequest|WorkRun|workflow_executions/.test(JSON.stringify(spine))) {
        errors.push('EXISTING_SPINE_SUFFICIENT must name the runtime primitives that already form the spine')
      }
    }
    if (spine.verdict === 'REQUIRED') {
      if (text(spine.intent_id) !== 'CORE-EXECUTION-SPINE-001') {
        errors.push('REQUIRED spine must issue CORE-EXECUTION-SPINE-001')
      }
      const gens = Array.isArray(spine.generalizes) ? spine.generalizes.map(text) : []
      if (!gens.includes('AI-DURABLE-001')) {
        errors.push('CORE-EXECUTION-SPINE-001 must generalize AI-DURABLE-001, not invent a second job system')
      }
      if (spine.do_not_invent_duplicate_framework !== true) {
        errors.push('spine_decision.do_not_invent_duplicate_framework must be true')
      }
      if (contracts.length) {
        const ids = contracts.map((c) => text(c.intent_id))
        if (!ids.includes('CORE-EXECUTION-SPINE-001')) {
          errors.push('CORE-EXECUTION-SPINE-001 contract file missing')
        }
        if (!ids.includes('AI-DURABLE-001')) errors.push('AI-DURABLE-001 must remain; do not replace it with a duplicate')
        const spineContract = contracts.find((c) => text(c.intent_id) === 'CORE-EXECUTION-SPINE-001')
        if (spineContract) {
          const related = JSON.stringify(spineContract)
          if (!/AI-DURABLE-001/.test(related)) {
            errors.push('CORE-EXECUTION-SPINE-001 contract must cite AI-DURABLE-001')
          }
          if (!/human|scheduler|webhook/i.test(related)) {
            errors.push('CORE-EXECUTION-SPINE-001 must name peer actors (human/scheduler/webhook), not only agents')
          }
          if (/temporal cluster|vendor temporal|rewrite in go/i.test(related)) {
            errors.push('CORE-EXECUTION-SPINE-001 must not vendor Temporal or trigger LANGUAGE_REWRITE_THEATER')
          }
        }
      }
    }
  }

  const shapes = Array.isArray(doc.system_shapes) ? doc.system_shapes : []
  const shapeNames = new Set(shapes.map((s) => text(s?.shape)))
  for (const name of SYSTEM_SHAPES) {
    if (!shapeNames.has(name)) errors.push(`missing_system_shape:${name}`)
  }
  for (const row of shapes) {
    if (!PLANE_STATES.includes(row?.mapping_state) && !['PARTIAL', 'STRONG', 'WEAK', 'MISSING'].includes(row?.mapping_state)) {
      errors.push(`${text(row?.shape)}: invalid mapping_state`)
    }
    requireEvidence(errors, row || {}, `shape:${text(row?.shape) || '?'}`)
  }

  const inversions = Array.isArray(doc.architectural_inversions) ? doc.architectural_inversions : []
  if (inversions.length === 0) errors.push('architectural_inversions required')
  for (const row of inversions) {
    if (!text(row?.framed_as_agent_permission) || !text(row?.should_be_deterministic_execution)) {
      errors.push('inversion must reframe agent permission as deterministic execution plus actors')
    }
    requireEvidence(errors, row || {}, 'inversion')
  }

  if (!text(doc.verdict_rationale)) errors.push('verdict_rationale required')
  if (!Array.isArray(doc.anti_claims) || doc.anti_claims.length === 0) errors.push('anti_claims required')

  validateFreeze(errors, doc, contracts)

  return { ok: errors.length === 0, errors }
}

function validateFreeze(errors, doc, contracts) {
  const freeze = doc.freeze
  if (!isObject(freeze)) {
    errors.push('freeze required: this snapshot is PROVISIONAL_BASELINE, not an authoritative architecture verdict')
    return
  }
  if (text(freeze.five_plane_audit_status) !== 'PROVISIONAL_BASELINE') {
    errors.push('freeze.five_plane_audit_status must be PROVISIONAL_BASELINE')
  }
  if (freeze.authoritative_architecture_verdict !== false) {
    errors.push('freeze.authoritative_architecture_verdict must be false until POST_THREAD4_PRODUCT_TRUTH_MAIN')
  }
  if (text(freeze.benchmark_controller_status) !== 'FROZEN_PROVISIONAL_BASELINE') {
    errors.push('freeze.benchmark_controller_status must be FROZEN_PROVISIONAL_BASELINE')
  }
  if (text(freeze.rerun_trigger) !== 'POST_THREAD4_PRODUCT_TRUTH_MAIN') {
    errors.push('freeze.rerun_trigger must be POST_THREAD4_PRODUCT_TRUTH_MAIN')
  }
  if (text(freeze.core_spine) !== 'PROVISIONALLY_REQUIRED') {
    errors.push('freeze.core_spine must be PROVISIONALLY_REQUIRED (do not implement now)')
  }
  if (freeze.do_not_implement_core_spine !== true) {
    errors.push('freeze.do_not_implement_core_spine must be true')
  }
  if (freeze.do_not_start_builder_wave_from_this_result !== true) {
    errors.push('freeze.do_not_start_builder_wave_from_this_result must be true')
  }
  if (freeze.do_not_change_roadmap_priority !== true) {
    errors.push('freeze.do_not_change_roadmap_priority must be true')
  }
  if (freeze.do_not_change_fic_from_this_addendum !== true) {
    errors.push('freeze.do_not_change_fic_from_this_addendum must be true')
  }
  const waiting = Array.isArray(freeze.waiting_for) ? freeze.waiting_for.map(text) : []
  for (const item of [
    '#4762 P0 completion and merge',
    'Thread 4 product consolidation',
    'Thread 4 final green merge to main',
    'exact PRODUCT_TRUTH_MAIN_SHA',
  ]) {
    if (!waiting.includes(item)) errors.push(`freeze.waiting_for missing ${item}`)
  }
  if (text(freeze.until_then) !== 'NO_NEW_BENCHMARK_ARCHITECTURE_WORK') {
    errors.push('freeze.until_then must be NO_NEW_BENCHMARK_ARCHITECTURE_WORK')
  }
  if (freeze.no_new_benchmark_architecture_work !== true) {
    errors.push('freeze.no_new_benchmark_architecture_work must be true')
  }
  const after = Array.isArray(freeze.after_product_truth_main_sha)
    ? freeze.after_product_truth_main_sha.map(text)
    : []
  for (const item of [
    'rerun the five-plane audit from repository truth',
    'recompute FIC',
    'rerun NO_AI matrix',
    'reassess CORE-EXECUTION-SPINE-001',
    'then and only then issue next Builder architecture contracts',
  ]) {
    if (!after.includes(item)) errors.push(`freeze.after_product_truth_main_sha missing ${item}`)
  }

  const corrections = freeze.rerun_corrections
  if (!isObject(corrections)) {
    errors.push('freeze.rerun_corrections required')
    return
  }
  const split = corrections.no_ai_provider_split
  if (!isObject(split)) {
    errors.push('rerun must split AI inference providers from deterministic external-effect providers')
  } else {
    if (text(split.ai_inference_provider) !== 'REQUIRED_INFERENCE') {
      errors.push('AI_INFERENCE_PROVIDER must rerun as REQUIRED_INFERENCE')
    }
    if (text(split.deterministic_external_effect_provider) !== 'STILL_WORKS_WITHOUT_AI') {
      errors.push('DETERMINISTIC_EXTERNAL_EFFECT_PROVIDER must rerun as STILL_WORKS_WITHOUT_AI')
    }
    const examples = JSON.stringify(split)
    if (!/stripe|email|webhook|carrier|browser/i.test(examples)) {
      errors.push('deterministic external-effect split must name Stripe/email/carrier/webhook/browser-effect')
    }
  }
  const spineRel = corrections.spine_relationship
  if (!isObject(spineRel)) {
    errors.push('freeze.rerun_corrections.spine_relationship required')
  } else {
    if (text(spineRel.parent_or_superseding_contract) !== 'CORE-EXECUTION-SPINE-001') {
      errors.push('CORE-EXECUTION-SPINE-001 must be the parent/superseding contract')
    }
    if (text(spineRel.consumer_use_case) !== 'AI-DURABLE-001') {
      errors.push('AI-DURABLE-001 must be a consumer use-case of the core spine, not a peer table family')
    }
    if (spineRel.one_actor_neutral_substrate !== true) {
      errors.push('future architecture must have ONE actor-neutral execution substrate')
    }
  }
  const authority = corrections.authority
  if (!isObject(authority)) {
    errors.push('freeze.rerun_corrections.authority required')
  } else {
    if (text(authority.authority_model) !== 'EXISTING_MODEL_SUFFICIENT') {
      errors.push('AUTHORITY_MODEL must remain EXISTING_MODEL_SUFFICIENT')
    }
    if (text(authority.execution_grant_bridge) !== 'NOT_YET_PROVEN') {
      errors.push('EXECUTION_GRANT_BRIDGE must be NOT_YET_PROVEN')
    }
    if (!/BoardAuthority/.test(text(authority.do_not_invent))) {
      errors.push('authority correction must forbid inventing BoardAuthority')
    }
  }

  if (contracts.length) {
    const ids = contracts.map((c) => text(c.intent_id)).sort()
    const expected = [...FROZEN_CONTRACT_IDS].sort()
    if (ids.length !== expected.length || ids.some((id, i) => id !== expected[i])) {
      errors.push('no new architecture contracts: frozen set is the existing seven intent files')
    }
    const spineContract = contracts.find((c) => text(c.intent_id) === 'CORE-EXECUTION-SPINE-001')
    if (spineContract && text(spineContract.assignment) !== 'DEFER') {
      errors.push('CORE-EXECUTION-SPINE-001 assignment must be DEFER until POST_THREAD4 rerun')
    }
    if (spineContract && text(spineContract.relationship_to_ai_durable) !== 'PARENT_OR_SUPERSEDE') {
      errors.push('CORE-EXECUTION-SPINE-001 must be PARENT_OR_SUPERSEDE of AI-DURABLE-001')
    }
    const durable = contracts.find((c) => text(c.intent_id) === 'AI-DURABLE-001')
    if (durable && text(durable.role) !== 'CONSUMER_USE_CASE') {
      errors.push('AI-DURABLE-001 role must be CONSUMER_USE_CASE of the core spine')
    }
    if (durable && durable.do_not_create_independent_workrun_schema !== true) {
      errors.push('AI-DURABLE-001 must not create an independent WorkRun schema')
    }
    if (durable && text(durable.shared_substrate_implementation) !== 'DEFER_UNTIL_POST_THREAD4_RERUN') {
      errors.push('shared WorkRun substrate implementation is frozen until POST_THREAD4 rerun')
    }
  }
}

export function loadFivePlaneAudit(root) {
  const path = join(root, 'docs/_machine/five-plane-architecture-audit.json')
  if (!existsSync(path)) return null
  return JSON.parse(readFileSync(path, 'utf8'))
}
