#!/usr/bin/env node
// Pure, zero-dependency, zero-file-I/O derivation of dependency/path/risk
// safety for System Atlas dispatch waves. The caller (build-dispatch-plan.mjs
// or a future coordinator script) reads Task/Wave JSON off disk and hands the
// resulting in-memory objects in here; this module never touches the
// filesystem itself, which is what makes it trivially unit-testable.
//
// This exists because build-dispatch-plan.mjs currently hardcodes
// `proves_dependency_safe: true` as a JS boolean literal on every wave,
// without deriving it from anything. That is the exact bug this hardening
// pass calls out: "Derive dependency safety. Never hard-code
// proves_dependency_safe=true." The functions below are the real derivation
// logic; wiring them into build-dispatch-plan.mjs is the coordinator's job.
import { fileURLToPath } from "node:url";

// Normalize a trailing "/**" off a path. Two paths overlap if they are equal
// after normalization, or one is a path-prefix of the other. This must match
// build-dispatch-plan.mjs's pathsOverlap() exactly so results agree with what
// wave-building already checked.
function normPath(p) {
  return p.replace(/\*\*$/, "");
}

function pathsOverlap(a, b) {
  for (const pa of a) for (const pb of b) {
    const na = normPath(pa);
    const nb = normPath(pb);
    if (na === nb || na.startsWith(nb) || nb.startsWith(na)) return true;
  }
  return false;
}

// Finds one overlapping path between two path lists (for reporting which
// path caused the overlap) using the same semantics as pathsOverlap().
function firstOverlappingPath(a, b) {
  for (const pa of a) for (const pb of b) {
    const na = normPath(pa);
    const nb = normPath(pb);
    if (na === nb || na.startsWith(nb) || nb.startsWith(na)) return na.length <= nb.length ? pa : pb;
  }
  return null;
}

// Generic transitive-closure-with-cycle-detection over a field ("dependencies"
// or "blocked_by") that holds arrays of task_ids. Straightforward DFS with a
// visiting stack (not memoized SCC/tarjan -- kept simple per the brief).
function closureOf(tasksById, field) {
  const closure = new Map();
  const cycles = [];

  for (const taskId of tasksById.keys()) {
    if (closure.has(taskId)) continue;
    const visiting = new Set();
    const result = new Set();
    const stack = []; // path of task_ids currently being visited, for cycle reporting

    const dfs = (id) => {
      if (visiting.has(id)) {
        // Found a cycle: everything from the first occurrence of `id` in
        // `stack` through the end, plus `id` again to close the loop.
        const startIdx = stack.indexOf(id);
        const cyclePath = stack.slice(startIdx).concat(id);
        cycles.push(cyclePath);
        return;
      }
      if (closure.has(id)) {
        for (const d of closure.get(id)) result.add(d);
        return;
      }
      const task = tasksById.get(id);
      if (!task) return; // unresolvable reference -- not this function's job to flag
      visiting.add(id);
      stack.push(id);
      for (const dep of task[field] ?? []) {
        result.add(dep);
        dfs(dep);
      }
      stack.pop();
      visiting.delete(id);
    };

    dfs(taskId);
    closure.set(taskId, result);
  }

  // De-duplicate cycles that were discovered from multiple starting points
  // (same cycle, rotated). Compare by their sorted node-id set.
  const seenCycleKeys = new Set();
  const dedupedCycles = [];
  for (const cyclePath of cycles) {
    const key = [...new Set(cyclePath)].sort().join(",");
    if (seenCycleKeys.has(key)) continue;
    seenCycleKeys.add(key);
    dedupedCycles.push(cyclePath);
  }

  return { closure, cycles: dedupedCycles };
}

export function dependencyClosure(tasksById) {
  return closureOf(tasksById, "dependencies");
}

export function blockedByClosure(tasksById) {
  return closureOf(tasksById, "blocked_by");
}

export function ownedPathOverlaps(taskList) {
  const overlaps = [];
  for (let i = 0; i < taskList.length; i += 1) {
    for (let j = i + 1; j < taskList.length; j += 1) {
      const a = taskList[i];
      const b = taskList[j];
      if (pathsOverlap(a.owned_paths ?? [], b.owned_paths ?? [])) {
        overlaps.push({ taskA: a.task_id, taskB: b.task_id, path: firstOverlappingPath(a.owned_paths ?? [], b.owned_paths ?? []) });
      }
    }
  }
  return overlaps;
}

