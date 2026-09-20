#!/usr/bin/env node
// query-benchmark-ratio.mjs — computes the benchmark coverage ratio for the 11 canonical platform
// replication families defined in docs/doctrines/008-doctrine-platform-replication-plane.md section 7.
//
// Sources read (no root docs/capabilities.db or docs/architecture.db dependency -- cloud-safe):
//   - tools/benchmark/benchmark-family-registry.json (hand-curated doctrine evidence, cited)
//   - crates/chronica-platform-plane/.chronica/sub-cap-arch.jsonl (live crate-local shard: capability
//     status + capability_architecture_link rows for each platform.replicate_* row)
//   - docs/_machine/world-class-benchmark-portfolio.jsonl + docs/_machine/reference-repo-grounding-ledger.jsonl
//     (the two required artifacts named by docs/benchmarks/116-operating-world-class-benchmark-gates.md section 7;
//     read live and cross-referenced by name/repository keyword against each family, never modified here)
//
// This tool never claims root-DB verified status. Every capability-tracking dimension it reports is
// LOCAL_AUDIT_REQUIRED unless the crate-local shard itself says otherwise, per
// docs/doctrines/023-five-dimension-cloud-shard-contract.md. Its evidence-vocabulary and portfolio-minimum
// fields follow docs/benchmarks/116-operating-world-class-benchmark-gates.md sections 1, 3, and 5.
import { readFileSync, existsSync } from 'node:fs'
import { fileURLToPath } from 'node:url'
import path from 'node:path'
import { portfolioMinimumForFamily } from './benchmark-pin-check-lib.mjs'

const __dirname = path.dirname(fileURLToPath(import.meta.url))
const ROOT = path.resolve(__dirname, '..', '..')

const REGISTRY_PATH = path.join(__dirname, 'benchmark-family-registry.json')
const SHARD_PATH = path.join(ROOT, 'crates/chronica-platform-plane/.chronica/sub-cap-arch.jsonl')
const PORTFOLIO_PATH = path.join(ROOT, 'docs/_machine/world-class-benchmark-portfolio.jsonl')
const GROUNDING_LEDGER_PATH = path.join(ROOT, 'docs/_machine/reference-repo-grounding-ledger.jsonl')

export function loadRegistry(registryPath = REGISTRY_PATH) {
  return JSON.parse(readFileSync(registryPath, 'utf8'))
}

export function loadShard(shardPath = SHARD_PATH) {
  const lines = readFileSync(shardPath, 'utf8').split('\n').filter((l) => l.trim().length > 0)
  const records = lines.map((l) => JSON.parse(l))
  const capabilityByKey = new Map()
  const linksByCapabilityKey = new Map()
  for (const rec of records) {
    if (rec.record_type === 'capability' && rec.capability_key) {
      capabilityByKey.set(rec.capability_key, rec)
    }
    if (rec.record_type === 'capability_architecture_link' && rec.capability_key) {
      const arr = linksByCapabilityKey.get(rec.capability_key) || []
      arr.push(rec)
      linksByCapabilityKey.set(rec.capability_key, arr)
    }
  }
  return { records, capabilityByKey, linksByCapabilityKey }
}

function loadJsonl(jsonlPath) {
  if (!existsSync(jsonlPath)) return []
  return readFileSync(jsonlPath, 'utf8')
    .split('\n')
    .filter((l) => l.trim().length > 0)
    .map((l) => JSON.parse(l))
}

export function loadPortfolio(portfolioPath = PORTFOLIO_PATH) {
  return loadJsonl(portfolioPath)
}

export function loadGroundingLedger(ledgerPath = GROUNDING_LEDGER_PATH) {
  return loadJsonl(ledgerPath)
}

