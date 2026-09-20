// tracking-lib.mjs — shared compute for the Company-OS Planning Brain dimensions, used by track.mjs
// (the CLI) and the gen-*-doc.mjs projections. Every function takes an open better-sqlite3 handle and
// is pure/read-only. No duplication of the graph/donor/end-state math.
import { attachCanonicalCapabilityFallback } from './canonical-fallback.mjs'

export function loadEdges(d) { return d.prepare('SELECT from_kind,from_id,relation,to_kind,to_id,weight,rationale FROM strategy_edge').all() }
function adjUnlocks(edges) { const a = new Map(); for (const e of edges) if (e.relation === 'unlocks') { if (!a.has(e.from_id)) a.set(e.from_id, []); a.get(e.from_id).push(e.to_id) } return a }
function transitiveDown(adj, start) { const seen = new Set(), st = [...(adj.get(start) || [])]; while (st.length) { const n = st.pop(); if (seen.has(n)) continue; seen.add(n); for (const m of (adj.get(n) || [])) st.push(m) } return seen }
const hasTable = (d, n) => {
  const main = d.prepare("SELECT count(*) c FROM sqlite_master WHERE type='table' AND name=?").get(n).c
  if (main > 0) return true
  return d.prepare("SELECT count(*) c FROM sqlite_temp_master WHERE type IN ('table','view') AND name=?").get(n).c > 0
}
function ensureCanonical(d) {
  if (hasTable(d, 'canonical_capability')) return true
  try {
    attachCanonicalCapabilityFallback(d, { includeOverrideView: true })
    return true
  } catch {
    return hasTable(d, 'canonical_capability')
  }
}

// DIM 5/8: per-active-opportunity graph stats (transitive unlocks, obsoleted-open-issues, blockers,
// pillars) + a graph_priority that boosts leverage and halves a blocked node.
export function graphStats(d) {
  const edges = loadEdges(d), adj = adjUnlocks(edges)
  const opps = d.prepare("SELECT id, priority FROM opportunity WHERE status NOT IN ('implemented','rejected')").all()
  const openIssues = new Set(d.prepare("SELECT id FROM tracking_issue WHERE status IN ('open','in_progress')").all().map(r => r.id))
  const m = new Map()
  for (const o of opps) {
    const trans = transitiveDown(adj, o.id)
    const direct = edges.filter(e => e.relation === 'unlocks' && e.from_id === o.id).length
    const obsoletes = edges.filter(e => e.relation === 'obsoletes' && e.from_id === o.id && openIssues.has(e.to_id)).map(e => e.to_id)
    const blockedBy = edges.filter(e => e.relation === 'blocks' && e.to_id === o.id && openIssues.has(e.from_id)).map(e => e.from_id)
    const pillars = edges.filter(e => e.relation === 'contributes_to' && e.from_id === o.id).map(e => e.to_id)
    const gp = Math.round(o.priority * (1 + trans.size * 0.15 + obsoletes.length * 0.1) * (blockedBy.length ? 0.5 : 1) * 100) / 100
    m.set(o.id, { priority: o.priority, direct, transitive: trans.size, obsoletes, blockedBy, pillars, graph_priority: gp, dead_end: trans.size === 0 && obsoletes.length === 0 && pillars.length === 0 })
  }
  return m
}

// DIM 6: donor utilization — per donor, scouted%/built%/exploited%/remaining over its true_capabilities.
export function donorUtil(d) {
  ensureCanonical(d)
  return d.prepare(`SELECT ds.donor, ds.true_capabilities tc, ds.disposition,
      count(s.id) source_caps, count(DISTINCT s.canonical_id) canonicals,
      count(DISTINCT CASE WHEN c.status='verified' THEN s.canonical_id END) verified
    FROM donor_scope ds
    LEFT JOIN source_capability s ON s.donor=ds.donor
    LEFT JOIN canonical_capability c ON c.id=s.canonical_id
    GROUP BY ds.donor ORDER BY ds.true_capabilities DESC`).all().map(r => ({
    ...r,
    extracted_pct: r.tc ? Math.round(100 * r.canonicals / r.tc) : 0,
    built_pct: r.canonicals ? Math.round(100 * r.verified / r.canonicals) : 0,
    exploited_pct: r.tc ? Math.round(100 * r.verified / r.tc) : 0,
    remaining: Math.max(0, r.tc - r.verified),
  }))
}