// A owns a path B has forbidden (or vice versa) -- a real safety property
// distinct from ownedPathOverlaps: two tasks can have non-overlapping
// owned_paths yet one still touches a path the other explicitly forbade
// (e.g. Cargo.lock).
export function forbiddenPathOverlaps(taskList) {
  const overlaps = [];
  for (let i = 0; i < taskList.length; i += 1) {
    for (let j = i + 1; j < taskList.length; j += 1) {
      const a = taskList[i];
      const b = taskList[j];
      const aOwnsBForbidden = firstOverlappingPath(a.owned_paths ?? [], b.forbidden_paths ?? []);
      if (aOwnsBForbidden) {
        overlaps.push({ taskA: a.task_id, taskB: b.task_id, path: aOwnsBForbidden });
      }
      const bOwnsAForbidden = firstOverlappingPath(b.owned_paths ?? [], a.forbidden_paths ?? []);
      if (bOwnsAForbidden) {
        overlaps.push({ taskA: b.task_id, taskB: a.task_id, path: bOwnsAForbidden });
      }
    }
  }
  return overlaps;
}

export function sharedFileConflicts(taskList) {
  const conflicts = [];
  for (let i = 0; i < taskList.length; i += 1) {
    for (let j = i + 1; j < taskList.length; j += 1) {
      const a = taskList[i];
      const b = taskList[j];
      if (a.shared_files_required !== true && b.shared_files_required !== true) continue;
      const aAllows = Array.isArray(a.parallel_safe_with) && a.parallel_safe_with.includes(b.task_id);
      const bAllows = Array.isArray(b.parallel_safe_with) && b.parallel_safe_with.includes(a.task_id);
      if (aAllows && bAllows) continue;
      const reason = a.shared_files_required === true && b.shared_files_required === true
        ? `both ${a.task_id} and ${b.task_id} require shared_files_required and do not mutually list each other in parallel_safe_with`
        : `${a.shared_files_required === true ? a.task_id : b.task_id} requires shared_files_required alongside ${a.shared_files_required === true ? b.task_id : a.task_id} without mutual parallel_safe_with`;
      conflicts.push({ taskA: a.task_id, taskB: b.task_id, reason });
    }
  }
  return conflicts;
}

export function riskRequirements(task) {
  const violations = [];
  if (task?.risk !== "T3") return violations;

  if (task.cross_account_verification_required !== true) {
    violations.push(`T3 task ${task.task_id} must have cross_account_verification_required === true`);
  }
  const verificationRequired = Array.isArray(task.verification_required) ? task.verification_required : [];
  const hasCrossAccountVerification = verificationRequired.some(
    (v) => typeof v === "string" && v.toLowerCase().includes("cross_account"),
  );
  if (!hasCrossAccountVerification) {
    violations.push(`T3 task ${task.task_id} must have a verification_required entry containing "cross_account"`);
  }

  if (task.action === "REPAIR" || task.action === "BUILD") {
    const documentedOwnership =
      (typeof task.notes === "string" && task.notes.trim().length > 0) ||
      (typeof task.canonical_ownership_note === "string" && task.canonical_ownership_note.trim().length > 0);
    if (!documentedOwnership && task.status !== "DESIGN_REQUIRED") {
      violations.push(
        `T3 ${task.action} task ${task.task_id} has no documented target ownership/migration semantics (notes or canonical_ownership_note) and must be DESIGN_REQUIRED, not ${task.status}`,
      );
    }
  }

  return violations;
}

const UNSAFE_STATUSES = new Set(["BLOCKED_DEPENDENCY", "BLOCKED_ENV", "BLOCKED_SEMANTICS", "FAILED"]);

// Which statuses count as "this task's work has actually happened" for the
// purpose of clearing another task's dependencies/blocked_by. Conservative
// default, matches the hardening brief's "prefer conservative behavior":
// READY_TO_INTEGRATE is deliberately excluded (integration can still fail or
// be rejected after that point), as are PLANNED/LEASED/IN_PROGRESS/
// READY_TO_VERIFY/DESIGN_REQUIRED/BLOCKED_*/FAILED/RECOVERY_REQUIRED (the
// lease-expiry recovery status) -- all of those are treated as unsatisfying,
// same as any other incomplete state.
export const SATISFYING_DEPENDENCY_STATUSES = new Set(["VERIFIED", "INTEGRATED"]);

