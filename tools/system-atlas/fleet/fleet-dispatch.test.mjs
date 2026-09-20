import { test } from "node:test";
import assert from "node:assert/strict";
import { allocateDonors, loadFleetRegistry } from "./donor-allocator.mjs";
import { buildFleetPlan } from "./fleet-plan.mjs";
import { dispatchFleetWave, validateWorkerResult } from "./fleet-dispatcher.mjs";

test("allocation stays closed-world while executable donor work remains sparse", () => {
  const allocation = allocateDonors({ now: "2026-09-19T00:00:00.000Z" });
  const registry = loadFleetRegistry();
  const plan = buildFleetPlan({
    allocation, registry, reports: [], chronicaSha: "c".repeat(40), now: "2026-09-19T00:00:00.000Z"
  });
  const donorTasks = plan.tasks.filter((task) => task.kind === "DONOR_CENSUS");
  const deepAdmitted = allocation.allocations.filter((row) =>
    !["DUPLICATE","REFERENCE_ONLY","NOT_NEEDED","REDUNDANT","RETIRED"].includes(row.status)
    && (row.needed === "YES" || ["NEEDED","CENSUSED","ABSORBING","PARTIALLY_ABSORBED"].includes(row.status))
  );
  assert.equal(allocation.allocations_total, allocation.donor_corpus_total);
  assert.equal(allocation.skipped_total, 0);
  assert.equal(allocation.unallocated_total, 0);
  assert.equal(donorTasks.length, deepAdmitted.length);
  assert.equal(plan.tasks.filter((task) => task.kind === "DONOR_RELOCATE").length, 0);
  assert.ok(plan.tasks.some((task) => task.kind === "DONOR_INTAKE"));
  assert.equal(new Set(donorTasks.map((task) => task.donor_id)).size, deepAdmitted.length);
  const activeCells = registry.development_cells.filter((row) => row.active !== false);
  const resetTasks = plan.tasks.filter((task) => task.kind === "CELL_RESET_BASELINE");
  assert.equal(resetTasks.length, activeCells.length);
  const ready = new Set(plan.ready_wave);
  assert.ok(resetTasks.every((task) => ready.has(task.id)));
  assert.ok(plan.ready_wave_total >= activeCells.length);
});

test("unknown discovered donors remain allocated cold inventory with zero work items", () => {
  const allocation = allocateDonors({ now: "2026-09-19T00:00:00.000Z" });
  const registry = loadFleetRegistry();
  const donor = allocation.allocations.find((row) => row.status === "DISCOVERED" && row.needed === "UNKNOWN");
  assert.ok(donor, "fixture corpus should contain at least one unknown discovered donor");
  const plan = buildFleetPlan({
    allocation, registry, reports: [], chronicaSha: "c".repeat(40), now: "2026-09-19T00:00:00.000Z"
  });
  assert.equal(plan.tasks.some((task) => task.donor_id === donor.donor_id), false);
});

test("explicitly needed donors retain the deep intake or census path", () => {
  const allocation = allocateDonors({ now: "2026-09-19T00:00:00.000Z" });
  const registry = loadFleetRegistry();
  const donor = allocation.allocations.find((row) =>
    row.needed === "YES" && !["DUPLICATE","REFERENCE_ONLY","NOT_NEEDED","REDUNDANT","RETIRED"].includes(row.status)
  );
  assert.ok(donor, "fixture corpus should contain at least one explicitly needed donor");
  const plan = buildFleetPlan({
    allocation, registry, reports: [], chronicaSha: "c".repeat(40), now: "2026-09-19T00:00:00.000Z"
  });
  assert.ok(plan.tasks.some((task) => task.donor_id === donor.donor_id && task.kind === "DONOR_CENSUS"));
  assert.equal(plan.tasks.some((task) => task.donor_id === donor.donor_id && task.kind === "DONOR_ACCOUNT"), false);
});

test("ready fleet wave never mutates the same repository twice", () => {
  const plan = buildFleetPlan({ reports: [], chronicaSha: "c".repeat(40), now: "2026-09-19T00:00:00.000Z" });
  const byId = new Map(plan.tasks.map((task) => [task.id, task]));
  const repos = plan.ready_wave.map((id) => byId.get(id).target_repo);
  assert.equal(new Set(repos).size, repos.length);
});

