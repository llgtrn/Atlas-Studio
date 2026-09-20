#!/usr/bin/env node
// track.mjs (pnpm caps:track) — the STANDARD state + progress + issue tracker over docs/capabilities.db.
//
// Issues are durably stored in the committed tools/capabilities/tracking-issues.json (caps.db is
// gitignored); `add`/`resolve` edit BOTH the JSON and the DB so the JSON stays the source of truth and
// survives a census rebuild (reconcile-tracking-issues.mjs re-applies it). `list`/`snapshot` read the DB.
//
// Usage:
//   pnpm caps:track snapshot                      # STATE + PROGRESS + ISSUES in one view
//   pnpm caps:track list [--open] [--area money] [--sev P1]
//   pnpm caps:track add --id <slug> --sev P2 --area money --title "..." [--detail ".."] [--evidence ".."] [--source ".."] [--auto]
//   pnpm caps:track resolve <id> [--status fixed] [--fix <commit>] [--by <who>]
import Database from 'better-sqlite3'
import { readFileSync, writeFileSync } from 'node:fs'
import { join } from 'node:path'
import { execFileSync } from 'node:child_process'
import { CAPABILITIES_DB } from '../_paths.mjs'
import { loadEdges, graphStats, donorUtil, endStatePillars, assumptionRisk, valueFlow, marketGaps, debtLoad, organismRanking, evidenceLedger, confidencePropagation, externalSignals, realityEngine, predictionMarket, realityLeverage, timeToReality, planningFreeze } from './tracking-lib.mjs'

const HERE = join(process.cwd(), 'tools', 'capabilities')
const SRC = join(HERE, 'tracking-issues.json')
const OPP_SRC = join(HERE, 'opportunities.json')
const SEVERITIES = ['P0', 'P1', 'P2', 'P3', 'P4']
const STATUSES = ['open', 'in_progress', 'fixed', 'wontfix', 'deferred']
const OPP_CATEGORIES = ['capability', 'workflow', 'integration', 'dedup', 'architecture', 'product', 'revenue', 'automation', 'ux', 'observability', 'security', 'testing', 'synergy']
const OPP_STATUSES = ['discovered', 'validated', 'planned', 'executing', 'implemented', 'rejected']

const [cmd, ...rest] = process.argv.slice(2)
function flag(name, def = undefined) { const i = rest.indexOf(`--${name}`); return i >= 0 ? rest[i + 1] : def }
function has(name) { return rest.includes(`--${name}`) }
const positional = rest.filter((v, i) => !v.startsWith('--') && !(i > 0 && rest[i - 1].startsWith('--') && !['open', 'auto'].includes(rest[i - 1].slice(2))))

const readJson = () => JSON.parse(readFileSync(SRC, 'utf8'))
const writeJson = (j) => writeFileSync(SRC, JSON.stringify(j, null, 2) + '\n')
const applyReconcile = () => execFileSync(process.execPath, [join(HERE, 'reconcile-tracking-issues.mjs')], { stdio: 'inherit' })
const readOpp = () => JSON.parse(readFileSync(OPP_SRC, 'utf8'))
const writeOpp = (j) => writeFileSync(OPP_SRC, JSON.stringify(j, null, 2) + '\n')
const applyOppReconcile = () => execFileSync(process.execPath, [join(HERE, 'reconcile-opportunities.mjs')], { stdio: 'inherit' })
const db = (ro = true) => new Database(CAPABILITIES_DB, ro ? { readonly: true } : {})

function add() {
  const id = flag('id'), severity = flag('sev'), area = flag('area'), title = flag('title')
  if (!id || !severity || !area || !title) { console.error('add requires --id --sev --area --title'); process.exit(2) }
  if (!SEVERITIES.includes(severity)) { console.error(`--sev must be one of ${SEVERITIES.join('|')}`); process.exit(2) }
  const j = readJson()
  if (j.issues.some(i => i.id === id)) { console.error(`id '${id}' already exists — use resolve/edit`); process.exit(2) }
  j.issues.push({
    id, severity, area, title,
    detail: flag('detail') ?? null, evidence_ref: flag('evidence') ?? null,
    status: 'open', source: flag('source') ?? 'manual', autonomous_safe: has('auto') ? 1 : 0,
    created_at: new Date().toISOString().slice(0, 10),
  })
  writeJson(j); applyReconcile()
  console.log(`added issue '${id}' (${severity}/${area}).`)
}

function resolve() {
  const id = positional[0]
  if (!id) { console.error('resolve <id> [--status fixed] [--fix <commit>] [--by <who>]'); process.exit(2) }
  const status = flag('status', 'fixed')
  if (!STATUSES.includes(status)) { console.error(`--status must be one of ${STATUSES.join('|')}`); process.exit(2) }
  const j = readJson()
  const it = j.issues.find(i => i.id === id)
  if (!it) { console.error(`no issue '${id}'`); process.exit(2) }
  it.status = status
  if (status === 'fixed' || status === 'wontfix' || status === 'deferred') {
    it.resolved_at = new Date().toISOString().slice(0, 10)
    it.resolved_by = flag('by', 'claude')
    if (flag('fix')) it.fix_ref = flag('fix')
  }
  writeJson(j); applyReconcile()
  console.log(`issue '${id}' -> ${status}${flag('fix') ? ` (fix ${flag('fix')})` : ''}.`)
}

function list() {
  const where = [], args = []
  if (has('open')) where.push("status IN ('open','in_progress')")
  if (flag('area')) { where.push('area=?'); args.push(flag('area')) }
  if (flag('sev')) { where.push('severity=?'); args.push(flag('sev')) }
  const sql = `SELECT id, severity, area, status, autonomous_safe, title FROM tracking_issue
    ${where.length ? 'WHERE ' + where.join(' AND ') : ''} ORDER BY (status IN ('open','in_progress')) DESC, severity, area, id`
  const d = db()
  const rows = d.prepare(sql).all(...args); d.close()
  for (const r of rows) {
    const mark = r.status === 'fixed' ? '✓' : r.status === 'wontfix' ? '∅' : r.status === 'deferred' ? '⏸' : '○'
    const auto = r.status === 'open' || r.status === 'in_progress' ? (r.autonomous_safe ? ' [auto-safe]' : ' [human]') : ''
    console.log(`${mark} ${r.severity} ${r.id} (${r.area})${auto} — ${r.title}`)
  }
  console.log(`\n${rows.length} issue(s).`)
}

