#!/usr/bin/env node
/**
 * expand-slice-plan.mjs — Apply the long-tail donor audit synthesis to the slice plan.
 *
 * Transforms the 78 recommendedNewSlices from .chronica-dev/longtail-audit-synthesis.json
 * into properly-formatted `## Slice N — ...` blocks (renumbered S43+, after the real
 * S42) in the EXACT field schema tools/parity/build-registry.mjs parses, then appends
 * them to docs/implementation/14-vertical-slice-plan.md between two sentinel markers.
 *
 * Idempotent: re-running replaces the generated block (between the sentinels) so the
 * registry regen never double-counts. The audit's local slice-number guesses
 * (S32..S50 — collided with the real plan) are DISCARDED; canonical numbers are S43+.
 *
 * Per-slice simulation-lifecycle relevance (input/engine/execution/measurement/learning)
 * is preserved verbatim from the audit — the user's hard requirement.
 */
import { readFileSync, writeFileSync } from 'node:fs';
import { SLICE_PLAN } from '../_paths.mjs';

const SYN = '.chronica-dev/longtail-audit-synthesis.json';
const PLAN = SLICE_PLAN;
const BEGIN = '<!-- BEGIN GENERATED long-tail-expansion (S43+) — tools/build/expand-slice-plan.mjs -->';
const END = '<!-- END GENERATED long-tail-expansion -->';
const START_NUM = 43; // first canonical number after the real S42 (Debate/Evidence Judge)

const syn = JSON.parse(readFileSync(SYN, 'utf8')).result.synthesis;
const raw = syn.recommendedNewSlices;

// Money / side-effect keyword detection (mirrors build-registry.mjs intent, applied to
// the slice's full text so the registry's MONEY_RE/SIDEEFFECT_RE classify correctly).
const MONEY_RE = /\b(money|spend|payment|payout|order|purchase|invoic|price|pricing|ad spend|budget|charge|refund|trade|trading|broker|portfolio|VaR|slippage)\b/i;
const SIDEEFFECT_RE = /\b(execut|dispatch|deploy|provision|publish|send|submit|notif|order|rotat|outreach|distribut)\b/i;

const SIM_LABEL = {
  simulation_input: 'simulation input',
  simulation_engine: 'simulation engine',
  execution_action: 'execution action',
  measurement_output: 'measurement output',
  learning_calibration: 'learning/calibration',
  none: 'none',
};

function parse(entry) {
  // <Name> [<crates> | <sim>] (<donor>: <desc>)  [optional trailing flags]
  const m = entry.match(/^(.+?)\s*\[([^|]+)\|([^\]]+)\]\s*\(([^)]*)\)\s*(.*)$/);
  if (!m) throw new Error('unparseable slice entry: ' + entry);
  let [, name, cratesRaw, simRaw, donorBlob, trailing] = m;
  // strip an audit-local "Sxx-" / "Slice xx —" prefix from the NAME (not canonical)
  name = name.replace(/^S\d+(?:\.\d+)?[A-Z]?-/i, '').replace(/^Slice\s+\d+(?:\.\d+)?[A-Z]?\s*[—-]\s*/i, '').trim();
  const crateTokens = cratesRaw
    .split('/')
    .map((c) => c.trim())
    .map((c) => c.replace(/\s*\(NEW CRATE\)\s*/i, '').trim())
    .filter(Boolean);
  const newCrate = /\(NEW CRATE\)/i.test(cratesRaw);
  const sim = simRaw.split(',').map((x) => x.trim()).filter(Boolean);
  // donorBlob: "prometheus: scraper lifecycle ..."  OR  "FinceptTerminal: S33.5 — broker ..."
  const dm = donorBlob.match(/^([^:]+):\s*(.*)$/);
  const donor = dm ? dm[1].trim() : donorBlob.trim();
  let desc = dm ? dm[2].trim() : '';
  // strip an audit-local "Sxx.x —" / "Slice xx.x —" prefix from the description (not canonical)
  desc = desc.replace(/^(?:Slice\s+|S)?\d+(?:\.\d+)?[A-Z]?\s*[—-]\s*/i, '').trim();
  const moneyFlag = /MONEY/i.test(trailing) || MONEY_RE.test(name + ' ' + desc + ' ' + donor);
  return { name: name.trim(), crateTokens, newCrate, sim, donor, desc, trailing: trailing.trim(), moneyFlag };
}

function titleCase(name) {
  // "Metrics-Scraper-Service-Discovery" -> "Metrics Scraper Service Discovery"
  return name.replace(/-/g, ' ').replace(/\s+/g, ' ').trim();
}