// DIM 7: end-state distance — per pillar, verified/total caps within its domains (or by crate, or a
// manual_pct override for surfaces caps.db doesn't track). overall = importance-weighted average.
export function endStatePillars(d) {
  ensureCanonical(d)
  const pillars = d.prepare('SELECT * FROM end_state_pillar ORDER BY ordinal').all()
  const out = []
  for (const p of pillars) {
    let verified = 0, total = 0, pct = 0, basis = 'n/a'
    const mp = /manual_pct=(\d+)/.exec(p.notes || '')
    if (mp) { pct = Number(mp[1]); basis = 'manual' }
    else if (p.domains) {
      const doms = p.domains.split(',').map(s => s.trim()).filter(Boolean), ph = doms.map(() => '?').join(',')
      total = d.prepare(`SELECT count(*) n FROM canonical_capability WHERE domain IN (${ph})`).get(...doms).n
      verified = d.prepare(`SELECT count(*) n FROM canonical_capability WHERE status='verified' AND domain IN (${ph})`).get(...doms).n
      pct = total ? Math.round(100 * verified / total) : 0; basis = 'domains'
    } else if (p.crates) {
      const cr = p.crates.split(',').map(s => s.trim()).filter(Boolean)
      const like = cr.map(() => 'target_crate LIKE ?').join(' OR '), args = cr.map(c => `%${c}%`)
      total = d.prepare(`SELECT count(*) n FROM canonical_capability WHERE ${like}`).get(...args).n
      verified = d.prepare(`SELECT count(*) n FROM canonical_capability WHERE status='verified' AND (${like})`).get(...args).n
      pct = total ? Math.round(100 * verified / total) : 0; basis = 'crates'
    }
    out.push({ ...p, verified, total, pct, basis })
  }
  const wsum = out.reduce((a, p) => a + (p.weight || 1), 0)
  const overall = Math.round(out.reduce((a, p) => a + p.pct * (p.weight || 1), 0) / (wsum || 1))
  return { pillars: out, overall }
}

// DIM 10: assumption risk — fragility = risk_if_wrong * (6 - confidence); a bet is DANGEROUS when it is
// fragile (>=12) AND not yet validated. Those are the premises most worth pressure-testing before we
// build months of work on them.
export function assumptionRisk(d) {
  if (!hasTable(d, 'assumption')) return { all: [], fragile: [] }
  const all = d.prepare('SELECT * FROM assumption ORDER BY fragility DESC, id').all()
  const fragile = all.filter(a => a.fragility >= 12 && ['unvalidated', 'validating', 'invalidated'].includes(a.validation_status))
  return { all, fragile }
}

// DIM 12: value-flow — does each capability reach revenue? Reachability is computed by a cycle-safe
// reverse-BFS from the revenue nodes (a forward DFS with shared memo + path-state poisons the cache on
// any cycle). A capability that cannot reach revenue is split into: `flagged` (the grounded, explicitly
// "no revenue path identified" findings — the real warning) and `untraced` (a branch that simply dead-
// ends earlier in the DAG — trust-infra or value chain not yet wired all the way to a revenue node).
export function valueFlow(d) {
  if (!hasTable(d, 'value_node')) return { nodes: [], edges: [], orphans: [], flagged: [], untraced: [], revenueCount: 0 }
  const nodes = d.prepare('SELECT * FROM value_node').all()
  const edges = d.prepare('SELECT from_id, to_id FROM value_edge').all()
  const radj = new Map(); for (const e of edges) { if (!radj.has(e.to_id)) radj.set(e.to_id, []); radj.get(e.to_id).push(e.from_id) }
  const revenue = nodes.filter(n => n.kind === 'revenue').map(n => n.id)
  const canReach = new Set(revenue), stack = [...revenue]
  while (stack.length) { const n = stack.pop(); for (const p of (radj.get(n) || [])) if (!canReach.has(p)) { canReach.add(p); stack.push(p) } }
  const orphans = nodes.filter(n => n.kind === 'capability' && !canReach.has(n.id))
  const flagged = orphans.filter(n => /orphan-flagged/.test(n.notes || ''))
  const untraced = orphans.filter(n => !/orphan-flagged/.test(n.notes || ''))
  return { nodes, edges, orphans, flagged, untraced, revenueCount: revenue.length }
}

