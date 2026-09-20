import { existsSync } from 'node:fs'
import { join } from 'node:path'
import { openReadOnlyDatabase } from '../db/sqlite-open.mjs'

function sqlString(value) {
  return value == null ? null : String(value)
}

//
// capabilities.db tags each canonical capability with a `target_crate`. Some match a LIVE workspace
// crate directly; the rest are LOGICAL crate names (a domain homed INSIDE a live crate), non-Rust /
// legacy donor names, coherent-but-unbuilt domains, or missing metadata. This classifier assigns
// EVERY capability exactly one honest coverage state — it NEVER fakes a missing crate as
// implemented. `LOGICAL_CRATE_HOME` entries are code-verified logical→live homes (each confirmed by
// a real module path under the live crate); an uncurated logical name defaults to the SAFE
// `planned_logical_domain`, never a fabricated mapping.

export const COVERAGE_STATES = [
  'linked_to_live_crate',
  'linked_to_live_module',
  'mapped_to_existing_architecture_node',
  'planned_logical_domain',
  'legacy_or_non_rust_target',
  'missing_target_crate',
  'invalid_target',
  'missing_target_metadata',
]

// Logical (non-workspace) crate name -> the LIVE crate that owns that domain in code.
const LOGICAL_CRATE_HOME = {
  // chronica-erp — ERP / business-operations domains
  // finance→edi/+accounting/ (37/42 erp.* caps, 26 verified); algo→accounting/market_data.rs (code-verified test homes)
  'chronica-finance': 'chronica-erp', 'chronica-algo': 'chronica-erp',
  'chronica-accounting': 'chronica-erp', 'chronica-inventory': 'chronica-erp',
  'chronica-crm': 'chronica-erp', 'chronica-inbox': 'chronica-erp', 'chronica-hr': 'chronica-erp',
  'chronica-sales': 'chronica-erp', 'chronica-selling': 'chronica-erp',
  'chronica-procurement': 'chronica-erp', 'chronica-buying': 'chronica-erp',
  'chronica-products': 'chronica-erp', 'chronica-supply-chain': 'chronica-erp',
  'chronica-shipping': 'chronica-erp', 'chronica-calendar': 'chronica-erp',
  'chronica-contacts': 'chronica-erp', 'chronica-leads': 'chronica-erp',
  'chronica-gamification': 'chronica-erp', 'chronica-business-apps': 'chronica-erp',
  // chronica-security — auth / crypto / vault / organization
  'chronica-vault': 'chronica-security', 'chronica-auth': 'chronica-security',
  'chronica-organization': 'chronica-security', 'chronica-organizations': 'chronica-security',
  'chronica-crypto': 'chronica-security', 'chronica-tls': 'chronica-security',
  'chronica-webhook': 'chronica-security', 'chronica-api-key': 'chronica-security',
  'chronica-oauth': 'chronica-security', 'chronica-access': 'chronica-security',
  'chronica-sessions': 'chronica-security', 'chronica-secrets': 'chronica-security',
  'chronica-account': 'chronica-security', 'chronica-attest': 'chronica-security',
  'chronica-licensing': 'chronica-security', 'chronica-region': 'chronica-security',
  // chronica-policy — governance / billing / compliance / config
  'chronica-governance': 'chronica-policy', 'chronica-billing': 'chronica-policy',
  'chronica-payments': 'chronica-policy', 'chronica-compliance': 'chronica-policy',
  'chronica-user': 'chronica-policy', 'chronica-user-preferences': 'chronica-policy',
  'chronica-admin': 'chronica-policy', 'chronica-administration': 'chronica-policy',
  'chronica-quality': 'chronica-policy', 'chronica-legal': 'chronica-policy',
  'chronica-config': 'chronica-policy', 'chronica-configuration': 'chronica-policy',
  'chronica-customization': 'chronica-policy',
  // chronica-integrations — external channels / connectors
  // 'chronica-integration' is a singular name-variant of the live crate itself (same precedent as
  // organization/organizations); its caps are connector-shaped (registry/web-fetch/file-transfer/URI).
  'chronica-integration': 'chronica-integrations',
  'chronica-communications': 'chronica-integrations', 'chronica-communication': 'chronica-integrations',
  'chronica-notifications': 'chronica-integrations', 'chronica-channels': 'chronica-integrations',
  'chronica-messaging': 'chronica-integrations', 'chronica-mcp': 'chronica-integrations',
  'chronica-mcp-adapter': 'chronica-integrations', 'chronica-data-connectors': 'chronica-integrations',
  'chronica-gmail': 'chronica-integrations',
  // chronica-workflows — graph / automation / app / job-queue
  'chronica-operations': 'chronica-workflows', // operational workflows → sop.rs (code-verified test home)
  'chronica-graphs': 'chronica-workflows', 'chronica-collaboration': 'chronica-workflows',
  'chronica-collab': 'chronica-workflows', 'chronica-automation': 'chronica-workflows',
  'chronica-orchestration': 'chronica-workflows', 'chronica-coordination': 'chronica-workflows',
  'chronica-job-queue': 'chronica-workflows', 'chronica-message-queue': 'chronica-workflows',
  'chronica-skills': 'chronica-workflows', 'chronica-applications': 'chronica-workflows',
  'chronica-functions': 'chronica-workflows', 'chronica-scripting': 'chronica-workflows',
  'chronica-async': 'chronica-workflows', 'chronica-realtime': 'chronica-workflows',
  'chronica-io': 'chronica-workflows', 'chronica-binary-management': 'chronica-workflows',
  'chronica-static-data': 'chronica-workflows', 'chronica-credentials': 'chronica-workflows',
  'chronica-recovery': 'chronica-workflows', 'chronica-runner': 'chronica-workflows',
  // chronica-runtime — llm / sandbox / agent runtime
  'chronica-agent-runtime': 'chronica-runtime', 'chronica-agent-protocol': 'chronica-runtime',
  'chronica-agent-coordination': 'chronica-runtime', 'chronica-llm': 'chronica-runtime',
  'chronica-llm-providers': 'chronica-runtime', 'chronica-llm-integration': 'chronica-runtime',
  'chronica-ai-models': 'chronica-runtime', 'chronica-ai': 'chronica-runtime',
  'chronica-ml': 'chronica-runtime', 'chronica-sampling': 'chronica-runtime',
  'chronica-sandbox': 'chronica-runtime', 'chronica-sandbox-filesystem': 'chronica-runtime',
  'chronica-sandbox-preview': 'chronica-runtime', 'chronica-sandbox-ssh': 'chronica-runtime',
  'chronica-exitcode': 'chronica-runtime', 'chronica-barcode': 'chronica-runtime',
  // chronica-tools — agent tools
  'chronica-agents': 'chronica-tools', 'chronica-image-decoder': 'chronica-tools',
  // chronica-memory — knowledge / rag / search / storage / content
  'chronica-agent-memory': 'chronica-memory', 'chronica-agent-knowledge': 'chronica-memory',
  'chronica-knowledge': 'chronica-memory', 'chronica-knowledge-graph': 'chronica-memory',
  'chronica-rag': 'chronica-memory', 'chronica-embeddings': 'chronica-memory',
  'chronica-embedding': 'chronica-memory', 'chronica-content': 'chronica-memory',
  'chronica-content-generation': 'chronica-memory', 'chronica-generation': 'chronica-memory',
  'chronica-content-management': 'chronica-memory', 'chronica-documents': 'chronica-memory',
  'chronica-search': 'chronica-memory', 'chronica-storage': 'chronica-memory',
  'chronica-distributed-storage': 'chronica-memory', 'chronica-code-extraction': 'chronica-memory',
  'chronica-regional-prompts': 'chronica-memory', 'chronica-pdf-export': 'chronica-memory',
  // chronica-temporal-execution/-persistence, chronica-execution, chronica-infra-execution,
  // chronica-history, chronica-snapshot: previously homed in chronica-execution-state, which was
  // removed (FULL_CRATE_AUDIT_FAIL, zero reverse dependents, no real product requirement; see
  // docs/design/lanes/a0-burst1/chronica-execution-state.md and issue #550). chronica-workflows::durable
  // is the canonical durable-execution owner but does not implement history-branching, child-workflow,
  // or shard-fencing semantics, so these logical names have no code-verified live home and correctly
  // fall through to planned_logical_domain rather than a fabricated mapping.
  // chronica-analytics — BI / dashboards / experiments / mbql
  'chronica-mbql': 'chronica-analytics', 'chronica-dashboards': 'chronica-analytics',
  'chronica-reporting': 'chronica-analytics', 'chronica-reports': 'chronica-analytics',
  'chronica-visualization': 'chronica-analytics', 'chronica-experiments': 'chronica-analytics',
  'chronica-experimentation': 'chronica-analytics', 'chronica-feature-flags': 'chronica-analytics',
  'chronica-feature-management': 'chronica-analytics', 'chronica-cache': 'chronica-analytics',
  'chronica-caching': 'chronica-analytics', 'chronica-alerting': 'chronica-analytics',
  'chronica-cohort': 'chronica-analytics', 'chronica-product-experience': 'chronica-analytics',
  'chronica-product-intent': 'chronica-analytics',
  // chronica-api — orm / database / schema
  'chronica-orm': 'chronica-api', 'chronica-database': 'chronica-api', 'chronica-data': 'chronica-api',
  'chronica-data-persistence': 'chronica-api', 'chronica-data-management': 'chronica-api',
  'chronica-db': 'chronica-api', 'chronica-schema': 'chronica-api', 'chronica-metadata': 'chronica-api',
  'chronica-data-model': 'chronica-api', 'chronica-sql-tools': 'chronica-api',
  // chronica-olap — columnar query engine
  'chronica-data-warehouse': 'chronica-olap', 'chronica-aggregate-functions': 'chronica-olap',
  'chronica-query-execution': 'chronica-olap', 'chronica-query-parsing': 'chronica-olap',
  'chronica-queries': 'chronica-olap', 'chronica-query-cache': 'chronica-olap',
  'chronica-database-catalog': 'chronica-olap', 'chronica-data-formats': 'chronica-olap',
  'chronica-encoding': 'chronica-olap', 'chronica-text-encoding': 'chronica-olap',
  // storage-engine caps are ClickHouse MergeTree (mutations/partitions/TTL) — code-verified home
  // crates/chronica-olap/src/mergetree/ (merge.rs/prune.rs/ttl.rs/projection.rs)
  'chronica-storage-engine': 'chronica-olap',
  // chronica-observability — audit / telemetry / metrics
  'chronica-audit': 'chronica-observability', 'chronica-error-tracking': 'chronica-observability',
  'chronica-telemetry': 'chronica-observability', 'chronica-metrics': 'chronica-observability',
  'chronica-monitoring': 'chronica-observability', 'chronica-llm-analytics': 'chronica-observability',
  'chronica-surveys': 'chronica-observability', 'chronica-session-recordings': 'chronica-observability',
  'chronica-feedback': 'chronica-observability', 'chronica-usage': 'chronica-observability',
  'chronica-coverage': 'chronica-observability',
  // chronica-traces — distributed tracing
  'chronica-tracing': 'chronica-traces', 'chronica-streaming': 'chronica-traces', 'chronica-sse': 'chronica-traces',
  // tempo backend/generator → obs.traces caps homed in forward.rs / metrics.rs (code-verified test homes)
  'chronica-tempo-backend': 'chronica-traces', 'chronica-tempo-generator': 'chronica-traces',
  // tempo trace-path domains with real chronica-traces modules: querier → src/query.rs,
  // distributor + pusher (span ingest path) → src/ingest.rs, metrics (PromQL/metrics-generator)
  // → src/metrics.rs. Remaining tempo-* (frontend/backend-worker/info/livestore/overrides/stats)
  // have NO owning module yet and stay planned via the /^chronica-tempo-/ pattern rule.
  'chronica-tempo-querier': 'chronica-traces', 'chronica-tempo-distributor': 'chronica-traces',
  'chronica-tempo-pusher': 'chronica-traces', 'chronica-tempo-metrics': 'chronica-traces',
  // chronica-events — person / identity / ingest
  'chronica-persons': 'chronica-events', 'chronica-people': 'chronica-events',
  'chronica-identity': 'chronica-events', 'chronica-ingest': 'chronica-events',
  'chronica-ingestion': 'chronica-events', 'chronica-data-ingestion': 'chronica-events',
  'chronica-quota': 'chronica-events',
  // chronica-world-model / evidence-debate / strategy / company / simulation / commerce / k8s / internet-hand
  'chronica-prediction': 'chronica-world-model', 'chronica-reasoning': 'chronica-world-model',
  'chronica-evaluation': 'chronica-evidence-debate',
  'chronica-trading': 'chronica-strategy', 'chronica-markets': 'chronica-strategy',
  'chronica-portfolio': 'chronica-strategy', 'chronica-economics': 'chronica-strategy',
  'chronica-projects': 'chronica-company', 'chronica-collections': 'chronica-company',
  'chronica-workspace': 'chronica-company', 'chronica-equity': 'chronica-company',
  'chronica-social-simulation': 'chronica-simulation', 'chronica-social': 'chronica-simulation',
  'chronica-community': 'chronica-simulation', 'chronica-progress-streaming': 'chronica-simulation',
  'chronica-storefronts': 'chronica-commerce', 'chronica-ecommerce': 'chronica-commerce',
  'chronica-container': 'chronica-kubernetes', 'chronica-browser-control': 'chronica-internet-hand',
  'chronica-media-capture': 'chronica-media',
  'chronica-reliability': 'chronica-engine',
  // chronica-core — cross-cutting primitives
  'chronica-system': 'chronica-core', 'chronica-utilities': 'chronica-core', 'chronica-migrations': 'chronica-core',
  // chronica-osint — recon / intel / extraction / geo
  'chronica-network-reconnaissance': 'chronica-osint', 'chronica-web-reconnaissance': 'chronica-osint',
  'chronica-cloud-reconnaissance': 'chronica-osint', 'chronica-threat-intelligence': 'chronica-osint',
  'chronica-data-extraction': 'chronica-osint', 'chronica-extraction': 'chronica-osint',
  'chronica-news': 'chronica-osint', 'chronica-network': 'chronica-osint',
  'chronica-networking': 'chronica-osint', 'chronica-domains': 'chronica-osint',
  'chronica-geolocation': 'chronica-osint', 'chronica-discovery': 'chronica-osint',
  'chronica-intelligence': 'chronica-osint', 'chronica-analysis': 'chronica-osint',
  'chronica-export': 'chronica-osint', 'chronica-validation': 'chronica-osint',
  'chronica-filter': 'chronica-osint', 'chronica-localization': 'chronica-osint',
}

