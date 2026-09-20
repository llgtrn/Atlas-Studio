#!/usr/bin/env node
// backend-truth-phase6-plans.mjs — Phase 6.3 reconstruction plans for debt>50 crates.
//
// Joins the already-gathered audit data (no new analysis, no build):
//   04-verdicts.json        — refactor_priority_score, architecture_debt_score, verdict per crate
//   22-god-file-analysis.json — god-file A/B/C/D types per crate
//   01-per-crate/*.json     — quality (unwrap_nontest, todo, circular, hidden) + architecture
// → 23-reconstruction-plans.json: a per-crate Phase-7 plan, ordered by Refactor Priority Score.
//
// Threshold: architecture_debt_score > 50 (program §6.3). RULE B: these are PLANS, not edits — Phase 7
// executes them per crate only after that crate's Phases 1-6 are complete, money files financial-control-
// gated, file-splits in a separate commit from logic.
//
// Usage: node tools/capabilities/backend-truth-phase6-plans.mjs

import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..', '..');
const OUT = path.join(ROOT, 'docs', 'backend-truth-audit');
const read = (p) => JSON.parse(fs.readFileSync(p, 'utf8'));

const verdicts = read(path.join(OUT, '04-verdicts.json')).ledger;
const godAll = read(path.join(OUT, '22-god-file-analysis.json')).files;
const THRESHOLD = 50;

const godByCrate = {};
for (const g of godAll) (godByCrate[g.crate] ??= []).push(g);

const plans = [];
for (const v of verdicts) {
  if ((v.architecture_debt_score || 0) <= THRESHOLD) continue;
  const skel = read(path.join(OUT, '01-per-crate', `${v.crate}.json`));
  const q = skel.quality;
  const gfs = godByCrate[v.crate] || [];
  const byType = { A: 0, B: 0, C: 0, D: 0 };
  for (const g of gfs) byType[g.type[0]]++;

  const actions = [];
  if (byType.A) actions.push(`Extract ${byType.A} inline test module(s) into sibling *_tests.rs (mechanical, separate commit, no logic change).`);
  for (const g of gfs.filter((x) => x.type.startsWith('B_'))) actions.push(`RELOCATE ${g.file}: ${g.recommendation}`);
  for (const g of gfs.filter((x) => x.type.startsWith('C_'))) actions.push(`ABSTRACT ${g.file}: ${g.recommendation}`);
  if (q.unwrap_nontest > 20) actions.push(`Triage ${q.unwrap_nontest} non-test unwrap()/expect() (heuristic upper bound; replace genuinely-fallible ones with ? / typed errors; many are inline test helpers above #[cfg(test)]).`);
  if (gfs.some((x) => x.type.startsWith('D_'))) actions.push(`${gfs.filter((x) => x.type.startsWith('D_')).length} god file(s) are Type-D (legitimate complexity, incl. money seams) — DOCUMENT + EXEMPT, do NOT split.`);

  const effort = byType.B || byType.C ? 'LARGE' : byType.A > 4 || q.unwrap_nontest > 80 ? 'MEDIUM' : 'SMALL';
  plans.push({
    crate: v.crate,
    verdict: v.verdict,
    refactor_priority_score: v.refactor_priority_score,
    architecture_debt_score: v.architecture_debt_score,
    debt_breakdown: {
      god_files_total: gfs.length,
      god_files_by_type: byType,
      unwrap_nontest: q.unwrap_nontest,
      todo_fixme_hack: q.todo_fixme_hack_count,
      circular_dependencies: skel.architecture.circular_dependencies.length,
      hidden_dependencies: skel.architecture.hidden_dependencies.length,
    },
    reconstruction_plan: actions,
    money_sensitive: v.owner_pillars?.some((p) => ['Payment', 'Policy', 'ERP'].includes(p)) || false,
    estimated_effort: effort,
    rule_b_gate: 'Phase 7 only after this crate completes Phases 1-6; RULE E financial-control test green before+after any money-file change.',
  });
}

plans.sort((a, b) => b.refactor_priority_score - a.refactor_priority_score);
fs.writeFileSync(
  path.join(OUT, '23-reconstruction-plans.json'),
  JSON.stringify(
    {
      generated_by: 'backend-truth-phase6-plans.mjs',
      threshold: `architecture_debt_score > ${THRESHOLD}`,
      count: plans.length,
      note: 'PLANS only (RULE B). Ordered by Refactor Priority Score (§7.1). File-splits are a separate commit from logic; money/gate/audit files need a financial-control test green before AND after.',
      plans,
    },
    null,
    2,
  ) + '\n',
);

console.log(`Phase 6.3: ${plans.length} reconstruction plans (arch_debt > ${THRESHOLD}).`);
for (const p of plans) console.log(`  rps=${p.refactor_priority_score.toString().padStart(5)} arch_debt=${p.architecture_debt_score.toString().padStart(3)} ${p.crate.padEnd(24)} ${p.estimated_effort.padEnd(6)} ${p.money_sensitive ? '[MONEY-SENSITIVE]' : ''} A/B/C/D=${Object.values(p.debt_breakdown.god_files_by_type).join('/')}`);