function snapshot() {
  const d = db()
  const meta = Object.fromEntries(d.prepare("SELECT k,v FROM meta WHERE k IN ('canonical_capability_count','canonical_verified_count','money_canonical_count','source_capability_count','meta_reconciled_at')").all().map(r => [r.k, r.v]))
  const byStatus = Object.fromEntries(d.prepare('SELECT status, count(*) n FROM canonical_capability GROUP BY status').all().map(r => [r.status, r.n]))
  const moneyUnguarded = d.prepare('SELECT count(*) n FROM canonical_capability WHERE moves_money=1 AND requires_approval=0').get().n
  const verifiedMoneyNoFin = d.prepare("SELECT count(*) n FROM canonical_capability WHERE moves_money=1 AND status='verified' AND COALESCE(financial_control_test,'')=''").get().n
  const issues = d.prepare('SELECT severity, area, status, autonomous_safe, id, title FROM tracking_issue').all()
  const hasOpp = d.prepare("SELECT count(*) n FROM sqlite_master WHERE type='table' AND name='opportunity'").get().n > 0
  const opps = hasOpp ? d.prepare("SELECT id, category, priority, ev, leverage, status, title FROM opportunity WHERE status NOT IN ('implemented','rejected') ORDER BY priority DESC").all() : []
  const hasEnd = d.prepare("SELECT count(*) n FROM sqlite_master WHERE type='table' AND name='end_state_pillar'").get().n > 0
  const hasEdges = d.prepare("SELECT count(*) n FROM sqlite_master WHERE type='table' AND name='strategy_edge'").get().n > 0
  const endst = hasEnd ? endStatePillars(d) : null
  const gstats = hasOpp && hasEdges ? graphStats(d) : null
  // DIM 10-15 (lib functions self-guard on missing tables)
  const arisk = assumptionRisk(d), vflow = valueFlow(d), mkt = marketGaps(d), debt = debtLoad(d), organism = organismRanking(d)
  const decCount = d.prepare("SELECT count(*) n FROM sqlite_master WHERE type='table' AND name='architecture_decision'").get().n ? d.prepare('SELECT count(*) n FROM architecture_decision').get().n : 0
  // Truth Layer + Reality Engine + Reality Leverage
  const led = evidenceLedger(d), xsig = externalSignals(d), realeng = realityEngine(d), predm = predictionMarket(d)
  const ttr = timeToReality(d), freeze = planningFreeze(d)
  d.close()
  const open = issues.filter(i => i.status === 'open' || i.status === 'in_progress')
  const bySev = (arr) => SEVERITIES.map(s => `${s}:${arr.filter(i => i.severity === s).length}`).filter(x => !x.endsWith(':0')).join(' ') || 'none'

  if (freeze && freeze.frozen) {
    console.log('⛔━━━ PLANNING FROZEN ━━━ Reality Ratio ' + freeze.realityRatio + '% (< 1%) and an L4 path exists. Reality > Planning.')
    console.log('   ▶ THE ONE THING: ' + (freeze.nextAction || '—') + (freeze.blocker ? '  ⛔ ' + freeze.blocker : '') + '   (first L4 in ' + freeze.effortToFirstL4 + ' effort-units)')
    console.log('   No new planning dimensions/subsystems. Budget ' + freeze.budget + '. See docs/design/first-l4-strike-plan.md. (caps:track freeze)\n')
  }
  console.log('═══ CHRONICA STATUS SNAPSHOT ═══\n')
  console.log('STATE (caps.db meta):')
  console.log(`  canonical=${meta.canonical_capability_count}  verified=${meta.canonical_verified_count}  money=${meta.money_canonical_count}  source=${meta.source_capability_count}`)
  console.log(`  meta last reconciled: ${meta.meta_reconciled_at ?? '(run caps:track or reconcile-meta)'}`)
  console.log('\nPROGRESS (canonical_capability.status):')
  for (const [s, n] of Object.entries(byStatus)) console.log(`  ${s}: ${n}`)
  console.log('\nMONEY SAFETY:')
  console.log(`  moves_money=1 ∧ requires_approval=0: ${moneyUnguarded} ${moneyUnguarded === 0 ? '✓' : '✗ UNGUARDED'}`)
  console.log(`  verified money caps without financial_control_test: ${verifiedMoneyNoFin} ${verifiedMoneyNoFin === 0 ? '✓' : '✗'}`)
  console.log('\nISSUES (tracking_issue):')
  console.log(`  OPEN/in-progress: ${open.length}  [${bySev(open)}]   (auto-safe ${open.filter(i => i.autonomous_safe).length} · human ${open.filter(i => !i.autonomous_safe).length})`)
  console.log(`  resolved: fixed ${issues.filter(i => i.status === 'fixed').length} · wontfix ${issues.filter(i => i.status === 'wontfix').length} · deferred ${issues.filter(i => i.status === 'deferred').length}`)
  if (open.length) {
    console.log('\n  open by severity:')
    for (const s of SEVERITIES) for (const i of open.filter(x => x.severity === s)) console.log(`    ${s} ${i.id} (${i.area})${i.autonomous_safe ? ' [auto-safe]' : ' [human]'} — ${i.title}`)
  }
  console.log(`\nOPPORTUNITIES (strategic backlog, ranked by priority = ev·confidence/effort):`)
  console.log(`  active: ${opps.length}   (ev = impact·reach·leverage)`)
  for (const o of opps.slice(0, 6)) console.log(`    ${String(o.priority).padStart(6)}  ev${String(o.ev).padStart(3)} lev${o.leverage}  ${o.id} (${o.category}/${o.status}) — ${o.title}`)
  if (opps.length > 6) console.log(`    … ${opps.length - 6} more — \`caps:track opp list\` / docs/027`)
  if (gstats) {
    const top = [...gstats.entries()].filter(([, s]) => s.transitive > 0).sort((a, b) => b[1].transitive - a[1].transitive).slice(0, 4)
    console.log(`\nSTRATEGY GRAPH (highest leverage = unlocks the most downstream):`)
    for (const [id, s] of top) console.log(`    unlocks ${String(s.transitive).padStart(2)} · obsoletes ${s.obsoletes.length} issue(s) · ${id}${s.blockedBy.length ? ' [BLOCKED]' : ''}`)
    console.log(`    (caps:track graph · caps:track sim <id>)`)
  }
  if (endst) {
    console.log(`\nEND-STATE — Company OS completion ${endst.overall}% (weighted):`)
    for (const p of endst.pillars.slice().sort((a, b) => a.pct - b.pct).slice(0, 5)) console.log(`    ${String(p.pct + '%').padStart(4)} ${p.name} (w${p.weight})`)
    console.log(`    (lowest 5 shown; caps:track endstate for all · docs/024)`)
  }
  console.log(`\n─── STRATEGIC BRAIN (DIM 10-15) — is the system right, not just advancing ───`)
  console.log(`  ASSUMPTIONS: ${arisk.all.length} tracked · ${arisk.fragile.length} fragile+unvalidated${arisk.fragile.length ? ' ⚠ ' + arisk.fragile.slice(0, 2).map(a => a.id).join(', ') : ''}`)
  console.log(`  DECISIONS:   ${decCount} ADRs recorded (why we chose what we chose)`)
  console.log(`  VALUE FLOW:  ${vflow.nodes.length} nodes → ${vflow.revenueCount} revenue sink(s) · ${vflow.flagged.length} GROUNDED orphan(s)${vflow.flagged.length ? ' ⚠' : ''} (+${vflow.untraced.length} untraced)`)
  console.log(`  MARKET:      ${mkt.competitorCount} competitors · ${mkt.counts.gap || 0} gaps / ${mkt.counts.differentiation || 0} differentiators`)
  console.log(`  ARCH DEBT:   ${debt.items.length} active item(s) · cost-load ${debt.total} (maint+migr)`)
  const topOrg = organism[0], sleeper = organism.find(r => r.riser >= 8)
  console.log(`  ORGANISM:    top future-leverage = ${topOrg ? topOrg.id + ' (organism ' + topOrg.organism_score + ')' : '—'}`)
  if (sleeper) console.log(`               sleeper (ROI sort buries it): ${sleeper.id} ↑${sleeper.riser} rank`)
  console.log(`  (caps:track assumptions [--fragile] · decisions [<id>] · value [--orphans] · market [--gaps] · debt · evolve)`)
  if (led) {
    const lv = led.byLevel
    console.log(`\n═══ TRUTH LAYER — do I KNOW it, or just THINK it? ═══`)
    console.log(`  EVIDENCE: L0 ${lv.L0 || 0} opinion · L1 ${lv.L1 || 0} inferred · L2 ${lv.L2 || 0} code · L3 ${lv.L3 || 0} runtime · L4 ${lv.L4 || 0} prod   (${led.selfAuthored} self-authored)`)
    if (led.cap) console.log(`            capabilities: ${led.cap.L2_code_verified} L2 code+test · ${led.cap.L3_money_runtime} money runtime-gated (L3) · 0 production (L4)`)
    if (led.riskyLoadBearing.length) console.log(`  ⚠ ${led.riskyLoadBearing.length} fragile opinion(s) LOAD-BEARING the roadmap: ${led.riskyLoadBearing.slice(0, 3).join(', ')}${led.riskyLoadBearing.length > 3 ? ' …' : ''}`)
    console.log(`  EXTERNAL: ${xsig.count} reality signals (latest ${xsig.lastObserved || '—'}) — markets re-scoring the roadmap`)
    console.log(`  (caps:track truth · confidence · external)`)
  }
  if (realeng) {
    console.log(`\n═══ REALITY ENGINE (DIM 16) — did it actually HAPPEN? ═══`)
    console.log(`  ★ REALITY RATIO = ${realeng.realityRatio}%   (L4 production ${realeng.l4Nodes} / ${realeng.strategic} strategic) — PRODUCTION reality. 0% ≠ "nothing happened"; it = no real user yet.`)
    console.log(`    ladder: RUNTIME (L3+L4) ${realeng.runtimeRatio}% · CODE (L2+L3+L4) ${realeng.codeRatio}% · HUMAN-USAGE ${realeng.humanUsageRatio}%  — engineering reality is real; production reality is the gap.`)
    console.log(`  reality events: ${realeng.l3events} L3 synthetic (real runtime, test data) · ${realeng.l4events} L4 production (real user + real data)`)
    console.log(`  outcomes measured: ${realeng.outcomes.measured}/${realeng.outcomes.total} · predictions resolved: ${predm ? predm.resolved : 0}/${predm ? predm.total : 0}${predm && predm.brierMean != null ? ` (Brier ${predm.brierMean})` : ' — never tested against reality'}`)
    if (predm && predm.overdue.length) console.log(`  ⏰ ${predm.overdue.length} prediction(s) past expiry — reality is due; resolve them`)
    if (ttr) {
      console.log(`  ▶ NEXT (reality-dominant objective): ${ttr.next ? ttr.next.label : '—'}${ttr.blocker ? `  ⛔ ${ttr.blocker}` : ''}`)
      console.log(`    Time-to-Reality: last L4 = ${ttr.lastL4 || 'NEVER'} · first L4 in ${ttr.effortToFirstL4 ?? '?'} effort-units · keystone money-L4 in ${ttr.effortToKeystone ?? '?'} (${ttr.milestonesRemaining}/${ttr.milestonesTotal} milestones left)`)
    }
    console.log(`  (caps:track reality · outcomes · predictions · realize ← roadmap re-scored by Reality Leverage)`)
  }
}

