#!/usr/bin/env node
/**
 * append-workspace-group-slices.mjs — Append the Workspace/Group hierarchy slices
 * (S121–S142) to the slice plan, AFTER the END-GENERATED long-tail sentinel so the
 * generator tools/build/expand-slice-plan.mjs stays idempotent and never renumbers them.
 *
 * Source: .chronica-dev/workspace-holding-model-changeplan.json (result.synth.newSlices).
 * These are HAND-AUTHORED canonical slices (the model correction), not generated from a
 * donor audit — but written through this script so the field schema build-registry.mjs
 * parses is exact and consistent. Idempotent: replaces the block between its own sentinels.
 *
 * Money slices MUST contain the literal '**MONEY**' + an explicit money word
 * (invoice/transfer/budget/spend/ledger/capital) so MONEY_RE/APPROVAL_RE in
 * build-registry.mjs classify movesMoney=true (the note: 'transfer' alone is not in MONEY_RE).
 */
import { readFileSync, writeFileSync } from 'node:fs';
import { SLICE_PLAN } from '../_paths.mjs';

const CHANGE = '.chronica-dev/workspace-holding-model-changeplan.json';
const PLAN = SLICE_PLAN;
const BEGIN = '<!-- BEGIN GENERATED workspace-group-hierarchy (S121-S142) — tools/build/append-workspace-group-slices.mjs -->';
const END = '<!-- END GENERATED workspace-group-hierarchy -->';

const newSlices = JSON.parse(readFileSync(CHANGE, 'utf8')).result.synth.newSlices;

const SIM_LABEL = {
  simulation_input: 'simulation input',
  simulation_engine: 'simulation engine',
  execution_action: 'execution action',
  measurement_output: 'measurement output',
  learning_calibration: 'learning/calibration',
  none: 'none',
};

// Explicit per-slice dependency edges (from the change plan toolingImpact note). These
// are written into the **Depends on:** line for human readability; the machine-enforced
// edges live in build-registry.mjs DEPS (added separately) so both stay in sync.
const DEPS_TEXT = {
  121: 'S133 (Workspace identity), S1 (company core)',
  122: 'S121 (Workspace core), S1',
  123: 'S121, S3 (project/template core)',
  124: 'S121, S15 (observability)',
  125: 'S121, S4 (money gate), S13 (workflows)',
  126: 'S121, S5 (artifacts), S16 (knowledge)',
  127: 'S121, S4',
  128: 'S121',
  129: 'S128 (Group), S15, S4',
  130: 'S127 (contract), S4',
  131: 'S121, S16',
  132: 'S128, S41 (Simulation Mind)',
  133: 'S1 (company core)',
  134: 'S123 (attachment), S13',
  135: 'S121, S4, S15',
  136: 'S121, S6 (runtime)',
  137: 'S37 (replication), S128, S132',
  138: 'S38 (RAG), S131',
  139: 'S39 (learning loop), S131',
  140: 'S40 (self-evolving), S131, S134',
  141: 'S41/S42 (sim/debate), S132',
  142: 'S121, S4',
};

function crateRefs(crateStr) {
  // "chronica-company, chronica-core" -> backticked list
  return crateStr.split(',').map((c) => '`' + c.trim() + '`').join(' + ');
}