// DIM 13: competitive posture — counts by relation + the concrete gap/differentiation lists (joined to
// competitor names) so we can see honestly where we trail and where we are uniquely ahead.
export function marketGaps(d) {
  if (!hasTable(d, 'competitor_feature')) return { counts: {}, gaps: [], differentiations: [], competitorCount: 0 }
  const counts = Object.fromEntries(d.prepare('SELECT relation, count(*) n FROM competitor_feature GROUP BY relation').all().map(r => [r.relation, r.n]))
  const join = (rel) => d.prepare(`SELECT cf.feature, cf.our_capability_ref, cf.notes, c.name comp, c.market
    FROM competitor_feature cf JOIN competitor c ON c.id=cf.competitor_id WHERE cf.relation=? ORDER BY c.market, c.name`).all(rel)
  return { counts, gaps: join('gap'), differentiations: join('differentiation'), competitorCount: d.prepare('SELECT count(*) n FROM competitor').get().n }
}

// DIM 14: architectural debt load — active debt ranked by cost-load (maintenance + migration), with the
// per-kind breakdown. Distinct from issues: this is the carrying cost of things that WORK.
export function debtLoad(d) {
  if (!hasTable(d, 'arch_debt')) return { total: 0, byKind: {}, items: [] }
  const active = d.prepare("SELECT *, (maintenance_cost+migration_cost) cost_load FROM arch_debt WHERE status='active' ORDER BY (maintenance_cost+migration_cost) DESC, id").all()
  const total = active.reduce((a, r) => a + r.cost_load, 0)
  const byKind = {}; for (const r of active) byKind[r.kind] = (byKind[r.kind] || 0) + r.cost_load
  return { total, byKind, items: active }
}

// DIM 15: organism mode — rank active opportunities by EVOLUTION potential, not near-term ROI.
// organism_score = evolution_score (learning*optionality*adaptability*knowledge) boosted by graph
// transitive-unlocks. `riser` = priority_rank - organism_rank (>0 => organism mode favors it MORE than
// the ROI sort does — a sleeper the priority view buries).
export function organismRanking(d) {
  if (!hasTable(d, 'evolution_score')) return []
  const edges = loadEdges(d), adj = adjUnlocks(edges)
  const opps = d.prepare("SELECT id, title, priority FROM opportunity WHERE status NOT IN ('implemented','rejected')").all()
  const evo = new Map(d.prepare('SELECT * FROM evolution_score').all().map(r => [r.opportunity_id, r]))
  const rows = opps.map(o => {
    const e = evo.get(o.id)
    const es = e ? e.evolution_score : 16
    const trans = transitiveDown(adj, o.id).size
    const organism_score = Math.round(es * (1 + trans * 0.1) * 100) / 100
    return { id: o.id, title: o.title, priority: o.priority, evolution_score: es, has_score: !!e, transitive: trans, organism_score, ...(e ? { learning_value: e.learning_value, optionality: e.optionality, adaptability: e.adaptability, knowledge_gain: e.knowledge_gain, rationale: e.rationale } : {}) }
  })
  const byOrg = rows.slice().sort((a, b) => b.organism_score - a.organism_score)
  const byPri = rows.slice().sort((a, b) => b.priority - a.priority)
  const oRank = new Map(byOrg.map((r, i) => [r.id, i + 1]))
  const pRank = new Map(byPri.map((r, i) => [r.id, i + 1]))
  for (const r of rows) { r.organism_rank = oRank.get(r.id); r.priority_rank = pRank.get(r.id); r.riser = r.priority_rank - r.organism_rank }
  return byOrg
}

// ── THE TRUTH LAYER — fact vs inference vs assumption vs validated reality ──

