import { test } from "node:test";
import assert from "node:assert/strict";
import { advanceFleet } from "./fleet-advance.mjs";

const sha = (c) => c.repeat(40);

function baseTask(overrides = {}) {
  return {
    id: "task",
    kind: "DONOR_ABSORB_SCOUT",
    target_repo: "llgtrn/TradeOps",
    target_cell_id: "trade-logistics",
    target_package_id: "trade-logistics",
    target_package_kind: "DOMAIN_PACKAGE",
    refoundation_generation: 1,
    target_ref: "main",
    base_sha: sha("a"),
    donor_id: "D123",
    donor_repo: "owner/donor",
    donor_revision: sha("d"),
    donor_source_path: "temporary/donors/D123/source",
    candidate_sha: null,
    state: "READY",
    depends_on: [],
    owned_scope: [],
    deletion_scope: [],
    chronica_reference_sha: sha("c"),
    instructions: "x",
    ...overrides,
  };
}

function plan(task) {
  return {
    schema: "chronica.system-atlas.fleet-work-plan.v1",
    generated_at: "2026-09-19T00:00:00.000Z",
    canonical_repo: "llgtrn/Chronica",
    tasks_total: 1,
    ready_wave_total: 1,
    deferred_total: 0,
    tasks: [task],
    ready_wave: [task.id],
  };
}

function result(task, overrides = {}) {
  return {
    schema: "chronica.system-atlas.fleet-worker-result.v1",
    task_id: task.id,
    provider: "CLAUDE",
    status: "COMPLETE",
    target_repo: task.target_repo,
    target_cell_id: task.target_cell_id ?? null,
    target_package_id: task.target_package_id ?? null,
    base_sha: task.base_sha,
    candidate_sha: null,
    thread_id: "thread",
    tests: [],
    evidence: ["evidence"],
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
    ...overrides,
  };
}

test("scout result becomes bounded build tasks without a fake candidate SHA", () => {
  const task = baseTask();
  const advanced = advanceFleet({
    plan: plan(task),
    results: [result(task, {
      discovered_slices: [{
        id: "retry",
        behavior: "retry unknown outcomes",
        target_responsibility: "RUNTIME",
        owned_scope: ["runtime/src/recovery/"],
        proof_expectation: ["retry tests"],
        deletion_scope: ["temporary/donors/D123/source/retry.rs"],
      }],
    })],
    currentChronicaSha: sha("c"),
    now: "2026-09-19T00:00:00.000Z",
  });
  assert.equal(advanced.next_plan.tasks_total, 1);
  assert.equal(advanced.next_plan.tasks[0].kind, "PACKAGE_BUILD");
  assert.equal(advanced.next_plan.tasks[0].candidate_sha, null);
});

test("package build result becomes independent package verify/integrate task", () => {
  const task = baseTask({ kind: "PACKAGE_BUILD", owned_scope: ["runtime/"], deletion_scope: ["temporary/donors/D123/source/retry.rs"] });
  const advanced = advanceFleet({
    plan: plan(task),
    results: [result(task, {
      candidate_sha: sha("b"),
      tests: ["cargo test"],
      extinction: {
        applicable: true,
        files_removed: 1,
        remaining_scoped_files: 0,
        state: "SOURCE_EXTINCT",
        blocker: null,
        deleted_paths: ["temporary/donors/D123/source/retry.rs"],
      },
    })],
    currentChronicaSha: sha("c"),
  });
  assert.equal(advanced.next_plan.tasks[0].kind, "PACKAGE_VERIFY_INTEGRATE");
  assert.equal(advanced.next_plan.tasks[0].candidate_sha, sha("b"));
});

test("fused package build carries actual deleted donor paths into verification", () => {
  const task = baseTask({ kind: "PACKAGE_BUILD", owned_scope: ["runtime/"], deletion_scope: [] });
  const advanced = advanceFleet({
    plan: plan(task),
    results: [result(task, {
      candidate_sha: sha("b"),
      tests: ["cargo test"],
      extinction: {
        applicable: true,
        files_removed: 2,
        remaining_scoped_files: 7,
        state: "PARTIAL",
        blocker: null,
        deleted_paths: [
          "temporary/donors/D123/source/retry.rs",
          "temporary/donors/D123/source/retry_test.rs",
        ],
      },
    })],
    currentChronicaSha: sha("c"),
  });
  const verify = advanced.next_plan.tasks[0];
  assert.equal(verify.kind, "PACKAGE_VERIFY_INTEGRATE");
  assert.deepEqual(verify.deletion_scope, [
    "temporary/donors/D123/source/retry.rs",
    "temporary/donors/D123/source/retry_test.rs",
  ]);
});

test("fused package build rejects deletion evidence outside its donor root", () => {
  const task = baseTask({ kind: "PACKAGE_BUILD", owned_scope: ["runtime/"], deletion_scope: [] });
  assert.throws(() => advanceFleet({
    plan: plan(task),
    results: [result(task, {
      candidate_sha: sha("b"),
      extinction: {
        applicable: true,
        files_removed: 1,
        remaining_scoped_files: 7,
        state: "PARTIAL",
        blocker: null,
        deleted_paths: ["runtime/src/not-donor.rs"],
      },
    })],
    currentChronicaSha: sha("c"),
  }), /deletion escapes donor root/);
});