// Non-Rust / legacy / external donor target names (never a chronica live or logical home).
const LEGACY_TARGET_RE =
  /^(n\/?a$|n\/?a\b|n\/?a[^a-z]|not applicable|.*\bpython\b|tradingagents$|openfang-|alertmanager-|chronica-python$|.*\b(typescript|electron|browser extension)\b)/i

// Planned domains stay gaps, but should still have a live crate owner when the repo has
// an obvious backend home. This is an execution-owner hint, not an implementation claim.
const PLANNED_OWNER_EXACT = {
  // chronica-inbox is the Chatwoot / Meta-Business-Suite helpdesk logical domain (manage_inbox,
  // contacts, labels, canned, csat, …). DECISION 2026-06-18 (docs/design/helpdesk-event-ai-db-
  // architecture.md): consolidate the FORKED implementation — chronica-erp/src/inbox/* (pure, inert) +
  // chronica-approvals::support_* (live stack) — into the dedicated event-driven `chronica-helpdesk`
  // crate (hexagonal domain core + transactional outbox + CQRS read-model + sync projection). The crate
  // is BUILT + tested but INERT (the chronica-api helpdesk routes are not yet cut over to it), so these
  // caps stay a PLANNED gap whose intended live owner is chronica-helpdesk — an execution-owner hint,
  // NOT a served/linked claim. The target_crate re-point + erp/inbox deletion happen AT the route cutover.
  'chronica-inbox': 'chronica-helpdesk',
  'chronica-platform': 'chronica-engine',
  // chronica-enterprise caps are erp.company.manage / erp.user.manage — ERP entity admin, not engine
  'chronica-enterprise': 'chronica-erp',
  'chronica-human-behavior': 'chronica-engine',
  'chronica-infrastructure': 'chronica-kubernetes',
  'chronica-provisioning': 'chronica-kubernetes',
  'chronica-devops': 'chronica-kubernetes',
  'chronica-deployments': 'chronica-kubernetes',
  'chronica-distribution': 'chronica-kubernetes',
  'chronica-gcp': 'chronica-kubernetes',
  'chronica-hardware': 'chronica-kubernetes',
  // chronica-volume caps are storage.volume.create/delete/list — container storage volumes,
  // NOT audio volume (the bare 'volume' word otherwise falls into the media pattern rule below)
  'chronica-volume': 'chronica-kubernetes',
  'chronica-plugin-system': 'chronica-tools',
  'chronica-plugin': 'chronica-tools',
  'chronica-extensions': 'chronica-tools',
  'chronica-development': 'chronica-tools',
  'chronica-ui': 'chronica-api',
  'chronica-agent-ui': 'chronica-api',
  'chronica-web': 'chronica-api',
  'chronica-frontend': 'chronica-api',
  'chronica-client': 'chronica-api',
  'chronica-server': 'chronica-api',
  'chronica-api-request': 'chronica-api',
  'chronica-accessibility': 'chronica-api',
  'chronica-android': 'chronica-api',
  'chronica-desktop': 'chronica-api',
  'chronica-grpc-server': 'chronica-api',
  'chronica-fetch-api': 'chronica-api',
  'chronica-collaboration-server': 'chronica-events',
  'chronica-collaboration-store': 'chronica-events',
  'chronica-collaboration-call': 'chronica-events',
  'chronica-collaboration-media': 'chronica-events',
  'chronica-collaboration-ui': 'chronica-events',
  'chronica-rpc-peer': 'chronica-events',
  'chronica-rpc-protocol': 'chronica-events',
  'chronica-sync': 'chronica-replication',
  'chronica-federation': 'chronica-replication',
  // chronica-observers is obs.browser.manage_dom_observers — browser DOM observers, not telemetry
  'chronica-observers': 'chronica-internet-hand',
  'chronica-signals': 'chronica-observability',
  'chronica-assets': 'chronica-artifacts',
  'chronica-data-export': 'chronica-artifacts',
  'chronica-file-analysis': 'chronica-artifacts',
  'chronica-file-chooser': 'chronica-artifacts',
  'chronica-filesystem': 'chronica-artifacts',
  'chronica-preview': 'chronica-artifacts',
  'chronica-sbom': 'chronica-security',
  'chronica-sbom-consume': 'chronica-security',
  'chronica-vuln-lib': 'chronica-security',
  'chronica-authconfig': 'chronica-security',
  'chronica-grant-permissions': 'chronica-security',
  'chronica-market': 'chronica-marketplaces',
  'chronica-playstore': 'chronica-marketplaces',
  'chronica-article-management': 'chronica-commerce',
  'chronica-site-management': 'chronica-commerce',
  'chronica-assetlinks': 'chronica-commerce',
  'chronica-climate': 'chronica-world-model',
  'chronica-aviation': 'chronica-world-model',
  'chronica-route-network': 'chronica-world-model',
  'chronica-maps': 'chronica-world-model',
  'chronica-playgames': 'chronica-simulation',
  // editor-automation caps are agent file ops (edit/manage-fs/search code) — the file-ops sibling
  // home is chronica-artifacts (file_match.rs owns file.search; write/edit deferred live-edge)
  'chronica-editor-automation': 'chronica-artifacts',
  'chronica-ops': 'chronica-workflows',
  'chronica-setup': 'chronica-cli',
  'chronica-init-script': 'chronica-cli',
  // chronica-clock is browser.clock_manipulate — Playwright-style browser clock control
  'chronica-clock': 'chronica-internet-hand',
  'chronica-url': 'chronica-core',
  // chronica-dictionary-engine is db.manage_dictionary — ClickHouse external dictionaries
  'chronica-dictionary-engine': 'chronica-olap',
  // chronica-memory-mgmt is agent-knowledge.manage_gpu_memory — VRAM/model unload (model runtime)
  'chronica-memory-mgmt': 'chronica-runtime',
  'chronica-wasm': 'chronica-runtime',
  'chronica-frameworks': 'chronica-runtime',
  'chronica-cloud-client': 'chronica-integrations',
  'chronica-drive': 'chronica-integrations',
  'chronica-firebase': 'chronica-integrations',
  'chronica-http-proxy': 'chronica-integrations',
  'chronica-drivers': 'chronica-integrations',
  'chronica-extensions-ui': 'chronica-tools',
  'chronica-vcs': 'chronica-tools',
  'chronica-vcs-integration': 'chronica-tools',
  'chronica-vex-repo': 'chronica-tools',
  'chronica-aria-snapshot': 'chronica-internet-hand',
  'chronica-console-messages': 'chronica-internet-hand',
  'chronica-geoip-detection': 'chronica-internet-hand',
  'chronica-hooks': 'chronica-internet-hand',
  'chronica-interruption': 'chronica-internet-hand',
  'chronica-viewport-size': 'chronica-internet-hand',
  'chronica-views': 'chronica-internet-hand',
  'chronica-mathml': 'chronica-media',
  'chronica-conditioning-ops': 'chronica-media',
  'chronica-gligen': 'chronica-media',
  'chronica-moderation': 'chronica-policy',
  // graph-* caps here are agent-knowledge KNOWLEDGE-graph verbs (build-from-extractions/traverse/
  // analyze) — the knowledge-graph logical home is chronica-memory, not BI analytics
  'chronica-graph-analysis': 'chronica-memory',
  'chronica-graph-assembly': 'chronica-memory',
  'chronica-graph-querying': 'chronica-memory',
}

