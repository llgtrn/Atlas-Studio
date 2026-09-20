#!/usr/bin/env node
// Repository-hygiene / branch-archaeology clearance gate for RETIRE tasks.
//
// Every RETIRE task in v0's frontiers is justified purely by CURRENT
// reachability ("no live caller on main today") -- see e.g. RETIRE-0003's
// ~40 OSINT probe binaries, or RETIRE-0001/0002's self-flagged islands.
// That is NOT sufficient justification to actually retire something: a
// separate, not-yet-integrated "Lane C" repository-hygiene/branch-archaeology
// effort exists specifically because historical branches can carry unique
// code/semantics invisible to a current-reachability scan.
//
// This module is the seam Lane C's real evidence plugs into later. It does
// NOT fabricate or guess what that evidence will say. Today, with zero real
// Lane C evidence anywhere in the Atlas, every RETIRE task must fail closed:
// dispatch is blocked until a task carries both a real clearance value AND
// at least one real evidence id backing it.
//
// Pure ESM. Zero dependencies. Zero file I/O in the exported logic -- the
// self-test block at the bottom may READ real files for reporting only
// (never write), guarded so it can never run on import (see lib.mjs's note
// on the Atlas already having been bitten by exactly that bug once).
import { fileURLToPath } from "node:url";

// The full state space for hygiene_clearance. NOT_CHECKED is the default/
// absent state and is deliberately NOT in CLEARED_FOR_RETIREMENT: absence of
// information is not evidence of safety. BLOCKED_UNIQUE_VALUE and
// BLOCKED_UNKNOWN are real negative findings from Lane C (unique historical
// value found, or investigation inconclusive) -- both must keep blocking
// retirement, same as NOT_CHECKED. AWAITING_CROSS_ACCOUNT_VERIFICATION is a
// positive-leaning-but-not-final state (one account cleared it, a second
// hasn't confirmed yet) and also must not clear retirement on its own.
export const HYGIENE_CLEARANCE_VALUES = [
  "NOT_CHECKED",
  "NO_HISTORICAL_UNIQUE_VALUE",
  "FULLY_HARVESTED",
  "BLOCKED_UNIQUE_VALUE",
  "BLOCKED_UNKNOWN",
  "AWAITING_CROSS_ACCOUNT_VERIFICATION",
];

// The only two values that represent real clearance to actually retire
// something: either Lane C found nothing of historical unique value, or it
// found something and fully harvested it out (into docs/tests/wherever it
// belongs) before signing off on the retirement.
export const CLEARED_FOR_RETIREMENT = new Set(["NO_HISTORICAL_UNIQUE_VALUE", "FULLY_HARVESTED"]);

// The seam object every RETIRE task should carry until real Lane C evidence
// exists. hygiene_clearance_source documents provenance/requiredness;
// hygiene_evidence_ids (top-level) is the one gate/other code should read.
// Kept duplicated with hygiene_clearance_source.evidence_ids on purpose --
// simple and JSON-serializable, not an attempt at a single source of truth.
export function defaultHygieneClearance() {
  return {
    hygiene_clearance: "NOT_CHECKED",
    hygiene_clearance_source: {
      required: true,
      source: "repository-hygiene",
      evidence_ids: [],
    },
    hygiene_evidence_ids: [],
  };
}

// Narrow, inline T3 check (deliberately not BF2's full riskRequirements()):
// a T3 RETIRE task is only exempt from blocking on the risk/ownership check
// when it has been explicitly marked as needing (and, implicitly, routed
// through) cross-account verification. Any other T3 shape fails this check.
// Ownership scope counted the same way mutation-ownership.mjs counts it: a
// blank/whitespace-only entry is not a real path (it matches no crate under
// pathsOverlap's prefix logic), so counting it would let a RETIRE task with
// owned_paths: ["   "] clear THIS gate while validateMutationOwnership()
// rejects the very same task -- two gates disagreeing about the same fact.
// BX6 adversarial finding; keep these two definitions in step.
function realOwnedPathCount(task) {
  return (task.owned_paths ?? []).filter((p) => typeof p === "string" && p.trim().length > 0).length;
}

function passesRiskCheck(task) {
  if (task.risk !== "T3") return true;
  return task.cross_account_verification_required === true && realOwnedPathCount(task) > 0;
}