// the support graph used for confidence propagation: a node RESTS ON the nodes that cap it
// (assumptions that underpin it + upstream graph enablers). Mirrors reconcile-evidence.
function buildSupports(d) {
  const supports = new Map(), dep = (k, on) => { if (!supports.has(k)) supports.set(k, []); supports.get(k).push(on) }
  const tagged = new Set(hasTable(d, 'node_evidence') ? d.prepare('SELECT node_kind,node_id FROM node_evidence').all().map(r => `${r.node_kind}:${r.node_id}`) : [])
  const oppIds = new Set(hasTable(d, 'opportunity') ? d.prepare('SELECT id FROM opportunity').all().map(r => r.id) : [])
  const decIds = new Set(hasTable(d, 'architecture_decision') ? d.prepare('SELECT id FROM architecture_decision').all().map(r => r.id) : [])
  if (hasTable(d, 'assumption')) for (const a of d.prepare('SELECT id,underpins FROM assumption').all())
    for (const tok of String(a.underpins || '').split(/[,\s]+/).map(s => s.trim()).filter(Boolean)) {
      if (oppIds.has(tok)) dep(`opportunity:${tok}`, `assumption:${a.id}`)
      if (decIds.has(tok)) dep(`decision:${tok}`, `assumption:${a.id}`)
    }
  if (hasTable(d, 'strategy_edge')) for (const e of d.prepare("SELECT from_kind,from_id,relation,to_kind,to_id FROM strategy_edge WHERE relation IN ('unlocks','depends_on')").all()) {
    const from = `${e.from_kind}:${e.from_id}`, to = `${e.to_kind}:${e.to_id}`
    if (!tagged.has(from) || !tagged.has(to)) continue
    if (e.relation === 'unlocks') dep(to, from); else dep(from, to)
  }
  return supports
}

// derived CAPABILITY evidence — verified caps are code+test (L2); the money path has a deployed smoke (L3);
// unimplemented caps are unproven claims (treated as below L2). Keeps the headline honest.
function capEvidence(d) {
  ensureCanonical(d)
  if (!hasTable(d, 'canonical_capability')) return null
  const verified = d.prepare("SELECT count(*) n FROM canonical_capability WHERE status='verified'").get().n
  const moneyRuntime = d.prepare("SELECT count(*) n FROM canonical_capability WHERE status='verified' AND moves_money=1 AND COALESCE(financial_control_test,'')<>''").get().n
  const unimpl = d.prepare("SELECT count(*) n FROM canonical_capability WHERE status='unimplemented'").get().n
  return { L2_code_verified: verified, L3_money_runtime: moneyRuntime, L0_unimplemented: unimpl, L4_production: 0 }
}

// the evidence ledger — what do I KNOW vs THINK vs have PROVEN? Plus the self-audit: how much of the
// strategic brain is self-authored opinion, and which fragile opinions are load-bearing the roadmap.
export function evidenceLedger(d) {
  if (!hasTable(d, 'node_evidence')) return null
  const meta = d.prepare('SELECT * FROM node_evidence').all()
  const byLevel = {}, byProv = {}
  for (const r of meta) { byLevel[r.evidence_level] = (byLevel[r.evidence_level] || 0) + 1; byProv[r.provenance] = (byProv[r.provenance] || 0) + 1 }
  const selfAuthored = meta.filter(r => /authored/.test(r.provenance || '')).length
  const supports = buildSupports(d)
  const lvOf = Object.fromEntries(meta.filter(r => r.node_kind === 'assumption').map(r => [r.node_id, r.evidence_level]))
  const loadBearing = new Set()
  for (const [node, deps] of supports) if (node.startsWith('opportunity:')) for (const s of deps) if (s.startsWith('assumption:')) loadBearing.add(s.slice('assumption:'.length))
  const riskyLoadBearing = [...loadBearing].filter(a => lvOf[a] === 'L0' || lvOf[a] === 'L1')
  return { total: meta.length, byLevel, byProv, selfAuthored, loadBearing: [...loadBearing], riskyLoadBearing, cap: capEvidence(d) }
}

