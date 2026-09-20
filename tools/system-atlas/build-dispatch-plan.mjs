#!/usr/bin/env node
// Coordinator-only: builds an initial parallel-safe wave from the frontiers.
// WAVE-001 is a PLAN ONLY -- it never fabricates a Lease (see leases.mjs /
// leases.json) and never marks a task "LEASED" merely for appearing in it.
//
// B.2 hardening (Option A): wave inclusion is now PURELY DERIVED and
// NON-MUTATING. This script only ever READS the frontier files and only
// ever WRITES dispatch-plan.json -- it must never call writeFileSync (or
// any file-write function) on any *-frontier.json file. A task appearing
// in WAVE-001 has zero side effect on that task's own frontier-file
// record; it is not evidence that any worker owns it. A real worker must
// still claim a task via tools/system-atlas/leases.mjs before any state
// changes. This also makes the planner idempotent: running it twice in a
// row against unchanged frontier files now produces the same wave both
// times, since every run starts from the same READY population defined
// purely by build-frontiers.mjs's output.
//
// "PLANNED" remains a valid enum value in task.schema.json for
// forward-compatibility (e.g. a future durable-scheduler design), but this
// planner intentionally never produces it -- do not reintroduce writing it
// here.
//
// Both proves_no_owned_path_overlap and proves_dependency_safe are computed
// by tools/system-atlas/dependency-safety.mjs's analyzeWave() -- never
// hardcoded -- and the full safety report is kept in the plan for audit.
import { writeFileSync } from "node:fs";
import { resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { analyzeWave } from "./dependency-safety.mjs";
import { retireDispatchGate } from "./hygiene-gate.mjs";
import { REPO_ROOT, atlasGenerationSha, nowIso, productSourceSha, readJson, repoRel } from "./lib.mjs";
import { canBecomePlanned } from "./mutation-ownership.mjs";

const OUT_DIR = resolve(REPO_ROOT, "docs/_machine/system-atlas/v0");
const MAX_REFINEMENT_ITERATIONS = 20;

function pathsOverlap(a, b) {
  const norm = (p) => p.replace(/\*\*$/, "");
  for (const pa of a) for (const pb of b) {
    const na = norm(pa);
    const nb = norm(pb);
    if (na === nb || na.startsWith(nb) || nb.startsWith(na)) return true;
  }
  return false;
}

// Greedy candidate construction: only READY tasks are eligible (PLANNED,
// DESIGN_REQUIRED, LEASED, BLOCKED_*, etc. are never wave-eligible). Prefer
// higher priority; skip a task if its owned_paths overlap one already picked.
function buildCandidate(eligibleTasks) {
  const selected = [];
  for (const t of eligibleTasks.slice().sort((a, b) => b.priority - a.priority)) {
    if (t.owned_paths.length > 0 && selected.some((s) => pathsOverlap(s.owned_paths, t.owned_paths))) continue;
    selected.push(t);
  }
  return selected;
}

// Iteratively drop whatever analyzeWave() flags until the wave is genuinely
// safe (or nothing more can be removed) -- proves_* must describe the
// wave that ships, not a wave we hoped was safe.
function refineToSafety(candidateTasks, allTasksById) {
  let tasks = candidateTasks;
  for (let i = 0; i < MAX_REFINEMENT_ITERATIONS; i += 1) {
    const wave = { wave_id: "WAVE-001", accounts: { _probe: tasks.map((t) => t.task_id) } };
    const report = analyzeWave(wave, allTasksById);
    if (report.provesDependencySafe && report.provesNoOwnedPathOverlap) return { tasks, report };

    const offending = new Set();
    for (const c of [...report.cycles ?? [], ...report.blockedByCycles ?? []]) for (const id of c) offending.add(id);
    for (const o of [...report.ownedPathOverlaps ?? [], ...report.forbiddenPathOverlaps ?? []]) offending.add(o.taskB);
    for (const c of report.sharedFileConflicts ?? []) offending.add(c.taskB);
    for (const v of report.riskViolations ?? []) offending.add(v.task_id);
    for (const id of report.unresolvedTaskIds ?? []) offending.add(id);
    for (const u of report.unsatisfiedDependencies ?? []) offending.add(u.task_id);
    for (const u of report.unclearedBlockers ?? []) offending.add(u.task_id);

    if (offending.size === 0) return { tasks, report }; // can't refine further; report the honest (unsafe) result
    tasks = tasks.filter((t) => !offending.has(t.task_id));
  }
  const wave = { wave_id: "WAVE-001", accounts: { _probe: tasks.map((t) => t.task_id) } };
  return { tasks, report: analyzeWave(wave, allTasksById) };
}

function main() {
  const sourceBaseSha = productSourceSha();
  const atlasSha = atlasGenerationSha();

  const frontierNames = ["repair-frontier", "build-frontier", "retire-frontier", "verify-frontier"];
  const frontiers = Object.fromEntries(frontierNames.map((name) => [name, readJson(resolve(OUT_DIR, `${name}.json`))]));

  const allTasksById = new Map();
  for (const name of frontierNames) for (const t of frontiers[name].tasks) allTasksById.set(t.task_id, t);

  // A task is only wave-eligible if: it's READY (never PLANNED/DESIGN_REQUIRED/
  // LEASED/etc. -- see status semantics in task.schema.json); it's not a
  // mutating task lacking real owned_paths (mutation-ownership.mjs's
  // canBecomePlanned gate -- defense in depth, build-frontiers.mjs should
  // already have kept such tasks out of READY, but the dispatcher must not
  // trust that alone); and if it's a RETIRE task, the hygiene-clearance gate
  // must actively allow it (retireDispatchGate) -- current reachability
  // alone is never sufficient justification, per the B.1.1 hardening brief.
  const eligible = [...allTasksById.values()].filter((t) => {
    if (t.status !== "READY") return false;
    if (!canBecomePlanned(t)) return false;
    if (t.action === "RETIRE" && !retireDispatchGate(t).allowed) return false;
    return true;
  });
  const candidate = buildCandidate(eligible);
  const { tasks: safeTasks, report } = refineToSafety(candidate, allTasksById);

  if (!report.provesDependencySafe || !report.provesNoOwnedPathOverlap) {
    console.error("WARNING: could not refine to a fully safe wave; shipping the best achieved state with honest proves_* = false where still violated.");
  }

  const accounts = { accountA: [], accountB: [] };
  safeTasks
    .slice()
    .sort((a, b) => b.priority - a.priority)
    .forEach((t, i) => accounts[i % 2 === 0 ? "accountA" : "accountB"].push(t.task_id));

  const wave = {
    wave_id: "WAVE-001",
    description: "PLAN ONLY, purely derived from current READY frontier-file state -- selecting a task into this wave has no side effect on any frontier file; a real worker must still claim it via tools/system-atlas/leases.mjs before any state changes. Selects READY tasks whose dependency/owned-path/risk safety is machine-proven by dependency-safety.mjs's analyzeWave() -- see safety_report. No task is marked LEASED or PLANNED by this plan.",
    accounts,
    proves_no_owned_path_overlap: report.provesNoOwnedPathOverlap,
    proves_dependency_safe: report.provesDependencySafe,
    safety_report: report,
  };

  const plan = {
    schema: "chronica.system-atlas.dispatch.v0",
    dispatch_contract_version: 3,
    source_base_sha: sourceBaseSha,
    atlas_generation_sha: atlasSha,
    generated_at: nowIso(),
    waves: [wave],
    lease_states: ["READY", "PLANNED", "DESIGN_REQUIRED", "LEASED", "IN_PROGRESS", "READY_TO_VERIFY", "VERIFIED", "READY_TO_INTEGRATE", "INTEGRATED", "RECOVERY_REQUIRED", "BLOCKED_DEPENDENCY", "BLOCKED_ENV", "BLOCKED_SEMANTICS", "FAILED"],
  };
  writeFileSync(resolve(OUT_DIR, "dispatch-plan.json"), `${JSON.stringify(plan, null, 2)}\n`);

  // Non-mutating by design: no frontier file is ever written here. Selecting
  // a task into the wave is purely a planning-time derivation and has zero
  // effect on that task's own frontier-file record -- see module header.

  console.log(`WAVE-001: accountA=${accounts.accountA.length} tasks, accountB=${accounts.accountB.length} tasks`);
  console.log(`eligible READY tasks: ${eligible.length}; candidate: ${candidate.length}; refined-safe: ${safeTasks.length}`);
  console.log(`proves_dependency_safe=${report.provesDependencySafe} proves_no_owned_path_overlap=${report.provesNoOwnedPathOverlap}`);
  console.log(`wrote ${repoRel(resolve(OUT_DIR, "dispatch-plan.json"))} (no fake leases, no frontier mutation; leases.json and *-frontier.json unchanged)`);
}

// Guarded so this module can be imported without the side effect of
// re-planning WAVE-001 against real committed files -- see atlas.test.mjs's
// test J for why this specifically matters (a prior version of that test
// re-invoked this script and destabilized the committed wave size).
if (process.argv[1] === fileURLToPath(import.meta.url)) main();