function unresolvedEntries(tasksById, field, taskIdKey, statusKey) {
  const results = [];
  for (const task of tasksById.values()) {
    for (const refId of task[field] ?? []) {
      const refTask = tasksById.get(refId);
      const status = refTask ? refTask.status : "UNRESOLVED";
      if (!refTask || !SATISFYING_DEPENDENCY_STATUSES.has(status)) {
        results.push({ task_id: task.task_id, [taskIdKey]: refId, [statusKey]: status });
      }
    }
  }
  return results;
}

// For every task, for every entry in its `dependencies` array: satisfied iff
// the referenced task's status is in SATISFYING_DEPENDENCY_STATUSES. An
// unresolvable reference (task_id not in tasksById) counts as unsatisfied,
// reported with dependency_status "UNRESOLVED".
export function dependencySatisfaction(tasksById) {
  return { unsatisfiedDependencies: unresolvedEntries(tasksById, "dependencies", "dependency", "dependency_status") };
}

// Same shape and same clearing rule (SATISFYING_DEPENDENCY_STATUSES) applied
// to `blocked_by` -- for consistency, since the brief doesn't specify a
// different clearing rule and asks for an explicit, documented, conservative
// choice.
export function blockedByClearance(tasksById) {
  return { unclearedBlockers: unresolvedEntries(tasksById, "blocked_by", "blocker", "blocker_status") };
}

export function analyzeWave(wave, allTasksById) {
  const taskIds = Object.values(wave?.accounts ?? {}).flat();
  const taskList = [];
  const unresolvedTaskIds = [];
  for (const taskId of taskIds) {
    const task = allTasksById.get(taskId);
    if (!task) unresolvedTaskIds.push(taskId);
    else taskList.push(task);
  }

  if (taskList.length === 0) {
    return {
      provesDependencySafe: unresolvedTaskIds.length === 0,
      provesNoOwnedPathOverlap: true,
      cycles: [],
      blockedByCycles: [],
      ownedPathOverlaps: [],
      forbiddenPathOverlaps: [],
      sharedFileConflicts: [],
      riskViolations: [],
      unresolvedTaskIds,
      unsatisfiedDependencies: [],
      unclearedBlockers: [],
    };
  }

  const { closure: depClosure, cycles } = dependencyClosure(allTasksById);
  const { cycles: blockedByCycles } = blockedByClosure(allTasksById);

  // A dependency/blocked_by cycle "belongs" to the wave if it involves at
  // least one task actually in the wave.
  const waveTaskIdSet = new Set(taskList.map((t) => t.task_id));
  const relevantCycles = cycles.filter((c) => c.some((id) => waveTaskIdSet.has(id)));
  const relevantBlockedByCycles = blockedByCycles.filter((c) => c.some((id) => waveTaskIdSet.has(id)));

  // Every dependencies/blocked_by reference for every wave task must resolve
  // to a real task_id in allTasksById -- an unresolvable reference is itself
  // a safety failure.
  let hasUnresolvedGraphRef = false;
  for (const t of taskList) {
    for (const dep of [...(t.dependencies ?? []), ...(t.blocked_by ?? [])]) {
      if (!allTasksById.has(dep)) hasUnresolvedGraphRef = true;
    }
  }
  void depClosure; // computed for completeness/reuse by callers; not needed further here

  const hasUnsafeStatus = taskList.some((t) => UNSAFE_STATUSES.has(t.status));

  const ownedOverlaps = ownedPathOverlaps(taskList);
  const forbiddenOverlaps = forbiddenPathOverlaps(taskList);
  const sharedConflicts = sharedFileConflicts(taskList);

  const riskViolations = taskList
    .map((t) => ({ task_id: t.task_id, violations: riskRequirements(t) }))
    .filter((r) => r.violations.length > 0);

  // Restricted to the wave's own task list -- only report entries whose
  // task_id is in the wave, even though the dependency/blocker itself may be
  // outside the wave (that's fine and expected; it's the point of v0.1's
  // "all dependencies satisfied BEFORE the wave" rule).
  const { unsatisfiedDependencies: allUnsatisfiedDeps } = dependencySatisfaction(allTasksById);
  const { unclearedBlockers: allUnclearedBlockers } = blockedByClearance(allTasksById);
  const unsatisfiedDependencies = allUnsatisfiedDeps.filter((e) => waveTaskIdSet.has(e.task_id));
  const unclearedBlockers = allUnclearedBlockers.filter((e) => waveTaskIdSet.has(e.task_id));

  const provesDependencySafe =
    unresolvedTaskIds.length === 0 &&
    relevantCycles.length === 0 &&
    relevantBlockedByCycles.length === 0 &&
    !hasUnsafeStatus &&
    !hasUnresolvedGraphRef &&
    unsatisfiedDependencies.length === 0 &&
    unclearedBlockers.length === 0;

  const provesNoOwnedPathOverlap =
    ownedOverlaps.length === 0 && forbiddenOverlaps.length === 0 && sharedConflicts.length === 0;

  return {
    provesDependencySafe,
    provesNoOwnedPathOverlap,
    cycles: relevantCycles,
    blockedByCycles: relevantBlockedByCycles,
    ownedPathOverlaps: ownedOverlaps,
    forbiddenPathOverlaps: forbiddenOverlaps,
    sharedFileConflicts: sharedConflicts,
    riskViolations,
    unresolvedTaskIds,
    unsatisfiedDependencies,
    unclearedBlockers,
  };
}