function plannedOwnerFor(targetCrate) {
  if (PLANNED_OWNER_EXACT[targetCrate]) return PLANNED_OWNER_EXACT[targetCrate]
  if (/^chronica-tempo-/.test(targetCrate)) return 'chronica-traces'
  if (/^chronica-(browser|page|element)-/.test(targetCrate)) return 'chronica-internet-hand'
  if (/^chronica-(add|clear|get)-cookies$/.test(targetCrate)) return 'chronica-internet-hand'
  if (/^chronica-set-/.test(targetCrate)) return 'chronica-internet-hand'
  if (/^chronica-(wait-element|close-browser|close-page|bring-to-front|bounding-box|dispatch-event|expose-binding|emulate-media|frame-navigation|forms|focus-blur|handle-dialog|handle-popups|history-nav|highlight|keyboard|locator-composition|locator-handler|mouse|navigation-events|network-capture|pause|scroll-view)$/.test(targetCrate)) {
    return 'chronica-internet-hand'
  }
  if (/^chronica-(image|model|vae)-/.test(targetCrate)) return 'chronica-media'
  if (/^chronica-(3d-generation|animations|canvas|clip-vision|controlnet|css-engine|graphics|imagery|inpainting|layout|lora|material-library|multi-gpu|quantization|style-transfer|svg|transforms|unclip|upscaling|video|vision|voice|volume|web-rendering|webaudio|webgl)$/.test(targetCrate)) {
    return 'chronica-media'
  }
  if (/^chronica-graph-/.test(targetCrate)) return 'chronica-analytics'
  if (targetCrate === 'chronica-community-detection') return 'chronica-analytics'
  if (/^chronica-(websocket-events|worker-events)$/.test(targetCrate)) return 'chronica-events'
  return null
}

