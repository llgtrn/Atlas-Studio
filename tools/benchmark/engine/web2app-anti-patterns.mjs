// WEB2APP anti-pattern evaluation from evidence objects, not keyword grep.

import { WEB2APP_ANTI_PATTERNS } from './web2app-vocab.mjs'
import { asArray, isObject, text } from './util.mjs'

function foundIf(condition) {
  return condition ? 'FOUND' : 'NOT_FOUND'
}

export function evaluateWeb2AppAntiPattern(tag, evidence) {
  const errors = []
  if (!WEB2APP_ANTI_PATTERNS.includes(tag)) {
    return { ok: false, finding: null, errors: [`unknown WEB2APP anti-pattern ${tag}`] }
  }
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
    case 'HARNESS_WITHOUT_ENGINE':
      finding = foundIf(
        evidence.harness_entrypoint_exists === true &&
          evidence.durable_command_run_boundary !== true &&
          evidence.contract_is_harness !== true,
      )
      break
    case 'WORK_WITHOUT_DURABILITY':
      finding = foundIf(
        evidence.requires_persistence === true &&
          (evidence.persistence_kind === 'IN_MEMORY' ||
            evidence.persistence_kind === 'REQUEST_LOCAL' ||
            evidence.persistence_present !== true),
      )
      break
    case 'EXECUTION_WITHOUT_RECEIPT':
      finding = foundIf(evidence.external_effect === true && evidence.receipt_persisted !== true)
      break
    case 'PREVIEW_AS_PRODUCT':
      finding = foundIf(evidence.only_preview_surface === true && evidence.requires_durable_lifecycle === true)
      break
    case 'CALLER_FARMING':
      finding = foundIf(
        evidence.caller_exists === true &&
          evidence.intended_product_outcome !== true &&
          evidence.caller_created_to_escape_island === true,
      )
      break
    case 'LABEL_AS_ENFORCEMENT':
      finding = foundIf(evidence.token_stored === true && evidence.behavior_enforced !== true)
      break
    case 'UI_WITHOUT_RUNTIME':
      finding = foundIf(evidence.ui_exists === true && evidence.runtime_chain !== true)
      break
    case 'ROUTE_WITHOUT_PRODUCT':
      finding = foundIf(evidence.route_exists === true && evidence.required_state_or_effect !== true)
      break
    case 'TRACKING_AS_PRODUCT':
      finding = foundIf(evidence.only_canonical_or_arch_row === true && evidence.implementation_claimed === true)
      break
    case 'DONOR_NAME_WITHOUT_DONOR_SEMANTICS':
      finding = foundIf(evidence.donor_cited === true && evidence.donor_product_behavior_ported !== true)
      break
    case 'GOALPOST_MUTATION':
      finding = foundIf(evidence.semantic_meaning_changed === true && evidence.independent_review !== true)
      break
    case 'DENOMINATOR_SHRINKING':
      finding = foundIf(evidence.denominator_changed_between_passes === true && evidence.independent_justification !== true)
      break
    case 'OWNERSHIP_LAUNDERING':
      finding = foundIf(evidence.ownership_by_crate_or_word_or_prefix === true && evidence.semantic_contract_owns !== true)
      break
    case 'TEST_AS_RUNTIME':
      finding = foundIf(evidence.tests_are_only_caller === true && evidence.claimed_shipped_runtime === true)
      break
    case 'MOCK_AS_PRODUCTION':
      finding = foundIf(evidence.mock_or_fake_success === true && evidence.claimed_provider_effect === true)
      break
    case 'AI_IN_HOT_PATH':
      finding = foundIf(evidence.hot_path_requires_llm === true && evidence.deterministic_capability === true)
      break
    case 'AGENT_AS_KERNEL':
      finding = foundIf(
        evidence.business_state_progresses_only_because_agent_reasons === true &&
          evidence.deterministic_kernel_path_exists !== true,
      )
      break
    case 'PROMPT_AS_SOURCE_OF_TRUTH':
      finding = foundIf(evidence.canonical_record_source === 'prompt' && evidence.persisted_command !== true)
      break
    case 'LLM_SELF_AUTHORIZATION':
      finding = foundIf(evidence.agent_may_authorize_money === true || evidence.model_output_grants_approval === true)
      break
    case 'DATA_WITHOUT_OPERATIONS':
      finding = foundIf(
        evidence.normalized_records === true &&
          evidence.typed_operations_present !== true &&
          evidence.requires_typed_operations === true,
      )
      break
    case 'ADAPTER_AS_APP':
      finding = foundIf(evidence.highest_live_plane === 'P1' && evidence.claimed_application_complete === true)
      break
    case 'DOM_AS_KERNEL':
      finding = foundIf(evidence.agent_must_understand_dom === true)
      break
    case 'UNTYPED_DO_WHATEVER':
      finding = foundIf(evidence.unbounded_agent_action_api === true)
      break
    case 'JURISDICTION_COLLAPSE':
      finding = foundIf(evidence.multi_jurisdiction === true && evidence.flattened_into_one_law === true)
      break
    case 'TRANSPORT_LEAKAGE':
      finding = foundIf(evidence.caller_must_select_transport === true && evidence.contract_transport_independent === true)
      break
    case 'ASSISTANT_WITHOUT_STATE_CHANGE':
      finding = foundIf(
        evidence.recommendation_only === true &&
          evidence.authorized_s0_to_s1 !== true &&
          evidence.claimed_execution === true,
      )
      break
    case 'PULL_ONLY_AS_REALTIME':
      finding = foundIf(
        evidence.claimed_realtime === true &&
          evidence.push_supported !== true &&
          (evidence.poll_supported === true || evidence.on_demand_refresh === true),
      )
      break
    case 'WEBHOOK_WITHOUT_AUTH':
      finding = foundIf(evidence.webhook_used === true && evidence.signature_verified !== true)
      break
    case 'WEBHOOK_WITHOUT_DEDUPE':
      finding = foundIf(evidence.webhook_used === true && evidence.durable_dedupe !== true)
      break
    case 'POLL_WITHOUT_CHECKPOINT':
      finding = foundIf(evidence.persistent_poll === true && evidence.checkpoint_durable !== true)
      break
    case 'EVENT_WITHOUT_NORMALIZATION':
      finding = foundIf(evidence.provider_event_ingested === true && evidence.normalized_canonical !== true)
      break
    case 'EVENT_AS_COMMAND':
      finding = foundIf(evidence.external_event_becomes_command === true && evidence.authorized_execution !== true)
      break
    case 'PUSH_WITHOUT_RECONCILIATION':
      finding = foundIf(
        evidence.push_received === true &&
          evidence.reconciliation !== true &&
          evidence.claimed_as_business_truth === true,
      )
      break
    case 'STALE_STATE_AS_CURRENT':
      finding = foundIf(evidence.stale === true && evidence.staleness_visible !== true && evidence.presented_as_current === true)
      break
    case 'CREDENTIAL_IN_APP_DEFINITION':
      finding = foundIf(evidence.credential_plaintext_in_app_definition === true)
      break
    case 'CREDENTIAL_IN_AGENT_CONTEXT':
      finding = foundIf(evidence.credential_plaintext_in_agent_context === true)
      break
    case 'ENTITLEMENT_WITHOUT_TENANT_SCOPE':
      finding = foundIf(
        evidence.entitlement_without_tenant_scope === true || evidence.paid_entitlement_reused_cross_tenant === true,
      )
      break
    case 'FULL_RECOMPILE_ON_EVERY_POLL':
      finding = foundIf(evidence.full_recompile_on_every_poll === true)
      break
    case 'RAW_PROVIDER_PAYLOAD_AS_WORLD_MODEL':
      finding = foundIf(evidence.raw_provider_payload_as_world_model === true)
      break
    default:
      break
  }

  if (finding === 'FOUND' && asArray(evidence.evidence_paths).length === 0 && !text(evidence.note)) {
    errors.push(`${tag}: FOUND requires evidence_paths or note`)
  }

  return { ok: errors.length === 0, finding, errors }
}

export function evaluateWeb2AppAntiPatternSet(entries) {
  const errors = []
  const findings = {}
  const found = []
  for (const tag of WEB2APP_ANTI_PATTERNS) {
    const entry = asArray(entries).find((row) => text(row.tag) === tag)
    if (!entry) {
      findings[tag] = 'NOT_EVALUATED'
      continue
    }
    const result = evaluateWeb2AppAntiPattern(tag, entry.evidence || entry)
    findings[tag] = result.finding
    if (!result.ok) errors.push(...result.errors)
    if (result.finding === 'FOUND') found.push(tag)
  }
  return { ok: errors.length === 0, errors, findings, found }
}
