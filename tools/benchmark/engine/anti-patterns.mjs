// Anti-pattern evaluation from evidence objects, not keyword grep.

import { ANTI_PATTERNS } from './vocab.mjs'
import { asArray, isObject, text } from './util.mjs'

export function evaluateAntiPattern(tag, evidence) {
  const errors = []
  if (!ANTI_PATTERNS.includes(tag)) return { ok: false, finding: null, errors: [`unknown anti-pattern ${tag}`] }
  if (typeof evidence === 'string') {
    return { ok: true, finding: 'NOT_EVALUATED', errors: [] }
  }
  if (!isObject(evidence)) {
    return { ok: false, finding: null, errors: [`${tag}: evidence object required (keyword grep is not enough)`] }
  }
  if (evidence.keyword_only === true || evidence.detected_by === 'grep') {
    return { ok: false, finding: null, errors: [`${tag}: do not mark merely by keyword grep`] }
  }

  let finding = 'NOT_FOUND'
  switch (tag) {
    case 'AGENT_AS_KERNEL':
      if (evidence.business_state_progresses_only_because_agent_reasons === true && evidence.deterministic_kernel_path_exists !== true) {
        finding = 'FOUND'
      }
      break
    case 'HARNESS_WITHOUT_ENGINE':
      if (evidence.harness_entrypoint_exists === true && evidence.durable_command_run_boundary !== true) {
        finding = 'FOUND'
      }
      break
    case 'AI_IN_HOT_PATH':
      if (evidence.hot_path_requires_llm === true && evidence.business_commit_blocked_without_llm === true) {
        finding = 'FOUND'
      }
      break
    case 'CONTROL_PLANE_OWNS_BUSINESS_STATE':
      if (evidence.control_plane_store_is_canonical_business_state === true) {
        finding = 'FOUND'
      }
      break
    case 'PROMPT_AS_SOURCE_OF_TRUTH':
      if (evidence.canonical_record_source === 'prompt' && evidence.persisted_command !== true) {
        finding = 'FOUND'
      }
      break
    case 'LLM_SELF_AUTHORIZATION':
      if (evidence.agent_may_authorize_money === true || evidence.model_output_grants_approval === true) {
        finding = 'FOUND'
      }
      break
    case 'EXECUTION_WITHOUT_RECEIPT':
      if (evidence.external_effect === true && evidence.receipt_persisted !== true) {
        finding = 'FOUND'
      }
      break
    case 'WORK_WITHOUT_DURABILITY':
      if (evidence.durable_job_claimed === true && evidence.persistence_kind === 'IN_MEMORY') {
        finding = 'FOUND'
      }
      break
    case 'RECOVERY_REQUIRES_AGENT_REASONING':
      if (evidence.recovery_requires_llm === true || evidence.recovery_reconstructs_intent_from_chat === true) {
        finding = 'FOUND'
      }
      break
    default:
      break
  }

  if (finding === 'FOUND' && asArray(evidence.evidence_paths).length === 0 && !text(evidence.note)) {
    errors.push(`${tag}: FOUND requires evidence_paths or note`)
  }

  return { ok: errors.length === 0, finding, errors }
}

export function evaluateAntiPatternSet(entries) {
  const errors = []
  const findings = {}
  for (const tag of ANTI_PATTERNS) {
    const entry = asArray(entries).find((row) => text(row.tag) === tag)
    if (!entry) {
      findings[tag] = 'NOT_EVALUATED'
      continue
    }
    const result = evaluateAntiPattern(tag, entry.evidence || entry)
    findings[tag] = result.finding
    if (!result.ok) errors.push(...result.errors)
  }
  return { ok: errors.length === 0, errors, findings }
}