test("dispatcher dry-run exposes provider work without pretending a Claude session exists", async () => {
  const plan = buildFleetPlan({ reports: [], chronicaSha: "c".repeat(40), now: "2026-09-19T00:00:00.000Z" });
  const result = await dispatchFleetWave({ plan, provider: "CLAUDE", execute: false });
  assert.equal(result.mode, "DRY_RUN");
  assert.equal(result.tasks_total, plan.ready_wave_total);
});

test("worker provider result is evidence-linked and cannot drift from assigned repo/base", () => {
  const task = {
    id: "task-1",
    kind: "PACKAGE_BUILD",
    target_repo: "llgtrn/TradeOps",
    target_cell_id: "trade-logistics",
    target_package_id: "trade-logistics",
    donor_source_path: "temporary/donors/D123/source",
    base_sha: "a".repeat(40),
  };
  const result = {
    schema: "chronica.system-atlas.fleet-worker-result.v1",
    task_id: "task-1",
    provider: "CLAUDE",
    status: "COMPLETE",
    target_repo: "llgtrn/TradeOps",
    target_cell_id: "trade-logistics",
    target_package_id: "trade-logistics",
    donor_source_path: "temporary/donors/D123/source",
    base_sha: "a".repeat(40),
    candidate_sha: "b".repeat(40),
    thread_id: "provider-thread-123",
    tests: ["cargo test"],
    evidence: ["mirror-report.json"],
    discovered_slices: [],
    chronica_candidates: [],
    extinction: {
      applicable: true,
      files_removed: 1,
      remaining_scoped_files: 1,
      state: "PROGRESSED",
      blocker: null,
      deleted_paths: ["temporary/donors/D123/source/old.rs"],
    },
    reset_evidence: {
      applicable: false,
      generation: null,
      pre_reset_head_sha: null,
      preservation_ref: null,
    },
  };
  assert.equal(validateWorkerResult(task, result).candidate_sha, "b".repeat(40));
  assert.throws(() => validateWorkerResult(task, { ...result, target_repo: "llgtrn/FinOps" }));
  assert.throws(() => validateWorkerResult(task, { ...result, target_cell_id: "finance" }));
  assert.throws(() => validateWorkerResult(task, { ...result, target_package_id: "finance" }));
});

test("worker result channels are task-scoped", () => {
  const scoutTask = {
    id: "scout-1",
    kind: "DONOR_ABSORB_SCOUT",
    target_repo: "llgtrn/TradeOps",
    target_cell_id: "trade-logistics",
    target_package_id: "trade-logistics",
    base_sha: "a".repeat(40),
  };
  const scout = {
    schema: "chronica.system-atlas.fleet-worker-result.v1",
    task_id: "scout-1",
    provider: "CLAUDE",
    status: "COMPLETE",
    target_repo: "llgtrn/TradeOps",
    target_cell_id: "trade-logistics",
    target_package_id: "trade-logistics",
    base_sha: "a".repeat(40),
    candidate_sha: null,
    thread_id: "thread-scout",
    tests: [],
    evidence: ["read-only"],
    discovered_slices: [{
      id: "retry",
      behavior: "retry",
      target_responsibility: "RUNTIME",
      owned_scope: ["runtime/"],
      proof_expectation: ["test"],
      deletion_scope: ["temporary/donors/D123/source/retry"],
    }],
    chronica_candidates: [],
    extinction: {
      applicable: false,
      files_removed: 0,
      remaining_scoped_files: null,
      state: "NOT_APPLICABLE",
      blocker: null,
      deleted_paths: [],
    },
    reset_evidence: {
      applicable: false,
      generation: null,
      pre_reset_head_sha: null,
      preservation_ref: null,
    },
  };
  assert.equal(validateWorkerResult(scoutTask, scout).discovered_slices.length, 1);
  assert.throws(() => validateWorkerResult(scoutTask, { ...scout, candidate_sha: "b".repeat(40) }));

  const buildTask = { ...scoutTask, id: "build-1", kind: "PACKAGE_BUILD" };
  const build = {
    ...scout,
    task_id: "build-1",
    candidate_sha: "b".repeat(40),
    discovered_slices: [],
    extinction: {
      applicable: true,
      files_removed: 1,
      remaining_scoped_files: 0,
      state: "SOURCE_EXTINCT",
      blocker: null,
      deleted_paths: ["temporary/donors/D123/source/retry"],
    },
  };
  assert.equal(validateWorkerResult(buildTask, build).candidate_sha, "b".repeat(40));
  assert.throws(() => validateWorkerResult(buildTask, {
    ...build,
    chronica_candidates: [{
      schema: "chronica.system-atlas.chronica-candidate.v1",
      candidate_id: "bad",
      source_cell_id: "trade-logistics",
      source_package_id: "trade-logistics",
      source_repo: "llgtrn/TradeOps",
      source_repo_sha: "b".repeat(40),
      chronica_reference_sha: "c".repeat(40),
      semantic_owner: "runtime/",
      summary: "bad channel",
      donor_ids: [],
      affected_packages: [],
      evidence: [],
    }],
  }));
});