// Live keyword cross-reference: does docs/_machine/world-class-benchmark-portfolio.jsonl already
// carry a pinned entry for one of this family's named external benchmarks? Matching requires the
// FULL benchmark phrase (case-insensitive, punctuation-normalized) to appear in the portfolio entry's
// name or repository -- deliberately strict, never a single leading word or token. A loose
// first-word match previously produced a false positive ("Open Design" matching "OpenAI Agents SDK"
// via "open"), which is exactly the anti-gaming failure docs/116 section 9 forbids ("no provider/
// brand-name counted as capability", "no default branch citation without pin"). A false negative here
// only means "run the tool again after the entry is added," never a fabricated CONNECTED claim.
function normalizeForMatch(value) {
  return String(value || '')
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, ' ')
    .trim()
}

export function matchPortfolioEntries(family, portfolioEntries) {
  const needles = (family.external_benchmarks || [])
    .map((b) => normalizeForMatch(b))
    // Require a genuine multi-word phrase, not a single brand token. A bare company name (e.g.
    // "Vercel") would otherwise match ANY portfolio entry that mentions the brand for an unrelated
    // product line (e.g. "Vercel AI provider interface v3", a JS SDK, when the family actually means
    // Vercel-the-hosting-platform) -- exactly the "provider/brand-name counted as capability"
    // anti-gaming failure docs/116 section 9 forbids.
    .filter((n) => n.length >= 4 && n.split(' ').length >= 2)
  const byPhrase = portfolioEntries.filter((entry) => {
    const haystack = normalizeForMatch(`${entry.name || ''} ${entry.repository || ''}`)
    return needles.some((needle) => haystack.includes(needle))
  })
  // Explicit family_keys on a portfolio row is the anti-gaming-safe way to attach a single-token
  // brand (Medusa, Zendesk) without loosening phrase matching. The family_key must be exact.
  const byFamilyKey = portfolioEntries.filter(
    (entry) => Array.isArray(entry.family_keys) && entry.family_keys.includes(family.family_key),
  )
  const seen = new Set()
  const merged = []
  for (const entry of [...byFamilyKey, ...byPhrase]) {
    const id = entry.id || `${entry.repository}@${entry.pinned_sha}`
    if (seen.has(id)) continue
    seen.add(id)
    merged.push(entry)
  }
  return merged
}

const EXTERNAL_MAP_NAMED = new Set([
  'EXPLICITLY_DOCUMENTED',
  'IMPLIED_BY_CAPABILITY_ROWS',
  'CODE_SUBSTRATE_PRESENT',
  'DESIGN_ONLY',
  'NEEDS_DONOR_EXTRACTION',
  'ABSENT_OR_NOT_FOUND',
])

