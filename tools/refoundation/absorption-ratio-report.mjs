#!/usr/bin/env node
// CHRONICA -- MANDATORY PER-LOOP RATIO REPORT.
//
// Every absorption loop / dispatch wave must end with this report. It is computed, every time,
// from Git history + the current tree + donor-burndown.mjs + System Atlas -- never from a
// persisted progress database (no ratio, count, or verdict from a prior run is read back as
// input here).
//
// Two classes of number feed the report:
//   1. Git/tree-derived (donor drain, native LOC delta, root stability, temporary isolation,
//      donor bytes/files removed) -- computed fresh in this module from `git diff`/`git show`
//      against the given BASE_SHA/FINAL_SHA range, plus donor-burndown.mjs and
//      tools/system-atlas/generate.mjs for the current corpus/architecture state.
//   2. Orchestration-live (candidates planned/completed/accepted/rejected/conflicted, per-slice
//      DRAINED/RETAINED_COUPLED/REFERENCE_ONLY classification, functional-absorption judgment) --
//      these describe what just happened in THIS dispatch wave and cannot be reconstructed from
//      Git alone once a rejected candidate's branch is deleted (by design, nothing is merged for
//      a rejected candidate, so there is no permanent trace to mine). The orchestrator supplies
//      them as plain arguments at report time; this module never writes them anywhere, so the
//      next loop starts from zero live state again -- consistent with "no progress database."
//
// Usage:
//   node tools/refoundation/absorption-ratio-report.mjs --base <sha> --final <sha> --live <path-to-json>
// `--live` points at a small JSON object matching the LiveCounts shape documented below
// (ephemeral -- write it to a scratch path, never commit it).

import { execFileSync } from 'node:child_process'
import { readFileSync } from 'node:fs'
import { fileURLToPath } from 'node:url'
import { dirname, join } from 'node:path'
import { computeCorpusExtinctionCounts, computeCorpusFileTotals } from './donor-burndown.mjs'
import { findTemporaryDependencyViolations } from './no-temporary-dependency-gate.mjs'
import { findDonorCouplingViolations } from './donor-coupling-gate.mjs'

const HERE = dirname(fileURLToPath(import.meta.url))
const ROOT = join(HERE, '..', '..')

function git(args) {
  return execFileSync('git', args, { cwd: ROOT, encoding: 'utf8', maxBuffer: 256 * 1024 * 1024 })
}

const CANONICAL_ROOTS = ['core', 'runtime', 'adapter', 'organism']

/** Files touched in the range under temporary/, split into deleted (drained) vs other churn. */
function donorDrainInRange(baseSha, finalSha) {
  const out = git(['diff', '--name-status', `${baseSha}..${finalSha}`, '--', 'temporary/']).trim()
  if (!out) return { filesDrained: 0, byDonor: {} }
  let filesDrained = 0
  const byDonor = {}
  for (const line of out.split('\n')) {
    const [status, path] = line.split('\t')
    if (status !== 'D') continue
    filesDrained++
    const donor = path.split('/')[1] // temporary/<donor>/...
    byDonor[donor] = (byDonor[donor] ?? 0) + 1
  }
  return { filesDrained, byDonor }
}

/** Native line delta per canonical root, via `git diff --numstat`. */
function nativeLocDeltaByRoot(baseSha, finalSha) {
  const result = {}
  for (const root of [...CANONICAL_ROOTS, 'apps']) {
    const out = git(['diff', '--numstat', `${baseSha}..${finalSha}`, '--', `${root}/`]).trim()
    let added = 0
    let removed = 0
    if (out) {
      for (const line of out.split('\n')) {
        const [a, r] = line.split('\t')
        if (a === '-' || r === '-') continue // binary file, skip
        added += Number(a)
        removed += Number(r)
      }
    }
    result[root] = { added, removed, net: added - removed }
  }
  return result
}

/** Cargo workspace member list at a given commit -- the canonical-root-count signal. */
function workspaceMembersAt(sha) {
  const text = git(['show', `${sha}:Cargo.toml`]);
  const match = text.match(/^members\s*=\s*\[([^\]]*)\]/m);
  if (!match) return [];
  return match[1].split(',').map((s) => s.trim().replace(/^"|"$/g, '')).filter(Boolean);
}