export function retireDispatchGate(task) {
  const hygieneClearanceOk = CLEARED_FOR_RETIREMENT.has(task.hygiene_clearance);
  const hygieneEvidenceOk = (task.hygiene_evidence_ids ?? []).length > 0;
  const noProductDecisionPending = task.product_decision_required !== true;
  const riskOk = passesRiskCheck(task);
  const ownershipOk = realOwnedPathCount(task) > 0;

  const allowed = hygieneClearanceOk && hygieneEvidenceOk && noProductDecisionPending && riskOk && ownershipOk;

  if (allowed) {
    return { allowed: true, reason: "hygiene clearance, evidence, product decision, risk, and ownership checks all pass", recommendedStatus: null };
  }

  const failedChecks = [];
  if (!hygieneClearanceOk) {
    failedChecks.push(
      `hygiene_clearance is ${JSON.stringify(task.hygiene_clearance ?? null)}, not one of ${[...CLEARED_FOR_RETIREMENT].join("/")}`,
    );
  }
  if (!hygieneEvidenceOk) {
    failedChecks.push("hygiene_evidence_ids is empty (fail-closed: clearance value alone is not enough without real evidence backing it)");
  }
  if (!noProductDecisionPending) {
    failedChecks.push("product_decision_required is true");
  }
  if (!riskOk) {
    failedChecks.push("risk is T3 without both cross_account_verification_required=true and non-empty owned_paths");
  }
  if (!ownershipOk) {
    failedChecks.push("owned_paths has no real entries -- empty, or blank/whitespace-only strings only (mutating RETIRE task has no ownership scope)");
  }

  // BLOCKED_SEMANTICS is the primary status this module exists to produce --
  // "needs investigation before dispatch" is the existing Atlas convention
  // for missing/incomplete evidence (mirrors legacy-debt-census.mjs's
  // UNKNOWN classification: "needs manual root-cause investigation before
  // repair dispatch"). It wins whenever hygiene clearance/evidence is the
  // (or a) blocking reason, even alongside other failures.
  let recommendedStatus;
  if (!hygieneClearanceOk || !hygieneEvidenceOk) {
    recommendedStatus = "BLOCKED_SEMANTICS";
  } else if (!noProductDecisionPending) {
    recommendedStatus = "BLOCKED_DEPENDENCY";
  } else {
    recommendedStatus = "DESIGN_REQUIRED";
  }

  return { allowed: false, reason: failedChecks.join("; "), recommendedStatus };
}

