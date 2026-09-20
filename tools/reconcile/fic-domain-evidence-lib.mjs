// fic-domain-evidence-lib.mjs -- aggregates real, machine-computed sub-signals per FIC domain
// (docs/doctrines/180-doctrine-world-class-infrastructure-coverage.md section 6's 55-domain denominator)
// from the sync-anchor-v2 substrate (tools/reconcile/sync-anchor-v2-lib.mjs, PR #2403/#2404).
//
// This is an EVIDENCE AGGREGATOR, not a promoter. It never assigns or suggests an L0-L6 maturity
// level, and it never edits docs/_machine/world-class-infrastructure-coverage-v1.json's level/
// state/wave fields. Doc 180's L3 bar requires a named owner, a contract+anti-claims, failure
// semantics and a documented operating owner (section 2's hard rules: "a crate name, enum,
// preflight, mock, claim gate or generated document does not raise maturity") -- those are
// judgment calls reserved for the human/local-audit process that already owns v1.json. What this
// file computes, per domain, from real current repo state:
//   - how many capability_keys map to the domain, and by what method (see FIC_DOMAIN_MAPPING)
//   - of those, how many AGREE/DISAGREE in the current sync-anchor-v2 pass, by reason code
//   - how many have real reachable code (sync-anchor-v2's Tier 1/2: real body + reachable from an
//     entry point)
//   - how many have at least one test file reference (Tier 3)
//   - whether the backing crate(s) have a docs/3NN-crate-<crate>.md doc at all -- a weak proxy for
//     "does this have SOME documented surface", explicitly NOT a full owner/contract/failure-
//     semantics check
//
// Domain mapping honesty note: a crate-local shard's own `domain` field (e.g. "agent-knowledge",
// "osint", "infra-execution", "browser-internet-hand") is a donor/scraping-capability taxonomy
// used by the capability catalog generator -- it is NOT the same taxonomy as doc 180's 55
// infrastructure problem families, and the two only coincide by exact string equality for
// "commerce". Everywhere else this file falls back to a hand-curated crate-name heuristic
// (FIC_DOMAIN_MAPPING below, checked into source so it is reviewable and correctable), and where
// neither signal exists it reports NOT_MAPPABLE rather than forcing a guess.
import { readFileSync } from 'node:fs'
import { join } from 'node:path'
import {
  buildCrateModuleIndex,
  classifyCapability,
  classifyFileBody,
  discoverWorkspaceCrates,
  hasTestEvidence,
  loadCrateDocCapabilityStatus,
  loadCrateShard,
  matchCapabilityModule,
} from './sync-anchor-v2-lib.mjs'

export const V1_JSON_RELATIVE_PATH = 'docs/_machine/world-class-infrastructure-coverage-v1.json'

export function loadV1Snapshot(root) {
  return JSON.parse(readFileSync(join(root, V1_JSON_RELATIVE_PATH), 'utf8'))
}

export const MAPPING_METHOD = {
  EXPLICIT_DOMAIN_FIELD: 'explicit_domain_field',
  CRATE_NAME_HEURISTIC: 'crate_name_heuristic',
  NOT_MAPPABLE: 'NOT_MAPPABLE',
}

