#!/usr/bin/env node
// backend-truth-phase2.mjs — Phase 2 architecture.db reconciliation (doc-level projection).
//
// Projects the Phase-1 verdict data (docs/backend-truth-audit/01-per-crate/*.json) into:
//   arch-reconciliation/decision-memory.json   — one §2.2 Decision Memory record per crate
//   arch-reconciliation/arch-node-truth.json    — §2.1 extended node fields, ready to write to
//                                                 architecture.db when disk permits (the heavy DB
//                                                 write is DEFERRED — disk at 100%).
//
// RULE C: architecture.db is an OUTPUT verified against code reality. This script does NOT touch the
// 894MB-class DB; it produces the small-JSON reconciliation that a later `arch:build` will absorb.
//
// Usage: node tools/capabilities/backend-truth-phase2.mjs

import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..', '..');
const OUT = path.join(ROOT, 'docs', 'backend-truth-audit');
const PER_CRATE = path.join(OUT, '01-per-crate');
const ARCH = path.join(OUT, 'arch-reconciliation');

const read = (p) => fs.readFileSync(p, 'utf8');
const isBinary = (crate) => /\[\[bin\]\]/.test((() => { try { return read(path.join(ROOT, 'crates', crate, 'Cargo.toml')); } catch { return ''; } })());

const DECIDED_BY = {
  Doctrine: 'Planning Brain',
  'Planning Brain': 'Planning Brain',
  'Donor Origin': 'Donor Mining',
  Unknown: 'Unknown',
};
const TRUTH_VERDICT = {
  JUSTIFIED_AND_SOUND: 'JUSTIFIED',
  JUSTIFIED_BUT_UNSOUND: 'JUSTIFIED',
  UNJUSTIFIED: 'UNJUSTIFIED',
  DUPLICATE: 'DUPLICATE',
  DEPRECATED: 'DEPRECATED',
};

function confidenceOf(reality, doctrine) {
  const lo = Math.min(reality ?? 0, doctrine ?? 0);
  if (lo >= 85) return 'HIGH';
  if (lo >= 60) return 'MEDIUM';
  return 'LOW';
}

function runtimeEvidence(skel, crate) {
  const by = skel.architecture.depended_on_by || [];
  const tests = skel.quality?.test_coverage_files ?? 0;
  if (by.length > 0) return { evidence: `called by ${by.length} crate(s): ${by.join(', ')}`, flag: null };
  if (isBinary(crate)) return { evidence: 'binary entrypoint (no in-workspace dependents by design)', flag: null };
  // depended_on_by empty and not a binary. Distinguish unwired-but-tested (capability frontier) from dead.
  if (tests > 0) {
    return {
      evidence: `hermetic tests only (${tests} test file(s)); NO in-workspace dependents — capability crate not yet wired into the api/runtime binary (verify intended integration point)`,
      flag: 'UNWIRED_CAPABILITY',
    };
  }
  return { evidence: 'NO_RUNTIME_EVIDENCE — no dependents, not a binary, no tests', flag: 'NO_RUNTIME_EVIDENCE' };
}