// ── OPPORTUNITIES (the 4th dimension) ──
function oppList() {
  const where = ["status NOT IN ('implemented','rejected')"], args = []
  if (has('all')) where.length = 0
  if (flag('cat')) { where.push('category=?'); args.push(flag('cat')) }
  if (flag('status')) { where.push('status=?'); args.push(flag('status')) }
  const d = db()
  const rows = d.prepare(`SELECT id, category, status, impact, reach, leverage, effort, confidence, ev, priority, source_repo, title
    FROM opportunity ${where.length ? 'WHERE ' + where.join(' AND ') : ''} ORDER BY priority DESC`).all(...args)
  d.close()
  for (const o of rows) console.log(`${String(o.priority).padStart(6)}  ev${String(o.ev).padStart(3)} (i${o.impact}·r${o.reach}·lev${o.leverage}) eff${o.effort} conf${o.confidence}  ${o.id} [${o.category}/${o.status}] ${o.source_repo ? '«' + o.source_repo + '» ' : ''}— ${o.title}`)
  console.log(`\n${rows.length} opportunity(ies).`)
}

function oppAdd() {
  const id = flag('id'), title = flag('title'), category = flag('cat')
  if (!id || !title || !category) { console.error('opp add requires --id --title --cat'); process.exit(2) }
  if (!OPP_CATEGORIES.includes(category)) { console.error(`--cat must be one of ${OPP_CATEGORIES.join('|')}`); process.exit(2) }
  const j = readOpp()
  if (j.opportunities.some(o => o.id === id)) { console.error(`opportunity '${id}' already exists`); process.exit(2) }
  const n = (name, d = 3) => Math.max(1, Math.min(5, Number(flag(name, d))))
  j.opportunities.push({
    id, title, description: flag('desc') ?? null, category,
    impact: n('impact'), reach: n('reach'), leverage: n('leverage'), effort: n('effort'), confidence: n('confidence'),
    source_repo: flag('repo') ?? 'native', related_capabilities: flag('caps') ?? '', related_architecture_nodes: flag('nodes') ?? '',
    status: 'discovered', created_at: new Date().toISOString().slice(0, 10),
  })
  writeOpp(j); applyOppReconcile()
  console.log(`added opportunity '${id}' (${category}).`)
}