// Convenience batch form: given an array of RETIRE-action tasks, returns
// proposed { task_id, status, reason } patches for every task the gate does
// NOT allow. Never mutates input, never writes files -- the coordinator (not
// this module) is responsible for applying these patches to the real
// frontier files.
export function applyRetireDispatchGate(tasks) {
  const patches = [];
  for (const task of tasks ?? []) {
    const result = retireDispatchGate(task);
    if (result.allowed) continue;
    patches.push({ task_id: task.task_id, status: result.recommendedStatus, reason: result.reason });
  }
  return patches;
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

  const mkRetireTask = (overrides) => ({
    task_id: "RETIRE-TEST",
    action: "RETIRE",
    risk: "T1",
    owned_paths: ["crates/cap/example/**"],
    ...defaultHygieneClearance(),
    ...overrides,
  });

  // (a) default/NOT_CHECKED clearance -> blocked, BLOCKED_SEMANTICS.
  {
    const task = mkRetireTask({});
    const result = retireDispatchGate(task);
    check(
      "(a) NOT_CHECKED clearance blocked with BLOCKED_SEMANTICS",
      result.allowed === false && result.recommendedStatus === "BLOCKED_SEMANTICS",
    );
  }

  // (b) FULLY_HARVESTED clearance but zero evidence ids -> still blocked
  // (fail-closed on missing evidence, explicitly called out in the brief).
  {
    const task = mkRetireTask({ hygiene_clearance: "FULLY_HARVESTED", hygiene_evidence_ids: [] });
    const result = retireDispatchGate(task);
    check(
      "(b) FULLY_HARVESTED with zero evidence ids still blocked, BLOCKED_SEMANTICS",
      result.allowed === false && result.recommendedStatus === "BLOCKED_SEMANTICS",
    );
  }

  // (c) FULLY_HARVESTED + real evidence + owned_paths + T1 risk -> allowed.
  {
    const task = mkRetireTask({
      hygiene_clearance: "FULLY_HARVESTED",
      hygiene_evidence_ids: ["harvest:123"],
      owned_paths: ["crates/cap/foo/**"],
      risk: "T1",
    });
    const result = retireDispatchGate(task);
    check("(c) fully cleared T1 task allowed", result.allowed === true && result.recommendedStatus === null);
  }

  // (d) same as (c) but product_decision_required: true -> blocked, BLOCKED_DEPENDENCY.
  {
    const task = mkRetireTask({
      hygiene_clearance: "FULLY_HARVESTED",
      hygiene_evidence_ids: ["harvest:123"],
      owned_paths: ["crates/cap/foo/**"],
      risk: "T1",
      product_decision_required: true,
    });
    const result = retireDispatchGate(task);
    check(
      "(d) product_decision_required blocks with BLOCKED_DEPENDENCY",
      result.allowed === false && result.recommendedStatus === "BLOCKED_DEPENDENCY",
    );
  }

  // (e) same as (c) but owned_paths: [] -> blocked, DESIGN_REQUIRED.
  {
    const task = mkRetireTask({
      hygiene_clearance: "FULLY_HARVESTED",
      hygiene_evidence_ids: ["harvest:123"],
      owned_paths: [],
      risk: "T1",
    });
    const result = retireDispatchGate(task);
    check(
      "(e) empty owned_paths blocks with DESIGN_REQUIRED",
      result.allowed === false && result.recommendedStatus === "DESIGN_REQUIRED",
    );
  }

  // applyRetireDispatchGate: only non-allowed tasks produce patches.
  {
    const allowedTask = mkRetireTask({
      task_id: "RETIRE-OK",
      hygiene_clearance: "NO_HISTORICAL_UNIQUE_VALUE",
      hygiene_evidence_ids: ["harvest:1"],
      owned_paths: ["crates/cap/bar/**"],
    });
    const blockedTask = mkRetireTask({ task_id: "RETIRE-BLOCKED" });
    const patches = applyRetireDispatchGate([allowedTask, blockedTask]);
    check(
      "(f) applyRetireDispatchGate skips allowed, patches blocked",
      patches.length === 1 && patches[0].task_id === "RETIRE-BLOCKED" && patches[0].status === "BLOCKED_SEMANTICS",
    );
  }

  if (failures > 0) {
    console.log(`\n${failures} self-test case(s) FAILED`);
  } else {
    console.log("\nAll self-test cases PASSED");
  }

  // Extra real-data sanity check: read-only, printed but NOT asserted
  // against. Loads the real retire-frontier.json (if present), merges
  // defaultHygieneClearance() onto any task missing hygiene fields, and
  // runs every task through the gate. Expectation: ALL real RETIRE tasks
  // today are blocked, since none carry real Lane C hygiene evidence yet --
  // that is the correct, intended fail-closed outcome, not a bug.
  try {
    const { readFileSync } = await import("node:fs");
    const { resolve, dirname } = await import("node:path");
    const path = resolve(dirname(fileURLToPath(import.meta.url)), "..", "..", "docs/_machine/system-atlas/v0/retire-frontier.json");
    const frontier = JSON.parse(readFileSync(path, "utf8"));
    const tasks = frontier.tasks ?? [];
    let allowedCount = 0;
    let blockedCount = 0;
    console.log(`\nReal retire-frontier.json sanity check (${tasks.length} task(s)):`);
    for (const rawTask of tasks) {
      const task = { ...defaultHygieneClearance(), ...rawTask };
      const result = retireDispatchGate(task);
      if (result.allowed) {
        allowedCount += 1;
      } else {
        blockedCount += 1;
      }
      console.log(`  ${task.task_id}: ${result.allowed ? "ALLOWED" : `BLOCKED (${result.recommendedStatus})`} -- ${result.reason}`);
    }
    console.log(`Summary: ${allowedCount} allowed, ${blockedCount} blocked out of ${tasks.length}`);
  } catch (err) {
    console.log(`\n(real retire-frontier.json sanity check skipped: ${err.message})`);
  }

  process.exit(failures > 0 ? 1 : 0);
}