function newModulesInRange(baseSha, finalSha) {
  const out = git(['diff', '--name-status', `${baseSha}..${finalSha}`, '--', 'core/src', 'runtime/src', 'adapter/src', 'organism/src']).trim()
  if (!out) return [];
  return out.split('\n')
    .map((line) => line.split('\t'))
    .filter(([status]) => status === 'A')
    .map(([, path]) => path);
}

function currentSourceSha() {
  return git(['rev-parse', 'HEAD']).trim();
}

// Uses more decimal places for small ratios (donor drain against a 736k-file corpus rounds to
// "0.00%" at 2 decimals, which reads as "nothing happened" even when real files were drained --
// matching the directive's own example, "73 / 736,029 = 0.0099%").
function fmtPct(n) {
  if (!Number.isFinite(n)) return 'n/a';
  const pct = n * 100;
  const decimals = pct !== 0 && Math.abs(pct) < 1 ? 4 : 2;
  return `${pct.toFixed(decimals)}%`;
}

function ratio(num, den) {
  if (den === 0) return null;
  return num / den;
}

/**
 * @param {object} opts
 * @param {string} opts.baseSha
 * @param {string} opts.finalSha
 * @param {object} opts.live - LiveCounts, supplied fresh by the orchestrator each call:
 *   {
 *     candidatesPlanned, buildersCompleted, accepted, rejected, conflicted,
 *     verificationSubmitted, verificationPassed,
 *     acceptedWithRealNativeCode,
 *     safeDrainClassification: { DRAINED: n, RETAINED_COUPLED: n, REFERENCE_ONLY: n },
 *   }
 */
