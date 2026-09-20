#!/usr/bin/env node
// One-shot consolidation script: reconstructs the master donor corpus from every real evidence
// source found in the repository + this session's own ingestion history, and writes
// tools/refoundation/donor-corpus.yaml. Run once to build the checklist; the checklist itself
// (not this script) is the ongoing source of truth afterward -- re-running this script recomputes
// the same deterministic union from its tracked source files below and overwrites the YAML, so any
// manual status/needed/absorption edits made directly to donor-corpus.yaml after the initial build
// must be re-applied if this script is ever re-run.
//
// Source provenance (CHRONICA CORPUS MASTER CHECKLIST directive, 2026-09-16):
//   - temporary/manifest.yaml: this session's own 90-donor ingestion (real upstream_sha).
//   - tools/repo/donors.manifest: pre-existing 81-entry reference manifest (predates this session).
//   - tools/refoundation/donor-corpus-round2-source.tsv: the user's own pasted 212-repo expanded
//     donor target list (categorized web2app/CRM/OSINT/PlatformOps/FinOps/Robotics/Industrial/Edge/
//     Fleet/Simulation/Legal/Banking/Trading/Estate/etc.), transcribed verbatim as tracked data so
//     it survives beyond this session's /tmp scratch space.
//   - tools/refoundation/donor-corpus-scattered-refs-source.txt: every github.com URL already
//     present in docs/ and tools/ (Ops census docs, capability census JSON, prior manifests) as of
//     2026-09-16 -- regeneration command is in that section below.
import { readFileSync, writeFileSync } from 'node:fs'
import yaml from 'yaml'

const ROOT = process.cwd()

// ---- family mapping (repo's own category vocabularies -> directive's FAMILY taxonomy) ----
const FAMILY_MAP = {
  data_graph_search: 'DURABLE_STATE',
  workflow_events: 'EXECUTION',
  security_authority: 'AUTHORITY',
  platform_deploy: 'PLATFORM_ENGINEERING',
  observability: 'OBSERVABILITY',
  observability2: 'OBSERVABILITY',
  ai_agents: 'AI_RUNTIME',
  web2app_browser: 'WEB2APP',
  machines_robotics: 'ROBOTICS',
  customerops_crm: 'CUSTOMER',
  ai_memory_rag: 'MEMORY',
  ai_model_serving: 'MODEL_SERVING',
  ai_workforce_agent: 'AI_RUNTIME',
  bankingops: 'BANKING',
  bnbops: 'HOSPITALITY',
  bnbops_forecast: 'HOSPITALITY',
  building_home: 'BUILDING',
  data_analytics: 'DURABLE_STATE',
  document_ocr: 'DOCUMENT',
  ecops_commerce: 'COMMERCE',
  ecops_experiment: 'COMMERCE',
  ecops_pim: 'COMMERCE',
  edgeops: 'EDGE',
  embedded: 'EDGE',
  estateops_geo: 'ESTATE',
  finops_ledger: 'FINANCE',
  fleetops: 'FLEET',
  geo_routing: 'LOGISTICS',
  guest_engagement: 'HOSPITALITY',
  identity_supplychain: 'IDENTITY',
  industrialops: 'INDUSTRIAL',
  legalops: 'LEGAL',
  legalops_caselaw: 'LEGAL',
  metering_billing: 'FINANCE',
  network_gateway: 'NETWORK',
  osintops_search: 'OSINT',
  platformops_cloud: 'CLOUD',
  platformops_engineering: 'PLATFORM_ENGINEERING',
  platformops_serverless: 'SERVERLESS',
  roboticsops_comm: 'ROBOTICS',
  roboticsops_slam: 'ROBOTICS',
  sandbox_wasm: 'CONTAINER',
  security_runtime: 'SECURITY',
  simulationops: 'SIMULATION',
  streaming: 'EVENTING',
  tradeops_logistics: 'TRADE',
  tradingops: 'TRADING',
}

function canon(ownerRepo) {
  return ownerRepo.toLowerCase().replace(/\.git$/, '')
}

const donors = new Map() // canon -> record

