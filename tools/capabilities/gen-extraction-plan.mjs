#!/usr/bin/env node
// gen-extraction-plan.mjs --donor <d> — normalize a donor's cluster extraction proposals
// (docs/_machine/capability-reviews/extraction/<d>/*.json) into one plan: new canonical rows to
// mint + source_capability defs + mapped file-remap rules. Existing-vs-new is decided by querying
// the DB (NOT by trusting an agent's claim). Read-only DB. Writes extraction/<d>/_plan.json.
import Database from 'better-sqlite3'
import { readdirSync, readFileSync, writeFileSync, existsSync } from 'node:fs'
import { join } from 'node:path'
import { CAPABILITIES_DB } from '../_paths.mjs'

const donor = (() => { const i = process.argv.indexOf('--donor'); return i >= 0 ? process.argv[i + 1] : null })()
if (!donor) { console.error('usage: gen-extraction-plan.mjs --donor <donor>'); process.exit(2) }
const DIR = join(process.cwd(), 'docs', '_machine', 'capability-reviews', 'extraction', donor)
const DONOR_ROOT = join(process.cwd(), 'Temporary', donor)
if (!existsSync(DIR)) { console.error('no extraction dir: ' + DIR); process.exit(2) }
const DOMAINS = new Set(['other','osint','agent-knowledge','workflow-runtime','observability-analytics','erp-finance','infra-execution','browser-internet-hand','policy-approval-audit','media-generation','crm-support','unclustered','commerce','trading-markets'])
const ALIAS = { policy: 'policy-approval-audit', auth: 'policy-approval-audit', security: 'policy-approval-audit', api: 'other', obs: 'observability-analytics', infra: 'infra-execution', erp: 'erp-finance', erp_finance: 'erp-finance', analytics: 'observability-analytics', workflow: 'workflow-runtime', crm: 'crm-support',
  // legacy key-prefixes whose prefix != domain (existing keys reused by later donors) -> map to the domain they actually live in
  billing: 'other', config: 'other', noise: 'infra-execution', cms: 'other', search: 'other', collab: 'other', graph: 'other', comms: 'crm-support', notify: 'other', bi: 'observability-analytics', media: 'media-generation', browser: 'browser-internet-hand', trading: 'trading-markets', agent: 'agent-knowledge', app: 'other', cache: 'infra-execution', db: 'infra-execution', data: 'other', files: 'infra-execution', wf: 'workflow-runtime', elearning: 'crm-support', portal: 'other', analytics: 'observability-analytics' }

const db = new Database(CAPABILITIES_DB, { readonly: true })
const canonKeys = new Set(db.prepare('SELECT key FROM canonical_capability').all().map(r => r.key))
db.close()

const pick = (c, ...ks) => { if (c == null) return ''; for (const k of ks) if (c[k] != null && c[k] !== '') return c[k]; return '' }
const norm = (s) => (s == null ? '' : (typeof s === 'object' ? (Array.isArray(s) ? s.join('; ') : JSON.stringify(s)) : String(s)))