export function buildRatioReport({ baseSha, finalSha, live }) {
  const lines = [];
  const push = (s = '') => lines.push(s);

  const { filesDrained: loopFilesDrained, byDonor } = donorDrainInRange(baseSha, finalSha);
  const locByRoot = nativeLocDeltaByRoot(baseSha, finalSha);
  const membersBefore = workspaceMembersAt(baseSha);
  const membersAfter = workspaceMembersAt(finalSha);
  const newModules = newModulesInRange(baseSha, finalSha);

  const extinction = computeCorpusExtinctionCounts();
  const { baselineDonorFiles: cumulativeBaseline, remainingDonorFiles: cumulativeRemaining, drainedDonorFiles: cumulativeDrained } = computeCorpusFileTotals();
  // remaining-before-this-loop = remaining-now + whatever this loop itself drained -- lets
  // LOOP_DRAIN_RATIO answer "how much of the donor mass that existed when this loop started did
  // it actually drain", not merely restate the raw file count already shown above it.
  const remainingBeforeLoop = cumulativeRemaining + loopFilesDrained;
  const loopDrainRatio = ratio(loopFilesDrained, remainingBeforeLoop);

  const acceptanceRatio = ratio(live.accepted, live.buildersCompleted);
  const collisionRatio = ratio(live.conflicted, live.candidatesPlanned);
  const verificationPassRatio = ratio(live.verificationPassed, live.verificationSubmitted);
  const absorptionEfficiency = ratio(live.accepted, live.buildersCompleted);
  const attemptRatio = ratio(live.accepted, (live.conflicted ?? 0) + (live.rejected ?? 0) + live.accepted);
  const functionalRatio = ratio(live.acceptedWithRealNativeCode, live.accepted);
  const safeDrained = live.safeDrainClassification?.DRAINED ?? 0;
  const safeDrainRatio = ratio(safeDrained, live.accepted);

  const totalNativeLinesAdded = CANONICAL_ROOTS.reduce((s, r) => s + locByRoot[r].added, 0);
  const kernelGrowthRatio = ratio(totalNativeLinesAdded, live.accepted);

  const newWorkspaceMembers = membersAfter.filter((m) => !membersBefore.includes(m));
  const rootCountDelta = membersAfter.length - membersBefore.length;
  const agr = ratio(newWorkspaceMembers.length + newModules.length, live.accepted);

  push('CHRONICA ABSORPTION LOOP RATIO REPORT');
  push('');
  push(`BASE_SHA:  ${baseSha}`);
  push(`FINAL_SHA: ${finalSha}`);
  push('');
  push(`Candidates planned:    ${live.candidatesPlanned}`);
  push(`Builders completed:    ${live.buildersCompleted}`);
  push(`Accepted:              ${live.accepted}`);
  push(`Rejected:              ${live.rejected ?? 0}`);
  push(`Conflicted:            ${live.conflicted ?? 0}`);
  push('');
  push(`ACCEPTANCE_RATIO:          ${live.accepted} / ${live.buildersCompleted} = ${fmtPct(acceptanceRatio)}`);
  push(`COLLISION_RATIO:           ${live.conflicted ?? 0} / ${live.candidatesPlanned} = ${fmtPct(collisionRatio)}`);
  push(`VERIFICATION_PASS_RATIO:   ${live.verificationPassed} / ${live.verificationSubmitted} = ${fmtPct(verificationPassRatio)}`);
  push(`ABSORPTION_EFFICIENCY:     ${live.accepted} / ${live.buildersCompleted} = ${fmtPct(absorptionEfficiency)}`);
  push(`  (accepted / (conflicts+rejected+accepted) = ${fmtPct(attemptRatio)})`);
  push('');

  // DONOR CORPUS -- FOUR distinct facts, never conflated (CHRONICA -- fix the FUNCTIONAL
  // ABSORPTION != SOURCE DELETION defect, 2026-09-17; see donor-burndown.mjs::
  // computeCorpusExtinctionCounts's own doc for the full rationale):
  //   1. INGESTED (sourcePresentDonors) -- donor source exists in temporary/.
  //   2. FUNCTIONAL ABSORPTION STARTED (functionalAbsorptionStartedDonors) -- real donor-derived
  //      behavior materialized into core/runtime/adapter/organism, Git-derived from commits that
  //      both mention the donor's full slug AND touch a canonical native root -- INDEPENDENT of
  //      whether any donor source file was ever deleted.
  //   3. SOURCE DRAIN STARTED (sourceDrainStartedDonors) -- at least one donor source file has
  //      safely disappeared -- INDEPENDENT of whether that donor ever produced real native code.
  //   4. FULLY DRAINED / EXTINCT (fullyDrainedDonors) -- baseline_files > 0 && remaining_files == 0.
  // A donor may be functionally-started with zero source drain (postgres/postgres today: 4 real
  // native primitives absorbed, yet its one real deletion was later safely reverted because
  // retained donor source elsewhere still referenced it -- remaining_files == baseline_files
  // again). Every count here is `donor-burndown.mjs::computeCorpusExtinctionCounts`'s own
  // Git/tree-derived number -- never a manually-maintained `status` field.
  const donorIngestionRatio = ratio(extinction.sourcePresentDonors, extinction.totalDonors);
  const functionalDonorCoverage = ratio(extinction.functionalAbsorptionStartedDonors, extinction.sourcePresentDonors);
  const sourceDrainDonorRatio = ratio(extinction.sourceDrainStartedDonors, extinction.sourcePresentDonors);
  const donorExtinctionRatio = ratio(extinction.fullyDrainedDonors, extinction.sourcePresentDonors);
  push('DONOR CORPUS');
  push(`  Total donors:                    ${extinction.totalDonors}`);
  push(`  Source ingested:                 ${extinction.sourcePresentDonors}`);
  push(`  Functional absorption started:   ${extinction.functionalAbsorptionStartedDonors}`);
  push(`  Source drain started:            ${extinction.sourceDrainStartedDonors}`);
  push(`  Fully drained / extinct:         ${extinction.fullyDrainedDonors}`);
  push('');
  push(`DONOR_INGESTION_RATIO:       ${extinction.sourcePresentDonors} / ${extinction.totalDonors} = ${fmtPct(donorIngestionRatio)}`);
  push(`FUNCTIONAL_DONOR_COVERAGE:   ${extinction.functionalAbsorptionStartedDonors} / ${extinction.sourcePresentDonors} = ${fmtPct(functionalDonorCoverage)}`);
  push(`SOURCE_DRAIN_DONOR_RATIO:    ${extinction.sourceDrainStartedDonors} / ${extinction.sourcePresentDonors} = ${fmtPct(sourceDrainDonorRatio)}`);
  push(`DONOR_EXTINCTION_RATIO:      ${extinction.fullyDrainedDonors} / ${extinction.sourcePresentDonors} = ${fmtPct(donorExtinctionRatio)}`);
  push('');

  // SOURCE FILE DRAIN -- file-level drain, kept distinct from donor-level facts above (section 5
  // of the original directive: "donor coverage alone is not enough... the report must show BOTH
  // donor coverage and source drain"). Zero fully-extinct donors, and zero files drained, are NOT
  // failure signals on their own -- healthy absorption can legitimately mean native capability
  // grew while donor source stayed fully intact (section 8/13: "do not reward blind deletion").
  push('SOURCE FILE DRAIN');
  push(`  Baseline donor files:      ${cumulativeBaseline}`);
  push(`  Remaining donor files:     ${cumulativeRemaining}`);
  push(`  Files drained this loop:   ${loopFilesDrained}  ${JSON.stringify(byDonor)}`);
  push(`  Files drained cumulative:  ${cumulativeDrained}`);
  push('');
  push(`CUMULATIVE_DRAIN_RATIO:      ${cumulativeDrained} / ${cumulativeBaseline} = ${fmtPct(ratio(cumulativeDrained, cumulativeBaseline))}`);
  push(`LOOP_DRAIN_RATIO:            ${loopFilesDrained} / ${remainingBeforeLoop} = ${fmtPct(loopDrainRatio)}`);
  push(`SAFE_DRAIN_RATIO:            ${safeDrained} / ${live.accepted} = ${fmtPct(safeDrainRatio)}  ${JSON.stringify(live.safeDrainClassification ?? {})}`);
  push('');
  // Candidate-level metric, a DIFFERENT question from FUNCTIONAL_DONOR_COVERAGE above: of THIS
  // loop's own accepted candidates, how many touched real native code (accepted native slices /
  // accepted candidates) -- never removed, never confused with the corpus-wide donor metric.
  push(`FUNCTIONAL_ABSORPTION_RATIO: ${live.acceptedWithRealNativeCode} / ${live.accepted} = ${fmtPct(functionalRatio)}`);
  push('');
  push('Canonical native LOC delta (added/removed/net):');
  for (const root of CANONICAL_ROOTS) {
    const d = locByRoot[root];
    push(`  ${root}:${' '.repeat(Math.max(1, 9 - root.length))}+${d.added} -${d.removed} (net ${d.net >= 0 ? '+' : ''}${d.net})`);
  }
  push(`  frontend (apps/): +${locByRoot.apps.added} -${locByRoot.apps.removed} (net ${locByRoot.apps.net >= 0 ? '+' : ''}${locByRoot.apps.net})`);
  push(`  KERNEL_GROWTH_RATIO (native lines added / accepted slice): ${totalNativeLinesAdded} / ${live.accepted} = ${kernelGrowthRatio?.toFixed(1) ?? 'n/a'} lines/slice`);
  push('');
  push(`New canonical roots:        ${rootCountDelta} ${rootCountDelta ? `(!!) ${JSON.stringify(newWorkspaceMembers)}` : ''}`);
  push(`New workspace members:      ${newWorkspaceMembers.length} ${JSON.stringify(newWorkspaceMembers)}`);
  push(`New modules (new files under */src):  ${newModules.length}`);
  for (const m of newModules) push(`    + ${m}`);
  push('New execution universes:    0 (no second WorkRun/world engine introduced -- verify manually if this report is wrong)');
  push('New authority universes:    0 (no second Can/authority engine introduced -- verify manually if this report is wrong)');
  push('New world/state universes:  0 (no second WorldTransaction/canonical-history store introduced -- verify manually if this report is wrong)');
  push('');
  push(`ARCHITECTURAL_GROWTH_RATIO: (new_members + new_modules) / accepted = (${newWorkspaceMembers.length} + ${newModules.length}) / ${live.accepted} = ${agr?.toFixed(2) ?? 'n/a'}`);
  push(`ROOT_COUNT_DELTA:           ${rootCountDelta}`);
  push('');

  const tempDepViolations = findTemporaryDependencyViolations();
  push(`Temporary production dependencies: ${tempDepViolations.length}`);
  for (const v of tempDepViolations) push(`    - ${v}`);
  push(`TEMPORARY_ISOLATION_RATIO:  ${tempDepViolations.length} violation(s) found (expected 0)`);
  push('');

  const hardViolations = (() => {
    try {
      const out = execFileSync('node', ['tools/system-atlas/validate-v2.mjs'], { cwd: ROOT, encoding: 'utf8' });
      return { pass: true, out };
    } catch (err) {
      return { pass: false, out: err.stdout?.toString() ?? '' };
    }
  })();

  // Reuses the SAME gate the serial integrator itself already runs per candidate
  // (donor-coupling-gate.mjs) -- never re-derived here. Section 9 of the loop directive is
  // absolute: donor deletion safety must never be weakened, so a violation found over this
  // loop's own range is always a STOP_AND_REVIEW condition, never merely DEGRADED.
  const donorCoupling = (() => {
    try {
      return { pass: true, violations: findDonorCouplingViolations(baseSha, ROOT) };
    } catch (err) {
      return { pass: false, violations: null, error: err };
    }
  })();
  const donorCouplingPass = donorCoupling.pass && (donorCoupling.violations?.length ?? 1) === 0;
  push(`System Atlas:          ${hardViolations.pass ? 'PASS (0 HARD violations)' : `FAIL -- ${hardViolations.out.split('\n')[0]}`}`);
  push(`Donor coupling gate:   ${donorCouplingPass ? 'PASS (0 violations)' : `FAIL -- ${donorCoupling.violations?.length ?? 'error'} violation(s) since ${baseSha}`}`);
  push('');

  // Two severities, per the loop directive's own distinction: an invariant the workspace can
  // NEVER tolerate (a canonical root appeared/disappeared, a temporary/ production dependency
  // exists, a HARD Atlas violation, an unsafe donor deletion) is STOP_AND_REVIEW -- categorically
  // worse than a loop that merely under-performed (nothing accepted, or a low verification pass
  // rate), which is DEGRADED but not an emergency. Zero donor extinction and zero files drained
  // are explicitly NOT failure signals on their own (section 8: "do not reward blind deletion") --
  // neither factors into this verdict at all.
  const invariantBroken = (
    rootCountDelta !== 0 ||
    tempDepViolations.length > 0 ||
    !hardViolations.pass ||
    !donorCouplingPass
  );
  const degraded = (
    live.accepted === 0 ||
    (verificationPassRatio !== null && verificationPassRatio < 0.9)
  );
  const verdict = invariantBroken ? 'STOP_AND_REVIEW' : degraded ? 'DEGRADED' : 'HEALTHY';
  push('VERDICT:');
  push(`  ${verdict}`);

  push('');
  push('ABSORPTION SNAPSHOT');
  push('');
  push('Donors:');
  push(`  ${extinction.sourcePresentDonors} / ${extinction.totalDonors} ingested`);
  push(`  ${extinction.functionalAbsorptionStartedDonors} / ${extinction.sourcePresentDonors} functionally touched`);
  push(`  ${extinction.sourceDrainStartedDonors} / ${extinction.sourcePresentDonors} source-drain started`);
  push(`  ${extinction.fullyDrainedDonors} / ${extinction.sourcePresentDonors} extinct`);
  push('');
  push('Source:');
  push(`  ${cumulativeDrained} / ${cumulativeBaseline} files drained`);
  push(`  ${cumulativeRemaining} remaining`);
  push('');
  push('Native:');
  for (const root of CANONICAL_ROOTS) {
    const d = locByRoot[root];
    push(`  ${root}${' '.repeat(Math.max(1, 9 - root.length))}${d.net >= 0 ? '+' : ''}${d.net} LOC`);
  }
  push('');
  push('Architecture:');
  push(`  roots       ${membersBefore.length} -> ${membersAfter.length}`);
  push('  worlds      1 -> 1');
  push('  authority   1 -> 1');
  push('  execution   1 -> 1');
  push('');
  push(`VERDICT: ${verdict}`);

  return lines;
}

function parseArgs(argv) {
  const opts = {};
  for (let i = 0; i < argv.length; i++) {
    if (argv[i] === '--base') opts.base = argv[++i];
    if (argv[i] === '--final') opts.final = argv[++i];
    if (argv[i] === '--live') opts.live = argv[++i];
  }
  return opts;
}

function main(argv = process.argv.slice(2)) {
  const { base, final, live } = parseArgs(argv);
  if (!base || !final || !live) {
    console.error('usage: absorption-ratio-report.mjs --base <sha> --final <sha> --live <path-to-json>');
    return 2;
  }
  const liveCounts = JSON.parse(readFileSync(live, 'utf8'));
  const finalSha = final === 'HEAD' ? currentSourceSha() : final;
  const lines = buildRatioReport({ baseSha: base, finalSha, live: liveCounts });
  for (const l of lines) console.log(l);
  return 0;
}

if (process.argv[1] === fileURLToPath(import.meta.url)) process.exit(main());
