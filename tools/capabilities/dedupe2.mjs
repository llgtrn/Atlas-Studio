#!/usr/bin/env node
// dedupe2.mjs — stronger deterministic dedupe of source_capability -> canonical_capability.
// Normalizes verb synonyms + extracts the noun-head so cross-donor variants merge
// (create/post/record/make/generate journal-entry -> one; collect/export X-metrics -> one).
// Honest label: this is a DETERMINISTIC dedupe; a deeper semantic merge would collapse a bit more.
import Database from 'better-sqlite3'
import { join } from 'node:path'

const db = new Database(join(process.cwd(), 'docs', 'capabilities.db'))
db.pragma('journal_mode = WAL')
db.exec('DELETE FROM canonical_capability; DELETE FROM provenance; UPDATE source_capability SET canonical_id=NULL;')

// verb synonym classes -> canonical verb
const VERB = new Map(Object.entries({
  create: 'create', post: 'create', record: 'create', make: 'create', add: 'create', insert: 'create', register: 'create', new: 'create', generate: 'create', issue: 'create', build: 'create', write: 'create', emit: 'create', open: 'create', submit: 'create',
  get: 'read', fetch: 'read', read: 'read', retrieve: 'read', list: 'read', query: 'read', view: 'read', show: 'read', search: 'read', discover: 'read', find: 'read', lookup: 'read', load: 'read', pull: 'read', scan: 'read', enumerate: 'read', inspect: 'read', check: 'read', detect: 'read', compute: 'read', calculate: 'read', analyze: 'read', aggregate: 'read', collect: 'read', export: 'read', extract: 'read', gather: 'read', report: 'read', monitor: 'read', observe: 'read', stream: 'read', scrape: 'read', trace: 'read', measure: 'read', evaluate: 'read', score: 'read',
  update: 'update', edit: 'update', modify: 'update', set: 'update', change: 'update', patch: 'update', configure: 'update', manage: 'update', apply: 'update', reconcile: 'update', adjust: 'update', sync: 'update', merge: 'update', rebalance: 'update',
  delete: 'delete', remove: 'delete', cancel: 'delete', destroy: 'delete', drop: 'delete', purge: 'delete', archive: 'delete', revoke: 'delete', deactivate: 'delete', close: 'delete',
  run: 'execute', execute: 'execute', start: 'execute', launch: 'execute', invoke: 'execute', trigger: 'execute', dispatch: 'execute', schedule: 'execute', process: 'execute', deploy: 'execute', provision: 'execute', perform: 'execute', send: 'execute', route: 'execute', place: 'execute', publish: 'execute', validate: 'execute', enforce: 'execute', gate: 'execute', approve: 'execute', authorize: 'execute', verify: 'execute', sample: 'execute', encode: 'execute', decode: 'execute', transform: 'execute', render: 'execute',
}))

const STOP = new Set(['a', 'an', 'the', 'via', 'with', 'by', 'to', 'for', 'of', 'and', 'in', 'on', 'from', 'into', 'across', 'per', 'over', 'or', 'using', 'all', 'multi', 'multiple'])

function normKey(name, domain) {
  const toks = (name || '').toLowerCase().replace(/[^a-z0-9 ]+/g, ' ').split(/\s+/).filter(t => t && !STOP.has(t))
  if (!toks.length) return domain + '::misc'
  // verb = first token if it's a known verb, else 'do'
  let verb = 'do', nouns = toks
  if (VERB.has(toks[0])) { verb = VERB.get(toks[0]); nouns = toks.slice(1) }
  else {
    // verb may be embedded; find first verb-ish token
    const vi = toks.findIndex(t => VERB.has(t))
    if (vi >= 0) { verb = VERB.get(toks[vi]); nouns = toks.filter((_, i) => i !== vi) }
  }
  // noun-head set: singularize crude, sort, keep up to 3 most-specific tokens
  const nset = [...new Set(nouns.map(t => t.replace(/ies$/, 'y').replace(/s$/, '')))].filter(Boolean).sort()
  return domain + '::' + verb + ':' + nset.slice(0, 3).join('-')
}