function block(n, p) {
  const title = titleCase(p.name);
  const crates = p.crateTokens.map((c) => '`' + c + '`').join(' + ');
  const simStr = p.sim.map((x) => SIM_LABEL[x] || x).join(', ');
  const isMoney = p.moneyFlag;
  const isSideEffect = isMoney || SIDEEFFECT_RE.test(p.name + ' ' + p.desc) || p.sim.includes('execution_action');
  // Approval/Security line: money slices are gated; execution_action slices are side-effecting.
  let sec;
  if (isMoney) {
    sec = '**MONEY** — this capability can move money or commit a financial side effect; it is a `SideEffectAction` gated through `chronica-policy` + `chronica-approvals` (dollar threshold + simulation RiskClass per [../architecture/13-simulation-lifecycle.md](../architecture/13-simulation-lifecycle.md)), records a `CostRecord`, and appends to the audit chain.';
  } else if (isSideEffect) {
    sec = 'side-effecting (`execution_action`) — any external write/dispatch is approval-gated; reads are company-scoped.';
  } else {
    sec = 'read-only/internal; company-scoped access. Feeds the simulation/measurement loop, never the money-write path.';
  }
  const newCrateNote = p.newCrate
    ? ` _(requires standing up the new \`${p.crateTokens[0]}\` crate — higher effort.)_`
    : '';
  const verify = isMoney
    ? `\`cargo test -p ${p.crateTokens[0].replace(/[^a-z-]/g, '')}\` green; a money/side-effect path raises an approval above threshold and is blocked on negative simulated EV.`
    : `\`cargo test -p ${p.crateTokens[0].replace(/[^a-z-]/g, '')}\` green; the capability produces its ${simStr || 'output'} and is exercised by a unit/integration test.`;
  return [
    `## Slice ${n} — ${title}`,
    `- **Behavior:** ${p.desc || title}. (Long-tail absorption of **${p.donor}**.)`,
    `- **Simulation-lifecycle relevance:** ${simStr || 'none'}.`,
    `- **Donors (native_absorption):** ${p.donor} → ${crates}.${newCrateNote}`,
    `- **Target:** ${crates}.`,
    `- **Approval/Security:** ${sec}`,
    `- **Observability:** capability trace${isMoney ? ' + CostRecord + audit-chain append' : ''}; feeds the ${simStr || 'measurement'} loop.`,
    `- **Verify:** ${verify}`,
    `- **Parity:** long-tail-expansion ledger; absorbs ${p.donor}.${p.trailing && !/MONEY/i.test(p.trailing) ? ' ' + p.trailing : ''}`,
    '',
  ].join('\n');
}

const blocks = raw.map((entry, i) => block(START_NUM + i, parse(entry)));

const header = [
  BEGIN,
  '',
  '---',
  '',
  `> **Long-tail donor absorption expansion (Slices ${START_NUM}–${START_NUM + raw.length - 1}).** Generated from the 48-donor long-tail audit (\`.chronica-dev/longtail-audit-synthesis.json\`, ${syn.totalDonorsAudited} donors, overall ${syn.overallCoveragePercent}% prior coverage). These ${raw.length} slices bring the plan to **~${syn.totalSliceCountTarget} slices** for TRUE full absorption of all 74 donors at the feature level (not capability-cluster level). Each slice carries its **simulation-lifecycle relevance** per the user's hard requirement. The biggest structural gap was an entire **observability pillar** (Prometheus ecosystem + ClickHouse analytics, 15 slices) — sequence it first; it is the foundational \`measurement_output\` substrate for the simulation/calibration loop. The **agent-runtime cluster** (openfang/openclaw/zeroclaw/9router/opencode/zed/OpenHarness/everything-claude-code — 8 donors) generated **ZERO** new slices: every recommendation re-pointed at existing slices (S4/S6/S8/S13/S14/S37), confirming the core spine is correctly scoped. \`ladybird\` (standalone browser engine, 5%) and desktop-UI shells are reference-only, not absorbed now.`,
  '',
].join('\n');

const generated = header + '\n' + blocks.join('\n') + '\n' + END + '\n';

let plan = readFileSync(PLAN, 'utf8');
if (plan.includes(BEGIN) && plan.includes(END)) {
  plan = plan.replace(new RegExp(BEGIN.replace(/[.*+?^${}()|[\]\\]/g, '\\$&') + '[\\s\\S]*?' + END.replace(/[.*+?^${}()|[\]\\]/g, '\\$&') + '\\n?'), generated);
} else {
  // insert before the "## Slice sequencing & parallelism" closing section
  const anchor = '## Slice sequencing & parallelism';
  const idx = plan.indexOf(anchor);
  if (idx === -1) { plan = plan.trimEnd() + '\n\n' + generated; }
  else { plan = plan.slice(0, idx) + generated + '\n' + plan.slice(idx); }
}
writeFileSync(PLAN, plan, 'utf8');
console.log(`Applied ${raw.length} long-tail slices (S${START_NUM}–S${START_NUM + raw.length - 1}) to ${PLAN}`);
