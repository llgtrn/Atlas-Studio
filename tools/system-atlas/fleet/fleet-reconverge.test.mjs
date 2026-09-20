import { test } from "node:test";
import assert from "node:assert/strict";
import { buildChronicaCandidateTask, buildChronicaCandidatePlan, buildReconvergePlan } from "./fleet-reconverge.mjs";

const candidate = {
  schema: "chronica.system-atlas.chronica-candidate.v1",
  candidate_id: "trade-retry-v1",
  source_cell_id: "trade-logistics",
  source_package_id: "trade-logistics",
  source_repo: "llgtrn/TradeOps",
  source_repo_sha: "a".repeat(40),
  chronica_reference_sha: "b".repeat(40),
  semantic_owner: "runtime/",
  summary: "bounded retry/reconciliation semantic proven in TradeOps",
  donor_ids: ["D123"],
  affected_packages: [{ repo: "llgtrn/TradingOps", cell_id: "trading", package_id: "trading" }],
  evidence: ["test:retry-parity"],
};

test("package proof becomes a separate Chronica candidate task", () => {
  const task = buildChronicaCandidateTask({ candidate, currentChronicaSha: "c".repeat(40) });
  assert.equal(task.kind, "CHRONICA_CANDIDATE");
  assert.equal(task.target_repo, "llgtrn/Chronica");
  assert.equal(task.base_sha, "c".repeat(40));
});

test("Chronica candidate plan is directly dispatchable as a one-repo wave", () => {
  const plan = buildChronicaCandidatePlan({ candidate, currentChronicaSha: "c".repeat(40), now: "2026-09-19T00:00:00.000Z" });
  assert.equal(plan.tasks_total, 1);
  assert.equal(plan.ready_wave_total, 1);
  assert.equal(plan.tasks[0].kind, "CHRONICA_CANDIDATE");
});

test("after Chronica merge, affected Ops are reconverged in parallel only when exact Ops SHAs are known", () => {
  const reports = [
    { cell_repo: "llgtrn/TradeOps", cell_sha: "d".repeat(40) },
    { cell_repo: "llgtrn/TradingOps", cell_sha: "e".repeat(40) },
  ];
  const plan = buildReconvergePlan({ candidate, mergedChronicaSha: "f".repeat(40), reports, now: "2026-09-19T00:00:00.000Z" });
  assert.equal(plan.tasks_total, 2);
  assert.equal(plan.ready_wave_total, 2);
  assert.equal(new Set(plan.tasks.map((task) => task.target_repo)).size, 2);
  assert.ok(plan.tasks.every((task) => task.kind === "PACKAGE_RECONVERGE"));
  assert.ok(plan.tasks.every((task) => task.chronica_reference_sha === "f".repeat(40)));
  assert.deepEqual(new Set(plan.tasks.map((task) => task.target_package_id)), new Set(["trade-logistics","trading"]));
});