test("canonical integration cannot COMPLETE without resulting SHA", () => {
  const task = {
    id: "chronica-verify",
    kind: "CHRONICA_VERIFY_INTEGRATE",
    target_repo: "llgtrn/Chronica",
    target_cell_id: null,
    target_package_id: null,
    base_sha: "a".repeat(40),
  };
  const result = {
    schema: "chronica.system-atlas.fleet-worker-result.v1",
    task_id: task.id,
    provider: "CLAUDE",
    status: "COMPLETE",
    target_repo: task.target_repo,
    target_cell_id: null,
    target_package_id: null,
    base_sha: task.base_sha,
    candidate_sha: null,
    thread_id: "thread",
    tests: ["pnpm system-atlas:test"],
    evidence: [],
    discovered_slices: [],
    chronica_candidates: [],
    extinction: {
      applicable: false,
      files_removed: 0,
      remaining_scoped_files: null,
      state: "NOT_APPLICABLE",
      blocker: null,
      deleted_paths: [],
    },
    reset_evidence: {
      applicable: false,
      generation: null,
      pre_reset_head_sha: null,
      preservation_ref: null,
    },
  };
  assert.throws(() => validateWorkerResult(task, result));
});

test("package build cannot COMPLETE without donor source deletion", () => {
  const task = {
    id: "build-ratchet",
    kind: "PACKAGE_BUILD",
    target_repo: "llgtrn/TradeOps",
    target_cell_id: "trade-logistics",
    target_package_id: "trade-logistics",
    base_sha: "a".repeat(40),
    refoundation_generation: 1,
  };
  const result = {
    schema: "chronica.system-atlas.fleet-worker-result.v1",
    task_id: task.id,
    provider: "CLAUDE",
    status: "COMPLETE",
    target_repo: task.target_repo,
    target_cell_id: task.target_cell_id,
    target_package_id: task.target_package_id,
    base_sha: task.base_sha,
    candidate_sha: "b".repeat(40),
    thread_id: "thread",
    tests: ["cargo test"],
    evidence: [],
    discovered_slices: [],
    chronica_candidates: [],
    extinction: { applicable: true, files_removed: 0, remaining_scoped_files: 3, state: "BLOCKED", blocker: "dependency closure still retained", deleted_paths: [] },
    reset_evidence: { applicable: false, generation: null, pre_reset_head_sha: null, preservation_ref: null },
  };
  assert.throws(() => validateWorkerResult(task, result), /DELETE SOMETHING/);
});

test("clean Cell reset requires exact preservation evidence", () => {
  const task = {
    id: "cell-reset",
    kind: "CELL_RESET_BASELINE",
    target_repo: "llgtrn/TradeOps",
    target_cell_id: "trade-logistics",
    target_package_id: "trade-logistics",
    base_sha: "a".repeat(40),
    refoundation_generation: 1,
  };
  const result = {
    schema: "chronica.system-atlas.fleet-worker-result.v1",
    task_id: task.id,
    provider: "CLAUDE",
    status: "COMPLETE",
    target_repo: task.target_repo,
    target_cell_id: task.target_cell_id,
    target_package_id: task.target_package_id,
    base_sha: task.base_sha,
    candidate_sha: "b".repeat(40),
    thread_id: null,
    tests: ["deterministic Cell reset"],
    evidence: ["CLEAN_PACKAGE_BASELINE"],
    discovered_slices: [],
    chronica_candidates: [],
    extinction: { applicable: false, files_removed: 0, remaining_scoped_files: null, state: "NOT_APPLICABLE", blocker: null, deleted_paths: [] },
    reset_evidence: { applicable: true, generation: 1, pre_reset_head_sha: task.base_sha, preservation_ref: "archive/chronica-pre-reset/g1-aaaaaaaaaaaa" },
  };
  assert.equal(validateWorkerResult(task, result).reset_evidence.generation, 1);
});
