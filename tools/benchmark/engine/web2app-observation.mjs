// Observation-plane evaluator for Standard 205 addendum 1.1.0.
// Does not replace W0–W5. Does not invent CAP13 inbound-ingestion V2.

import { asArray, hasText, isObject, text } from './util.mjs'
import {
  FRESHNESS_CLASSES,
  OBSERVATION_LEVELS,
  SOURCE_HEALTH_STATES,
  minObservation,
  observationRank,
} from './web2app-vocab.mjs'

export function observationView(record) {
  const nested = isObject(record?.observation) ? record.observation : {}
  return {
    get(key) {
      if (nested[key] !== undefined) return nested[key]
      return record?.[key]
    },
    nested,
  }
}

export function validateObservationFields(record) {
  const errors = []
  if (!isObject(record)) return { ok: false, errors: ['record must be an object'] }
  const view = observationView(record)
  const claimed = text(view.get('claimed_observation_maturity') || view.get('observation_maturity'))
  if (claimed && !OBSERVATION_LEVELS.includes(claimed)) {
    errors.push(`observation_maturity is invalid: ${claimed}`)
  }
  const freshness = text(view.get('freshness_class') || view.get('freshness_target'))
  if (freshness) {
    const forbiddenRealtime = /^(real[\s_-]*time)$/i.test(freshness)
    if (forbiddenRealtime) {
      errors.push('freshness_class REALTIME is forbidden unless measured; use PUSH_REALTIME / NEAR_REALTIME / POLL_INTERVAL / ON_DEMAND / MANUAL_REFRESH')
    } else if (!FRESHNESS_CLASSES.includes(freshness)) {
      errors.push(`freshness_class is invalid: ${freshness}`)
    }
  }
  const health = text(view.get('source_health'))
  if (health && !SOURCE_HEALTH_STATES.includes(health)) {
    errors.push(`source_health is invalid: ${health}`)
  }
  return { ok: errors.length === 0, errors }
}

function truthy(value) {
  return value === true
}

function applyCap(level, max, reason, caps) {
  if (observationRank(level) > observationRank(max)) {
    caps.push(reason)
    return max
  }
  return level
}

export function assignObservationMaturity(record) {
  const view = observationView(record)
  const caps = []
  const claimed = text(view.get('claimed_observation_maturity') || view.get('observation_maturity'))
  const sources = asArray(view.get('observation_sources'))
  const onDemand = truthy(view.get('on_demand_refresh'))
  const poll = truthy(view.get('poll_supported'))
  const push = truthy(view.get('push_supported'))
  const checkpointDurable = truthy(view.get('checkpoint_durable'))
  const dedupeDurable = truthy(view.get('dedupe_durable'))
  const processLocalDedupe = truthy(view.get('process_local_dedupe_only'))
  const reconciliation = truthy(view.get('reconciliation'))
  const webhookAuth = truthy(view.get('webhook_authenticated')) || truthy(view.get('webhook_signature_verified'))
  const restartProof = truthy(view.get('restart_replay_proven'))
  const realProvider = truthy(view.get('real_provider_evidence'))
  const freshness = text(view.get('freshness_class') || view.get('freshness_target'))
  const persistentPoll = truthy(view.get('persistent_poll')) || poll

  const hasSignal =
    sources.length > 0 ||
    onDemand ||
    poll ||
    push ||
    freshness === 'ON_DEMAND' ||
    freshness === 'MANUAL_REFRESH'

  let level = 'O0'
  if (hasSignal || onDemand || freshness === 'ON_DEMAND' || freshness === 'MANUAL_REFRESH') {
    level = 'O1'
  }
  if (poll && checkpointDurable) level = 'O2'
  else if (poll && !checkpointDurable) level = minObservation(level === 'O0' ? 'O1' : level, 'O1')
  if (push && webhookAuth) {
    if (observationRank(level) < 3) level = 'O3'
  }
  if ((push || poll) && checkpointDurable && dedupeDurable && reconciliation && restartProof) {
    level = 'O4'
  }
  if (observationRank(level) >= 4 && realProvider) level = 'O5'

  if (persistentPoll && !checkpointDurable) {
    level = applyCap(level, 'O1', 'no durable checkpoint for persistent poll → max O1', caps)
  }
  if (processLocalDedupe) {
    level = applyCap(level, 'O2', 'process-local dedupe only → max O2', caps)
  }
  if (push && !webhookAuth) {
    level = applyCap(level, 'O2', 'webhook signature absent → cannot claim trusted O3', caps)
  }
  if (push && !reconciliation) {
    level = applyCap(level, 'O3', 'push received but canonical state not reconciled → max O3', caps)
  }
  if (!restartProof) {
    level = applyCap(level, 'O3', 'no restart/replay proof → cannot claim O4', caps)
  }
  if (!realProvider) {
    level = applyCap(level, 'O4', 'no real provider evidence → cannot claim O5', caps)
  }

  return {
    level,
    claimed: claimed || null,
    caps,
    sources,
    freshness_class: freshness || null,
    source_health: text(view.get('source_health')) || null,
    staleness_visible: view.get('staleness_visible') === true,
  }
}