const files = readdirSync(DIR).filter(f => f.endsWith('.json') && !f.startsWith('_'))
const caps = [], newCanon = new Map(), existingUsed = new Set(), issues = []
let tmp = 0
for (const f of files) {
  const j = JSON.parse(readFileSync(join(DIR, f), 'utf8'))
  const cluster = j.cluster || f.replace('.json', '')
  for (const c of (j.source_capabilities || j.capabilities || [])) {
    const canonical_name = pick(c, 'canonical_name')
    let canonicalKey = pick(c, 'canonical_key', 'proposed_key', 'reuse_key') || pick(c.canonical?.new, 'key') || pick(c.canonical, 'existing_key') || pick(c, 'proposed_canonical_key', 'key')
    let domain = pick(c, 'domain') || pick(c.canonical?.new, 'domain') || (canonicalKey.includes('.') ? canonicalKey.split('.')[0] : '')
    if (!DOMAINS.has(domain)) { const h = domain.replace(/_/g, '-'); if (DOMAINS.has(h)) domain = h }
    if (!DOMAINS.has(domain) && ALIAS[domain]) domain = ALIAS[domain]
    // resolve canonical key to the domain's ESTABLISHED prefix (short for policy/obs/infra/erp/media/browser),
    // preferring an existing key (exact OR short-twin) so we REUSE instead of minting a parallel duplicate.
    const PREFIX = { 'policy-approval-audit': 'policy', 'observability-analytics': 'obs', 'infra-execution': 'infra', 'erp-finance': 'erp', 'media-generation': 'media', 'browser-internet-hand': 'browser', 'crm-support': 'crm-support', 'agent-knowledge': 'agent-knowledge', 'workflow-runtime': 'workflow-runtime', 'commerce': 'commerce', 'osint': 'osint', 'trading-markets': 'trading-markets', 'unclustered': 'unclustered' }
    if (domain && canonicalKey && !canonKeys.has(canonicalKey)) {
      const pfx = PREFIX[domain]
      if (pfx) { const suffix = canonicalKey.includes('.') ? canonicalKey.slice(canonicalKey.indexOf('.') + 1) : canonicalKey; canonicalKey = pfx + '.' + suffix }
      else if (!canonicalKey.includes('.')) canonicalKey = 'other.' + canonicalKey
    }
    const sf = norm(c.source_files)
    let match = c.match && (c.match.regex || c.match.prefix || c.match.glob || c.match.exact || c.match.contains || c.match.suffix) ? c.match : null
    // harden: census paths are donor-relative; strip any leading Temporary/<donor>/ an agent wrongly baked into the match (else it matches 0 rows)
    if (match) { const pfx = new RegExp('(\\^?)Temporary/' + donor + '/', 'g')
      if (match.regex) match.regex = match.regex.replace(pfx, '$1')
      for (const k of ['exact', 'prefix', 'suffix', 'contains', 'glob']) if (Array.isArray(match[k])) match[k] = match[k].map(s => String(s).replace(pfx, '$1')) }
    if (!canonical_name || !canonicalKey) { issues.push(`${f}: cap missing name/key (${canonicalKey || canonical_name})`); continue }
    if (!DOMAINS.has(domain)) { issues.push(`${f}: bad domain '${domain}' for ${canonicalKey}`); continue }
    if (!sf) { issues.push(`${f}: ${canonicalKey} missing source_files`); continue }
    if (!match) { issues.push(`${f}: ${canonicalKey} missing match`); continue }
    // verify cited files exist (honesty guard) — flag (don't drop) so integrator sees fabrication
    const missing = sf.split(/[,;]/).map(s => s.trim()).filter(Boolean).map(s => s.replace(new RegExp('^Temporary/' + donor + '/'), '')).filter(p => !existsSync(join(DONOR_ROOT, p)))
    if (missing.length) issues.push(`${f}: ${canonicalKey} cites ${missing.length} NON-EXISTENT file(s) e.g. ${missing[0]}`)

    const moves_money = (c.moves_money || c.canonical?.new?.moves_money) ? 1 : 0
    const requires_approval = (c.requires_approval || c.canonical?.new?.requires_approval) ? 1 : 0
    const security = !!(c.security_sensitive || c.security_test_required || c.canonical?.new?.security_test_required)
    let fct = norm(pick(c, 'financial_control_test'))
    if (moves_money && !fct) fct = 'REQUIRED: policy->approval->CostRecord->SHA256 audit; no external charge when the money gate denies'
    let rt = norm(pick(c, 'required_tests'))
    if (security && !/secur/i.test(rt)) rt = (rt ? rt + '; ' : '') + 'security: RBAC/isolation enforced; invalid/expired credentials rejected'
    const sec_cls = pick(c, 'side_effect_class') || (moves_money ? 'money' : 'internal_write')
    const tcrate = pick(c, 'target_crate') || pick(c.canonical?.new, 'target_crate')
    const ac = norm(pick(c, 'acceptance_criteria')) || `${canonical_name}: modelled from ${donor} donor source; validated by ${rt || 'a behavior test'}`
    const isNew = !canonKeys.has(canonicalKey)
    if (isNew) {
      if (!newCanon.has(canonicalKey)) newCanon.set(canonicalKey, { key: canonicalKey, canonical_name, domain, target_crate: tcrate, target_module: norm(pick(c, 'target_module')), side_effect_class: sec_cls, moves_money, requires_approval, acceptance_criteria: ac, required_tests: rt, financial_control_test: fct })
      else { const e = newCanon.get(canonicalKey); e.moves_money ||= moves_money; e.requires_approval ||= requires_approval; if (moves_money && !e.financial_control_test) e.financial_control_test = fct }
    } else existingUsed.add(canonicalKey)
    caps.push({ tempKey: `x::${donor}::${++tmp}`, canonicalKey, isNew, cluster, module: pick(c, 'module') || cluster, canonical_name, source_files: sf, source_symbols: norm(pick(c, 'source_symbols')), business_behavior: norm(pick(c, 'business_behavior')), technical_behavior: norm(pick(c, 'technical_behavior')), inputs: norm(pick(c, 'inputs')), outputs: norm(pick(c, 'outputs')), persistence: norm(pick(c, 'persistence')), surface: pick(c, 'surface') || 'api', side_effect_class: sec_cls, moves_money, requires_approval, external_services: norm(pick(c, 'external_services')), target_crate: tcrate, target_module: norm(pick(c, 'target_module')), match })
  }
}
const rules = caps.map(c => ({ classification: 'mapped', ids: [c.tempKey], evidence: `${donor} ${c.canonicalKey}: ${c.canonical_name}`.slice(0, 200), match: c.match }))
const plan = { donor, new_canonicals: [...newCanon.values()], existing_canonicals_used: [...existingUsed], caps, rules,
  stats: { total_caps: caps.length, new_canonicals: newCanon.size, existing_used: existingUsed.size, money_caps: caps.filter(c => c.moves_money).length, approval_caps: caps.filter(c => c.requires_approval).length }, issues }
writeFileSync(join(DIR, '_plan.json'), JSON.stringify(plan, null, 2))
console.log(JSON.stringify(plan.stats))
if (issues.length) { console.log(`issues: ${issues.length}`); issues.slice(0, 30).forEach(i => console.log('  ! ' + i)) }
else console.log('issues: 0')