// Hand-curated FIC-domain-key -> {shard domain field value(s), crate name(s)} table. Every entry is
// a judgment call about which crate(s), if any, are a defensible proxy for a doc-180 problem
// family in the CURRENT capability catalog. Where no crate/domain-field signal is a fair,
// non-forced proxy (typically: cross-cutting concerns, or infra concerns that live in tooling/CI
// rather than a capability-bearing crate, or concerns no crate has been built for yet),
// method is NOT_MAPPABLE with a note explaining why -- do not stretch a match to fill this table.
export const FIC_DOMAIN_MAPPING = {
  // 6.1 Authority and edge
  'service-ownership': {
    method: MAPPING_METHOD.NOT_MAPPABLE,
    note: 'Ownership is an organizational/process property (docs/183 wave 0 exit criteria), not something one crate or shard-domain value represents.',
  },
  'tenancy-isolation': {
    method: MAPPING_METHOD.NOT_MAPPABLE,
    note: 'Several crates (chronica-company, chronica-organization, chronica-cross-holding) touch tenant/company scope, but none is the sole scope-isolation owner; picking one would misattribute evidence to a guess.',
  },
  'identity-session': { method: MAPPING_METHOD.CRATE_NAME_HEURISTIC, crate_names: ['chronica-identity'] },
  'authorization-policy': {
    method: MAPPING_METHOD.CRATE_NAME_HEURISTIC,
    crate_names: ['chronica-authorization', 'chronica-policy'],
  },
  'approval-sod': { method: MAPPING_METHOD.CRATE_NAME_HEURISTIC, crate_names: ['chronica-approvals'] },
  'vault-kms': { method: MAPPING_METHOD.CRATE_NAME_HEURISTIC, crate_names: ['chronica-vault', 'chronica-pki'] },
  'audit-evidence': { method: MAPPING_METHOD.CRATE_NAME_HEURISTIC, crate_names: ['chronica-envelope-audit-log'] },
  'api-edge': { method: MAPPING_METHOD.CRATE_NAME_HEURISTIC, crate_names: ['chronica-api', 'chronica-api-gateway'] },
  'quota-admission': {
    method: MAPPING_METHOD.NOT_MAPPABLE,
    note: 'No crate or shard-domain value names quota/admission control specifically.',
  },
  'config-discovery': {
    method: MAPPING_METHOD.NOT_MAPPABLE,
    note: 'No crate or shard-domain value names config/discovery specifically.',
  },
  // 6.2 Execution and data movement
  'workflow-orchestration': { method: MAPPING_METHOD.CRATE_NAME_HEURISTIC, crate_names: ['chronica-workflows'] },
  'durable-job-queue': {
    method: MAPPING_METHOD.NOT_MAPPABLE,
    note: 'No dedicated durable-job crate. Temporal-shaped WorkflowEngine and in-memory TaskQueueRegistry live inside chronica-workflows, which is already mapped to workflow-orchestration; reusing it here would double-count. Those modules are not a Temporal/Restate/River runtime.',
  },
  'scheduler': { method: MAPPING_METHOD.CRATE_NAME_HEURISTIC, crate_names: ['chronica-scheduler'] },
  'transactional-outbox': {
    method: MAPPING_METHOD.NOT_MAPPABLE,
    note: 'Helpdesk store dispatch (FOR UPDATE SKIP LOCKED) and CRM business_event_dispatch_store implement outbox-shaped relays, but chronica-events deleted its unused NATS/outbox island. No dedicated owner crate; attributing the domain to helpdesk would double-count with the helpdesk FIC domain.',
  },
  'stream-bus': { method: MAPPING_METHOD.NOT_MAPPABLE, note: 'No stream/broker crate exists in the workspace.' },
  'database': {
    method: MAPPING_METHOD.NOT_MAPPABLE,
    note: 'Relational persistence is cross-cutting (used from most crates via a shared client), not owned by one capability-bearing crate.',
  },
  'database-sharding': { method: MAPPING_METHOD.NOT_MAPPABLE, note: 'No sharding crate exists.' },
  'cache': { method: MAPPING_METHOD.NOT_MAPPABLE, note: 'No cache crate exists.' },
  'object-storage': {
    method: MAPPING_METHOD.NOT_MAPPABLE,
    note: 'No crate is specifically the object-storage owner; chronica-artifacts is media/artifact generation (mapped to media-pipeline below), not storage infrastructure -- reusing it here would conflate two different FIC domains.',
  },
  'search': { method: MAPPING_METHOD.CRATE_NAME_HEURISTIC, crate_names: ['chronica-search'] },
  'graph-store': { method: MAPPING_METHOD.NOT_MAPPABLE, note: 'No graph-store crate exists.' },
  'read-models': { method: MAPPING_METHOD.CRATE_NAME_HEURISTIC, crate_names: ['chronica-projection'] },
  // 6.3 Product operating surfaces
  'social-feed': { method: MAPPING_METHOD.CRATE_NAME_HEURISTIC, crate_names: ['chronica-social'] },
  'realtime-messaging': {
    method: MAPPING_METHOD.NOT_MAPPABLE,
    note: 'chronica-social already backs social-feed; no distinct realtime-messaging crate exists, and reusing chronica-social here would double-count the same capabilities under two domains.',
  },
  'media-pipeline': { method: MAPPING_METHOD.CRATE_NAME_HEURISTIC, crate_names: ['chronica-artifacts'] },
  'crm': { method: MAPPING_METHOD.CRATE_NAME_HEURISTIC, crate_names: ['chronica-crm'] },
  'helpdesk': { method: MAPPING_METHOD.CRATE_NAME_HEURISTIC, crate_names: ['chronica-helpdesk'] },
  'commerce': {
    method: MAPPING_METHOD.EXPLICIT_DOMAIN_FIELD,
    domain_field_values: ['commerce'],
    crate_names: ['chronica-commerce', 'chronica-commerce-order', 'chronica-commerce-signals', 'chronica-commerce-analytics'],
  },
  'erp-finance-tax': { method: MAPPING_METHOD.CRATE_NAME_HEURISTIC, crate_names: ['chronica-erp'] },
  'olap-metrics': { method: MAPPING_METHOD.CRATE_NAME_HEURISTIC, crate_names: ['chronica-olap'] },
  'data-governance': { method: MAPPING_METHOD.CRATE_NAME_HEURISTIC, crate_names: ['chronica-governance'] },
  'privacy-lifecycle': {
    method: MAPPING_METHOD.CRATE_NAME_HEURISTIC,
    crate_names: ['chronica-consent', 'chronica-erasure'],
  },
  // 6.4 Operations and reliability
  'telemetry': {
    method: MAPPING_METHOD.CRATE_NAME_HEURISTIC,
    crate_names: ['chronica-otel-foundation', 'chronica-tempo-tracing'],
  },
  'slo-error-budget': {
    method: MAPPING_METHOD.NOT_MAPPABLE,
    note: 'No dedicated crate; SLO/error-budget tooling, if any, lives outside the crate capability catalog.',
  },
  'incident-oncall': { method: MAPPING_METHOD.NOT_MAPPABLE, note: 'No dedicated crate.' },
  'release-delivery': {
    method: MAPPING_METHOD.NOT_MAPPABLE,
    note: 'Release/CD lives in .github/workflows and tools/, not a capability-bearing crate.',
  },
  'iac-gitops': { method: MAPPING_METHOD.NOT_MAPPABLE, note: 'No IaC/GitOps crate exists.' },
  'orchestration-runtime': { method: MAPPING_METHOD.CRATE_NAME_HEURISTIC, crate_names: ['chronica-kubernetes'] },
  'workload-identity-mesh': {
    method: MAPPING_METHOD.CRATE_NAME_HEURISTIC,
    crate_names: ['chronica-network-identity'],
  },
  'supply-chain': {
    method: MAPPING_METHOD.NOT_MAPPABLE,
    note: 'Supply-chain security tooling (SLSA/Sigstore-style) lives under tools/, not a capability-bearing crate.',
  },
  'backup-restore': { method: MAPPING_METHOD.NOT_MAPPABLE, note: 'No dedicated crate.' },
  'dr-cells': { method: MAPPING_METHOD.NOT_MAPPABLE, note: 'No dedicated crate.' },
  'capacity-chaos': { method: MAPPING_METHOD.NOT_MAPPABLE, note: 'No dedicated crate.' },
  'security-assurance': {
    method: MAPPING_METHOD.CRATE_NAME_HEURISTIC,
    crate_names: ['chronica-security', 'chronica-injection-defense'],
  },
  // 6.5 AI, providers and cross-company platform
  'ai-assignment': { method: MAPPING_METHOD.CRATE_NAME_HEURISTIC, crate_names: ['chronica-ai-workforce'] },
  'ai-provider-gateway': {
    method: MAPPING_METHOD.NOT_MAPPABLE,
    note: 'admit_provider_execution lives in chronica-ai-workforce::provider_gateway (already mapped to ai-assignment). It performs no I/O. llm_chat_gateway/comfyui/metabot_agent remain ISLAND (issue #580). Reusing the workforce crate here would double-count.',
  },
  'ai-durable-jobs': { method: MAPPING_METHOD.NOT_MAPPABLE, note: 'No dedicated crate.' },
  'ai-artifacts-cost': {
    method: MAPPING_METHOD.NOT_MAPPABLE,
    note: 'chronica-artifacts is already mapped to media-pipeline and has no cost-tracking-specific capabilities; reusing it here would conflate domains.',
  },
  'ai-feed-publication': {
    method: MAPPING_METHOD.NOT_MAPPABLE,
    note: 'chronica-social is already mapped to social-feed; no distinct AI-feed-publication crate exists (consistent with v1.json\'s NOT_BUILT state for this key).',
  },
  'provider-facade': { method: MAPPING_METHOD.CRATE_NAME_HEURISTIC, crate_names: ['chronica-integrations'] },
  'feature-flags': { method: MAPPING_METHOD.NOT_MAPPABLE, note: 'No dedicated crate.' },
  'federation': { method: MAPPING_METHOD.CRATE_NAME_HEURISTIC, crate_names: ['chronica-federation'] },
  'developer-platform': {
    method: MAPPING_METHOD.CRATE_NAME_HEURISTIC,
    crate_names: ['chronica-cli', 'chronica-tools'],
  },
  'finops': { method: MAPPING_METHOD.NOT_MAPPABLE, note: 'No dedicated crate.' },
  'compliance-operations': { method: MAPPING_METHOD.CRATE_NAME_HEURISTIC, crate_names: ['chronica-compliance'] },
}