test("integrated package may emit a separate Chronica candidate task", () => {
  const task = baseTask({ kind: "PACKAGE_VERIFY_INTEGRATE", candidate_sha: sha("b"), owned_scope: ["runtime/"] });
  const candidate = {
    schema: "chronica.system-atlas.chronica-candidate.v1",
    candidate_id: "retry-v1",
    source_cell_id: "trade-logistics",
    source_package_id: "trade-logistics",
    source_repo: "llgtrn/TradeOps",
    source_repo_sha: sha("d"),
    chronica_reference_sha: sha("c"),
    semantic_owner: "runtime/",
    summary: "universal retry semantics",
    donor_ids: ["D123"],
    affected_packages: [{ repo: "llgtrn/TradingOps", cell_id: "trading", package_id: "trading" }],
    evidence: ["ops integration"],
  };
  const advanced = advanceFleet({
    plan: plan(task),
    results: [result(task, { candidate_sha: sha("d"), chronica_candidates: [candidate] })],
    currentChronicaSha: sha("e"),
  });
  assert.equal(advanced.next_plan.tasks[0].kind, "CHRONICA_CANDIDATE");
  assert.equal(advanced.next_plan.tasks[0].base_sha, sha("e"));
  assert.equal(advanced.next_plan.tasks[0].chronica_candidate.candidate_id, "retry-v1");
});

test("Chronica build result requires independent canonical verify/integrate", () => {
  const candidate = {
    schema: "chronica.system-atlas.chronica-candidate.v1",
    candidate_id: "retry-v1",
    source_cell_id: "trade-logistics",
    source_package_id: "trade-logistics",
    source_repo: "llgtrn/TradeOps",
    source_repo_sha: sha("d"),
    chronica_reference_sha: sha("c"),
    semantic_owner: "runtime/",
    summary: "universal retry semantics",
    donor_ids: ["D123"],
    affected_packages: [],
    evidence: ["ops integration"],
  };
  const task = baseTask({
    id: "chronica-candidate:retry-v1",
    kind: "CHRONICA_CANDIDATE",
    target_repo: "llgtrn/Chronica",
    target_cell_id: null,
    target_package_id: null,
    base_sha: sha("e"),
    donor_repo: null,
    chronica_candidate: candidate,
    owned_scope: ["runtime/"],
  });
  const advanced = advanceFleet({
    plan: plan(task),
    results: [result(task, { target_repo: "llgtrn/Chronica", candidate_sha: sha("f") })],
    currentChronicaSha: sha("1"),
  });
  assert.equal(advanced.next_plan.tasks[0].kind, "CHRONICA_VERIFY_INTEGRATE");
  assert.equal(advanced.next_plan.tasks[0].candidate_sha, sha("f"));
  assert.equal(advanced.next_plan.tasks[0].base_sha, sha("1"));
});

test("canonical integration result fans back out as parallel Ops reconvergence", () => {
  const candidate = {
    schema: "chronica.system-atlas.chronica-candidate.v1",
    candidate_id: "retry-v1",
    source_cell_id: "trade-logistics",
    source_package_id: "trade-logistics",
    source_repo: "llgtrn/TradeOps",
    source_repo_sha: sha("d"),
    chronica_reference_sha: sha("c"),
    semantic_owner: "runtime/",
    summary: "universal retry semantics",
    donor_ids: ["D123"],
    affected_packages: [{ repo: "llgtrn/TradingOps", cell_id: "trading", package_id: "trading" }],
    evidence: ["ops integration"],
  };
  const task = baseTask({
    id: "chronica-verify-integrate:retry-v1",
    kind: "CHRONICA_VERIFY_INTEGRATE",
    target_repo: "llgtrn/Chronica",
    target_cell_id: null,
    target_package_id: null,
    base_sha: sha("1"),
    donor_repo: null,
    candidate_sha: sha("f"),
    chronica_candidate: candidate,
    owned_scope: ["runtime/"],
  });
  const reports = [
    { cell_repo: "llgtrn/TradeOps", cell_sha: sha("4") },
    { cell_repo: "llgtrn/TradingOps", cell_sha: sha("5") },
  ];
  const advanced = advanceFleet({
    plan: plan(task),
    results: [result(task, { target_repo: "llgtrn/Chronica", candidate_sha: sha("2") })],
    reports,
    currentChronicaSha: sha("1"),
  });
  assert.equal(advanced.next_plan.tasks_total, 2);
  assert.equal(advanced.next_plan.ready_wave_total, 2);
  assert.ok(advanced.next_plan.tasks.every((row) => row.kind === "PACKAGE_RECONVERGE"));
  assert.ok(advanced.next_plan.tasks.every((row) => row.chronica_reference_sha === sha("2")));
  assert.deepEqual(new Set(advanced.next_plan.tasks.map((row) => row.target_repo)), new Set(["llgtrn/TradeOps","llgtrn/TradingOps"]));
});
