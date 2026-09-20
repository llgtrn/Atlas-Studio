export const meta = {
  name: 'opportunity-discovery',
  description: 'Multi-lens Architecture-Intelligence discovery: mine donors + caps.db + architecture.db for high-leverage opportunities',
  phases: [{ title: 'Discover' }],
}

const RO = `READ-ONLY discovery. Tools: Read, Grep, Glob, and \`node tools/capabilities/query.mjs sql "SELECT ..."\` (caps.db) / \`node tools/architecture/query.mjs ...\` (arch.db). Do NOT write any file or DB, do NOT run cargo/build, do NOT run audit-truth.mjs.`

const CONTEXT = `Chronica = an AI-operated holding-company OS: an X/Twitter-like operational feed over ERP truth, agents, approvals, a ONE money gate, Merkle audit, ScopePath tenancy. 74 donor repos under Temporary/ were scouted into caps.db (source_capability 5677 rows; donor_scope has per-donor true_capabilities + disposition port_to_rust|preserve_as_reference_only). canonical_capability=3013 (1189 verified, 1823 unimplemented). architecture.db has nodes/links/gaps + the control-plane (gate:money, gate:approval, audit:merkle, scope:ScopePath). INVARIANTS you must NOT violate in any proposal: exactly one money gate (advisory/cognition NEVER authorizes money), deny-by-default tenancy, VerifiedPrincipal sole authority, secrets are name/presence-only + redacted on egress, preview/dry-run commits nothing. DE-DUP: FIRST read the CURRENT backlog (tools/capabilities/opportunities.json) and the open issues (tools/capabilities/tracking-issues.json or \`node tools/capabilities/query.mjs sql "SELECT id,title FROM tracking_issue WHERE status IN ('open','in_progress')"\`); do NOT propose anything already there — only NET-NEW opportunities, or a materially sharper/deeper angle on an existing one (and say so).`

const SCORING = `Score each 1-5 and GROUND every claim (cite the donor concept / the unimplemented cap key / the arch gap / the current-state file). impact=value if realized; reach=how many flows/companies/domains it touches; leverage=ARCHITECTURAL leverage (foundational / unlocks other work — score this honestly, it dominates ranking); effort=build cost; confidence=certainty it's real + pays off. Return your TOP 3-6 HIGHEST-LEVERAGE opportunities only — quality over quantity, no padding. id = a stable kebab slug prefixed by category (e.g. 'observability-prometheus-alerting-engine').`

const SCHEMA = {
  type: 'object', required: ['opportunities', 'summary'], additionalProperties: false,
  properties: {
    opportunities: { type: 'array', items: { type: 'object',
      required: ['id', 'title', 'category', 'impact', 'reach', 'leverage', 'effort', 'confidence', 'source_repo', 'rationale'], additionalProperties: false,
      properties: {
        id: { type: 'string' }, title: { type: 'string' }, description: { type: 'string' },
        category: { type: 'string', enum: ['capability', 'workflow', 'integration', 'dedup', 'architecture', 'product', 'revenue', 'automation', 'ux', 'observability', 'security', 'testing', 'synergy'] },
        impact: { type: 'integer' }, reach: { type: 'integer' }, leverage: { type: 'integer' }, effort: { type: 'integer' }, confidence: { type: 'integer' },
        source_repo: { type: 'string' }, related_capabilities: { type: 'string' }, related_architecture_nodes: { type: 'string' },
        rationale: { type: 'string', description: 'the grounding evidence — donor concept + cap/gap/file cited' },
      } } },
    summary: { type: 'string' },
  },
}

