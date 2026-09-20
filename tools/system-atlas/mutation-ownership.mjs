// Pure ESM, zero dependencies, zero file I/O in the exported logic.
//
// Guards against the class of task where a mutating action (REPAIR / BUILD /
// RETIRE) carries owned_paths: [] while sitting in (or being promoted to) a
// dispatchable status. An empty owned_paths on a mutating task makes the
// path-conflict-overlap safety proof vacuous -- there is nothing to prove
// non-overlapping -- while the task could still be scheduled to run. This
// module makes that mechanically impossible to reach: DESIGN_REQUIRED remains
// the one legitimate resting state for a broad-scope mutating task with no
// known ownership yet (e.g. BUILD-0001 "ERP/Finance" spanning 38 core crates,
// or RETIRE-0003 "~40 OSINT probe binaries"), and canBecomePlanned refuses to
// ever let such a task leave that state until real paths are known.
//
// Consumers (coordinator-wired, not this file): build-frontiers.mjs should
// never emit a mutating task in a dispatchable status with owned_paths: [];
// build-dispatch-plan.mjs's wave planner must call canBecomePlanned() before
// moving a task READY -> PLANNED; validate.mjs should run auditFrontiers()
// (or validateMutationOwnership() per task) as a hard gate.

import { fileURLToPath } from "node:url";

/** Actions that mutate product code/state and therefore require concrete
 * ownership before they may be dispatched. VERIFY is deliberately excluded:
 * it is read-only investigation and an empty owned_paths is expected. */
export const MUTATING_ACTIONS = new Set(["REPAIR", "BUILD", "RETIRE"]);

/** @param {string} action */
export function isMutatingAction(action) {
  return MUTATING_ACTIONS.has(action);
}

/** Statuses a mutating task with owned_paths: [] must never sit in, or be
 * promoted to. DESIGN_REQUIRED is intentionally excluded -- that's the
 * legitimate escape hatch for pre-ownership design work; it just must never
 * be promoted past it (see canBecomePlanned). */
export const DISPATCHABLE_MUTATION_STATUSES = new Set(["READY", "PLANNED", "LEASED", "IN_PROGRESS"]);

/** Filters owned_paths down to entries that are real, non-blank path strings.
 * A blank/whitespace-only string is functionally identical to owned_paths: []
 * -- it satisfies no schema minLength (owned_paths has none) and matches no
 * real crate path via pathsOverlap's prefix logic, so it would otherwise let
 * a task past this gate while still making the path-overlap safety proof
 * vacuous. Found by BF5's adversarial review; treat any such fixture the
 * same as an empty array. */
function realOwnedPaths(task) {
  return (task.owned_paths ?? []).filter((p) => typeof p === "string" && p.trim().length > 0);
}

/**
 * Validates a single task against the mutation-ownership rule.
 * @param {{task_id?: string, action?: string, status?: string, owned_paths?: string[]}} task
 * @returns {string[]} violation strings; empty array means OK.
 */
export function validateMutationOwnership(task) {
  if (isMutatingAction(task.action) && realOwnedPaths(task).length === 0 && DISPATCHABLE_MUTATION_STATUSES.has(task.status)) {
    return [
      `${task.task_id ?? "<unknown task_id>"}: mutating action ${task.action} in dispatchable status ${task.status} ` +
        `has no real owned_paths (empty, or blank/whitespace-only entries only) -- a mutating task in a dispatchable ` +
        `status must have at least one concrete owned path (the path-conflict-overlap safety proof is vacuous otherwise). ` +
        `Move it to DESIGN_REQUIRED until real ownership is known.`,
    ];
  }
  return [];
}

/**
 * Strict boolean gate the wave-planner must call BEFORE ever moving a task
 * from READY to PLANNED. Does not itself decide what "genuinely
 * pre-ownership design work" means or invent paths -- it only refuses to let
 * a mutating task with no concrete owned path become PLANNED.
 * @param {{action?: string, owned_paths?: string[]}} task
 * @returns {boolean}
 */
export function canBecomePlanned(task) {
  if (!isMutatingAction(task.action)) return true;
  return realOwnedPaths(task).length > 0;
}

/**
 * Convenience reporting helper: runs validateMutationOwnership over every
 * task in every frontier file. Purely for reporting/self-test; the real
 * validator wiring into validate.mjs is the coordinator's job.
 * @param {Record<string, Array<{task_id?: string, action?: string, status?: string, owned_paths?: string[]}>>} frontierTasksByFile
 * @returns {{violations: Array<{file: string, task_id: string, violation: string}>, violationCount: number}}
 */