function block(s) {
  const crates = crateRefs(s.crate);
  const sim = (s.simulationRelevance || []).map((x) => SIM_LABEL[x] || x).join(', ') || 'none';
  const firstCrate = s.crate.split(',')[0].trim().replace(/[^a-z-]/g, '');
  let sec;
  if (s.money) {
    sec = '**MONEY** — moves money / commits an inter-company financial side effect (inter-company invoice/transfer, group budget ceiling, or group capital allocation). Classified as a `SideEffectAction`, gated through `chronica-policy` + `chronica-approvals` at workspace/group scope (most-restrictive threshold across project→workspace→group; inter-company invoice/transfer & group capital allocation are reversible:false → ALWAYS board-gated regardless of amount), records a `CostRecord`/`FinanceEvent`, and appends to BOTH the Workspace and Group audit chains (per [../architecture/13-simulation-lifecycle.md](../architecture/13-simulation-lifecycle.md) the RISK axis also fires at group scope).';
  } else if ((s.simulationRelevance || []).includes('execution_action')) {
    sec = 'side-effecting (`execution_action`) — any cross-Workspace write/dispatch is contract-gated (`WorkspaceContract`) and approval-gated; reads are scope-chain bounded (no cross-group leakage).';
  } else {
    sec = 'read-only/internal; scope-chain bounded (project/workspace/group). No cross-group leakage; feeds the simulation/measurement loop, never the money-write path.';
  }
  const verify = s.money
    ? `\`cargo test -p ${firstCrate}\` green; an inter-company money path (invoice/transfer/budget) raises a board approval at workspace/group scope and writes offsetting FinanceEvents + a dual-chain audit entry.`
    : `\`cargo test -p ${firstCrate}\` green; the capability is exercised at project/workspace/group scope by a unit/integration test (scope isolation + no cross-group leakage where applicable).`;
  return [
    `## Slice ${s.number} — ${s.title}`,
    `- **Behavior:** ${s.behavior}`,
    `- **Simulation-lifecycle relevance:** ${sim}.`,
    `- **MoneyClass:** ${s.money ? 'money' : 'none'}`,
    `- **Target:** ${crates}.`,
    `- **Approval/Security:** ${sec}`,
    `- **Observability:** capability trace${s.money ? ' + CostRecord/FinanceEvent + dual (Workspace+Group) audit-chain append' : ''}; scope_path recorded; feeds the ${sim} loop.`,
    `- **Verify:** ${verify}`,
    `- **Parity:** workspace-group-hierarchy ledger.`,
    `- **Depends on:** ${DEPS_TEXT[s.number] || 'S121 (Workspace core)'}.`,
    '',
  ].join('\n');
}

// Order S121..S142 numerically for readability.
const ordered = [...newSlices].sort((a, b) => a.number - b.number);
const blocks = ordered.map(block);

const header = [
  BEGIN,
  '',
  '---',
  '',
  '> **Workspace / Group hierarchy expansion (Slices 121–142).** Implements the DEFINING model correction (per [../foundation/00-product-identity-reconciliation.md](../foundation/00-product-identity-reconciliation.md) invariant #6): a **Workspace** is a *running operating company* (not a project); **CompanyTemplate** frames a whole Workspace; **ProjectTemplate** is an attachable *module* (a Workspace runs MANY); **ChronicaGroup**/**GroupTemplate** add the holding-company tier. ERP/analytics/approval/memory/simulation/audit become **hierarchical cores** (project/workspace/group). Cross-Workspace collaboration is **contract-based**; inter-company money is Slice-4-gated at workspace/group scope. **S133 (Workspace identity) is the architectural anchor** and precedes S121–S132, S134–S142. Money slices (S125, S127, S129, S130, S135, S142) all route through S4.',
  '',
].join('\n');

const generated = header + '\n' + blocks.join('\n') + '\n' + END + '\n';

let plan = readFileSync(PLAN, 'utf8');
if (plan.includes(BEGIN) && plan.includes(END)) {
  const esc = (x) => x.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
  plan = plan.replace(new RegExp(esc(BEGIN) + '[\\s\\S]*?' + esc(END) + '\\n?'), generated);
} else {
  // Insert AFTER the long-tail END sentinel (so it sits after S120, before sequencing section).
  const ltEnd = '<!-- END GENERATED long-tail-expansion -->';
  const idx = plan.indexOf(ltEnd);
  if (idx === -1) throw new Error('long-tail END sentinel not found; cannot place workspace-group block');
  const after = idx + ltEnd.length;
  plan = plan.slice(0, after) + '\n\n' + generated + plan.slice(after);
}
writeFileSync(PLAN, plan, 'utf8');
console.log(`Appended ${ordered.length} workspace/group slices (S${ordered[0].number}–S${ordered[ordered.length - 1].number}) to ${PLAN}`);
console.log('money slices: ' + ordered.filter((s) => s.money).map((s) => 'S' + s.number).join(', '));