// ── gap status labeling ────────────────────────────────────────────────────────────────────────
// architecture.db is wiped and rebuilt on every arch:build, so a gap's deferral label cannot live
// in architecture_status_override (that table never survives the rebuild). Labels are therefore
// DERIVED deterministically from the gap state + target name here. A labeled deferral is still an
// unresolved architecture gap — it is just explicitly classified instead of silently 'open', and
// it can NEVER be 'implemented'/'verified' (arch:verify rejects that).
//   deferred:python_bound        — donor target is Python-bound (incl. tradingagents, a Python framework)
//   deferred:external_donor_crate — named external donor crate not in this workspace (openfang-*/alertmanager-*)
//   deferred:non_rust_external   — N/A / Electron / browser-extension TypeScript pseudo-targets
//   deferred:planned_unbuilt     — coherent planned chronica-* domain with no owning module in code yet
//                                  (intended live owner recorded in planned_architecture_node)
//   open                         — actionable metadata bug (missing/invalid target) awaiting backfill
export function gapStatusFor(state, targetCrate) {
  const tc = targetCrate == null ? '' : String(targetCrate)
  if (state === 'legacy_or_non_rust_target') {
    if (/python|tradingagents/i.test(tc)) return 'deferred:python_bound'
    if (/^(openfang-|alertmanager-)/i.test(tc)) return 'deferred:external_donor_crate'
    return 'deferred:non_rust_external'
  }
  if (state === 'planned_logical_domain') return 'deferred:planned_unbuilt'
  return 'open'
}