function oppPromote() {
  const id = rest[1]
  if (!id) { console.error('opp promote <id> --status <discovered|validated|planned|executing|implemented|rejected>'); process.exit(2) }
  const status = flag('status')
  if (!OPP_STATUSES.includes(status)) { console.error(`--status must be one of ${OPP_STATUSES.join('|')}`); process.exit(2) }
  const j = readOpp()
  const o = j.opportunities.find(x => x.id === id)
  if (!o) { console.error(`no opportunity '${id}'`); process.exit(2) }
  o.status = status; o.updated_at = new Date().toISOString().slice(0, 10)
  if (flag('notes')) o.notes = flag('notes')
  writeOpp(j); applyOppReconcile()
  console.log(`opportunity '${id}' -> ${status}.`)
}

function opp() {
  const sub = rest[0]
  if (sub === 'add') oppAdd()
  else if (sub === 'promote') oppPromote()
  else if (sub === 'list' || sub === undefined) oppList()
  else console.error(`opp <list|add|promote>`)
}

// DIM 5/6/7/8 compute lives in tracking-lib.mjs (graphStats, donorUtil, endStatePillars, loadEdges).
function graphCmd() {
  const id = rest[0]
  const d = db(); const edges = loadEdges(d)
  if (id) {
    console.log(`STRATEGY GRAPH for ${id}:`)
    const out = edges.filter(e => e.from_id === id)
    const inb = edges.filter(e => e.to_id === id)
    for (const e of inb) console.log(`   ${e.from_id} --${e.relation}--> (this)`)
    console.log(`  ── ${id} ──`)
    for (const e of out) console.log(`   (this) --${e.relation}--> ${e.to_id}${e.rationale ? '  // ' + e.rationale : ''}`)
    const st = graphStats(d).get(id)
    if (st) console.log(`\n  leverage: unlocks ${st.transitive} downstream (direct ${st.direct}), obsoletes ${st.obsoletes.length} issue(s), contributes ${st.pillars.length} pillar(s)${st.blockedBy.length ? `, BLOCKED by ${st.blockedBy.join(',')}` : ''}${st.dead_end ? ', DEAD-END' : ''}`)
  } else {
    const st = graphStats(d)
    const rows = [...st.entries()].sort((a, b) => b[1].transitive - a[1].transitive || b[1].graph_priority - a[1].graph_priority)
    console.log(`STRATEGY GRAPH — opportunities by leverage (downstream unlocks):`)
    for (const [oid, s] of rows.slice(0, 20)) console.log(`  unlocks ${String(s.transitive).padStart(2)}  obs ${s.obsoletes.length}  gpri ${String(s.graph_priority).padStart(6)}  ${oid}${s.blockedBy.length ? ' [BLOCKED]' : ''}${s.dead_end ? ' [dead-end]' : ''}`)
    const dead = rows.filter(([, s]) => s.dead_end).map(([i]) => i)
    if (dead.length) console.log(`\n  dead-ends (no downstream unlock/obsolete/pillar): ${dead.join(', ')}`)
  }
  d.close()
}
function simCmd() {
  const id = rest[0]
  if (!id) { console.error('sim <opportunity-id>'); process.exit(2) }
  const d = db(); const st = graphStats(d).get(id)
  const row = d.prepare('SELECT title, impact, reach, leverage, effort, ev, priority FROM opportunity WHERE id=?').get(id)
  d.close()
  if (!row) { console.error(`no opportunity '${id}'`); process.exit(2) }
  if (!st) { console.log('opportunity is not active (implemented/rejected).'); return }
  const roi = Math.round(((st.transitive + st.obsoletes.length + st.pillars.length) * row.ev / 10) / row.effort * 10) / 10
  console.log(`EXECUTION SIMULATION — if we build "${row.title}":`)
  console.log(`  + ${st.transitive} downstream opportunities unlocked (transitive)`)
  console.log(`  + ${st.obsoletes.length} open issue(s) resolved: ${st.obsoletes.join(', ') || '—'}`)
  console.log(`  + ${st.pillars.length} end-state pillar(s) advanced: ${st.pillars.join(', ') || '—'}`)
  console.log(`  effort ${row.effort}/5 · ev ${row.ev} · base priority ${row.priority} · graph priority ${st.graph_priority}`)
  console.log(`  ${st.blockedBy.length ? `⚠ BLOCKED until resolved: ${st.blockedBy.join(', ')}` : '✓ not blocked'}`)
  console.log(`  ROI ≈ ${roi}  (unlocked+resolved+pillars)·ev / 10 / effort`)
}
function donorsCmd() {
  const d = db(); const rows = donorUtil(d); d.close()
  const sortKey = has('under') ? (a, b) => a.exploited_pct - b.exploited_pct || b.tc - a.tc : (a, b) => b.tc - a.tc
  console.log(`DONOR UTILIZATION (74 repos) — exploited% = verified canonicals / true_capabilities:`)
  console.log(`  ${'donor'.padEnd(26)} true  scouted%  built%  exploit%  remain  disposition`)
  for (const r of rows.sort(sortKey).slice(0, has('all') ? 99 : 40))
    console.log(`  ${String(r.donor).padEnd(26)} ${String(r.tc).padStart(4)}  ${String(r.extracted_pct + '%').padStart(7)}  ${String(r.built_pct + '%').padStart(5)}  ${String(r.exploited_pct + '%').padStart(7)}  ${String(r.remaining).padStart(5)}  ${r.disposition || ''}`)
  const exhausted = rows.filter(r => r.exploited_pct >= 70).length, under = rows.filter(r => r.exploited_pct < 20).length
  console.log(`\n  ${exhausted} donor(s) ≥70% exploited · ${under} donor(s) <20% (under-utilized — re-mine via discovery).`)
}
function endstateCmd() {
  const d = db(); const { pillars, overall } = endStatePillars(d); d.close()
  console.log(`COMPANY OS COMPLETION — overall ${overall}% (weighted by pillar importance)\n`)
  for (const p of pillars) {
    const bar = '█'.repeat(Math.round(p.pct / 5)).padEnd(20, '░')
    console.log(`  ${String(p.pct + '%').padStart(4)} ${bar} ${p.name} (w${p.weight})${p.basis === 'manual' ? '' : ` ${p.verified}/${p.total}`}`)
  }
}