// confidence propagation — opportunities whose effective confidence is dragged below their own, and the
// weakest support (the culprit) doing the dragging. "A roadmap is at most as trustworthy as its weakest bet."
export function confidencePropagation(d) {
  if (!hasTable(d, 'node_evidence')) return []
  const eff = new Map(d.prepare('SELECT node_kind,node_id,effective_confidence FROM node_evidence').all().map(r => [`${r.node_kind}:${r.node_id}`, r.effective_confidence]))
  const supports = buildSupports(d)
  const rows = d.prepare("SELECT node_id,intrinsic_confidence ic,effective_confidence ec,evidence_level lv FROM node_evidence WHERE node_kind='opportunity' AND effective_confidence < intrinsic_confidence - 0.001").all()
  return rows.map(r => {
    let culprit = null, min = 2
    for (const s of (supports.get(`opportunity:${r.node_id}`) || [])) { const e = eff.get(s); if (e != null && e < min) { min = e; culprit = s } }
    return { id: r.node_id, level: r.lv, intrinsic: r.ic, effective: r.ec, culprit, culprit_conf: culprit ? eff.get(culprit) : null }
  }).sort((a, b) => a.effective - b.effective)
}

// DIM 16: THE REALITY ENGINE — did it actually HAPPEN, not just "is it built"? The Reality Ratio
// (L4 production nodes / strategic nodes) is the single most honest metric in the system. Today it is ~0
// by design — nothing has run with a real user + real data in production. Reality is the one truth source
// the system cannot generate from code/docs/inference.
export function realityEngine(d) {
  if (!hasTable(d, 'reality_event')) return null
  ensureCanonical(d)
  const byLevel = Object.fromEntries(d.prepare('SELECT reality_level, count(*) n FROM reality_event GROUP BY reality_level').all().map(r => [r.reality_level, r.n]))
  const byType = Object.fromEntries(d.prepare('SELECT event_type, count(*) n FROM reality_event GROUP BY event_type ORDER BY n DESC').all().map(r => [r.event_type, r.n]))
  const events = d.prepare('SELECT * FROM reality_event ORDER BY occurred_at DESC').all()
  // strategic nodes = our own claims (exclude external_signal — those are observations, not our state)
  const strategic = hasTable(d, 'node_evidence') ? d.prepare("SELECT count(*) n FROM node_evidence WHERE node_kind<>'external_signal'").get().n : 0
  // The evidence LADDER over strategic nodes. Reality Ratio (L4/strategic) is PRODUCTION reality only;
  // 0 L4 does NOT mean 0 reality — L3 runtime-verified + L2 code-verified are real ENGINEERING reality.
  // Surfacing the whole ladder stops the cognitive error "L4=0 ⇒ nothing has happened".
  const evNodes = (levels) => hasTable(d, 'node_evidence')
    ? d.prepare(`SELECT count(*) n FROM node_evidence WHERE node_kind<>'external_signal' AND evidence_level IN (${levels.map(() => '?').join(',')})`).get(...levels).n
    : 0
  const l4Nodes = evNodes(['L4'])              // production-verified evidence (real user + real data)
  const l3Nodes = evNodes(['L3'])              // runtime-verified (ran, synthetic user/data)
  const l2Nodes = evNodes(['L2'])              // code-verified (built + tests pass)
  const runtimeNodes = evNodes(['L3', 'L4'])   // runtime reality (ran for real, any data)
  const codeNodes = evNodes(['L2', 'L3', 'L4'])// built + verified or better
  const touched = new Set(events.map(e => `${e.node_kind}:${e.node_id}`)).size  // nodes with ANY real runtime event
  // human usage = strategic nodes a REAL user has actually touched (the one gap only a real user closes)
  const humanUsageNodes = d.prepare('SELECT count(DISTINCT node_kind || \':\' || node_id) n FROM reality_event WHERE user_real=1').get().n
  const caps = hasTable(d, 'canonical_capability') ? d.prepare('SELECT count(*) n FROM canonical_capability').get().n : 0
  const pct = (num, den) => den ? Math.round(10000 * num / den) / 100 : 0
  const realityRatio = pct(l4Nodes, strategic)        // PRODUCTION reality (the North Star; honest 0 until a real slice ships)
  const runtimeRatio = pct(runtimeNodes, strategic)   // RUNTIME / engineering reality (L3+L4) — NOT zero
  const codeRatio = pct(codeNodes, strategic)         // CODE reality (L2+L3+L4)
  const exercisedRatio = pct(touched, strategic)      // nodes exercised by ANY real runtime event
  const humanUsageRatio = pct(humanUsageNodes, strategic)
  const companyRatio = pct(l4Nodes, strategic + caps)
  const outcomes = { total: 0, measured: 0, predicted: 0 }
  if (hasTable(d, 'outcome')) { outcomes.total = d.prepare('SELECT count(*) n FROM outcome').get().n; outcomes.measured = d.prepare("SELECT count(*) n FROM outcome WHERE status='measured'").get().n; outcomes.predicted = outcomes.total - outcomes.measured }
  return { byLevel, byType, events, strategic, l4Nodes, l3Nodes, l2Nodes, runtimeNodes, codeNodes, touched, humanUsageNodes, realityRatio, runtimeRatio, codeRatio, exercisedRatio, humanUsageRatio, companyRatio, l3events: byLevel.L3_synthetic || 0, l4events: byLevel.L4_production || 0, outcomes }
}

