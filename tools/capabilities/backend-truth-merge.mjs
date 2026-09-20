#!/usr/bin/env node
// backend-truth-merge.mjs — merge the Phase-1 reasoning verdicts into the per-crate truth records.
//
// The deterministic engine (backend-truth-audit.mjs) leaves reasoning fields PENDING_PLANNING_BRAIN.
// The reasoning pass (a Workflow, one agent per crate, grounded in the measured skeletons) produced
// docs/backend-truth-audit/phase1-verdicts.json. This script folds those verdicts INTO the skeletons,
// computes the Phase-7 Refactor Priority Score per program §7.1, writes a consolidated verdict ledger
// (04-verdicts.json, sorted by refactor priority desc), and prints the verdict distribution + flags.
//
// Refactor Priority = arch_debt*0.30 + (100-reality)*0.25 + (100-doctrine)*0.25 + (100-business_value)*0.20
//
// Usage: node tools/capabilities/backend-truth-merge.mjs

import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..', '..');
const OUT = path.join(ROOT, 'docs', 'backend-truth-audit');
const PER_CRATE = path.join(OUT, '01-per-crate');

const verdicts = JSON.parse(fs.readFileSync(path.join(OUT, 'phase1-verdicts.json'), 'utf8'));
const ledger = [];
const dist = {};

for (const v of verdicts) {
  const skelPath = path.join(PER_CRATE, `${v.crate}.json`);
  if (!fs.existsSync(skelPath)) {
    console.error(`! no skeleton for ${v.crate} — skipped`);
    continue;
  }
  const skel = JSON.parse(fs.readFileSync(skelPath, 'utf8'));

  skel.purpose = {
    business_purpose: v.business_purpose,
    technical_purpose: v.technical_purpose,
    is_justified: v.is_justified,
    justification_source: v.justification_source,
  };
  skel.donor_origins = {
    primary_donors: v.primary_donors || [],
    donor_logic_challenged: !!v.donor_logic_challenged,
    donor_assumptions_retained: v.donor_assumptions_retained || [],
    donor_assumptions_rejected: v.donor_assumptions_rejected || [],
  };
  skel.scores.reality_score = v.reality_score;
  skel.scores.doctrine_compliance_score = v.doctrine_compliance_score;
  skel.scores.business_value_score = v.business_value_score;
  skel.scores.business_value_reason = v.business_value_reason;
  skel.verdict = v.verdict;
  skel.verdict_reason = v.verdict_reason;
  skel.owner_pillars = v.owner_pillars || [];

  const archDebt = skel.scores.architecture_debt_score || 0;
  const rps = +(
    archDebt * 0.3 +
    (100 - v.reality_score) * 0.25 +
    (100 - v.doctrine_compliance_score) * 0.25 +
    (100 - v.business_value_score) * 0.2
  ).toFixed(1);
  skel.scores.refactor_priority_score = rps;
  skel.scores.refactor_priority_basis =
    'arch_debt*0.30 + (100-reality)*0.25 + (100-doctrine)*0.25 + (100-business_value)*0.20';

  fs.writeFileSync(skelPath, JSON.stringify(skel, null, 2) + '\n');

  dist[v.verdict] = (dist[v.verdict] || 0) + 1;
  ledger.push({
    crate: v.crate,
    verdict: v.verdict,
    is_justified: v.is_justified,
    reality: v.reality_score,
    doctrine: v.doctrine_compliance_score,
    business_value: v.business_value_score,
    debt_score: skel.scores.debt_score,
    architecture_debt_score: archDebt,
    refactor_priority_score: rps,
    owner_pillars: v.owner_pillars || [],
    verdict_reason: v.verdict_reason,
  });
}

ledger.sort((a, b) => b.refactor_priority_score - a.refactor_priority_score);
fs.writeFileSync(
  path.join(OUT, '04-verdicts.json'),
  JSON.stringify(
    {
      generated_by: 'backend-truth-merge.mjs',
      crate_count: ledger.length,
      verdict_distribution: dist,
      refactor_priority_formula:
        'arch_debt*0.30 + (100-reality)*0.25 + (100-doctrine)*0.25 + (100-business_value)*0.20',
      ledger,
    },
    null,
    2,
  ) + '\n',
);

// Console report.
const order = ['JUSTIFIED_AND_SOUND', 'JUSTIFIED_BUT_UNSOUND', 'UNJUSTIFIED', 'DUPLICATE', 'DEPRECATED'];
console.log('Verdict distribution:');
for (const k of order) console.log(`  ${(dist[k] || 0).toString().padStart(3)}  ${k}`);
const flagged = ledger.filter((l) => l.verdict !== 'JUSTIFIED_AND_SOUND');
console.log(`\nNot-clean (${flagged.length}):`);
for (const l of flagged) console.log(`  ${l.verdict.padEnd(22)} ${l.crate.padEnd(26)} rps=${l.refactor_priority_score} doctrine=${l.doctrine}`);
const lowDoctrine = ledger.filter((l) => l.doctrine < 90).sort((a, b) => a.doctrine - b.doctrine);
console.log(`\nDoctrine < 90 (money/reuse scrutiny — ${lowDoctrine.length}):`);
for (const l of lowDoctrine) console.log(`  doctrine=${l.doctrine}  ${l.crate}`);
console.log('\nTop 8 Phase-7 refactor priority:');
for (const l of ledger.slice(0, 8)) console.log(`  ${l.refactor_priority_score.toString().padStart(5)}  ${l.crate.padEnd(26)} (${l.verdict})`);