function dimensionForFamily(family, shard) {
  const shardCap = shard.capabilityByKey.get(family.capability_key)
  const shardLinks = shard.linksByCapabilityKey.get(family.capability_key) || []

  const named_read = {
    status: isNamedRead(family.external_map_status) ? 'CONNECTED' : 'LOCAL_AUDIT_REQUIRED',
    evidence: family.external_map_citation,
    external_map_status: family.external_map_status,
  }

  const mapped_to_native_architecture = {
    status: family.native_homes && family.native_homes.length > 0 ? 'CONNECTED' : 'BLOCKED_WITH_EVIDENCE',
    evidence: family.native_homes_citation,
    native_homes: family.native_homes,
  }

  // CODE_SUBSTRATE_PRESENT and CONTRACT_ONLY are deliberately NOT merged into one 'CONNECTED'
  // bucket: a contract-only crate (e.g. chronica-meta-plane for data_platform -- typed dependency
  // contracts, no runtime persistence) is materially weaker evidence than real behavior code
  // implementing part of the family, and collapsing them previously let a reader mistake
  // has_code_substrate's 8/11 for "8 families have real code" when only 7 do (data_platform's 1 was
  // contract-only). Flagged by controller audit of PR #2153; see world_class_gate.evidence_vocabulary
  // (CODE_PRESENT vs PRIMITIVE) for the doctrine-116-level distinction this dimension now matches.
  const has_code_path = {
    status:
      family.code_substrate === 'CODE_SUBSTRATE_PRESENT'
        ? 'CONNECTED'
        : family.code_substrate === 'CONTRACT_ONLY'
          ? 'CONNECTED_CONTRACT_ONLY'
          : family.code_substrate === 'FRAGMENTED_PARTIAL'
            ? 'LOCAL_AUDIT_REQUIRED'
            : 'BLOCKED_WITH_EVIDENCE',
    evidence: family.code_substrate_note,
    code_substrate: family.code_substrate,
  }

  const has_tests_evidence = {
    status: family.tests_evidence ? 'CONNECTED' : 'BLOCKED_WITH_EVIDENCE',
    evidence: family.tests_evidence || 'no test evidence cited',
  }

  const has_docs = {
    status: 'CONNECTED',
    evidence: (family.external_map_citation || '') + '; ' + (family.native_homes_citation || ''),
  }

  const has_shard_evidence = {
    status: shardCap && shardLinks.length > 0 ? 'CONNECTED_CONTRACT_ONLY' : 'BLOCKED_WITH_EVIDENCE',
    evidence: shardCap
      ? `crates/chronica-platform-plane/.chronica/sub-cap-arch.jsonl :: ${family.capability_key} status=${shardCap.status}, ${shardLinks.length} architecture link row(s)`
      : `${family.capability_key} not found in crates/chronica-platform-plane/.chronica/sub-cap-arch.jsonl`,
    reason:
      'This is claim-gate tracking evidence in chronica-platform-plane\'s own shard, per docs/008 section 7 note. It is NOT evidence that the family\'s own implementation crate(s) carry a matching capability row in their own crate-local shard.',
    shard_capability_status: shardCap ? shardCap.status : null,
    architecture_link_count: shardLinks.length,
  }

  const no_white_zone_ecosystem = family.ecosystem_no_white_zone_applicable
    ? {
        status: 'LOCAL_AUDIT_REQUIRED',
        evidence: family.ecosystem_no_white_zone_reason,
        reason:
          'No ecosystem_connections matrix (docs/006 section 4, 14 domains) has been filed for this family in this branch; a web/commerce/design/hosting cloud shard report carrying that matrix is required before this can move to CONNECTED.',
      }
    : {
        status: 'NOT_APPLICABLE',
        evidence: family.ecosystem_no_white_zone_reason,
      }

  const capability_tracking = {
    status: shardCap && shardCap.status === 'verified' ? 'LOCAL_VERIFIED' : 'LOCAL_AUDIT_REQUIRED',
    evidence: shardCap
      ? `crate-local shard status=${shardCap.status}; root docs/capabilities.db is local-only and not present in this checkout, so final capability-row verification cannot be confirmed here`
      : 'no crate-local shard row found for this capability key',
    root_capabilities_db_present: false,
  }

  return {
    named_read,
    mapped_to_native_architecture,
    has_code_path,
    has_tests_evidence,
    has_docs,
    has_shard_evidence,
    no_white_zone_ecosystem,
    capability_tracking,
  }
}

function isNamedRead(status) {
  return EXTERNAL_MAP_NAMED.has(status)
}

const EVIDENCE_VOCABULARY = new Set([
  'DOC_ONLY',
  'TRACKING_ONLY',
  'PRIMITIVE',
  'CODE_PRESENT',
  'RUNTIME_CLOSED',
  'CONFORMANCE_GREEN',
  'PRODUCTION_PROVEN',
  'LOCAL_AUDIT_REQUIRED',
])

const STRATEGIC_MINIMUM =
  'strategic/high-risk per docs/116 section 5: >=3 independent external implementation repositories, >=2 architectural lineages, >=1 standards/protocol benchmark, >=1 internal baseline, >=1 adversarial/failure benchmark'
const ORDINARY_MINIMUM =
  'ordinary bounded per docs/116 section 5: >=2 independent external repositories, >=1 applicable standard, >=1 internal baseline'

