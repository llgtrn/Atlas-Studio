#!/usr/bin/env node
// dedupe.mjs — collapse source_capability (3,629 donor rows) into canonical_capability
// (the TRUE Chronica capability scope), with many-to-one provenance.
//
// Dedupe key = normalized canonical_name (lowercased, verb-noun stem) within a domain bucket.
// Heuristic but deterministic; preserves every source row's provenance link.
import Database from 'better-sqlite3'
import { join } from 'node:path'

const db = new Database(join(process.cwd(), 'docs', 'capabilities.db'))
db.pragma('journal_mode = WAL')

// reset canonical tables (idempotent rebuild)
db.exec('DELETE FROM canonical_capability; DELETE FROM provenance; UPDATE source_capability SET canonical_id=NULL;')

// Map a target_crate (donor-suggested, sometimes noisy) to a Chronica domain.
function domainOf(crate, name) {
  const c = (crate || '').toLowerCase()
  const n = (name || '').toLowerCase()
  if (/erp|account|ledger|invoice|journal|stock|inventory|procure|buying|selling|finance|payment|tax/.test(c + ' ' + n)) return 'erp-finance'
  if (/commerce|storefront|listing|product|order|fulfil|cart|checkout|marketplace/.test(c + ' ' + n)) return 'commerce'
  if (/osint|recon|username|email|phone|social|footprint|breach|enumerate|scan-/.test(c + ' ' + n)) return 'osint'
  if (/observ|metric|trace|telemetr|alert|log|monitor|scrape|timeseries|dashboard|insight|funnel|analytic/.test(c + ' ' + n)) return 'observability-analytics'
  if (/workflow|durable|temporal|node-graph|automation|trigger|schedul|cron|queue|job|orchestrat/.test(c + ' ' + n)) return 'workflow-runtime'
  if (/agent|llm|prompt|memory|rag|embed|graph|knowledge|tool-use|skill|mcp/.test(c + ' ' + n)) return 'agent-knowledge'
  if (/browser|crawl|scrape-page|fetch-web|playwright|navigate|render-page/.test(c + ' ' + n)) return 'browser-internet-hand'
  if (/policy|approval|audit|secret|vault|rbac|permission|govern/.test(c + ' ' + n)) return 'policy-approval-audit'
  if (/deploy|container|sandbox|server|infra|provision|build|ci|image|k8s|cloud/.test(c + ' ' + n)) return 'infra-execution'
  if (/media|video|image|diffus|render|comfy|remotion|audio|model-load|sampl|latent|vae|lora/.test(c + ' ' + n)) return 'media-generation'
  if (/crm|support|inbox|ticket|conversation|contact|chat|message/.test(c + ' ' + n)) return 'crm-support'
  if (/trad|broker|market|portfolio|backtest|quote|order-rout|equit/.test(c + ' ' + n)) return 'trading-markets'
  return 'other'
}

// Normalize a capability name to a dedupe stem.
function stem(name) {
  return (name || '')
    .toLowerCase()
    .replace(/[^a-z0-9 ]+/g, ' ')        // drop punctuation
    .replace(/\b(a|an|the|via|with|by|to|for|of|and|in|on|from)\b/g, ' ')
    .replace(/s\b/g, '')                  // crude singularize
    .split(/\s+/).filter(Boolean).sort().join(' ')  // order-independent token set
}

const rows = db.prepare('SELECT id, canonical_name, target_crate, side_effect_class, moves_money, requires_approval FROM source_capability').all()

const buckets = new Map() // key -> { name, domain, ids[], money, approval }
for (const r of rows) {
  const domain = domainOf(r.target_crate, r.canonical_name)
  const key = domain + '::' + stem(r.canonical_name)
  if (!buckets.has(key)) {
    buckets.set(key, { canonical_name: r.canonical_name, domain, ids: [], money: 0, approval: 0, crates: new Set() })
  }
  const b = buckets.get(key)
  b.ids.push(r.id)
  if (r.moves_money) b.money = 1
  if (r.requires_approval) b.approval = 1
  if (r.target_crate) b.crates.add(r.target_crate)
}

const insCanon = db.prepare(`INSERT INTO canonical_capability
 (key,canonical_name,domain,target_crate,side_effect_class,moves_money,requires_approval,acceptance_criteria,financial_control_test,status,donor_count)
 VALUES (@key,@canonical_name,@domain,@target_crate,@side_effect_class,@moves_money,@requires_approval,@acceptance_criteria,@financial_control_test,@status,@donor_count)`)
const insProv = db.prepare('INSERT OR IGNORE INTO provenance (source_id,canonical_id) VALUES (?,?)')
const setCanon = db.prepare('UPDATE source_capability SET canonical_id=? WHERE id=?')

let n = 0
const tx = db.transaction(() => {
  for (const [key, b] of buckets) {
    const donorCount = new Set(db.prepare(`SELECT donor FROM source_capability WHERE id IN (${b.ids.join(',')})`).all().map(x => x.donor)).size
    const info = insCanon.run({
      key,
      canonical_name: b.canonical_name,
      domain: b.domain,
      target_crate: [...b.crates][0] || '',
      side_effect_class: b.money ? 'money' : '',
      moves_money: b.money,
      requires_approval: b.approval,
      // acceptance criteria: every canonical capability gets a concrete, checkable criterion
      acceptance_criteria: b.money
        ? `A test proves: ${b.canonical_name} gates through policy -> board approval -> CostRecord -> audit chain; blocked path leaves no side effect; agent cannot self-approve.`
        : `A test proves: ${b.canonical_name} executes its documented behavior natively in Rust and returns the expected output for representative inputs.`,
      financial_control_test: b.money ? 'REQUIRED (not yet written)' : null,
      status: 'unimplemented',
      donor_count: donorCount,
    })
    const cid = info.lastInsertRowid
    for (const sid of b.ids) { insProv.run(sid, cid); setCanon.run(cid, sid) }
    n++
  }
})
tx()

const canon = db.prepare('SELECT count(*) n FROM canonical_capability').get().n
const src = db.prepare('SELECT count(*) n FROM source_capability').get().n
const money = db.prepare('SELECT count(*) n FROM canonical_capability WHERE moves_money=1').get().n
const byDomain = db.prepare('SELECT domain, count(*) n, sum(moves_money) money FROM canonical_capability GROUP BY domain ORDER BY n DESC').all()
console.log(`DEDUPE complete:`)
console.log(`  source capabilities: ${src}`)
console.log(`  canonical (deduped) capabilities: ${canon}  (dedupe ratio ${(100 * (1 - canon / src)).toFixed(0)}% collapse)`)
console.log(`  money canonical capabilities: ${money}`)
console.log(`  by domain:`, JSON.stringify(byDomain))
db.close()