const LENSES = [
  // ── donor-cluster intelligence: per cluster, what concepts/patterns/architecture are NOT YET adopted ──
  { id: 'donor-erp-finance', prompt: `DONOR INTELLIGENCE — ERP/finance/markets cluster: erpnext-main, odoo-main, frappe-main, FinceptTerminal-main. These are the deepest financial knowledge bases. Read their structure under Temporary/ + query the unbuilt erp-finance caps (\`SELECT key,canonical_name FROM canonical_capability WHERE domain='erp-finance' AND status='unimplemented'\`). Find the HIGH-LEVERAGE financial CONCEPTS/ENGINES not yet adopted (e.g. market-data/valuation engines from FinceptTerminal, the consolidation/multi-entity accounting from Odoo, the framework/doctype runtime from Frappe) that would deepen Chronica's ERP truth + capital-allocation brain.` },
  { id: 'donor-observability', prompt: `DONOR INTELLIGENCE — observability/analytics cluster: prometheus-main, grafana-main, clickhouse, posthog-master, metabase-main, blackbox_exporter. observability-analytics has 238 unimplemented caps — the biggest backlog. Find the high-leverage patterns not adopted: a real metrics/TSDB + alerting/dispatch (prometheus/alertmanager), dashboards (grafana/metabase), product-analytics/funnels/experiments (posthog), columnar OLAP (clickhouse). What would give Chronica a real measurement→decision substrate?` },
  { id: 'donor-ai-agent', prompt: `DONOR INTELLIGENCE — AI/agent cluster: dify, refly-main, openfang-main, zeroclaw-master, OpenHarness-main, n8n-master. agent-knowledge has 199 unimplemented caps. Find high-leverage agent-platform concepts not adopted: RAG/knowledge pipelines (dify/refly), skill/template generation (refly skill-forge), multi-agent orchestration, the visual workflow/automation engine (n8n), agent eval/harness (OpenHarness). Respect: agents never authorize money; tools route through invoke_tool→gate.` },
  { id: 'donor-lowcode-automation', prompt: `DONOR INTELLIGENCE — low-code/builder/automation cluster: tooljet-main, n8n-master, temporal-main, supabase-master. Find high-leverage concepts not adopted: an internal app/admin builder (tooljet), the durable-workflow engine (temporal — note automation-durable-workflow-runtime already exists, go BEYOND it), a BaaS/edge-function + RLS model (supabase, preserve_as_reference_only — adopt the PATTERN not the code). What gives operators/agents a way to build internal tools + automations safely?` },
  { id: 'donor-browser-support-deploy', prompt: `DONOR INTELLIGENCE — browser/support/deploy cluster: playwright-main, ladybird-master, pinchtab-main, chatwoot-main, coolify-4.x, podman-desktop-main. Find high-leverage concepts not adopted: governed browser automation (playwright patterns into chronica-internet-hand), the support/omnichannel depth beyond current helpdesk (chatwoot), self-hosted deploy/orchestration (coolify/podman) for instantiating company stacks. Respect: browser execution is armed-only + gated; F3 (un-wired internet-hand) must be gated before wiring.` },
  { id: 'donor-media-security-osint', prompt: `DONOR INTELLIGENCE — media/security/osint cluster: remotion-main, ComfyUI-master, trivy-main, GEOFlow-main, zed-main, posthog. osint has 148 + media-generation 49 unimplemented. Find high-leverage concepts not adopted: programmatic video/image generation pipelines (remotion/ComfyUI) for content/marketing companies, security/vuln scanning (trivy) as a governance gate, geo/intel flows (GEOFlow). What creates leverage for content-producing or security-conscious companies?` },
  // ── cross-cutting strategic lenses ──
  { id: 'cross-repo-synergy', prompt: `CROSS-REPOSITORY SYNERGY — find COMBINATIONS that yield a capability no single repo gives. Examples to reason about (find better): posthog experiments + temporal durable + the strategy engine = automated experiment-driven capital reallocation; clickhouse OLAP + erpnext ERP + the decision-brief loop = real-KPI-driven governance; refly skill-gen + n8n automation + agent runtime = self-authoring agent workflows. Each opportunity MUST name 2+ donors/subsystems and the new capability the combination unlocks. category=synergy.` },
  { id: 'architecture-target-gap', prompt: `ARCHITECTURE GAP — ask: what architecture would exist if this platform were COMPLETE? Compare target vs current. Examine architecture.db (\`node tools/architecture/query.mjs summary\`, \`gaps\`, \`nodes\`) + the crate tree. Find structural gaps with high leverage: missing runtime consumers, missing read-models, missing event backbone wiring, missing state machines, missing control-plane edges, a missing 'company instantiation' pipeline (template→running company stack), a missing connector/execution-provider layer for external arenas. category=architecture or integration.` },
  { id: 'product-revenue-ux', prompt: `PRODUCT + REVENUE + UX — the feed-first product (X-like operational social network for an AI-run holding). Find high-leverage gaps: product surfaces that would make the feed the true operating console (beyond the known agent-profile/strategy/helpdesk gaps), monetization/revenue surfaces (subscription tiers, usage billing, marketplace take-rate — beyond the known erp-billing suite), and operator UX leverage (command palette, keyboard-first ops, cross-company switching). Tie each to caps/architecture. categories: product|revenue|ux.` },
  { id: 'automation-obs-security-testing', prompt: `ENGINEERING LEVERAGE — automation, observability, security, testing gaps that make the WHOLE system safer/faster to evolve. Examine tracking_issue (\`node tools/capabilities/query.mjs sql "SELECT id,title,area FROM tracking_issue WHERE status IN ('open','in_progress')"\`) for systemic themes. Find high-leverage: a CI policy that runs the db-gated #[ignore]d tests, a money-safety property-test harness, a self-serve observability of the money gate / approvals, a secrets-scanning gate (trivy) in CI, a side_effect_class taxonomy normalizer. categories: testing|observability|security|automation.` },
]

phase('Discover')
const results = await parallel(LENSES.map(l => () => agent(
  `${RO}\n\n${CONTEXT}\n\n${l.prompt}\n\n${SCORING}`,
  { label: l.id, phase: 'Discover', schema: SCHEMA, agentType: 'Explore' },
).then(r => ({ lens: l.id, ...r })).catch(() => null)))

return { lenses: results.filter(Boolean).length, results: results.filter(Boolean) }