// docs/benchmarks/116-operating-world-class-benchmark-gates.md section 3's required per-benchmark matrix,
// computed from the registry's own cited fields. Every field this session cannot honestly source
// (a pinned external commit/version, a read license, a real adopted/rejected comparison) is marked
// NOT_YET_PINNED / LOCAL_AUDIT_REQUIRED rather than fabricated -- no donor repository was cloned in
// this session (docs/doctrines/094-operating-donor-grounding-protocol.md admission step was not performed for
// any of these 11 families).
function requiredMatrixForFamily(family, matchedPortfolioEntries, groundingLedgerEntries = []) {
  const pinned = matchedPortfolioEntries.filter((e) => /^[0-9a-f]{40}$/i.test(String(e.pinned_sha || '')))
  const ledgerById = new Map((groundingLedgerEntries || []).map((e) => [e.source_id, e]))
  const grounded = pinned
    .map((e) => ledgerById.get(e.id))
    .filter(Boolean)
  const adopted = grounded.flatMap((e) => (Array.isArray(e.adopted) ? e.adopted : []))
  const rejected = grounded.flatMap((e) => (Array.isArray(e.rejected) ? e.rejected : []))
  const compared = grounded.length > 0
  return {
    benchmark_name: (family.external_benchmarks || []).join(' / '),
    benchmark_type: compared
      ? 'External implementation + product/industry behavior benchmark (docs/008 + docs/116)'
      : 'Product/industry behavior benchmark (docs/008 + docs/design/2026-07-09 external map); implementation pin incomplete',
    pinned_version_or_sha: pinned.length
      ? pinned.map((e) => `${e.repository}@${e.pinned_sha}`).join(', ')
      : 'NOT_YET_PINNED -- no donor repository cloned/pinned for this family in this session',
    exact_paths_or_sections: grounded.length
      ? grounded.flatMap((e) => e.exact_paths_read || []).join(', ')
      : pinned.length
        ? 'PINNED_BUT_PATHS_UNREAD'
        : 'NOT_YET_PINNED',
    license: pinned.length
      ? pinned.map((e) => `${e.repository}:${e.license || 'LOCAL_AUDIT_REQUIRED'}`).join(', ')
      : 'LOCAL_AUDIT_REQUIRED -- unknown until a source is pinned',
    compared_behavior: family.claim_allowed_now || 'LOCAL_AUDIT_REQUIRED',
    chronica_before: family.code_substrate_note || 'LOCAL_AUDIT_REQUIRED',
    chronica_after: 'controller wave: no product runtime change; pins/census/contracts only',
    delta: compared ? 'source pins + extracted invariants recorded; no family runtime implementation' : 'none (no family product code change)',
    adopted,
    rejected,
    where_chronica_is_weaker: family.claim_forbidden || 'LOCAL_AUDIT_REQUIRED',
    where_chronica_is_stronger:
      'MUST_EXCEED tenant/approval/vault/audit/provider-facade where Chronica safety doctrine requires it; otherwise LOCAL_AUDIT_REQUIRED',
    what_is_unproven: compared
      ? 'Family-wide ExternalClass/ProductionReady and PEC/CSE remain unproven even when individual sources are pinned.'
      : 'Real external-system behavior at a pinned version/commit has not been read for this family in this session; portfolio-minimum sourcing (docs/116 section 5) is not met.',
    posture: family.target_posture || 'LOCAL_AUDIT_REQUIRED',
    evidence: [family.external_map_citation, family.native_homes_citation].filter(Boolean),
    verdict: pinned.length
      ? compared
        ? 'BENCHMARK_EVIDENCE_FAIL -- pins and some source reads exist; family-wide comparison is not WAVE_ADMISSION_READY'
        : 'BENCHMARK_EVIDENCE_FAIL -- pinned source exists but 094 ledger comparison is incomplete'
      : 'BENCHMARK_BLOCKED_WITH_EVIDENCE -- no pinned source read this session',
  }
}