/**
 * Classify a canonical capability's architecture coverage into exactly one honest state.
 * `liveCrates` is a Set of live workspace crate names. Module-level upgrade
 * (`linked_to_live_module`) is applied by the caller when target_module matches a live module node.
 */
export function classifyCapabilityCoverage(targetCrate, liveCrates) {
  const tc = targetCrate == null ? null : String(targetCrate).trim()
  if (tc == null || tc === '') {
    return { state: 'missing_target_metadata', suggestedNode: null, reason: 'no target_crate on the canonical capability' }
  }
  // Multi-crate target (comma-separated): the deterministic primary owner is the FIRST listed
  // live crate. Honest — it links to a real live crate; secondary owners are noted in the reason.
  if (tc.includes(',')) {
    const parts = tc.split(',').map((s) => s.trim()).filter(Boolean)
    const primary = parts.find((p) => liveCrates.has(p))
    if (primary) {
      const others = parts.filter((p) => p !== primary).join(', ') || 'none'
      return { state: 'linked_to_live_crate', suggestedNode: `crate:${primary}`, reason: `multi-crate target; primary live owner '${primary}' (also: ${others})` }
    }
    const mappedPart = parts.find((p) => LOGICAL_CRATE_HOME[p] && liveCrates.has(LOGICAL_CRATE_HOME[p]))
    if (mappedPart) {
      return { state: 'mapped_to_existing_architecture_node', suggestedNode: `crate:${LOGICAL_CRATE_HOME[mappedPart]}`, reason: `multi-crate target; primary logical owner '${mappedPart}' homed in '${LOGICAL_CRATE_HOME[mappedPart]}'` }
    }
  }
  if (liveCrates.has(tc)) {
    return { state: 'linked_to_live_crate', suggestedNode: `crate:${tc}`, reason: 'target_crate is a live workspace crate' }
  }
  if (LEGACY_TARGET_RE.test(tc)) {
    return { state: 'legacy_or_non_rust_target', suggestedNode: null, reason: `non-Rust / legacy donor target: ${tc}` }
  }
  const home = LOGICAL_CRATE_HOME[tc]
  if (home && liveCrates.has(home)) {
    return { state: 'mapped_to_existing_architecture_node', suggestedNode: `crate:${home}`, reason: `logical crate '${tc}' is homed in live crate '${home}'` }
  }
  if (/^chronica-[a-z0-9-]+$/.test(tc)) {
    const plannedOwner = plannedOwnerFor(tc)
    if (plannedOwner && liveCrates.has(plannedOwner)) {
      return {
        state: 'planned_logical_domain',
        suggestedNode: `crate:${plannedOwner}`,
        reason: `coherent logical domain planned for live owner crate '${plannedOwner}': ${tc}`,
      }
    }
    return { state: 'planned_logical_domain', suggestedNode: null, reason: `coherent logical domain with no live home in code: ${tc}` }
  }
  return { state: 'invalid_target', suggestedNode: null, reason: `unrecognized / malformed target_crate: ${tc}` }
}

