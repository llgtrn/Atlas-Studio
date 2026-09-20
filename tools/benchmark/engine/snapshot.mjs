// Assemble a machine-verifiable truth snapshot from existing controller artifacts.
// Does not rewrite the frozen architecture verdict.

import { loadJson, loadV1Denominator } from '../fic-recompute-lib.mjs'
import { loadContracts } from '../intent-contract-validate.mjs'
import { loadFivePlaneAudit, validateFivePlaneAudit } from '../five-plane-audit-lib.mjs'
import { loadFirstVertical, validateFirstVertical } from '../first-vertical-gap.mjs'
import { FINDING_DRIFT, FIC_DENOMINATOR, FROZEN_VERDICT, GENERATOR_VERSION, SCHEMA } from './vocab.mjs'
import { bindProvenance, readRepositorySha } from './provenance.mjs'
import { engineProviderSplitFromFreeze, validateNoAiTaxonomy } from './no-ai.mjs'
import { liveAuditToFivePlaneData, validateFivePlaneData } from './five-plane.mjs'
import { relationshipsFromLiveContracts, detectDuplicateExecutionSubstrate } from './contracts.mjs'
import { frozenAuthorityFromLive, validateAuthorityDistinction } from './authority.mjs'
import { evaluateActorNeutralSubstrate } from './actor-neutral.mjs'
import { evaluateAntiPatternSet } from './anti-patterns.mjs'
import { evaluateEvidenceGraph } from './evidence-graph.mjs'
import { evaluateInflationClaims } from './anti-inflation.mjs'
import { guardFicDenominator, rejectCapabilityCountAsFic } from './fic-guard.mjs'
import { computeAndAdmit } from './family-admission.mjs'
import { recomputeFirstVertical } from './first-vertical.mjs'
import { join } from 'node:path'
import { asArray, text } from './util.mjs'

function frozenArchitecture(audit) {
  const freeze = audit?.freeze || {}
  return {
    five_plane_audit_status: freeze.five_plane_audit_status || FROZEN_VERDICT.five_plane_audit_status,
    verdict: freeze.verdict || audit?.verdict || FROZEN_VERDICT.verdict,
    verdict_label: freeze.verdict_label || audit?.verdict_label || FROZEN_VERDICT.verdict_label,
    core_spine: freeze.core_spine || FROZEN_VERDICT.core_spine,
    authoritative_architecture_verdict: freeze.authoritative_architecture_verdict === true,
  }
}

function driftIf(condition, detail) {
  return condition ? [{ code: FINDING_DRIFT, detail }] : []
}