export function domainMappingFor(key) {
  return (
    FIC_DOMAIN_MAPPING[key] ?? {
      method: MAPPING_METHOD.NOT_MAPPABLE,
      note: 'No mapping authored for this key -- it is missing from FIC_DOMAIN_MAPPING, which should track v1.json\'s denominator exactly. Treat this as a taxonomy-drift signal, not a real NOT_MAPPABLE finding.',
    }
  )
}

// ── per-capability evidence ──────────────────────────────────────────────────────────────────

function evaluateOneCapability({ capability, crateName, moduleIndex, docStatuses }) {
  const docStatus = docStatuses.get(capability.capability_key) ?? null
  const verdict = classifyCapability({
    targetModule: capability.target_module,
    shardStatus: capability.status,
    moduleIndex,
    docStatus,
  })

  // Tier-3 test evidence for EVERY real-code capability, not only shard-status "verified" ones --
  // classifyCapability only runs hasTestEvidence for that one status (it is a *disagreement* check
  // there). This reuses the same exported matching/classification/test-evidence primitives; it
  // does not reimplement module resolution or test detection.
  let tested = false
  if (verdict.evidence.code_state === 'real') {
    const { matches } = matchCapabilityModule(capability.target_module, moduleIndex.modules)
    let bestModule = null
    for (const candidate of matches) {
      if (classifyFileBody(readFileSync(candidate.absPath, 'utf8')) === 'real') {
        bestModule = candidate
        break
      }
    }
    if (bestModule) tested = hasTestEvidence(moduleIndex, bestModule)
  }

  return {
    capability_key: capability.capability_key,
    crate: crateName,
    shard_domain: capability.domain ?? null,
    shard_status: capability.status,
    target_module: capability.target_module,
    verdict: verdict.verdict,
    reason: verdict.reason ?? null,
    match_confidence: verdict.evidence.match_confidence,
    code_state: verdict.evidence.code_state,
    reachable: verdict.evidence.reachable === true,
    tested,
  }
}