export function deriveObservationAntiPatternEvidence(record) {
  const view = observationView(record)
  const derived = []
  const push = (tag, evidence) => derived.push({ tag, evidence: { ...evidence, note: evidence.note || tag } })
  const freshness = text(view.get('freshness_class') || view.get('freshness_target'))
  const poll = view.get('poll_supported') === true
  const pushSupported = view.get('push_supported') === true
  const claimedRealtime =
    /real[\s_-]*time/i.test(freshness) || freshness === 'PUSH_REALTIME' || view.get('claimed_realtime') === true

  if (claimedRealtime && !pushSupported) {
    push('PULL_ONLY_AS_REALTIME', {
      claimed_realtime: true,
      push_supported: false,
      poll_supported: poll,
      on_demand_refresh: view.get('on_demand_refresh') === true,
      evidence_paths: asArray(view.get('observation_sources')),
    })
  }
  if (pushSupported && view.get('webhook_authenticated') !== true && view.get('webhook_signature_verified') !== true) {
    push('WEBHOOK_WITHOUT_AUTH', {
      webhook_used: true,
      signature_verified: false,
      evidence_paths: asArray(view.get('observation_sources')),
    })
  }
  if (pushSupported && view.get('dedupe_durable') !== true) {
    push('WEBHOOK_WITHOUT_DEDUPE', {
      webhook_used: true,
      durable_dedupe: false,
      evidence_paths: asArray(view.get('observation_sources')),
    })
  }
  if ((poll || view.get('persistent_poll') === true) && view.get('checkpoint_durable') !== true) {
    push('POLL_WITHOUT_CHECKPOINT', {
      persistent_poll: true,
      checkpoint_durable: false,
      evidence_paths: asArray(view.get('observation_sources')),
    })
  }
  if (view.get('provider_event_ingested') === true && view.get('normalized_canonical') !== true) {
    push('EVENT_WITHOUT_NORMALIZATION', {
      provider_event_ingested: true,
      normalized_canonical: false,
      evidence_paths: asArray(view.get('observation_sources')),
    })
  }
  if (view.get('external_event_becomes_command') === true && view.get('authorized_execution') !== true) {
    push('EVENT_AS_COMMAND', {
      external_event_becomes_command: true,
      authorized_execution: false,
      evidence_paths: asArray(view.get('observation_sources')),
    })
  }
  if (pushSupported && view.get('reconciliation') !== true && view.get('webhook_treated_as_business_truth') === true) {
    push('PUSH_WITHOUT_RECONCILIATION', {
      push_received: true,
      reconciliation: false,
      claimed_as_business_truth: true,
      evidence_paths: asArray(view.get('observation_sources')),
    })
  }
  if (view.get('source_health') === 'STALE' && view.get('staleness_visible') !== true && view.get('presented_as_current') === true) {
    push('STALE_STATE_AS_CURRENT', {
      stale: true,
      staleness_visible: false,
      presented_as_current: true,
      evidence_paths: asArray(view.get('observation_sources')),
    })
  }
  if (view.get('credential_plaintext_in_app_definition') === true) {
    push('CREDENTIAL_IN_APP_DEFINITION', {
      credential_plaintext_in_app_definition: true,
      evidence_paths: asArray(view.get('leak_surfaces')),
    })
  }
  if (view.get('credential_plaintext_in_agent_context') === true) {
    push('CREDENTIAL_IN_AGENT_CONTEXT', {
      credential_plaintext_in_agent_context: true,
      evidence_paths: asArray(view.get('leak_surfaces')),
    })
  }
  if (view.get('entitlement_without_tenant_scope') === true || view.get('paid_entitlement_reused_cross_tenant') === true) {
    push('ENTITLEMENT_WITHOUT_TENANT_SCOPE', {
      entitlement_without_tenant_scope: true,
      paid_entitlement_reused_cross_tenant: view.get('paid_entitlement_reused_cross_tenant') === true,
      evidence_paths: asArray(view.get('observation_sources')),
    })
  }
  if (view.get('full_recompile_on_every_poll') === true) {
    push('FULL_RECOMPILE_ON_EVERY_POLL', {
      full_recompile_on_every_poll: true,
      evidence_paths: asArray(view.get('observation_sources')),
    })
  }
  if (view.get('raw_provider_payload_as_world_model') === true) {
    push('RAW_PROVIDER_PAYLOAD_AS_WORLD_MODEL', {
      raw_provider_payload_as_world_model: true,
      evidence_paths: asArray(view.get('observation_sources')),
    })
  }
  return derived
}

export function hasObservationPlane(record) {
  const view = observationView(record)
  return (
    isObject(record?.observation) ||
    asArray(view.get('observation_sources')).length > 0 ||
    view.get('poll_supported') === true ||
    view.get('push_supported') === true ||
    view.get('on_demand_refresh') === true ||
    hasText(view.get('claimed_observation_maturity'))
  )
}