export function buildSnapshot(options = {}) {
  const errors = []
  const root = options.root
  const generated_at = options.generated_at
  const repository_sha = text(options.repository_sha) || readRepositorySha(root) || ''
  const audit = options.audit || loadFivePlaneAudit(root)
  const census = options.census || loadJson(join(root, 'docs/_machine/fic-revalidation-census.json'))
  const v1 = options.v1 || loadV1Denominator(root)
  const firstVertical = options.firstVertical || loadFirstVertical(root)
  const contracts = options.contracts || loadContracts(join(root, 'docs/_machine/intent-contracts')).map((row) => row.contract)
  const inflationClaims = asArray(options.inflation_claims)
  const evidencePaths = asArray(options.evidence_paths)
  const antiPatternEntries = asArray(options.anti_patterns)

  const liveAudit = validateFivePlaneAudit(audit, { contracts })
  if (!liveAudit.ok) errors.push(...liveAudit.errors.map((e) => `live_five_plane:${e}`))

  const provenance = bindProvenance({
    repository_sha,
    evaluated_sha: repository_sha,
    base_sha: text(options.base_sha) || text(audit?.evidence_sha),
    product_truth_sha: options.product_truth_sha === undefined ? null : options.product_truth_sha,
    generated_at,
    tests_passing: options.tests_passing === true,
  })
  if (provenance.freshness === 'INVALID') errors.push(...provenance.errors)

  const ficGuard = guardFicDenominator(v1, census)
  if (!ficGuard.ok) errors.push(...ficGuard.errors)
  const ficReject = rejectCapabilityCountAsFic({
    as: 'FIC',
    not_capability_inventory: true,
    require_explicit_guard: true,
    denominator: ficGuard.fic?.denominator,
  })
  if (!ficReject.ok) errors.push(...ficReject.errors)

  const freeze = frozenArchitecture(audit)
  const fic = {
    denominator: ficGuard.fic?.denominator || FIC_DENOMINATOR,
    l3_or_higher: ficGuard.fic?.l3_or_higher,
    counts: ficGuard.fic?.counts,
    percent: ficGuard.fic?.fic_percent,
    pec: ficGuard.fic?.pec,
    cse: ficGuard.fic?.cse,
    not_capability_inventory: true,
  }

  const finding_drift = [
    ...driftIf(
      freeze.verdict !== FROZEN_VERDICT.verdict,
      `engine would see verdict ${freeze.verdict} vs frozen ${FROZEN_VERDICT.verdict}`,
    ),
    ...driftIf(
      fic.l3_or_higher != null && `${fic.l3_or_higher}/${fic.denominator}` !== '6/55',
      `recomputed FIC ${fic.l3_or_higher}/${fic.denominator} vs frozen 6/55`,
    ),
  ]

  const noAiSurfaces = [
    ...engineProviderSplitFromFreeze(audit?.freeze),
    ...asArray(audit?.no_ai_matrix).map((row) => ({
      domain: row.domain,
      provider_kind: row.domain === 'external-provider-execution' ? undefined : 'NOT_A_PROVIDER',
      no_ai_class: row.no_ai,
      taxonomy_status: row.taxonomy_status,
      name: row.domain,
    })),
  ]
  const noAi = validateNoAiTaxonomy(noAiSurfaces)

  const planeData = liveAuditToFivePlaneData(audit, { no_ai_surfaces: noAiSurfaces })
  const planeCheck = validateFivePlaneData(planeData)
  if (!planeCheck.ok) errors.push(...planeCheck.errors)

  const relationships = options.relationships || relationshipsFromLiveContracts(contracts)
  const contractDup = detectDuplicateExecutionSubstrate(contracts, relationships)
  if (!contractDup.ok) errors.push(...contractDup.errors)

  const authority = frozenAuthorityFromLive(firstVertical.board_authority_decision || audit?.freeze?.rerun_corrections?.authority)
  const authorityCheck = validateAuthorityDistinction(authority)
  if (!authorityCheck.ok) errors.push(...authorityCheck.errors)

  const actorNeutral = evaluateActorNeutralSubstrate(
    options.actor_neutral || {
      actors: { human: true, agent: true, API: true, scheduler: true, webhook: true, system: true },
      shared_deterministic_command_run_boundary: false,
      table_family: '',
      durable_job_claimed: true,
      persistence_kind: 'IN_MEMORY',
      agent_special_path: false,
      human_uses_same_boundary: false,
      evidence_paths: ['docs/_machine/intent-contracts/CORE-EXECUTION-SPINE-001.json'],
    },
  )

  const firstVerticalLegacy = validateFirstVertical(firstVertical)
  if (!firstVerticalLegacy.ok) errors.push(...firstVerticalLegacy.errors.map((e) => `first_vertical:${e}`))
  const firstVerticalEngine = recomputeFirstVertical(firstVertical)

  const families = options.skip_families
    ? { skipped: true, family_count: 11, admission: { ok: true, findings: [] } }
    : computeAndAdmit(options.family_ratio_options || {})

  const graph = evaluateEvidenceGraph(evidencePaths)
  const inflation = evaluateInflationClaims(inflationClaims)
  const antiPatterns = evaluateAntiPatternSet(antiPatternEntries)

  const snapshot = {
    schema: SCHEMA,
    generator_version: GENERATOR_VERSION,
    provenance,
    frozen_architecture: freeze,
    finding_drift,
    fic,
    no_ai: {
      ok: noAi.ok,
      errors: noAi.errors,
      surfaces: noAiSurfaces,
    },
    five_plane: planeData,
    contracts: {
      relationships,
      duplicate_ok: contractDup.ok,
      errors: contractDup.errors,
    },
    authority,
    actor_neutral: actorNeutral,
    anti_patterns: antiPatterns.findings,
    evidence_graph: graph,
    anti_inflation: inflation,
    first_vertical: firstVerticalEngine,
    families: families.skipped
      ? families
      : {
          family_count: families.ratio?.ratios?.total_families,
          wave_admission_ready: families.ratio?.wave_admission_ready === true,
          admission: families.admission,
        },
  }

  return { ok: errors.length === 0 && provenance.freshness !== 'INVALID', errors, snapshot }
}