/**
 * Builds a flat per-capability evidence list for exactly the crates named (typically: every crate
 * referenced anywhere in FIC_DOMAIN_MAPPING), plus a crateName -> crate-doc-path-or-null map used
 * for the "has a crate doc at all" proxy signal. Reuses discoverWorkspaceCrates/loadCrateShard/
 * loadCrateDocCapabilityStatus/buildCrateModuleIndex/classifyCapability verbatim from
 * sync-anchor-v2-lib.mjs -- no shard/doc/module parsing is reimplemented here.
 */
export function buildCapabilityEvidenceIndex(root, crateNames) {
  const crates = discoverWorkspaceCrates(root)
  const byName = new Map(crates.map((c) => [c.name, c]))
  const flat = []
  const cratesWithDocs = new Map()
  for (const crateName of crateNames) {
    const crate = byName.get(crateName)
    if (!crate) {
      cratesWithDocs.set(crateName, null)
      continue
    }
    const shard = loadCrateShard(root, crate)
    const doc = loadCrateDocCapabilityStatus(root, crateName)
    cratesWithDocs.set(crateName, doc.path)
    if (!shard.exists || !shard.capabilities.length) continue
    const moduleIndex = buildCrateModuleIndex(crate.abs_path)
    for (const capability of shard.capabilities) {
      flat.push(evaluateOneCapability({ capability, crateName, moduleIndex, docStatuses: doc.statuses }))
    }
  }
  return { capabilities: flat, cratesWithDocs }
}