function worldClassGateForFamily(family, portfolioEntries, groundingLedgerEntries = []) {
  const matched = matchPortfolioEntries(family, portfolioEntries)
  const minimum = portfolioMinimumForFamily(family, matched)
  const evidenceVocabulary = EVIDENCE_VOCABULARY.has(family.evidence_vocabulary_family_claim)
    ? family.evidence_vocabulary_family_claim
    : 'LOCAL_AUDIT_REQUIRED'

  return {
    evidence_vocabulary_family_claim: evidenceVocabulary,
    evidence_vocabulary_note: family.evidence_vocabulary_note || null,
    target_posture: family.target_posture || 'LOCAL_AUDIT_REQUIRED',
    runtime_closure: family.runtime_closure || 'LOCAL_AUDIT_REQUIRED',
    production_evidence: family.production_evidence || 'NOT_PROVEN',
    portfolio: {
      risk_tier: minimum.risk_tier,
      required_minimum: minimum.required.text,
      external_repos_named_count: (family.external_benchmarks || []).length,
      external_repos_pinned_count: matched.length,
      matched_portfolio_entries: matched.map((e) => ({
        id: e.id,
        name: e.name,
        repository: e.repository,
        pinned_sha: e.pinned_sha,
        source_class: e.source_class || null,
      })),
      meets_minimum: minimum.meets_minimum,
      counts: minimum.counts,
      gaps: minimum.gaps,
      reason: minimum.meets_minimum
        ? 'Pinned portfolio entries satisfy the docs/116 section 5 class counts for this family.'
        : `Portfolio minimum not met: ${minimum.gaps.join('; ') || 'no matching pins'}.`,
    },
    required_matrix: requiredMatrixForFamily(family, matched, groundingLedgerEntries),
  }
}

