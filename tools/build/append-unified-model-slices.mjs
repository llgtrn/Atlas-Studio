#!/usr/bin/env node
/**
 * append-unified-model-slices.mjs — Write the unified ERP-centered model slices
 * (S143–S195) into the slice plan, AFTER the workspace-group END sentinel, in their
 * own sentinel block (idempotent: re-running replaces the block).
 *
 * Source: .chronica-dev/unified-design-plan-v2.json (result.unified.slicePlan).
 * These are the 53 slices from the 7-cluster consolidated design pass (allocation,
 * governance, world-model, ERP cores, lenses, template store, external commerce) —
 * ERP-centered, sharing ScopePath/GovernanceLayerKind/money-gate/audit.
 *
 * Emits the exact field schema build-registry.mjs parses, with an explicit
 * **MoneyClass:** money|none marker (authoritative — set from the design's money flag).
 */
import { readFileSync, writeFileSync } from 'node:fs';
import { SLICE_PLAN } from '../_paths.mjs';

const PLAN = SLICE_PLAN;
const SRC = '.chronica-dev/unified-design-plan-v2.json';
const BEGIN = '<!-- BEGIN GENERATED unified-model (S143-S195) — tools/build/append-unified-model-slices.mjs -->';
const END = '<!-- END GENERATED unified-model -->';
const LT_END = '<!-- END GENERATED workspace-group-hierarchy -->';

const plan = JSON.parse(readFileSync(SRC, 'utf8'));
const slices = plan.slicePlan.slice().sort((a, b) => a.number - b.number);

const SIM_LABEL = {
  simulation_input: 'simulation input', simulation_engine: 'simulation engine',
  execution_action: 'execution action', measurement_output: 'measurement output',
  learning_calibration: 'learning/calibration', none: 'none',
};
const KIND_NOTE = {
  rust: 'Rust unit-tested',
  db_api_e2e: 'DB-backed / API E2E',
  docs_only: 'spec/docs',
};

function crateRefs(crateStr) {
  // "chronica-erp / chronica-finance" or "chronica-company" -> backticked list
  return crateStr.split(/[\/,]/).map((c) => '`' + c.trim() + '`').join(' + ');
}

function block(s) {
  const crates = crateRefs(s.crate);
  const sim = (s.simRelevance || []).map((x) => SIM_LABEL[x] || x).join(', ') || 'none';
  const firstCrate = s.crate.split(/[\/,]/)[0].trim().replace(/[^a-z-]/g, '');
  const verify = s.verifyCommand && /`/.test(s.verifyCommand)
    ? s.verifyCommand.replace(/`/g, '')
    : (s.verifyCommand || `cargo test -p ${firstCrate}`);
  let sec;
  if (s.money) {
    sec = '**MONEY** — moves money / commits a financial side effect; classified as a `SideEffectAction`, gated through `chronica-policy` + `chronica-approvals` at the resolving governance-layer scope (the ONE Slice-4 gate, most-restrictive across project→workspace→group; irreversible → always board), records a `CostRecord`/`FinanceEvent` with a `MoneyErpEffect` resolving to an ERP/finance object, and appends to the audit chain.';
  } else {
    sec = `read-only/internal or non-money side effect; scope-chain bounded (\`ScopePath\`), no cross-group leakage. ${(s.simRelevance || []).includes('execution_action') ? 'Cross-Workspace writes are contract-gated.' : 'Feeds the simulation/measurement loop.'}`;
  }
  return [
    `## Slice ${s.number} — ${s.title}`,
    `- **Behavior:** ${s.behavior}`,
    `- **Simulation-lifecycle relevance:** ${sim}.`,
    `- **MoneyClass:** ${s.money ? 'money' : 'none'}`,
    `- **Kind:** ${KIND_NOTE[s.kind] || s.kind}.`,
    `- **Target:** ${crates}.`,
    `- **Approval/Security:** ${sec}`,
    `- **Observability:** capability trace${s.money ? ' + CostRecord/FinanceEvent + MoneyErpEffect + audit-chain append' : ''}; `
      + `scope_path + ERP dimension refs recorded; feeds the ${sim} loop.`,
    `- **Verify:** \`${verify}\` green; ${s.kind === 'db_api_e2e' ? 'DB-backed/E2E' : 'unit'} test exercises the capability (real infra, no stub marked verified).`,
    `- **Parity:** unified-model ledger.`,
    `- **Depends on:** ${(s.dependsOn || []).map((n) => 'S' + n).join(', ') || 'S143 (shared base)'}.`,
    '',
  ].join('\n');
}

const blocks = slices.map(block);
const moneyList = slices.filter((s) => s.money).map((s) => 'S' + s.number).join(', ');

const header = [
  BEGIN,
  '',
  '---',
  '',
  `> **Unified ERP-centered model (Slices ${slices[0].number}–${slices[slices.length - 1].number}).** The 7-cluster consolidated design (allocation economics, governance map, external world model, M&A/PM template lenses, Template Store, ERPNext/Frappe/Odoo + PostHog + ClickHouse group cores, External Commerce Environment) unified into ONE system — every domain shares \`ScopePath\` + \`GovernanceLayerKind\` + the ONE money gate + the ONE audit chain, and **ERP is the central operating core** (allocation IS the ERP asset/cost-center layer; every project/template/workflow/money/commerce/analytics action links to ERP via an \`*ErpEffect\`/\`*ErpBinding\` or declares \`no_erp_impact\`). One account = one ChronicaGroup. Allocation is the DEFAULT economic mechanism (inter-company invoice = optional legal mode). Build order is the slice order (allocation S143–149 first). Money slices: ${moneyList}.`,
  '',
].join('\n');

const generated = header + '\n' + blocks.join('\n') + '\n' + END + '\n';

let doc = readFileSync(PLAN, 'utf8');
const esc = (x) => x.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
if (doc.includes(BEGIN) && doc.includes(END)) {
  doc = doc.replace(new RegExp(esc(BEGIN) + '[\\s\\S]*?' + esc(END) + '\\n?'), generated);
} else {
  const idx = doc.indexOf(LT_END);
  if (idx === -1) throw new Error('workspace-group END sentinel not found');
  const after = idx + LT_END.length;
  doc = doc.slice(0, after) + '\n\n' + generated + doc.slice(after);
}
writeFileSync(PLAN, doc, 'utf8');
console.log(`Wrote ${slices.length} unified-model slices (S${slices[0].number}–S${slices[slices.length - 1].number}) to ${PLAN}`);
console.log('money slices: ' + moneyList);