// ── per-domain aggregation ───────────────────────────────────────────────────────────────────

function pct(n, total) {
  return total > 0 ? Math.round((n / total) * 1000) / 10 : null
}

export function computeDomainEvidence({ domainKey, mapping, capabilityIndex, workspaceCrateNames }) {
  if (mapping.method === MAPPING_METHOD.NOT_MAPPABLE) {
    return {
      key: domainKey,
      mapping: { method: mapping.method, domain_field_values: [], crate_names_configured: [], crate_names_missing_from_workspace: [], note: mapping.note },
      backing_crates: [],
      crate_doc: {},
      mapped_capabilities: { total: 0, by_method: { explicit_domain_field: 0, crate_name_heuristic: 0 }, capability_keys: [] },
      sync_anchor_v2: null,
      code_reachability: null,
      test_evidence: null,
    }
  }

  const domainFieldValues = new Set(mapping.domain_field_values ?? [])
  const configuredCrateNames = mapping.crate_names ?? []
  const crateNames = configuredCrateNames.filter((name) => workspaceCrateNames.has(name))
  const missingCrates = configuredCrateNames.filter((name) => !workspaceCrateNames.has(name))
  const crateNameSet = new Set(crateNames)

  const matched = []
  for (const cap of capabilityIndex.capabilities) {
    const byDomainField = domainFieldValues.has(cap.shard_domain)
    const byCrateName = crateNameSet.has(cap.crate)
    if (!byDomainField && !byCrateName) continue
    matched.push({ ...cap, mapped_by: byDomainField ? MAPPING_METHOD.EXPLICIT_DOMAIN_FIELD : MAPPING_METHOD.CRATE_NAME_HEURISTIC })
  }

  const byMethod = { explicit_domain_field: 0, crate_name_heuristic: 0 }
  for (const m of matched) byMethod[m.mapped_by] += 1

  const agree = matched.filter((m) => m.verdict === 'AGREE').length
  const disagree = matched.filter((m) => m.verdict === 'DISAGREE').length
  const disagreeByReason = {}
  for (const m of matched) {
    if (m.verdict === 'DISAGREE') disagreeByReason[m.reason] = (disagreeByReason[m.reason] ?? 0) + 1
  }

  const reachableCount = matched.filter((m) => m.code_state === 'real' && m.reachable).length
  const testedCount = matched.filter((m) => m.tested).length

  const crateDoc = {}
  for (const name of crateNames) crateDoc[name] = capabilityIndex.cratesWithDocs.get(name) ?? null

  return {
    key: domainKey,
    mapping: {
      method: mapping.method,
      domain_field_values: [...domainFieldValues],
      crate_names_configured: configuredCrateNames,
      crate_names_missing_from_workspace: missingCrates,
      note: mapping.note ?? null,
    },
    backing_crates: crateNames,
    crate_doc: crateDoc,
    mapped_capabilities: {
      total: matched.length,
      by_method: byMethod,
      capability_keys: matched.map((m) => `${m.crate}/${m.capability_key}`),
    },
    sync_anchor_v2: matched.length
      ? { agree, disagree, agree_percent: pct(agree, matched.length), disagree_by_reason: disagreeByReason }
      : null,
    code_reachability: matched.length ? { reachable: reachableCount, reachable_percent: pct(reachableCount, matched.length) } : null,
    test_evidence: matched.length ? { tested: testedCount, tested_percent: pct(testedCount, matched.length) } : null,
  }
}