// DIM 9: AUTONOMOUS DISCOVERY signal — is the strategic backlog stale relative to recent code/donor/
// arch changes? Stateless: compares HEAD to the last commit that touched opportunities.json (the last
// discovery), counting commits that changed discovery inputs. Run as the execution feedback loop.
function git(args) { try { return execFileSync('git', args, { encoding: 'utf8' }).trim() } catch { return '' } }
function discoverDue() {
  const last = git(['log', '-1', '--format=%h', '--', 'tools/capabilities/opportunities.json'])
  if (!last) { console.log('discovery: no prior discovery commit found — DISCOVERY DUE.'); return }
  const since = Number(git(['rev-list', '--count', `${last}..HEAD`]) || 0)
  const changed = git(['diff', '--name-only', `${last}..HEAD`]).split('\n').filter(Boolean)
  const relevant = changed.filter(f => /^(crates\/|Temporary\/)|migrations\/|Cargo\.toml/.test(f))
  const due = since >= 8 || relevant.length >= 5
  console.log(`DISCOVERY STALENESS (DIM 9):`)
  console.log(`  last opportunity update: ${last} (${since} commit(s) ago)`)
  console.log(`  commits since touching code/donor/arch inputs: ${relevant.length}`)
  console.log(due
    ? `  → DISCOVERY DUE. Re-run: Workflow({scriptPath:'tools/capabilities/discover-opportunities.workflow.mjs'}), then merge + caps:rebuild.`
    : `  → fresh enough (threshold: >=8 commits or >=5 input changes).`)
}

// ── STRATEGIC COMPANY BRAIN (DIM 10-15) — correctness/purpose, not just progress ──
function assumptionsCmd() {
  const d = db(); const { all, fragile } = assumptionRisk(d); d.close()
  const rows = has('fragile') ? fragile : all
  const icon = s => s === 'validated' ? '✓' : s === 'invalidated' ? '✗' : s === 'validating' ? '◐' : '○'
  console.log(`ASSUMPTIONS (DIM 10) — ${all.length} tracked · ${fragile.length} fragile+unvalidated (fragility = risk×(6−confidence)):`)
  for (const a of rows) console.log(`  ${icon(a.validation_status)} frag ${String(a.fragility).padStart(2)} (c${a.confidence}/r${a.risk_if_wrong}) ${a.id} — ${a.statement}`)
  if (!has('fragile') && fragile.length) console.log(`\n  ⚠ pressure-test before building on them: ${fragile.map(a => a.id).join(', ')}`)
}
function decisionsCmd() {
  const id = rest[0], d = db()
  if (id) {
    const r = d.prepare('SELECT * FROM architecture_decision WHERE id=?').get(id); d.close()
    if (!r) { console.error(`no decision '${id}'`); process.exit(2) }
    console.log(`${r.id} — ${r.title}  [${r.status}]${r.decided_at ? ' · ' + r.decided_at : ''}\n`)
    console.log(`DECISION: ${r.decision}`)
    if (r.context) console.log(`\nCONTEXT: ${r.context}`)
    if (r.rationale) console.log(`\nRATIONALE: ${r.rationale}`)
    const alts = JSON.parse(r.alternatives || '[]')
    if (alts.length) { console.log(`\nALTERNATIVES REJECTED:`); for (const a of alts) console.log(`  ✗ ${a.option} — ${a.rejected_because}`) }
    if (r.consequences) console.log(`\nCONSEQUENCES: ${r.consequences}`)
    if (r.related_assumptions) console.log(`\nrests on assumptions: ${r.related_assumptions}`)
    return
  }
  const rows = d.prepare('SELECT id,title,status,decided_at FROM architecture_decision ORDER BY id').all(); d.close()
  console.log(`ARCHITECTURE DECISIONS (DIM 11) — ${rows.length} ADRs (caps:track decision <id> for the full record):`)
  for (const r of rows) console.log(`  ${r.id} [${r.status}]${r.decided_at ? ' ' + r.decided_at : ''} ${r.title}`)
}
function valueCmd() {
  const d = db(); const { nodes, edges, flagged, untraced, revenueCount } = valueFlow(d); d.close()
  if (has('orphans')) {
    console.log(`VALUE-FLOW ORPHANS (DIM 12):`)
    console.log(`\n  GROUNDED — capability areas with NO revenue path identified (${flagged.length}):`)
    for (const o of flagged) console.log(`  ⚠ ${o.label}`)
    console.log(`\n  UNTRACED — branch dead-ends before revenue in the DAG (trust-infra / not yet wired) (${untraced.length}):`)
    for (const o of untraced) console.log(`  · ${o.label.slice(0, 100)}`)
    return
  }
  const byKind = {}; for (const n of nodes) byKind[n.kind] = (byKind[n.kind] || 0) + 1
  console.log(`VALUE FLOW (DIM 12) — ${nodes.length} nodes (${Object.entries(byKind).map(([k, n]) => k + ':' + n).join(' ')}) · ${edges.length} edges · ${revenueCount} revenue sink(s):`)
  for (const r of nodes.filter(n => n.kind === 'revenue')) console.log(`  → ${r.label}`)
  console.log(`\n  ${flagged.length} GROUNDED orphan(s) (no revenue path) + ${untraced.length} untraced branch(es) — caps:track value --orphans`)
  for (const o of flagged) console.log(`    ⚠ ${o.label.slice(0, 100)}`)
}
function marketCmd() {
  const d = db(); const { counts, gaps, differentiations, competitorCount } = marketGaps(d); d.close()
  console.log(`COMPETITIVE INTEL (DIM 13) — ${competitorCount} competitors; features: ${Object.entries(counts).map(([k, n]) => k + ' ' + n).join(' · ')}`)
  if (has('gaps')) {
    console.log(`\nGAPS (they have it, we don't):`)
    for (const g of gaps) console.log(`  ✗ [${g.market}] ${g.comp}: ${g.feature}`)
  } else {
    console.log(`\nDIFFERENTIATION (uniquely ours):`)
    for (const x of differentiations) console.log(`  ★ ${x.feature}${x.our_capability_ref ? ' (' + x.our_capability_ref + ')' : ''}`)
    console.log(`\n  ${gaps.length} gap(s) — caps:track market --gaps`)
  }
}
function debtCmd() {
  const d = db(); const { total, byKind, items } = debtLoad(d); d.close()
  console.log(`ARCHITECTURAL DEBT (DIM 14) — ${items.length} active item(s) · cost-load ${total} (maint+migr); by kind: ${Object.entries(byKind).sort((a, b) => b[1] - a[1]).map(([k, n]) => k + ' ' + n).join(' · ')}`)
  for (const r of items) console.log(`  load ${String(r.cost_load).padStart(2)} [${r.kind}] ${r.id} — ${r.title}`)
}
function evolveCmd() {
  const d = db(); const rows = organismRanking(d); d.close()
  console.log(`ORGANISM MODE (DIM 15) — opportunities by EVOLUTION potential, not near-term ROI:`)
  console.log(`  organism=evo×(1+0.1·unlocks); riser = how many ranks above its ROI rank organism mode puts it`)
  for (const r of rows.slice(0, 20)) {
    const riser = r.riser > 0 ? `+${r.riser}` : `${r.riser}`
    console.log(`  ${String(r.organism_score).padStart(7)} (es${String(r.evolution_score).padStart(3)}·u${r.transitive})  riser ${riser.padStart(4)}  ${r.id}`)
  }
  const sleepers = rows.filter(r => r.riser >= 8).slice(0, 6)
  if (sleepers.length) { console.log(`\n  SLEEPERS (organism mode ranks far above ROI — build for what they unlock later):`); for (const s of sleepers) console.log(`    ↑${s.riser} ${s.id} — ${s.rationale || ''}`) }
}