if (process.argv[1] === fileURLToPath(import.meta.url)) {
  let failures = 0;
  const check = (name, cond) => {
    if (cond) {
      console.log(`PASS: ${name}`);
    } else {
      console.log(`FAIL: ${name}`);
      failures += 1;
    }
  };

  const mkTask = (overrides) => ({
    task_id: "T-0",
    action: "REPAIR",
    node_ids: [],
    priority: 1,
    risk: "T1",
    owned_paths: [],
    forbidden_paths: [],
    dependencies: [],
    blocked_by: [],
    status: "READY",
    verification_required: [],
    cross_account_verification_required: false,
    shared_files_required: false,
    parallel_conflict_group: "g",
    ...overrides,
  });

  // (a) 2-task dependency cycle correctly detected.
  {
    const tasksById = new Map([
      ["T-1", mkTask({ task_id: "T-1", dependencies: ["T-2"] })],
      ["T-2", mkTask({ task_id: "T-2", dependencies: ["T-1"] })],
    ]);
    const { cycles } = dependencyClosure(tasksById);
    check("(a) 2-task cycle detected", cycles.length === 1 && cycles[0].includes("T-1") && cycles[0].includes("T-2"));
  }

  // (b) two tasks with overlapping owned_paths correctly flagged.
  {
    const taskA = mkTask({ task_id: "T-A", owned_paths: ["crates/core/foo/**"] });
    const taskB = mkTask({ task_id: "T-B", owned_paths: ["crates/core/foo/bar.rs"] });
    const overlaps = ownedPathOverlaps([taskA, taskB]);
    check("(b) overlapping owned_paths flagged", overlaps.length === 1 && overlaps[0].taskA === "T-A" && overlaps[0].taskB === "T-B");
  }

  // (c) T3 task without cross_account_verification_required correctly flagged.
  {
    const t3 = mkTask({ task_id: "T-3", risk: "T3", cross_account_verification_required: false, verification_required: ["focused_compile"] });
    const violations = riskRequirements(t3);
    check("(c) T3 missing cross_account_verification_required flagged", violations.length >= 1);
  }

  // (d) a clean 2-task wave with no issues returns both proves-booleans true.
  {
    const taskA = mkTask({ task_id: "T-D1", owned_paths: ["crates/core/alpha/**"], status: "READY" });
    const taskB = mkTask({ task_id: "T-D2", owned_paths: ["crates/core/beta/**"], status: "READY" });
    const allTasksById = new Map([["T-D1", taskA], ["T-D2", taskB]]);
    const wave = { wave_id: "WAVE-TEST", accounts: { accountA: ["T-D1"], accountB: ["T-D2"] } };
    const report = analyzeWave(wave, allTasksById);
    check(
      "(d) clean 2-task wave proves safe",
      report.provesDependencySafe === true &&
        report.provesNoOwnedPathOverlap === true &&
        report.cycles.length === 0 &&
        report.blockedByCycles.length === 0 &&
        report.ownedPathOverlaps.length === 0 &&
        report.forbiddenPathOverlaps.length === 0 &&
        report.sharedFileConflicts.length === 0 &&
        report.riskViolations.length === 0 &&
        report.unresolvedTaskIds.length === 0,
    );
  }

  // Bonus: empty wave is trivially safe, not an error.
  {
    const report = analyzeWave({ wave_id: "WAVE-EMPTY", accounts: {} }, new Map());
    check("(e) empty wave trivially safe", report.provesDependencySafe === true && report.provesNoOwnedPathOverlap === true);
  }

  // Bonus: unresolved task_id in a wave fails provesDependencySafe.
  {
    const report = analyzeWave({ wave_id: "WAVE-BAD", accounts: { accountA: ["MISSING-1"] } }, new Map());
    check("(f) unresolved task_id fails provesDependencySafe", report.provesDependencySafe === false && report.unresolvedTaskIds.includes("MISSING-1"));
  }

  // (g) task with an unsatisfied dependency (dependency at status READY) is
  // excluded from provesDependencySafe:true and appears in unsatisfiedDependencies.
  {
    const depTask = mkTask({ task_id: "T-G-DEP", status: "READY" });
    const task = mkTask({ task_id: "T-G", dependencies: ["T-G-DEP"] });
    const allTasksById = new Map([["T-G-DEP", depTask], ["T-G", task]]);
    const wave = { wave_id: "WAVE-G", accounts: { accountA: ["T-G", "T-G-DEP"] } };
    const report = analyzeWave(wave, allTasksById);
    check(
      "(g) unsatisfied dependency (READY) excludes provesDependencySafe and is reported",
      report.provesDependencySafe === false &&
        report.unsatisfiedDependencies.some((e) => e.task_id === "T-G" && e.dependency === "T-G-DEP" && e.dependency_status === "READY"),
    );
  }

  // (h) task whose dependency is VERIFIED is NOT flagged (satisfied).
  {
    const depTask = mkTask({ task_id: "T-H-DEP", status: "VERIFIED" });
    const task = mkTask({ task_id: "T-H", dependencies: ["T-H-DEP"] });
    const allTasksById = new Map([["T-H-DEP", depTask], ["T-H", task]]);
    const wave = { wave_id: "WAVE-H", accounts: { accountA: ["T-H", "T-H-DEP"] } };
    const report = analyzeWave(wave, allTasksById);
    check(
      "(h) VERIFIED dependency is satisfied, not flagged",
      report.provesDependencySafe === true && report.unsatisfiedDependencies.length === 0,
    );
  }

  // (i) task whose dependency is READY_TO_INTEGRATE is STILL flagged unsatisfied
  // (proving the conservative exclusion of READY_TO_INTEGRATE).
  {
    const depTask = mkTask({ task_id: "T-I-DEP", status: "READY_TO_INTEGRATE" });
    const task = mkTask({ task_id: "T-I", dependencies: ["T-I-DEP"] });
    const allTasksById = new Map([["T-I-DEP", depTask], ["T-I", task]]);
    const { unsatisfiedDependencies } = dependencySatisfaction(allTasksById);
    check(
      "(i) READY_TO_INTEGRATE dependency still flagged unsatisfied",
      unsatisfiedDependencies.some((e) => e.task_id === "T-I" && e.dependency === "T-I-DEP" && e.dependency_status === "READY_TO_INTEGRATE"),
    );
  }

  // (j) task with an uncleared blocked_by entry is excluded and appears in
  // unclearedBlockers.
  {
    const blockerTask = mkTask({ task_id: "T-J-BLOCKER", status: "IN_PROGRESS" });
    const task = mkTask({ task_id: "T-J", blocked_by: ["T-J-BLOCKER"] });
    const allTasksById = new Map([["T-J-BLOCKER", blockerTask], ["T-J", task]]);
    const wave = { wave_id: "WAVE-J", accounts: { accountA: ["T-J", "T-J-BLOCKER"] } };
    const report = analyzeWave(wave, allTasksById);
    check(
      "(j) uncleared blocked_by excludes provesDependencySafe and is reported",
      report.provesDependencySafe === false &&
        report.unclearedBlockers.some((e) => e.task_id === "T-J" && e.blocker === "T-J-BLOCKER" && e.blocker_status === "IN_PROGRESS"),
    );
  }

  // (k) task whose blocker is VERIFIED is not flagged.
  {
    const blockerTask = mkTask({ task_id: "T-K-BLOCKER", status: "VERIFIED" });
    const task = mkTask({ task_id: "T-K", blocked_by: ["T-K-BLOCKER"] });
    const allTasksById = new Map([["T-K-BLOCKER", blockerTask], ["T-K", task]]);
    const { unclearedBlockers } = blockedByClearance(allTasksById);
    check(
      "(k) VERIFIED blocker is cleared, not flagged",
      !unclearedBlockers.some((e) => e.task_id === "T-K"),
    );
  }

  if (failures > 0) {
    console.log(`\n${failures} self-test case(s) FAILED`);
    process.exit(1);
  } else {
    console.log("\nAll self-test cases PASSED");
    process.exit(0);
  }
}