/**
 * Top-level entry point: reads v1.json's domain list (the taxonomy source of truth -- this file
 * never invents domain names), computes per-domain evidence for every one of them, and attaches
 * the CURRENT hand-assigned level/state/wave read-only, for side-by-side comparison. Never
 * computes or returns a suggested level.
 */
export function computeAllDomainEvidence(root) {
  const v1 = loadV1Snapshot(root)
  const workspaceCrates = discoverWorkspaceCrates(root)
  const workspaceCrateNames = new Set(workspaceCrates.map((c) => c.name))

  const allMappedCrateNames = new Set()
  for (const domain of v1.domains) {
    const mapping = domainMappingFor(domain.key)
    for (const c of mapping.crate_names ?? []) allMappedCrateNames.add(c)
  }

  const capabilityIndex = buildCapabilityEvidenceIndex(root, [...allMappedCrateNames])

  const perDomain = v1.domains.map((domain) => {
    const mapping = domainMappingFor(domain.key)
    const evidence = computeDomainEvidence({ domainKey: domain.key, mapping, capabilityIndex, workspaceCrateNames })
    return { ...evidence, hand_assigned: { level: domain.level, state: domain.state, wave: domain.wave ?? null } }
  })

  return {
    v1_as_of_date: v1.as_of_date,
    v1_status: v1.status,
    v1_canonical: v1.canonical,
    domains_total: v1.domains.length,
    per_domain: perDomain,
  }
}

// ── review-priority sort key (NOT a level judgment -- purely for ordering the human review list) ─

const LEVEL_RANK = { L0: 0, L1: 1, L2: 2, L3: 3, L4: 4, L5: 5, L6: 6 }
const EVIDENCE_STRENGTH_RANK_CEILING = 3 // scaled to compare against L0..L3, the only bar this evidence type speaks to

/**
 * A 0..1 composite of AGREE rate, reachable-code rate and tested rate among a domain's mapped
 * capabilities. This is a triage heuristic for sort order ONLY -- it is deliberately not named or
 * shaped like a maturity level, and callers must not treat it as one.
 */
export function evidenceStrengthScore(domainEvidence) {
  if (!domainEvidence.mapped_capabilities.total) return null
  const agreeRate = (domainEvidence.sync_anchor_v2?.agree_percent ?? 0) / 100
  const reachRate = (domainEvidence.code_reachability?.reachable_percent ?? 0) / 100
  const testRate = (domainEvidence.test_evidence?.tested_percent ?? 0) / 100
  return (agreeRate + reachRate + testRate) / 3
}

/**
 * Sort key for deliverable 2's report: how much this domain's raw evidence appears to contradict
 * its CURRENT hand-assigned level, in either direction. This never outputs an L-value -- only a
 * scalar "how surprising is this" score plus a direction label, so a human can triage the highest-
 * value re-reviews first. Domains that cannot be evaluated (NOT_MAPPABLE, or mapped to a crate with
 * zero capability-catalog rows) are marked not-evaluable and sorted after every evaluable domain.
 */
export function reviewPriority(domainEvidence) {
  const strength = evidenceStrengthScore(domainEvidence)
  if (strength === null) {
    return {
      evaluable: false,
      reason: domainEvidence.mapping.method === MAPPING_METHOD.NOT_MAPPABLE ? 'NOT_MAPPABLE' : 'MAPPED_ZERO_CAPABILITIES',
      contradiction_score: null,
      evidence_strength: null,
      direction: null,
    }
  }
  const handRank = LEVEL_RANK[domainEvidence.hand_assigned.level] ?? 0
  const strengthRank = strength * EVIDENCE_STRENGTH_RANK_CEILING
  const contradiction_score = Math.round(Math.abs(strengthRank - handRank) * 100) / 100
  const direction =
    strengthRank > handRank
      ? 'EVIDENCE_STRONGER_THAN_HAND_LEVEL'
      : strengthRank < handRank
        ? 'EVIDENCE_WEAKER_THAN_HAND_LEVEL'
        : 'EVIDENCE_ROUGHLY_MATCHES_HAND_LEVEL'
  return {
    evaluable: true,
    reason: null,
    contradiction_score,
    evidence_strength: Math.round(strength * 1000) / 1000,
    direction,
  }
}