// DIM 16: PREDICTION MARKET — each bet is a falsifiable prediction; as they resolve the brain learns its
// own forecasting accuracy (Brier score = mean (confidence - outcome)^2; lower is better). today is read
// at call time so overdue predictions surface as reality comes due.
export function predictionMarket(d, today = new Date().toISOString().slice(0, 10)) {
  if (!hasTable(d, 'prediction')) return null
  const all = d.prepare('SELECT * FROM prediction ORDER BY expiry').all()
  const resolved = all.filter(p => p.resolution !== 'pending')
  const pending = all.filter(p => p.resolution === 'pending')
  const briers = resolved.filter(p => p.brier != null).map(p => p.brier)
  const brierMean = briers.length ? Math.round(briers.reduce((a, b) => a + b, 0) / briers.length * 1000) / 1000 : null
  const overdue = pending.filter(p => p.expiry && p.expiry <= today)
  const soon = pending.filter(p => p.expiry && p.expiry > today)
  const byResolution = Object.fromEntries(d.prepare('SELECT resolution, count(*) n FROM prediction GROUP BY resolution').all().map(r => [r.resolution, r.n]))
  return { all, total: all.length, pending: pending.length, resolved: resolved.length, byResolution, brierMean, overdue, soon: soon.slice(0, 8) }
}

// REALITY LEVERAGE — the objective-function flip. reality_leverage = expected_new_L4 / effort. Most of
// the roadmap builds capability and creates ZERO L4, so it scores 0; the few items on the path to real-
// world evidence rise to the top. Re-ranks the WHOLE roadmap under the new objective (create L4 evidence,
// not build capability). Returns the re-scored opportunities + the L4 critical-path milestones.
export function realityLeverage(d) {
  if (!hasTable(d, 'reality_milestone')) return null
  const hasL4Col = d.prepare("SELECT count(*) n FROM pragma_table_info('opportunity') WHERE name='expected_l4'").get().n > 0
  const opps = hasL4Col ? d.prepare("SELECT id, title, effort, priority, COALESCE(expected_l4,0) expected_l4 FROM opportunity WHERE status NOT IN ('implemented','rejected')").all() : []
  for (const o of opps) o.reality_leverage = o.effort ? Math.round(100 * o.expected_l4 / o.effort) / 100 : 0
  const oppRanked = opps.slice().sort((a, b) => b.reality_leverage - a.reality_leverage || b.priority - a.priority)
  const milestones = d.prepare('SELECT * FROM reality_milestone ORDER BY ordinal').all()
  for (const m of milestones) m.reality_leverage = m.effort ? Math.round(100 * m.produces_l4 / m.effort) / 100 : 0
  const incomplete = milestones.filter(m => m.status !== 'done')
  const top = [
    ...incomplete.map(m => ({ kind: 'milestone', id: m.id, label: m.label, effort: m.effort, expected_l4: m.produces_l4, reality_leverage: m.reality_leverage, status: m.status })),
    ...oppRanked.filter(o => o.reality_leverage > 0).map(o => ({ kind: 'opportunity', id: o.id, label: o.title, effort: o.effort, expected_l4: o.expected_l4, reality_leverage: o.reality_leverage })),
  ].sort((a, b) => b.reality_leverage - a.reality_leverage)
  const zeroL4 = opps.filter(o => o.reality_leverage === 0).length
  return { oppRanked, milestones, incomplete, top, zeroL4, totalOpps: opps.length }
}