export function computeBenchmarkRatio({
  registryPath = REGISTRY_PATH,
  shardPath = SHARD_PATH,
  portfolioPath = PORTFOLIO_PATH,
  groundingLedgerPath = GROUNDING_LEDGER_PATH,
} = {}) {
  const registry = loadRegistry(registryPath)
  const shard = loadShard(shardPath)
  const portfolioEntries = loadPortfolio(portfolioPath)
  const groundingLedgerEntries = loadGroundingLedger(groundingLedgerPath)

  const families = registry.families.map((family) => ({
    family_key: family.family_key,
    capability_key: family.capability_key,
    external_benchmarks: family.external_benchmarks,
    claim_allowed_now: family.claim_allowed_now,
    claim_forbidden: family.claim_forbidden,
    claim_filed: !!family.claim_filed,
    claim_level_reached: family.claim_level_reached,
    claim_ready: !!family.claim_ready,
    money_adjacent: !!family.money_adjacent,
    donor_extraction_doc_exists: !!family.donor_extraction_doc_exists,
    dimensions: dimensionForFamily(family, shard),
    world_class_gate: worldClassGateForFamily(family, portfolioEntries, groundingLedgerEntries),
    runtime_closure: family.runtime_closure || 'LOCAL_AUDIT_REQUIRED',
    production_evidence: family.production_evidence || 'NOT_PROVEN',
  }))

  const total = families.length

  function countWhere(pred) {
    return families.filter(pred).length
  }

  const ratios = {
    total_families: total,
    named_and_mapped: {
      count: countWhere(
        (f) => f.dimensions.named_read.status === 'CONNECTED' && f.dimensions.mapped_to_native_architecture.status === 'CONNECTED',
      ),
      denominator: total,
      definition: 'Families where the external benchmark is named/read AND mapped to a Chronica native architecture home.',
    },
    has_code_substrate: {
      count: countWhere((f) => f.dimensions.has_code_path.status === 'CONNECTED'),
      denominator: total,
      definition: 'Families with real, non-donor-wrapper Chronica BEHAVIOR code implementing at least part of the family (code_substrate === CODE_SUBSTRATE_PRESENT). Does NOT include contract-only crates -- see has_contract_only_substrate below.',
    },
    has_contract_only_substrate: {
      count: countWhere((f) => f.dimensions.has_code_path.status === 'CONNECTED_CONTRACT_ONLY'),
      denominator: total,
      definition: 'Families whose only code is a typed dependency/contract crate with no runtime persistence or behavior (code_substrate === CONTRACT_ONLY, e.g. data_platform / chronica-meta-plane). Reported separately from has_code_substrate so a reader cannot mistake contract-only evidence for real behavior code (controller audit finding on PR #2153).',
    },
    has_tests_evidence: {
      count: countWhere((f) => f.dimensions.has_tests_evidence.status === 'CONNECTED'),
      denominator: total,
      definition: 'Families with at least one cited test command/path proving the code substrate.',
    },
    has_docs: {
      count: countWhere((f) => f.dimensions.has_docs.status === 'CONNECTED'),
      denominator: total,
      definition: 'Families with doctrine/design doc citations (always true by construction of this registry).',
    },
    has_donor_extraction_doc: {
      count: countWhere((f) => f.donor_extraction_doc_exists),
      denominator: total,
      definition: 'Families with a dedicated donor-extraction design doc on disk (not merely a doc citing the family in passing).',
    },
    has_filed_platform_claim: {
      count: countWhere((f) => f.claim_filed),
      denominator: total,
      definition: 'Families with a real chronica-platform-plane claim plan filed under tools/platform-plane/claims/ and checked by pnpm platform:verify-claims.',
    },
    claim_ready_external_class_or_better: {
      count: countWhere((f) => f.claim_ready),
      denominator: total,
      definition: 'Families whose filed claim reports ready=true (ExternalClass or ProductionReady evidence passing). Currently expected to be 0 -- no family has closed all required build paths and platform primitives.',
    },
    money_adjacent_families: {
      count: countWhere((f) => f.money_adjacent),
      denominator: total,
      definition: 'Families whose capability row is moves_money=1/requires_approval=1 per the crate-local shard (fintech_core, commerce_platform, web_commerce_estate).',
    },
    capability_tracking_local_audit_required: {
      count: countWhere((f) => f.dimensions.capability_tracking.status === 'LOCAL_AUDIT_REQUIRED'),
      denominator: total,
      definition: 'Families whose capability-row status cannot be confirmed against root docs/capabilities.db in this checkout (root DB is local-only/gitignored).',
    },
    meets_portfolio_minimum: {
      count: countWhere((f) => f.world_class_gate.portfolio.meets_minimum),
      denominator: total,
      definition: 'Families meeting docs/benchmarks/116-operating-world-class-benchmark-gates.md section 5 portfolio minimums (live cross-referenced against docs/_machine/world-class-benchmark-portfolio.jsonl). Expected 0 until a dedicated donor-grounded dispatch pins external repos for these families.',
    },
    evidence_vocabulary_runtime_closed_or_better: {
      count: countWhere((f) =>
        ['RUNTIME_CLOSED', 'CONFORMANCE_GREEN', 'PRODUCTION_PROVEN'].includes(f.world_class_gate.evidence_vocabulary_family_claim),
      ),
      denominator: total,
      definition: 'Families whose FAMILY-WIDE (not narrowest-slice) evidence_vocabulary_family_claim (docs/116 addendum section 1) reaches RUNTIME_CLOSED, CONFORMANCE_GREEN, or PRODUCTION_PROVEN. Expected 0 -- see per-family world_class_gate.evidence_vocabulary_note for narrower slices that individually go further (e.g. the chronica-meta-plane gate itself).',
    },
    runtime_closure_live: {
      count: countWhere((f) => f.runtime_closure === 'LIVE'),
      denominator: total,
      definition: 'Families whose registry runtime_closure is LIVE (shipped caller + business consumer). LIVE_BUT_UNVERIFIED and PARTIAL_RUNTIME are excluded.',
    },
    production_evidence_proven: {
      count: countWhere((f) => f.production_evidence === 'PROVEN'),
      denominator: total,
      definition: 'Families with PEC-shaped production evidence. Expected 0; PEC is separate from FIC and from family Substrate/NativeSlice.',
    },
  }

  const evidenceVocabularyDistribution = {}
  for (const f of families) {
    const label = f.world_class_gate.evidence_vocabulary_family_claim
    evidenceVocabularyDistribution[label] = (evidenceVocabularyDistribution[label] || 0) + 1
  }

  const waveAdmissionReady = ratios.meets_portfolio_minimum.count === total

  return {
    schema_version: 1,
    generated_at: null, // stamped by main() / by the caller
    truth_label: 'LOCAL_AUDIT_REQUIRED',
    authority:
      'crate-local cloud shard + hand-curated doctrine registry + docs/_machine/world-class-benchmark-portfolio.jsonl + docs/_machine/reference-repo-grounding-ledger.jsonl (read-only) only; root docs/capabilities.db and docs/architecture.db remain the local aggregate audit targets for final capability/architecture truth per docs/doctrines/023-five-dimension-cloud-shard-contract.md',
    world_class_benchmark_gate_doc: 'docs/benchmarks/116-operating-world-class-benchmark-gates.md',
    wave_admission_ready: waveAdmissionReady,
    wave_admission_ready_reason: waveAdmissionReady
      ? 'All families meet their docs/116 section 5 portfolio minimum.'
      : `${total - ratios.meets_portfolio_minimum.count}/${total} families do not yet meet their docs/116 section 5 portfolio minimum. Pins may exist; class counts (impl/lineage/standard/baseline/adversarial) are still short. Per docs/116 section 7 this tool never self-certifies WAVE_ADMISSION_READY while any family gap remains.`,
    evidence_vocabulary_distribution: evidenceVocabularyDistribution,
    ratios,
    families,
    grounding_ledger_entry_count: groundingLedgerEntries.length,
    portfolio_entry_count: portfolioEntries.length,
    non_claims: [
      'No family in this registry is claimed ExternalClass or ProductionReady.',
      'No family in this registry is claimed RUNTIME_CLOSED, CONFORMANCE_GREEN, or PRODUCTION_PROVEN at the family-wide level (docs/116 addendum section 1).',
      'has_shard_evidence (per-family, in the families[] array) reflects chronica-platform-plane\'s own claim-gate shard only, never the family\'s implementation crate shard.',
      'This tool does not read or require docs/capabilities.db or docs/architecture.db.',
      'This tool reads docs/_machine/world-class-benchmark-portfolio.jsonl and docs/_machine/reference-repo-grounding-ledger.jsonl but never writes to them.',
      'This branch does not self-certify WAVE_ADMISSION_READY or MAIN_MERGE_READY per docs/116 sections 5 and 7.',
    ],
  }
}