function upsert(ownerRepo, patch, sourceTag) {
  const key = canon(ownerRepo)
  if (!key.includes('/') || key.split('/').length !== 2) return null
  const [owner, repo] = key.split('/')
  if (owner === 'llgtrn' && repo === 'chronica') return null // this repo itself
  if (['acme', 'example'].includes(owner)) return null // placeholder/fixture refs, not real donors
  let rec = donors.get(key)
  if (!rec) {
    rec = {
      repo: ownerRepo,
      canonical_upstream: `https://github.com/${ownerRepo.replace(/\.git$/, '')}`,
      family: [],
      domain: null,
      source_origin: [],
      status: 'DISCOVERED',
      needed: 'UNKNOWN',
      why_needed: null,
      chronica_targets: [],
      source_present: false,
      census_complete: false,
      absorption_state: 'none',
      absorbed_into: [],
      evidence_paths: [],
      temporary_path: null,
      can_delete_source: false,
      duplicate_of: null,
      notes: null,
    }
    donors.set(key, rec)
  }
  if (sourceTag && !rec.source_origin.includes(sourceTag)) rec.source_origin.push(sourceTag)
  Object.assign(rec, patch)
  return rec
}

// ---- 1. temporary/manifest.yaml: already-ingested donors, real provenance ----
const manifest = yaml.parse(readFileSync(`${ROOT}/temporary/manifest.yaml`, 'utf8'))
for (const d of manifest.donors) {
  const m = /github\.com\/([^/]+\/[^/]+?)(?:\.git)?$/.exec(d.upstream_url)
  if (!m) continue
  const family = FAMILY_MAP[d.category] || 'OTHER'
  upsert(
    m[1],
    {
      family: [family],
      status: 'SOURCE_PRESENT',
      source_present: true,
      temporary_path: d.source_path,
      evidence_paths: [d.source_path, 'temporary/manifest.yaml'],
      notes: `Ingested 2026-09-16 hard-refoundation donor round; upstream_sha=${d.upstream_sha}, branch=${d.upstream_branch_or_tag}.`,
    },
    'temporary/manifest.yaml',
  )
}

// ---- 2. tools/repo/donors.manifest: pre-existing 81-donor reference list (resolved rows only) ----
const donorsManifestText = readFileSync(`${ROOT}/tools/repo/donors.manifest`, 'utf8')
for (const line of donorsManifestText.split('\n')) {
  const trimmed = line.trim()
  if (!trimmed || trimmed.startsWith('#')) continue
  const parts = trimmed.split(/\s+/)
  if (parts.length < 2) continue
  const url = parts[1]
  const m = /github\.com\/([^/]+\/[^/]+?)(?:\.git)?$/.exec(url)
  if (!m) continue
  const rec = upsert(m[1], {}, 'tools/repo/donors.manifest')
  if (rec && rec.family.length === 0) rec.family = ['OTHER']
}

// ---- 3. round-2 expanded target list (user-provided, ~212 rows, category/slug/repo) ----
const round2Text = readFileSync(`${ROOT}/tools/refoundation/donor-corpus-round2-source.tsv`, 'utf8')
for (const line of round2Text.split('\n')) {
  if (!line.trim()) continue
  const [category, slug, repo] = line.split('\t')
  if (!repo) continue
  const family = FAMILY_MAP[category] || 'OTHER'
  const rec = upsert(repo.trim(), {}, 'user-provided round-2 expansion list')
  if (rec && !rec.family.includes(family)) rec.family.push(family)
  if (rec && !rec.domain) rec.domain = category
}