export function auditFrontiers(frontierTasksByFile) {
  const violations = [];
  for (const [file, tasks] of Object.entries(frontierTasksByFile ?? {})) {
    for (const task of tasks ?? []) {
      for (const violation of validateMutationOwnership(task)) {
        violations.push({ file, task_id: task.task_id ?? "<unknown task_id>", violation });
      }
    }
  }
  return { violations, violationCount: violations.length };
}

if (process.argv[1] === fileURLToPath(import.meta.url)) {
  const results = [];
  const check = (label, actual, expected) => {
    const pass = JSON.stringify(actual) === JSON.stringify(expected);
    results.push({ label, pass, actual, expected });
  };

  // (a) REPAIR, READY, owned_paths=[] -> FAIL (violation reported)
  const a = { task_id: "REPAIR-A", action: "REPAIR", status: "READY", owned_paths: [] };
  check("(a) REPAIR/READY/[] has violation", validateMutationOwnership(a).length > 0, true);

  // (b) BUILD, PLANNED, owned_paths=[] -> FAIL
  const b = { task_id: "BUILD-B", action: "BUILD", status: "PLANNED", owned_paths: [] };
  check("(b) BUILD/PLANNED/[] has violation", validateMutationOwnership(b).length > 0, true);

  // (c) RETIRE, READY, owned_paths=[] -> FAIL
  const c = { task_id: "RETIRE-C", action: "RETIRE", status: "READY", owned_paths: [] };
  check("(c) RETIRE/READY/[] has violation", validateMutationOwnership(c).length > 0, true);

  // (d) VERIFY, owned_paths=[] -> PASS (read-only, no violation)
  const d = { task_id: "VERIFY-D", action: "VERIFY", status: "READY", owned_paths: [] };
  check("(d) VERIFY/READY/[] has no violation", validateMutationOwnership(d).length === 0, true);

  // (e) BUILD, DESIGN_REQUIRED, owned_paths=[] -> PASS, but canBecomePlanned -> false
  const e = { task_id: "BUILD-E", action: "BUILD", status: "DESIGN_REQUIRED", owned_paths: [] };
  check("(e) BUILD/DESIGN_REQUIRED/[] has no violation", validateMutationOwnership(e).length === 0, true);
  check("(e) canBecomePlanned(BUILD/DESIGN_REQUIRED/[]) === false", canBecomePlanned(e), false);

  // (f) REPAIR, READY, owned_paths=[real path] -> PASS, canBecomePlanned -> true
  const f = { task_id: "REPAIR-F", action: "REPAIR", status: "READY", owned_paths: ["crates/cap/foo/**"] };
  check("(f) REPAIR/READY/[path] has no violation", validateMutationOwnership(f).length === 0, true);
  check("(f) canBecomePlanned(REPAIR/READY/[path]) === true", canBecomePlanned(f), true);

  // (g) BF5 adversarial finding: REPAIR, READY, owned_paths=["   "] (whitespace
  // only) must be treated identically to owned_paths: [] -- a blank string is
  // not a real path and would otherwise make the overlap proof vacuous while
  // passing the naive `.length === 0` check.
  const g = { task_id: "REPAIR-G", action: "REPAIR", status: "READY", owned_paths: ["   "] };
  check("(g) REPAIR/READY/['   '] has violation (blank path rejected)", validateMutationOwnership(g).length > 0, true);
  check("(g) canBecomePlanned(REPAIR/READY/['   ']) === false", canBecomePlanned(g), false);

  const failed = results.filter((r) => !r.pass);
  for (const r of results) {
    console.log(`${r.pass ? "PASS" : "FAIL"} ${r.label}`);
  }

  // Extra real-data sanity check: read-only, informative only, never asserted.
  try {
    const { readFileSync, readdirSync } = await import("node:fs");
    const { resolve, dirname } = await import("node:path");
    const dir = resolve(dirname(fileURLToPath(import.meta.url)), "..", "..", "docs/_machine/system-atlas/v0");
    const frontierFiles = readdirSync(dir).filter((f) => f.endsWith("-frontier.json"));
    const frontierTasksByFile = {};
    for (const f of frontierFiles) {
      const parsed = JSON.parse(readFileSync(resolve(dir, f), "utf8"));
      frontierTasksByFile[f] = parsed.tasks ?? [];
    }
    const { violations, violationCount } = auditFrontiers(frontierTasksByFile);
    console.log(`\n[real-data, informative only] frontier violation count: ${violationCount}`);
    for (const v of violations.slice(0, 5)) {
      console.log(`  - [${v.file}] ${v.task_id}: ${v.violation}`);
    }
  } catch (err) {
    console.log(`\n[real-data check skipped: ${err.message}]`);
  }

  if (failed.length > 0) {
    console.error(`\n${failed.length} self-test assertion(s) FAILED`);
    process.exit(1);
  }
}