// ── THE TRUTH LAYER — fact vs inference vs assumption vs validated reality ──
function truthCmd() {
  const d = db(); const led = evidenceLedger(d); d.close()
  if (!led) { console.log('no evidence layer yet — run reconcile-evidence'); return }
  const order = ['L0', 'L1', 'L2', 'L3', 'L4']
  const lvLabel = { L0: 'L0 opinion', L1: 'L1 inferred', L2: 'L2 code-verified', L3: 'L3 runtime-verified', L4: 'L4 production' }
  console.log(`TRUTH LAYER — ${led.total} meta-nodes evidence-leveled (evidence CAPS confidence: L0→0.40 … L4→1.0):`)
  for (const l of order) if (led.byLevel[l]) console.log(`  ${lvLabel[l].padEnd(22)} ${led.byLevel[l]}`)
  const proved = (led.byLevel.L3 || 0) + (led.byLevel.L4 || 0), code = led.byLevel.L2 || 0, think = (led.byLevel.L0 || 0) + (led.byLevel.L1 || 0)
  console.log(`\n  KNOW (runtime/prod L3-L4): ${proved}   CODE-VERIFIED (L2): ${code}   THINK (opinion/inferred L0-L1): ${think}`)
  console.log(`  self-authored opinion/inference: ${led.selfAuthored} (the part most at risk of believing my own reasoning)`)
  if (led.cap) console.log(`  capability claims: ${led.cap.L2_code_verified} L2 code+test · ${led.cap.L3_money_runtime} money caps runtime-gated (L3) · ${led.cap.L4_production} production (L4) · ${led.cap.L0_unimplemented} unproven`)
  console.log(`\n  by provenance: ${Object.entries(led.byProv).sort((a, b) => b[1] - a[1]).map(([k, n]) => k + ' ' + n).join(' · ')}`)
  if (led.riskyLoadBearing.length) {
    console.log(`\n  ⚠ FRAGILE OPINIONS LOAD-BEARING THE ROADMAP (L0/L1 assumptions underpinning active opportunities):`)
    for (const a of led.riskyLoadBearing) console.log(`    ⚠ ${a}`)
    console.log(`    → validate these before building further on what rests on them.`)
  }
  console.log(`\n  (caps:track confidence — how this propagates · caps:track external — what reality says)`)
}
function confidenceCmd() {
  const d = db(); const rows = confidencePropagation(d); d.close()
  console.log(`CONFIDENCE PROPAGATION — weakest-link down the support graph (a roadmap is at most as`)
  console.log(`trustworthy as the weakest bet it rests on). Opportunities dragged below their own confidence:`)
  if (!rows.length) { console.log('  none — no active opportunity rests on a weaker node.'); return }
  for (const r of rows) console.log(`  ${r.intrinsic.toFixed(2)} → ${r.effective.toFixed(2)}  ${r.id}${r.culprit ? `   ⟵ capped by ${r.culprit} (${(r.culprit_conf ?? 0).toFixed(2)})` : ''}`)
  console.log(`\n  ${rows.length} opportunity(ies) carry less real confidence than they appear to. Validate the culprit assumptions.`)
}
function externalCmd() {
  const d = db(); const x = externalSignals(d); d.close()
  console.log(`EXTERNAL REALITY (the outward loop) — ${x.count} signals; latest ${x.lastObserved || '—'}; by category: ${Object.entries(x.byCategory).map(([k, n]) => k + ' ' + n).join(' · ')}`)
  const rows = has('all') ? x.all : x.recent
  for (const s of rows) {
    console.log(`\n  [${s.observed_at || '?'}] (${s.category}, c${s.confidence}) ${s.headline}`)
    if (s.recommended_action) console.log(`     ⇒ ${s.recommended_action}`)
    console.log(`     «re-scores ${s.affects_id}»${s.url ? ' · ' + s.url : ''}`)
  }
  if (!has('all') && x.count > rows.length) console.log(`\n  … ${x.count - rows.length} more — caps:track external --all`)
}