function hasSqliteTable(database, name) {
  return database.prepare("SELECT count(*) n FROM sqlite_master WHERE type='table' AND name=?").get(name).n > 0
}

function emptyCapabilityCoverage(source = 'unavailable') {
  return {
    links: 0,
    gaps: 0,
    planned: 0,
    canonical: 0,
    distinctLinked: 0,
    distinctGap: 0,
    accounted: 0,
    source,
  }
}

export function openCanonicalCapabilityDb(root) {
  const candidates = [
    {
      path: join(root, 'docs', 'capabilities.db'),
      source: 'docs/capabilities.db',
    },
    {
      path: join(root, 'docs', 'capabilities-cloud', 'cap-core.db'),
      source: 'docs/capabilities-cloud/cap-core.db fallback (LOCAL_AUDIT_REQUIRED)',
    },
  ]
  for (const candidate of candidates) {
    if (!existsSync(candidate.path)) continue
    const database = openReadOnlyDatabase(candidate.path)
    if (hasSqliteTable(database, 'canonical_capability')) {
      return { database, source: candidate.source }
    }
    database.close()
  }
  return null
}

export function importCapabilityLinks(db, root) {
  const canonical = openCanonicalCapabilityDb(root)
  if (!canonical) return emptyCapabilityCoverage()
  const caps = canonical.database
  const columns = caps.prepare('PRAGMA table_info(canonical_capability)').all().map((c) => c.name)
  const nameExpr = columns.includes('canonical_name') ? 'canonical_name' : columns.includes('title') ? 'title' : 'key'
  const moduleExpr = columns.includes('target_module') ? 'target_module' : "NULL AS target_module"
  const crateExpr = columns.includes('target_crate') ? 'target_crate' : "NULL AS target_crate"
  const statusExpr = columns.includes('status') ? 'status' : "NULL AS status"
  const movesExpr = columns.includes('moves_money') ? 'moves_money' : '0 AS moves_money'
  const approvalExpr = columns.includes('requires_approval') ? 'requires_approval' : '0 AS requires_approval'
  const rows = caps.prepare(`SELECT key, ${nameExpr} AS name, ${crateExpr}, ${moduleExpr}, ${statusExpr}, ${movesExpr}, ${approvalExpr} FROM canonical_capability`).all()
  caps.close()

  const liveCrates = new Set(
    db.prepare("SELECT name FROM architecture_node WHERE kind='crate'").all().map((r) => r.name),
  )
  const nodeExists = db.prepare('SELECT count(*) n FROM architecture_node WHERE id=?')
  const insertLink = db.prepare(`INSERT OR IGNORE INTO capability_architecture_link
    (capability_key, architecture_node, relationship, status, target_crate, target_module)
    VALUES (?, ?, ?, ?, ?, ?)`)
  const insertGap = db.prepare(`INSERT OR IGNORE INTO architecture_target_gap
    (capability_key, target_crate, target_module, gap_kind, suggested_architecture_node, reason, status, evidence_ref)
    VALUES (?, ?, ?, ?, ?, ?, ?, ?)`)
  const insertPlanned = db.prepare(`INSERT OR IGNORE INTO planned_architecture_node
    (id, kind, name, logical_domain, intended_owner_crate, status, reason, evidence_ref)
    VALUES (?, ?, ?, ?, ?, ?, ?, ?)`)

  const tx = db.transaction(() => {
    for (const row of rows) {
      const { state, suggestedNode, reason } = classifyCapabilityCoverage(row.target_crate, liveCrates)
      const status = sqlString(row.status)
      const tc = sqlString(row.target_crate)
      const tm = sqlString(row.target_module)
      if (state === 'linked_to_live_crate') {
        const primaryCrate = suggestedNode.replace(/^crate:/, '')
        insertLink.run(row.key, suggestedNode, 'targets', status, tc, tm)
        if (tm) {
          const moduleNode = `module:${primaryCrate}:${String(tm).replace(/\//g, '::')}`
          if (nodeExists.get(moduleNode).n > 0) {
            insertLink.run(row.key, moduleNode, 'implemented_by', status, tc, tm)
          }
        }
      } else if (state === 'mapped_to_existing_architecture_node' && suggestedNode) {
        // Logical crate homed in a live crate — link to the live node WITHOUT claiming the
        // specific capability is implemented (caps.db owns impl/verified status).
        insertLink.run(row.key, suggestedNode, 'mapped_to', status, tc, tm)
      } else {
        // planned_logical_domain | legacy_or_non_rust_target | missing_target_metadata | invalid_target.
        // gap.status = 'open' for actionable metadata bugs, or an explicit 'deferred:*' label —
        // NEVER 'implemented'/'verified', so a gap can never be read as implemented architecture
        // (the caps.db status is NOT copied).
        insertGap.run(row.key, tc, tm, state, suggestedNode, reason, gapStatusFor(state, tc), null)
        if (state === 'planned_logical_domain') {
          const intendedOwner = suggestedNode?.startsWith('crate:')
            ? suggestedNode.replace(/^crate:/, '')
            : null
          insertPlanned.run(`planned:${tc}`, 'logical_domain', tc, tc, intendedOwner, 'planned', reason, null)
        }
      }
      // ── control-plane links: a money cap is `gated_by` gate:money; an approval cap is `governed_by`
      // gate:approval. Attach ONLY to caps that took a PLACEMENT link above (live crate / mapped),
      // never to gap caps — so `bothLinkedAndGap` stays 0 and the honest "no home crate ⇒ no gate
      // claim" rule holds. status mirrors the cap's own status (a planned cap's intended routing). ──
      if (state === 'linked_to_live_crate' || (state === 'mapped_to_existing_architecture_node' && suggestedNode)) {
        if (Number(row.moves_money) === 1) insertLink.run(row.key, 'gate:money', 'gated_by', status, tc, tm)
        if (Number(row.requires_approval) === 1) insertLink.run(row.key, 'gate:approval', 'governed_by', status, tc, tm)
      }
    }
  })
  tx()

  const distinctLinked = db.prepare('SELECT count(DISTINCT capability_key) n FROM capability_architecture_link').get().n
  const distinctGap = db.prepare('SELECT count(DISTINCT capability_key) n FROM architecture_target_gap').get().n
  const accounted = db.prepare(`SELECT count(*) n FROM (
    SELECT capability_key FROM capability_architecture_link
    UNION
    SELECT capability_key FROM architecture_target_gap
  )`).get().n
  return {
    links: db.prepare('SELECT count(*) n FROM capability_architecture_link').get().n,
    gaps: db.prepare('SELECT count(*) n FROM architecture_target_gap').get().n,
    planned: db.prepare('SELECT count(*) n FROM planned_architecture_node').get().n,
    canonical: rows.length,
    distinctLinked,
    distinctGap,
    accounted,
    source: canonical.source,
  }
}