function printSummary(result) {
  console.log(`benchmark-ratio: total_families=${result.ratios.total_families}`)
  for (const [key, val] of Object.entries(result.ratios)) {
    if (key === 'total_families') continue
    const pct = ((val.count / val.denominator) * 100).toFixed(1)
    console.log(`  ${key}: ${val.count}/${val.denominator} (${pct}%)`)
  }
}

function main() {
  const args = process.argv.slice(2)
  const cmd = args[0] || 'summary'
  const result = computeBenchmarkRatio()
  result.generated_at = new Date().toISOString()

  if (cmd === 'summary') {
    printSummary(result)
  } else if (cmd === 'full') {
    console.log(JSON.stringify(result, null, 2))
  } else if (cmd === 'family') {
    const key = args[1]
    const family = result.families.find((f) => f.family_key === key)
    if (!family) {
      console.error(`unknown family_key: ${key}`)
      process.exitCode = 1
      return
    }
    console.log(JSON.stringify(family, null, 2))
  } else {
    console.error('usage: query-benchmark-ratio.mjs [summary|full|family <key>]')
    process.exitCode = 1
  }
}

// Compare filesystem paths, not URL strings: on Windows, import.meta.url is
// `file:///C:/...` while process.argv[1] is the native `C:\...` path, so a
// naive `file://${process.argv[1]}` comparison is always false there and the
// CLI silently no-ops (found by Windows/PowerShell controller audit on PR
// #2153). path.resolve()+fileURLToPath() matches the pattern already used by
// tools/capabilities/audit-truth.mjs and tools/reconcile/validate-*.mjs.
if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  main()
}