function main() {
  fs.mkdirSync(ARCH, { recursive: true });
  const files = fs.readdirSync(PER_CRATE).filter((f) => f.endsWith('.json')).sort();
  const decisions = [];
  const nodes = [];
  let unjustified = 0, dup = 0, unsound = 0, unwired = 0, runtimeGaps = 0;

  for (const f of files) {
    const skel = JSON.parse(read(path.join(PER_CRATE, f)));
    const crate = skel.crate;
    const p = skel.purpose || {};
    const d = skel.donor_origins || {};
    const s = skel.scores || {};
    const donors = (d.primary_donors || []);
    const retained = (d.donor_assumptions_retained || []);
    const truth = TRUTH_VERDICT[skel.verdict] || 'UNAUDITED';
    const rt = runtimeEvidence(skel, crate);
    if (truth === 'UNJUSTIFIED') unjustified++;
    if (skel.verdict === 'DUPLICATE') dup++;
    if (skel.verdict === 'JUSTIFIED_BUT_UNSOUND') unsound++;
    if (rt.flag === 'UNWIRED_CAPABILITY') unwired++;
    if (rt.flag === 'NO_RUNTIME_EVIDENCE') runtimeGaps++;

    const primaryAssumption = retained[0]
      || (donors.length
        ? `the ${donors[0]} donor pattern is the right shape for this domain in Chronica`
        : 'this domain warrants a dedicated crate boundary');

    decisions.push({
      decision_id: `DM-${crate.replace('chronica-', '').toUpperCase()}-001`,
      crate,
      decision_type: 'ARCHITECTURE',
      decision_date: 'UNKNOWN',
      decided_by: DECIDED_BY[p.justification_source] || 'Unknown',
      decision: `Maintain a dedicated crate for: ${p.business_purpose || p.technical_purpose || crate}`,
      alternatives_considered:
        skel.verdict === 'DUPLICATE'
          ? [`Fold into the crate that already owns this (${(skel.verdict_reason || '').includes('chronica-runtime') ? 'chronica-runtime' : 'the owning crate'})`]
          : [`Merge into a neighboring domain crate`, `Inline as a module of a consumer crate`],
      reason_chosen: p.business_purpose || 'PENDING',
      primary_assumption: primaryAssumption,
      assumption_confidence: confidenceOf(s.reality_score, s.doctrine_compliance_score),
      assumption_invalidation_trigger:
        skel.verdict === 'DUPLICATE'
          ? 'Another crate already owns this capability and is the one used at runtime → this crate is redundant.'
          : skel.verdict === 'JUSTIFIED_BUT_UNSOUND'
            ? 'The crate is not wired to any live runtime path / lacks the durable backend it claims.'
            : 'A consumer stops needing this boundary, or the domain collapses into a neighbor.',
      outcome_after:
        skel.verdict === 'JUSTIFIED_AND_SOUND' ? 'POSITIVE'
          : skel.verdict === 'JUSTIFIED_BUT_UNSOUND' ? 'MIXED'
            : skel.verdict === 'DUPLICATE' ? 'NEGATIVE' : 'UNKNOWN',
    });

    nodes.push({
      node: `crate:${crate}`,
      crate,
      decision_id: `DM-${crate.replace('chronica-', '').toUpperCase()}-001`,
      why_created: p.business_purpose || 'PENDING',
      primary_assumption: primaryAssumption,
      assumption_last_challenged: '2026-06-21',
      donor_source: donors.join(', ') || 'native',
      business_value: s.business_value_reason || 'PENDING',
      runtime_evidence: rt.evidence,
      truth_verdict: truth,
      truth_audit_date: '2026-06-21',
      debt_score: s.debt_score ?? 0,
      reality_score: s.reality_score ?? 0,
      doctrine_compliance_score: s.doctrine_compliance_score ?? 0,
      flags: [
        ...(skel.verdict === 'DUPLICATE' ? ['DUPLICATE'] : []),
        ...(skel.verdict === 'JUSTIFIED_BUT_UNSOUND' ? ['UNSOUND_REALITY'] : []),
        ...(rt.flag ? [rt.flag] : []),
      ],
    });
  }

  fs.writeFileSync(
    path.join(ARCH, 'decision-memory.json'),
    JSON.stringify({ generated_by: 'backend-truth-phase2.mjs', schema: 'program §2.2', count: decisions.length, decisions }, null, 2) + '\n',
  );
  fs.writeFileSync(
    path.join(ARCH, 'arch-node-truth.json'),
    JSON.stringify(
      {
        generated_by: 'backend-truth-phase2.mjs',
        schema: 'program §2.1 (architecture_nodes extended truth fields — ready for arch:build, DB write DEFERRED on full disk)',
        count: nodes.length,
        summary: { unjustified, duplicate: dup, unsound, unwired_capability: unwired, runtime_gaps: runtimeGaps },
        nodes,
      },
      null,
      2,
    ) + '\n',
  );

  console.log(`Phase 2: ${decisions.length} Decision-Memory records + ${nodes.length} arch-node truth records.`);
  console.log(`  truth_verdict: UNJUSTIFIED=${unjustified} DUPLICATE=${dup} UNSOUND=${unsound} UNWIRED_CAPABILITY=${unwired} NO_RUNTIME_EVIDENCE=${runtimeGaps}`);
  console.log('  flagged nodes:');
  for (const n of nodes.filter((x) => x.flags.length)) console.log(`    ${n.crate.padEnd(28)} [${n.flags.join(',')}]`);
}

main();