export function insertGeneratedArchitectureEvidence(db) {
  const insert = db.prepare(`INSERT INTO architecture_evidence
    (architecture_node, status, evidence_kind, evidence_ref, test_command, test_result, verified_at, verified_by, blocker_reason)
    VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)`)
  const now = new Date().toISOString()

  const nodes = db.prepare(`SELECT id, kind, file_path, evidence_ref FROM architecture_node`).all()
  for (const node of nodes) {
    const ref = node.file_path || node.evidence_ref
    if (!ref) continue
    insert.run(
      node.id,
      'implemented',
      node.kind === 'module' || node.kind === 'crate' ? 'repo_file' : 'architecture_contract',
      ref,
      null,
      'generated_present',
      null,
      'tools/architecture/build-db.mjs',
      null,
    )
  }

  const invariants = db.prepare(`SELECT key, enforcing_node, verification_command FROM architecture_invariant`).all()
  for (const inv of invariants) {
    insert.run(
      inv.enforcing_node,
      'verified',
      'architecture_invariant',
      `architecture_invariant:${inv.key}`,
      inv.verification_command,
      'verified_by_arch_verify',
      now,
      'tools/architecture/verify.mjs',
      null,
    )
  }

  const verifiedCaps = db.prepare(`SELECT capability_key, architecture_node, status
    FROM capability_architecture_link
    WHERE status='verified'`).all()
  for (const cap of verifiedCaps) {
    insert.run(
      cap.architecture_node,
      'verified',
      'capability_link',
      `capabilities.db:${cap.capability_key}`,
      'pnpm caps:verify-impl',
      'imported_verified_capability',
      now,
      'tools/architecture/build-db.mjs',
      null,
    )
  }
}