// ---- 4. scattered github.com references already in docs/ + tools/ (excluding temporary/) ----
// Regenerate this input with:
//   grep -rhoE "github\.com/[A-Za-z0-9_.-]+/[A-Za-z0-9_.-]+" docs/ tools/ \
//     --include="*.md" --include="*.json" --include="*.mjs" --include="*.yaml" --include="*.yml" \
//     --include="*.manifest" --include="*.txt" | sed 's#\.git$##' | sort -u
const scatteredRefs = readFileSync(`${ROOT}/tools/refoundation/donor-corpus-scattered-refs-source.txt`, 'utf8')
for (const line of scatteredRefs.split('\n')) {
  const ownerRepo = line.trim().replace(/^github\.com\//, '')
  if (!ownerRepo) continue
  upsert(ownerRepo, {}, 'docs/ or tools/ existing reference')
}

// ---- 5. name-based refinement for confident OTHER->real-family reclassification ----
// Only applied to unambiguous, well-known repos scattered-refs left uncategorized (no round-2
// category tag) -- conservative, name-anchored, not a blanket guess.
const NAME_FAMILY_HINTS = [
  [/^laramies\/theharvester$/, 'OSINT'],
  [/^megadose\/holehe$/, 'OSINT'],
  [/^mxrch\/ghunt$/, 'OSINT'],
  [/^sherlock-project\/sherlock$/, 'OSINT'],
  [/^smicallef\/spiderfoot$/, 'OSINT'],
  [/^soxoj\/maigret$/, 'OSINT'],
  [/^sundowndev\/phoneinfoga$/, 'OSINT'],
  [/^qeeqbox\/social-analyzer$/, 'OSINT'],
  [/^z4nzu\/hackingtool$/, 'OSINT'],
  [/^d4vinci\/scrapling$/, 'WEB2APP'],
  [/^vercel-labs\/agent-browser$/, 'WEB2APP'],
  [/^decolua\/9router$/, 'WEB2APP'],
  [/^frappe\/(frappe|erpnext)$/, 'COMMERCE'],
  [/^odoo\/odoo$/, 'COMMERCE'],
  [/^medusajs\/medusa$/, 'COMMERCE'],
  [/^documenso\/documenso$/, 'DOCUMENT'],
  [/^esig\/dss$/, 'LEGAL'],
  [/^fincept-corporation\/finceptterminal$/, 'TRADING'],
  [/^tauricresearch\/tradingagents$/, 'TRADING'],
  [/^chatwoot\/chatwoot$/, 'CUSTOMER'],
  [/^twentyhq\/twenty$/, 'CUSTOMER'],
  [/^mastodon\/mastodon$/, 'OTHER'], // genuinely social/fediverse, no closer family fits
  [/^aquasecurity\/trivy$/, 'SECURITY'],
  [/^opensignlabs\/opensign$/, 'DOCUMENT'],
  [/^paperclipai\/paperclip$/, 'DOCUMENT'],
  [/^zed-industries\/zed$/, 'DEVELOPER_TOOLING'],
  [/^yaojingang\/geoflow$/, 'LOGISTICS'],
  [/^openclaw\/openclaw$/, 'ROBOTICS'],
]
for (const rec of donors.values()) {
  if (!rec.family.includes('OTHER') || rec.family.length !== 1) continue
  const key = canon(rec.repo)
  const hint = NAME_FAMILY_HINTS.find(([re]) => re.test(key))
  if (hint) rec.family = [hint[1]]
}

// ---- finalize: fill defaults, sort, assign ids ----
const rows = [...donors.values()].sort((a, b) => a.repo.toLowerCase().localeCompare(b.repo.toLowerCase()))
rows.forEach((r, i) => {
  r.id = `D${String(i + 1).padStart(3, '0')}`
  if (r.family.length === 0) r.family = ['OTHER']
  if (r.source_present) {
    r.status = 'SOURCE_PRESENT'
    r.needed = 'UNKNOWN' // real relevance not yet individually assessed for most of the 90; do not fake NEEDED
  }
})

const out = {
  schema_version: 1,
  generated_at: new Date().toISOString(),
  generated_by:
    'tools/refoundation/donor-corpus.mjs --regenerate (or the one-shot consolidation script this file records provenance of)',
  source_evidence: [
    'temporary/manifest.yaml (90 already-ingested donors, real upstream_sha provenance)',
    'tools/repo/donors.manifest (81-entry pre-existing reference manifest)',
    "user-provided round-2 expansion list (212 rows, categorized: web2app/CRM/OSINT/PlatformOps/FinOps/Robotics/Industrial/Edge/Fleet/Simulation/Legal/Banking/Trading/Estate/etc.)",
    'docs/ and tools/ existing scattered github.com references (Ops census docs, capability census JSON, prior manifests)',
  ],
  donors: rows,
}

writeFileSync(`${ROOT}/tools/refoundation/donor-corpus.yaml`, yaml.stringify(out, { lineWidth: 0 }))
console.log(`TOTAL_UNIQUE_DONORS=${rows.length}`)
const byFamily = {}
for (const r of rows) for (const f of r.family) byFamily[f] = (byFamily[f] || 0) + 1
console.log('BY_FAMILY', JSON.stringify(byFamily, null, 2))
const byStatus = {}
for (const r of rows) byStatus[r.status] = (byStatus[r.status] || 0) + 1
console.log('BY_STATUS', JSON.stringify(byStatus, null, 2))