// reuse domain classifier from dedupe.mjs (inline copy)
function domainOf(crate, name) {
  const c = ((crate || '') + ' ' + (name || '')).toLowerCase()
  if (/erp|account|ledger|invoice|journal|stock|inventory|procure|buying|selling|finance|payment|tax|payroll|asset|depreciat/.test(c)) return 'erp-finance'
  if (/commerce|storefront|listing|product|order|fulfil|cart|checkout|marketplace|shipping/.test(c)) return 'commerce'
  if (/osint|recon|username|email|phone|social|footprint|breach|enumerat|sherlock|maigret|holehe|harvest|domain|whois|subdomain/.test(c)) return 'osint'
  if (/observ|metric|trace|telemetr|alert|log|monitor|scrape|timeseries|dashboard|insight|funnel|analytic|exporter|prometheus|grafana|histogram|gauge|counter/.test(c)) return 'observability-analytics'
  if (/workflow|durable|temporal|node-graph|automation|trigger|schedul|cron|queue|job|orchestrat|saga|activity|signal/.test(c)) return 'workflow-runtime'
  if (/agent|llm|prompt|memory|rag|embed|graph|knowledge|tool-use|skill|mcp|conversation|completion|chat-model/.test(c)) return 'agent-knowledge'
  if (/browser|crawl|fetch-web|playwright|navigate|render-page|scrape-page|proxy|captcha|stealth/.test(c)) return 'browser-internet-hand'
  if (/policy|approval|audit|secret|vault|rbac|permission|govern|consent|compliance/.test(c)) return 'policy-approval-audit'
  if (/deploy|container|sandbox|server|infra|provision|build|ci|image|k8s|cloud|docker|pod|node-pool/.test(c)) return 'infra-execution'
  if (/media|video|image|diffus|render|comfy|remotion|audio|model-load|sampl|latent|vae|lora|controlnet|checkpoint/.test(c)) return 'media-generation'
  if (/crm|support|inbox|ticket|conversation|contact|chat|message|agent-assign|sla|macro|canned/.test(c)) return 'crm-support'
  if (/trad|broker|market|portfolio|backtest|quote|order-rout|equit|crypto|position/.test(c)) return 'trading-markets'
  return 'other'
}

const rows = db.prepare('SELECT id, canonical_name, target_crate, moves_money, requires_approval FROM source_capability').all()
const buckets = new Map()
for (const r of rows) {
  const domain = domainOf(r.target_crate, r.canonical_name)
  const key = normKey(r.canonical_name, domain)
  if (!buckets.has(key)) buckets.set(key, { name: r.canonical_name, domain, ids: [], money: 0, approval: 0, crates: new Set() })
  const b = buckets.get(key)
  b.ids.push(r.id); if (r.moves_money) b.money = 1; if (r.requires_approval) b.approval = 1; if (r.target_crate) b.crates.add(r.target_crate)
}

const insCanon = db.prepare(`INSERT INTO canonical_capability
 (key,canonical_name,domain,target_crate,side_effect_class,moves_money,requires_approval,acceptance_criteria,financial_control_test,status,donor_count)
 VALUES (@key,@canonical_name,@domain,@target_crate,@side_effect_class,@moves_money,@requires_approval,@acceptance_criteria,@financial_control_test,@status,@donor_count)`)
const insProv = db.prepare('INSERT OR IGNORE INTO provenance (source_id,canonical_id) VALUES (?,?)')
const setCanon = db.prepare('UPDATE source_capability SET canonical_id=? WHERE id=?')

const tx = db.transaction(() => {
  for (const [key, b] of buckets) {
    const donorCount = new Set(db.prepare(`SELECT donor FROM source_capability WHERE id IN (${b.ids.join(',')})`).all().map(x => x.donor)).size
    const info = insCanon.run({
      key, canonical_name: b.name, domain: b.domain, target_crate: [...b.crates][0] || '',
      side_effect_class: b.money ? 'money' : '', moves_money: b.money, requires_approval: b.approval,
      acceptance_criteria: b.money
        ? `Test proves: ${b.name} runs policy -> board approval -> CostRecord -> audit chain; blocked path no side effect; no self-approval.`
        : `Test proves: ${b.name} executes its documented behavior natively and returns expected output for representative inputs.`,
      financial_control_test: b.money ? 'REQUIRED (not yet written)' : null,
      status: 'unimplemented', donor_count: donorCount,
    })
    const cid = info.lastInsertRowid
    for (const sid of b.ids) { insProv.run(sid, cid); setCanon.run(cid, sid) }
  }
})
tx()

const canon = db.prepare('SELECT count(*) n FROM canonical_capability').get().n
const src = db.prepare('SELECT count(*) n FROM source_capability').get().n
const money = db.prepare('SELECT count(*) n FROM canonical_capability WHERE moves_money=1').get().n
const byDomain = db.prepare('SELECT domain, count(*) n FROM canonical_capability GROUP BY domain ORDER BY n DESC').all()
console.log(`DEDUPE2: source=${src} -> canonical=${canon} (${(100 * (1 - canon / src)).toFixed(0)}% collapse), money canonical=${money}`)
console.log('by domain:', JSON.stringify(byDomain))
db.close()