// ── THE REALITY ENGINE (DIM 16) — did it actually HAPPEN? ──
function realityCmd() {
  const d = db(); const r = realityEngine(d); const p = predictionMarket(d); const led = evidenceLedger(d); d.close()
  if (!r) { console.log('no reality engine yet — run reconcile-reality'); return }
  console.log(`REALITY ENGINE (DIM 16) — "is it implemented?" is the wrong question; "did it actually HAPPEN?" is the right one.`)
  console.log(`\n  ★ REALITY RATIO = ${r.realityRatio}%   (L4 production nodes ${r.l4Nodes} / ${r.strategic} strategic claims) — PRODUCTION reality: real user + real data.`)
  console.log(`    0% here means PRODUCTION reality is 0 — it does NOT mean nothing happened. The reality LADDER over the same ${r.strategic} strategic nodes:`)
  console.log(`      RUNTIME reality (L3+L4)   = ${r.runtimeRatio}%  (${r.runtimeNodes} nodes ran for real)`)
  console.log(`      CODE reality (L2+L3+L4)   = ${r.codeRatio}%  (${r.codeNodes} nodes built + verified)`)
  console.log(`      HUMAN-USAGE (real user)   = ${r.humanUsageRatio}%  (${r.humanUsageNodes} nodes a real user has touched) ← the one gap only a real user closes`)
  if (led && led.cap) console.log(`    capabilities (${r.strategic ? '' : ''}separate denominator): ${led.cap.L2_code_verified} code+test (L2) · ${led.cap.L3_money_runtime} runtime-gated (L3) · ${led.cap.L4_production} production (L4) — engineering reality is substantial; production reality is the gap.`)
  console.log(`    company-wide incl. capabilities (L4 only): ${r.companyRatio}%`)
  console.log(`\n  reality events (${r.events.length}): ${r.l3events} L3_synthetic · ${r.l4events} L4_production`)
  if (Object.keys(r.byType).length) console.log(`  by type: ${Object.entries(r.byType).map(([k, n]) => k + ' ' + n).join(' · ')}`)
  for (const e of r.events) console.log(`    [${e.reality_level === 'L4_production' ? 'L4' : 'L3'}] ${e.occurred_at} ${String(e.description).slice(0, 88)}  (real-user ${e.user_real}/real-data ${e.data_real})`)
  console.log(`\n  outcomes measured: ${r.outcomes.measured}/${r.outcomes.total} (rest are predicted targets — caps:track outcomes)`)
  if (p) console.log(`  predictions: ${p.resolved}/${p.total} resolved${p.brierMean != null ? ` · Brier ${p.brierMean}` : ' · never tested against reality'} (caps:track predictions)`)
  console.log(`\n  L4 is NON-HAND-ASSIGNABLE — a node reaches L4 only via a reality_event with real user + real data (the standing human/real-user escalation).`)
  console.log(`  OBJECTIVE (not just maximize-L4): primary = raise PRODUCTION reality (needs a real user) · secondary = raise RUNTIME+CODE reality · tertiary = donor exploitation. Do NOT idle waiting for L4.`)
}
function outcomesCmd() {
  const d = db(); const rows = d.prepare('SELECT * FROM outcome ORDER BY status, id').all(); d.close()
  const measured = rows.filter(r => r.status === 'measured').length
  console.log(`OUTCOME GRAPH — results, not builds (${measured} measured / ${rows.length}):`)
  for (const o of rows) console.log(`  [${o.status}] ${o.subject_id} · ${o.metric}: ${o.baseline || '?'} → target ${o.target || '?'}${o.actual ? ` · ACTUAL ${o.actual}` : ' · actual —'}`)
  console.log(`\n  ${measured} measured = the system has shipped capability, not outcome. A target becomes real only when reality produces the number.`)
}
function predictionsCmd() {
  const d = db(); const p = predictionMarket(d); d.close()
  if (!p) { console.log('no prediction market yet — run reconcile-reality'); return }
  console.log(`PREDICTION MARKET — falsifiable bets the brain grades itself on (${p.resolved}/${p.total} resolved${p.brierMean != null ? `, Brier ${p.brierMean}` : ', never tested'}):`)
  const rows = has('due') ? p.overdue : p.all
  if (has('due') && !rows.length) { console.log('  no predictions past expiry yet — reality has not come due.'); return }
  for (const pr of rows) {
    const mark = pr.resolution === 'pending' ? '○' : pr.resolution === 'true' ? '✓' : pr.resolution === 'false' ? '✗' : '◐'
    console.log(`  ${mark} [exp ${pr.expiry}] c${pr.confidence} ${pr.subject_id}\n      ${String(pr.prediction).slice(0, 130)}`)
  }
  if (p.overdue.length && !has('due')) console.log(`\n  ⏰ ${p.overdue.length} past expiry — caps:track predictions --due, then set resolution in predictions.json + caps:rebuild.`)
}