// TIME-TO-REALITY — days_since_last_L4 (currently: never) + the remaining path to the first/keystone L4.
// A project can be 55% "complete" yet years from its first real-world evidence — a far stronger strategic
// signal than completion %.
export function timeToReality(d, today = new Date().toISOString().slice(0, 10)) {
  if (!hasTable(d, 'reality_milestone')) return null
  const lastL4row = hasTable(d, 'reality_event') ? d.prepare("SELECT occurred_at FROM reality_event WHERE reality_level='L4_production' ORDER BY occurred_at DESC LIMIT 1").get() : null
  const lastL4 = lastL4row?.occurred_at || null
  const daysSinceLastL4 = lastL4 ? Math.max(0, Math.floor((Date.parse(today) - Date.parse(lastL4)) / 86400000)) : null
  const ms = d.prepare('SELECT * FROM reality_milestone ORDER BY ordinal').all()
  const incomplete = ms.filter(m => m.status !== 'done')
  const next = incomplete[0] || null
  const firstL4 = ms.find(m => m.status !== 'done' && m.produces_l4 === 1) || null
  const keystone = [...ms].reverse().find(m => m.produces_l4 === 1) || null
  const effortTo = (maxOrd) => incomplete.filter(m => m.ordinal <= maxOrd).reduce((a, m) => a + (m.effort || 0), 0)
  return {
    lastL4, daysSinceLastL4, next, blocker: next?.blocked_by || null,
    firstL4Milestone: firstL4, keystoneMilestone: keystone,
    effortToFirstL4: firstL4 ? effortTo(firstL4.ordinal) : null,
    effortToKeystone: keystone ? effortTo(keystone.ordinal) : null,
    milestonesRemaining: incomplete.length, milestonesTotal: ms.length,
  }
}

// PLANNING FREEZE — the governance rule that prevents Planner Addiction (a planning brain optimizing
// itself instead of producing the result). When Reality Ratio < 1% AND an L4 path exists, the answer is
// already known (execute the path); building more planning machinery is lower-value than the first L4.
// This is NOT a new dimension — it reuses the reality engine to enforce: Reality > Planning, always.
export function planningFreeze(d) {
  const re = realityEngine(d), ttr = timeToReality(d)
  if (!re || !ttr) return null
  const frozen = re.realityRatio < 1 && ttr.milestonesRemaining > 0
  return {
    frozen, realityRatio: re.realityRatio,
    nextAction: ttr.next ? ttr.next.label : null, blocker: ttr.blocker, effortToFirstL4: ttr.effortToFirstL4,
    budget: '80% execution / 20% planning until Reality Ratio > 1%',
    rules: [
      'PLANNING FREEZE — no new planning dimensions / tracking subsystems / intelligence layers while Reality Ratio < 1% and an L4 path exists. Priority = remove blockers on the first L4.',
      'L4 FIRST — every roadmap item must answer "how does this raise L4 probability?"; if it cannot, it is demoted (reality_leverage 0).',
      'REALITY BUDGET — 80% execution / 20% planning until Reality Ratio > 1%.',
    ],
  }
}

// DIM external — the outward loop: developments that should re-score the roadmap + scan staleness.
export function externalSignals(d) {
  if (!hasTable(d, 'external_signal')) return { count: 0, byCategory: {}, recent: [], all: [], lastObserved: null }
  const all = d.prepare('SELECT * FROM external_signal ORDER BY observed_at DESC, id').all()
  const byCategory = {}; for (const r of all) byCategory[r.category] = (byCategory[r.category] || 0) + 1
  return { count: all.length, byCategory, recent: all.slice(0, 12), all, lastObserved: all[0]?.observed_at || null }
}