// ── REALITY LEVERAGE — the objective-function flip: progress = L4 evidence created, not capability built ──
function realizeCmd() {
  const d = db(); const rl = realityLeverage(d); const ttr = timeToReality(d); d.close()
  if (!rl) { console.log('no reality path yet — run reconcile-reality-path'); return }
  console.log(`REALITY LEVERAGE — the roadmap re-scored by what creates REAL-WORLD evidence (L4), not what builds capability.`)
  console.log(`  reality_leverage = expected_new_L4 / effort.  Most of the roadmap = 0.00 (it builds capability, creates no L4).`)
  if (ttr) {
    console.log(`\n  ⏱ TIME-TO-REALITY: last L4 = ${ttr.lastL4 || 'NEVER'}${ttr.daysSinceLastL4 != null ? ` (${ttr.daysSinceLastL4}d ago)` : ' (∞)'}`)
    console.log(`     next milestone: ${ttr.next ? ttr.next.label : '—'}${ttr.blocker ? `  ⛔ ${ttr.blocker}` : ''}`)
    console.log(`     to FIRST L4 (${ttr.firstL4Milestone?.label || '?'}): ${ttr.effortToFirstL4} effort-units · to keystone money-L4: ${ttr.effortToKeystone} (${ttr.milestonesRemaining}/${ttr.milestonesTotal} milestones left)`)
  }
  console.log(`\n  ── THE L4 CRITICAL PATH (what must happen in the real world, in order) ──`)
  for (const m of rl.milestones) {
    const mark = m.status === 'done' ? '✅' : m.status === 'in_progress' ? '◐' : m.status === 'blocked' ? '⛔' : '○'
    console.log(`  ${m.ordinal}. ${mark} ${m.label}  (eff ${m.effort}${m.produces_l4 ? ' · ★L4' : ''}${m.reality_leverage ? ' · rl ' + m.reality_leverage : ''})`)
  }
  console.log(`\n  ── ROADMAP RE-RANKED BY REALITY LEVERAGE (the dominant objective) ──`)
  for (const x of rl.top.slice(0, 10)) console.log(`  ${String(x.reality_leverage).padStart(4)}  [${x.kind}] ${x.id} (eff ${x.effort}, +${x.expected_l4} L4)`)
  console.log(`  …then ${rl.zeroL4} capability opportunities at reality_leverage 0.00 — incl. the old top picks (CQRS etc.): they BUILD, they don't create reality.`)
  console.log(`\n  OBJECTIVE FLIP: under the old objective "build 100 capabilities" wins; under this one, "drive 1 real invoice through the gate" wins outright.`)
}

// PLANNING FREEZE — guards against Planner Addiction: Reality > Planning, always.
function freezeCmd() {
  const d = db(); const f = planningFreeze(d); d.close()
  if (!f) { console.log('no reality engine yet'); return }
  if (!f.frozen) { console.log(`PLANNING NOT FROZEN — Reality Ratio ${f.realityRatio}% (>= 1% or no open L4 path). Planning work is allowed again.`); return }
  console.log(`⛔ PLANNING FROZEN — Reality Ratio ${f.realityRatio}% (< 1%) and an L4 path exists.`)
  console.log(`\nThe governance rules in force:`)
  for (const r of f.rules) console.log(`  • ${r}`)
  console.log(`\n▶ THE ONE THING THAT MATTERS: ${f.nextAction || '—'}${f.blocker ? `\n   ⛔ blocker: ${f.blocker}` : ''}`)
  console.log(`   first L4 is ${f.effortToFirstL4} effort-units away. Until it lands, every planning improvement is worth less than creating it.`)
  console.log(`\n   The plan to get there: docs/design/first-l4-strike-plan.md  ·  the path: caps:track realize`)
}

switch (cmd) {
  case 'add': add(); break
  case 'resolve': resolve(); break
  case 'list': list(); break
  case 'opp': opp(); break
  case 'graph': graphCmd(); break
  case 'sim': simCmd(); break
  case 'donors': donorsCmd(); break
  case 'endstate': endstateCmd(); break
  case 'discover-due': discoverDue(); break
  case 'assumptions': assumptionsCmd(); break
  case 'decisions': decisionsCmd(); break
  case 'value': valueCmd(); break
  case 'market': marketCmd(); break
  case 'debt': debtCmd(); break
  case 'evolve': evolveCmd(); break
  case 'truth': truthCmd(); break
  case 'confidence': confidenceCmd(); break
  case 'external': externalCmd(); break
  case 'reality': realityCmd(); break
  case 'outcomes': outcomesCmd(); break
  case 'predictions': predictionsCmd(); break
  case 'realize': realizeCmd(); break
  case 'freeze': freezeCmd(); break
  case 'snapshot': case undefined: snapshot(); break
  default:
    console.log(`caps:track — Reality-Grounded Company Brain over docs/capabilities.db
  dimensions: state·progress·issues·opportunities·graph·donors·end-state·sim·discovery + assumptions·decisions·value·market·debt·organism + TRUTH(evidence·confidence·external) + REALITY(events·outcomes·predictions)
  snapshot                       all 13 dimensions in one view (default)
  list [--open] [--area X] [--sev P1]                      issues
  add --id <slug> --sev P2 --area money --title "..." [--detail --evidence --source --auto]
  resolve <id> [--status fixed] [--fix <commit>] [--by <who>]
  opp list [--cat X] [--status discovered] [--all]         the strategic backlog (priority-ranked)
  opp add --id <slug> --cat synergy --title "..." [--impact/reach/leverage/effort/confidence N] [--repo/caps/nodes/desc]
  opp promote <id> --status validated|planned|executing|implemented|rejected [--notes ".."]
  graph [<id>]                   strategy graph: leverage paths, dead-ends, blocked (DIM 5)
  sim <opportunity-id>           execution simulation: unlocks/resolves/pillars/ROI if built (DIM 8)
  donors [--under] [--all]       donor utilization: exploited% / remaining per repo (DIM 6)
  endstate                       Company OS completion % by pillar (DIM 7)
  discover-due                   is the strategic backlog stale vs recent code/donor changes? (DIM 9)
  assumptions [--fragile]        load-bearing bets + fragility (DIM 10 — are they still true?)
  decisions [<id>]               architecture decision records / ADR memory (DIM 11 — why we chose X)
  value [--orphans]              capability→user→business→revenue flow + orphans (DIM 12)
  market [--gaps]                competitor features ↔ our capabilities: gaps/differentiation (DIM 13)
  debt                           architectural debt: works-but-expensive, by cost-load (DIM 14)
  evolve                         organism mode: rank by future-evolution potential, not ROI (DIM 15)
  truth                          evidence quality L0-L4: what I KNOW vs THINK vs PROVED + self-audit
  confidence                     confidence propagation: roadmap items dragged by weaker bets
  external [--all]               external reality loop: market/ecosystem signals re-scoring the roadmap
  reality                        DID IT HAPPEN? Reality Ratio (L4 production / strategic) — the honest number (DIM 16)
  outcomes                       outcome graph: results not builds (predicted vs measured)
  predictions [--due]            prediction market: falsifiable bets + the brain's own forecast calibration
  realize                        roadmap RE-SCORED by Reality Leverage + the L4 critical path + Time-to-Reality (the objective flip)`)
}
