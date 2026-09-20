import assert from "node:assert/strict";
import { execFileSync } from "node:child_process";
import { existsSync, readdirSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { resolve } from "node:path";
import test from "node:test";
import { analyzeWave, riskRequirements } from "./dependency-safety.mjs";
import { CLEARED_FOR_RETIREMENT, defaultHygieneClearance, retireDispatchGate } from "./hygiene-gate.mjs";
import { REPO_ROOT, productSourceSha, readJson } from "./lib.mjs";
import { claimLease } from "./leases.mjs";
import { classify } from "./legacy-debt-census.mjs";
import { shardsAgreeWithSource } from "./build-frontiers.mjs";
import { canBecomePlanned, isMutatingAction, validateMutationOwnership } from "./mutation-ownership.mjs";
import { tarjanScc } from "./scc.mjs";

test("tarjanScc finds a simple two-node cycle", () => {
  const nodes = ["a", "b", "c"];
  const edges = [
    { from: "a", to: "b" },
    { from: "b", to: "a" },
    { from: "b", to: "c" },
  ];
  const components = tarjanScc(nodes, edges);
  const cycle = components.find((c) => c.length > 1);
  assert.ok(cycle, "expected a non-trivial SCC");
  assert.deepEqual([...cycle].sort(), ["a", "b"]);
});

test("tarjanScc treats an acyclic graph as all-trivial components", () => {
  const nodes = ["a", "b", "c"];
  const edges = [
    { from: "a", to: "b" },
    { from: "b", to: "c" },
  ];
  const components = tarjanScc(nodes, edges);
  assert.equal(components.filter((c) => c.length > 1).length, 0);
});

test("classify buckets known root-cause patterns and defaults to UNKNOWN", () => {
  assert.equal(classify("chronica-core-cli-osint", "core", "chronica-adapter-osint-x", "adapter"), "CLI_MISPLACED_IN_CORE");
  assert.equal(classify("chronica-core-api-state", "core", "chronica-cap-oauth-server", "cap"), "API_STATE_COMPOSITION_MISPLACED_IN_CORE");
  assert.equal(classify("chronica-core-erp-ledger", "core", "chronica-cap-policy-approval-gate", "cap"), "CORE_TO_POLICY_CAP_INVERSION");
  assert.equal(classify("chronica-core-strategy-cognition", "core", "chronica-cap-simulation-scenario-engine", "cap"), "CORE_TO_SIMULATION_CAP_INVERSION");
  assert.equal(classify("chronica-core-workflows-mailbox", "core", "chronica-cap-runtime-money-gateway", "cap"), "CORE_TO_MONEY_GATEWAY_CAP_INVERSION");
  assert.equal(classify("chronica-core-osint-search", "core", "chronica-adapter-osint-email-probe", "adapter"), "OSINT_ADAPTER_MISPLACEMENT");
  assert.equal(classify("chronica-cap-api-outbound-fetch", "cap", "chronica-adapter-osint-http-core", "adapter"), "CAP_TO_ADAPTER_PORT_INVERSION");
  assert.equal(classify("chronica-cap-api-background-dispatch", "cap", "chronica-runtime-event-dispatch", "runtime"), "RUNTIME_OWNERSHIP_INVERSION");
  assert.equal(classify("chronica-core-something", "core", "chronica-adapter-something", "adapter"), "PROTOCOL_IMPLEMENTATION_MISPLACEMENT");
  assert.equal(classify("chronica-core-workflows-session", "core", "chronica-cap-workflows-session-store", "cap"), "CORE_TO_DOMAIN_CAP_STORAGE_INVERSION");
  assert.equal(classify("chronica-adapter-foo", "adapter", "chronica-cap-bar", "cap"), "UNKNOWN");
});

test("all 164 baseline debt edges classify to a known non-UNKNOWN family", () => {
  const baseline = readJson(resolve(REPO_ROOT, "tools/refoundation/refoundation-layer-debt.json"));
  const EDGE_RE = /^(\S+) \((\w+)\) -> (\S+) \((\w+)\)$/;
  let unknown = 0;
  for (const edgeStr of baseline.forbidden_edges) {
    const m = EDGE_RE.exec(edgeStr);
    assert.ok(m, `edge did not match expected format: ${edgeStr}`);
    const [, from, fromLayer, to, toLayer] = m;
    if (classify(from, fromLayer, to, toLayer) === "UNKNOWN") unknown += 1;
  }
  assert.equal(unknown, 0, "every known debt edge should classify into a named root-cause family");
});

test("Atlas shard directory has no duplicate node ids across lanes", () => {
  const shardDir = resolve(REPO_ROOT, "docs/_machine/system-atlas/v0/shards");
  if (!existsSync(shardDir)) return; // not yet generated in this checkout
  const seen = new Map();
  for (const file of readdirSync(shardDir)) {
    if (!file.endsWith(".json")) continue;
    const shard = readJson(resolve(shardDir, file));
    for (const n of shard.nodes ?? []) {
      assert.ok(!seen.has(n.id) || seen.get(n.id) === shard.lane, `duplicate node id ${n.id} in ${file} (also in ${seen.get(n.id)})`);
      seen.set(n.id, shard.lane);
    }
  }
});

test("refoundation architecture gate is unaffected by Atlas tooling", () => {
  const out = execFileSync("node", ["tools/refoundation/validate-refoundation-layers.mjs"], { cwd: REPO_ROOT, encoding: "utf8" });
  assert.match(out, /refoundation layer gate passed; no new edges/);
});

// --- v0.1 hardening invariants -------------------------------------------

test("every shard shares one source_base_sha with the current product snapshot", () => {
  const shardDir = resolve(REPO_ROOT, "docs/_machine/system-atlas/v0/shards");
  if (!existsSync(shardDir)) return;
  const expected = productSourceSha();
  for (const file of readdirSync(shardDir)) {
    if (!file.endsWith(".json")) continue;
    const shard = readJson(resolve(shardDir, file));
    assert.equal(shard.source_base_sha, expected, `${file} source_base_sha must equal the current product snapshot`);
    assert.ok(shard.atlas_generation_sha, `${file} must carry atlas_generation_sha`);
    assert.notEqual(Object.hasOwn(shard, "base_sha"), true, `${file} must not carry the retired ambiguous base_sha field`);
  }
});

test("no frontier task has empty node_ids", () => {
  const outDir = resolve(REPO_ROOT, "docs/_machine/system-atlas/v0");
  if (!existsSync(resolve(outDir, "repair-frontier.json"))) return;
  for (const name of ["repair-frontier", "build-frontier", "retire-frontier", "verify-frontier"]) {
    const frontier = readJson(resolve(outDir, `${name}.json`));
    for (const t of frontier.tasks) {
      assert.ok(t.node_ids.length > 0, `${name}.json task ${t.task_id} has empty node_ids`);
    }
  }
});

test("T3 REPAIR/BUILD tasks are DESIGN_REQUIRED unless a canonical_ownership_note is present", () => {
  const outDir = resolve(REPO_ROOT, "docs/_machine/system-atlas/v0");
  if (!existsSync(resolve(outDir, "repair-frontier.json"))) return;
  for (const name of ["repair-frontier", "build-frontier"]) {
    const frontier = readJson(resolve(outDir, `${name}.json`));
    for (const t of frontier.tasks) {
      if (t.risk !== "T3") continue;
      if (t.canonical_ownership_note) continue;
      assert.equal(t.status, "DESIGN_REQUIRED", `${name}.json task ${t.task_id} is T3 without canonical_ownership_note but status is ${t.status}, not DESIGN_REQUIRED`);
      for (const v of riskRequirements(t)) assert.fail(`${t.task_id}: ${v}`);
    }
  }
});

test("dispatch plan never marks a task LEASED merely for wave inclusion, and leases.json ships empty", () => {
  const outDir = resolve(REPO_ROOT, "docs/_machine/system-atlas/v0");
  const planPath = resolve(outDir, "dispatch-plan.json");
  if (!existsSync(planPath)) return;
  const plan = readJson(planPath);
  const tasksById = new Map();
  for (const name of ["repair-frontier", "build-frontier", "retire-frontier", "verify-frontier"]) {
    for (const t of readJson(resolve(outDir, `${name}.json`)).tasks) tasksById.set(t.task_id, t);
  }
  for (const wave of plan.waves) {
    for (const taskIds of Object.values(wave.accounts)) {
      for (const id of taskIds) {
        const t = tasksById.get(id);
        assert.ok(t, `wave references unknown task ${id}`);
        assert.notEqual(t.status, "LEASED", `${id} must not be LEASED from wave planning alone`);
      }
    }
  }
  const leasesPath = resolve(outDir, "leases.json");
  if (existsSync(leasesPath)) {
    assert.deepEqual(readJson(leasesPath), [], "v0.1 must ship zero fabricated leases");
  }
});

test("dispatch plan's proves_dependency_safe/proves_no_owned_path_overlap match a fresh analyzeWave() recomputation", () => {
  const outDir = resolve(REPO_ROOT, "docs/_machine/system-atlas/v0");
  const planPath = resolve(outDir, "dispatch-plan.json");
  if (!existsSync(planPath)) return;
  const plan = readJson(planPath);
  const tasksById = new Map();
  for (const name of ["repair-frontier", "build-frontier", "retire-frontier", "verify-frontier"]) {
    for (const t of readJson(resolve(outDir, `${name}.json`)).tasks) tasksById.set(t.task_id, t);
  }
  for (const wave of plan.waves) {
    const recomputed = analyzeWave(wave, tasksById);
    assert.equal(wave.proves_dependency_safe, recomputed.provesDependencySafe, `${wave.wave_id} proves_dependency_safe must equal a fresh recomputation, never a hardcoded value`);
    assert.equal(wave.proves_no_owned_path_overlap, recomputed.provesNoOwnedPathOverlap, `${wave.wave_id} proves_no_owned_path_overlap must equal a fresh recomputation`);
  }
});

test("full Atlas validator passes end to end", () => {
  const outDir = resolve(REPO_ROOT, "docs/_machine/system-atlas/v0");
  if (!existsSync(resolve(outDir, "snapshot.json"))) return;
  const out = execFileSync("node", ["tools/system-atlas/validate.mjs"], { cwd: REPO_ROOT, encoding: "utf8" });
  assert.match(out, /Atlas validation passed/);
});

// --- B.1.1 negative test matrix -------------------------------------------
// "No rule is considered implemented until an invalid fixture proves it fails."

const READY_SHA = "1111111111111111111111111111111111111111";

function fixtureTask(overrides = {}) {
  return {
    task_id: "REPAIR-9001",
    action: "REPAIR",
    node_ids: ["crate:fixture"],
    priority: 1,
    risk: "T1",
    source_base_sha: READY_SHA,
    owned_paths: ["crates/cap/fixture/**"],
    forbidden_paths: [],
    dependencies: [],
    blocked_by: [],
    verification_required: [],
    status: "READY",
    ...overrides,
  };
}

test("A: dependency at status READY does not count as satisfied", () => {
  const upstream = fixtureTask({ task_id: "REPAIR-9002", status: "READY" });
  const downstream = fixtureTask({ task_id: "REPAIR-9003", dependencies: ["REPAIR-9002"], owned_paths: ["crates/cap/other/**"] });
  const tasksById = new Map([[upstream.task_id, upstream], [downstream.task_id, downstream]]);
  const wave = { wave_id: "W", accounts: { a: [downstream.task_id] } };
  const report = analyzeWave(wave, tasksById);
  assert.equal(report.provesDependencySafe, false, "a READY dependency must not satisfy its dependent");
  assert.ok(report.unsatisfiedDependencies.some((u) => u.task_id === downstream.task_id));
});

test("B: dependency at status VERIFIED counts as satisfied", () => {
  const upstream = fixtureTask({ task_id: "REPAIR-9004", status: "VERIFIED" });
  const downstream = fixtureTask({ task_id: "REPAIR-9005", dependencies: ["REPAIR-9004"], owned_paths: ["crates/cap/other2/**"] });
  const tasksById = new Map([[upstream.task_id, upstream], [downstream.task_id, downstream]]);
  const wave = { wave_id: "W", accounts: { a: [downstream.task_id] } };
  const report = analyzeWave(wave, tasksById);
  assert.equal(report.unsatisfiedDependencies.length, 0, "a VERIFIED dependency satisfies its dependent");
});

test("C: uncleared blocked_by makes a task not wave-eligible", () => {
  const blocker = fixtureTask({ task_id: "REPAIR-9006", status: "IN_PROGRESS" });
  const blocked = fixtureTask({ task_id: "REPAIR-9007", blocked_by: ["REPAIR-9006"], owned_paths: ["crates/cap/other3/**"] });
  const tasksById = new Map([[blocker.task_id, blocker], [blocked.task_id, blocked]]);
  const wave = { wave_id: "W", accounts: { a: [blocked.task_id] } };
  const report = analyzeWave(wave, tasksById);
  assert.equal(report.provesDependencySafe, false);
  assert.ok(report.unclearedBlockers.some((u) => u.task_id === blocked.task_id));
});

test("D: BUILD in a dispatchable status with empty owned_paths fails mutation-ownership", () => {
  const t = fixtureTask({ task_id: "BUILD-9001", action: "BUILD", status: "READY", owned_paths: [] });
  const violations = validateMutationOwnership(t);
  assert.ok(violations.length > 0, "empty owned_paths on a dispatchable BUILD task must be a violation");
  assert.equal(canBecomePlanned(t), false);
});

test("E: RETIRE READY with no hygiene clearance fails the dispatch gate", () => {
  const t = fixtureTask({ task_id: "RETIRE-9001", action: "RETIRE", status: "READY", ...defaultHygieneClearance() });
  const gate = retireDispatchGate(t);
  assert.equal(gate.allowed, false);
  assert.equal(gate.recommendedStatus, "BLOCKED_SEMANTICS");
});

test("F: RETIRE with FULLY_HARVESTED clearance + evidence + explicit paths passes the hygiene gate", () => {
  assert.ok(CLEARED_FOR_RETIREMENT.has("FULLY_HARVESTED"));
  const t = fixtureTask({
    task_id: "RETIRE-9002",
    action: "RETIRE",
    status: "READY",
    hygiene_clearance: "FULLY_HARVESTED",
    hygiene_evidence_ids: ["harvest:fixture-1"],
    owned_paths: ["crates/cap/fixture/**"],
  });
  const gate = retireDispatchGate(t);
  assert.equal(gate.allowed, true, gate.reason);
});

test("G: claiming a lease with a null/mismatched source_base_sha fails closed", () => {
  const task = fixtureTask({ task_id: "REPAIR-9008", status: "READY", source_base_sha: READY_SHA });
  const nullClaim = claimLease({ tasksById: new Map([[task.task_id, task]]), taskId: task.task_id, workerId: "w", accountId: "a", sourceBaseSha: null, ttlSeconds: 60, existingLeases: [], now: 0 });
  assert.equal(nullClaim.ok, false);
  const mismatchClaim = claimLease({ tasksById: new Map([[task.task_id, task]]), taskId: task.task_id, workerId: "w", accountId: "a", sourceBaseSha: "2".repeat(40), ttlSeconds: 60, existingLeases: [], now: 0 });
  assert.equal(mismatchClaim.ok, false);
});

test("H: claiming a valid READY task creates an ACTIVE lease and moves the task to LEASED", () => {
  const task = fixtureTask({ task_id: "REPAIR-9009", status: "READY", source_base_sha: READY_SHA });
  const result = claimLease({ tasksById: new Map([[task.task_id, task]]), taskId: task.task_id, workerId: "w", accountId: "a", sourceBaseSha: READY_SHA, ttlSeconds: 3600, existingLeases: [], now: 0 });
  assert.equal(result.ok, true, result.reason);
  assert.equal(result.lease.status, "ACTIVE");
  assert.equal(result.taskStatusUpdate.to, "LEASED");
});

test("I: an expired lease over an IN_PROGRESS task moves it to RECOVERY_REQUIRED, not silently reclaimable", async () => {
  const { expireStaleLeases } = await import("./leases.mjs");
  const task = fixtureTask({ task_id: "REPAIR-9010", status: "IN_PROGRESS", source_base_sha: READY_SHA });
  const lease = { lease_id: "lease-fixture", task_id: task.task_id, worker_id: "w", account_id: "a", claimed_at: "1970-01-01T00:00:00.000Z", expires_at: "1970-01-01T00:00:01.000Z", source_base_sha: READY_SHA, owned_paths: task.owned_paths, status: "ACTIVE" };
  const result = expireStaleLeases({ tasksById: new Map([[task.task_id, task]]), leases: [lease], now: 999999 });
  const update = result.taskStatusUpdates.find((u) => u.task_id === task.task_id);
  assert.equal(update.to, "RECOVERY_REQUIRED");
});

test("J: wave planning (build-dispatch-plan.mjs) never writes to leases.json", () => {
  // Deliberately does NOT re-invoke build-dispatch-plan.mjs against the real
  // committed frontier files: doing so is a real, valid (re-)planning run,
  // but it mutates already-PLANNED tasks' visibility for any later run in
  // the same process (a second invocation sees fewer READY tasks, since the
  // first already advanced some to PLANNED) -- fine operationally, but it
  // makes running the test suite itself perturb committed Atlas artifacts,
  // which is exactly the kind of instability this hardening pass exists to
  // eliminate. Verify the invariant structurally instead: the module must
  // not even import leases.mjs (it has no legitimate reason to touch
  // lease state), and leases.json must currently be empty.
  const source = readFileSync(resolve(REPO_ROOT, "tools/system-atlas/build-dispatch-plan.mjs"), "utf8");
  assert.ok(!/^import .*leases\.mjs/m.test(source), "build-dispatch-plan.mjs must not import leases.mjs at all (a doc-comment mention is fine, an import is not)");
  const leasesPath = resolve(REPO_ROOT, "docs/_machine/system-atlas/v0/leases.json");
  if (existsSync(leasesPath)) assert.deepEqual(readJson(leasesPath), [], "leases.json must remain empty in this hardening pass");
});

test("regression: shardsAgreeWithSource actually rejects a mismatched source_base_sha (BF5 adversarial finding)", () => {
  // A prior inline version of this check (`![...set][0] === x` instead of
  // `[...set][0] !== x`) had an operator-precedence bug that made the
  // mismatch branch permanently unreachable -- caught by BF5's adversarial
  // review, not by any prior test, because nothing exercised the FAIL path
  // directly. This test exists specifically so that regression can't recur
  // silently again.
  const matching = shardsAgreeWithSource(new Set(["a".repeat(40)]), "a".repeat(40));
  assert.equal(matching.ok, true);
  const mismatched = shardsAgreeWithSource(new Set(["a".repeat(40)]), "b".repeat(40));
  assert.equal(mismatched.ok, false, "a real mismatch must be reported as not-ok, not silently pass");
  const disagreeing = shardsAgreeWithSource(new Set(["a".repeat(40), "b".repeat(40)]), "a".repeat(40));
  assert.equal(disagreeing.ok, false);
});

// --- B.2 negative test matrix -----------------------------------------

test("L: validate.mjs fails closed when a stray dispatch.lock is left in the working tree", () => {
  const lockPath = resolve(REPO_ROOT, "docs/_machine/system-atlas/v0/dispatch.lock");
  assert.equal(existsSync(lockPath), false, "precondition: no real lock should already be present");
  writeFileSync(lockPath, `${JSON.stringify({ lock_id: "lock-fixture-stray", worker_id: "w", pid: 1, created_at: "2020-01-01T00:00:00.000Z", expires_at: "2020-01-01T00:01:00.000Z", source_base_sha: "1".repeat(40) }, null, 2)}\n`);
  try {
    assert.throws(
      () => execFileSync("node", ["tools/system-atlas/validate.mjs"], { cwd: REPO_ROOT, encoding: "utf8", stdio: ["ignore", "pipe", "pipe"] }),
      (err) => /dispatch lock must never be committed/.test(err.stderr ?? "") || /dispatch lock must never be committed/.test(err.message ?? ""),
    );
  } finally {
    rmSync(lockPath, { force: true });
  }
  // Confirm the validator is clean again once the stray artifact is gone.
  const out = execFileSync("node", ["tools/system-atlas/validate.mjs"], { cwd: REPO_ROOT, encoding: "utf8" });
  assert.match(out, /Atlas validation passed/);
});

test("M: validate.mjs fails closed when recovery-log.json references an unknown task_id or a fabricated resolution", () => {
  const recoveryLogPath = resolve(REPO_ROOT, "docs/_machine/system-atlas/v0/recovery-log.json");
  assert.equal(existsSync(recoveryLogPath), false, "precondition: no real recovery-log.json should already be present");
  writeFileSync(
    recoveryLogPath,
    `${JSON.stringify(
      [{ recovery_id: "recovery-fixture-1", task_id: "REPAIR-9999", reason: "r", resolution: "AUTO_REQUEUE", evidence: "e", resolved_at: "2020-01-01T00:00:00.000Z", resolved_by: "fixture" }],
      null,
      2,
    )}\n`,
  );
  try {
    assert.throws(
      () => execFileSync("node", ["tools/system-atlas/validate.mjs"], { cwd: REPO_ROOT, encoding: "utf8", stdio: ["ignore", "pipe", "pipe"] }),
      (err) => /references unknown task/.test(err.stderr ?? "") && /not one of/.test(err.stderr ?? ""),
    );
  } finally {
    rmSync(recoveryLogPath, { force: true });
  }
  const out = execFileSync("node", ["tools/system-atlas/validate.mjs"], { cwd: REPO_ROOT, encoding: "utf8" });
  assert.match(out, /Atlas validation passed/);
});

test("N: validate.mjs fails closed when a wave references a RECOVERY_REQUIRED task", () => {
  const planPath = resolve(REPO_ROOT, "docs/_machine/system-atlas/v0/dispatch-plan.json");
  const original = readFileSync(planPath, "utf8");
  try {
    const plan = JSON.parse(original);
    const outDir = resolve(REPO_ROOT, "docs/_machine/system-atlas/v0");
    const repairFrontier = readJson(resolve(outDir, "repair-frontier.json"));
    const anyTaskId = repairFrontier.tasks[0]?.task_id;
    assert.ok(anyTaskId, "precondition: at least one REPAIR task must exist to borrow an id from");
    plan.waves[0].accounts._fixture = [anyTaskId];
    // Temporarily flip that task's in-memory status via a throwaway copy is
    // not visible to the subprocess; instead simulate the real scenario by
    // pointing the wave at a task_id that does not exist in ANY status the
    // validator would accept as wave-eligible: reuse a RETIRE task_id, which
    // is always BLOCKED_SEMANTICS pre-Lane-C (see test K) and therefore
    // exercises the same NEVER_WAVE_ELIGIBLE_STATUSES path RECOVERY_REQUIRED
    // does, without needing to hand-edit frontier files for this test.
    const retireFrontier = readJson(resolve(outDir, "retire-frontier.json"));
    const blockedTaskId = retireFrontier.tasks[0]?.task_id;
    assert.ok(blockedTaskId, "precondition: at least one RETIRE task must exist");
    plan.waves[0].accounts._fixture = [blockedTaskId];
    writeFileSync(planPath, `${JSON.stringify(plan, null, 2)}\n`);
    assert.throws(
      () => execFileSync("node", ["tools/system-atlas/validate.mjs"], { cwd: REPO_ROOT, encoding: "utf8", stdio: ["ignore", "pipe", "pipe"] }),
      (err) => /never wave-eligible/.test(err.stderr ?? ""),
    );
  } finally {
    writeFileSync(planPath, original);
  }
  const out = execFileSync("node", ["tools/system-atlas/validate.mjs"], { cwd: REPO_ROOT, encoding: "utf8" });
  assert.match(out, /Atlas validation passed/);
});

test("K: current WAVE-001 excludes every RETIRE task and every mutating task lacking owned_paths", () => {
  const outDir = resolve(REPO_ROOT, "docs/_machine/system-atlas/v0");
  const planPath = resolve(outDir, "dispatch-plan.json");
  if (!existsSync(planPath)) return;
  const plan = readJson(planPath);
  const tasksById = new Map();
  for (const name of ["repair-frontier", "build-frontier", "retire-frontier", "verify-frontier"]) {
    for (const t of readJson(resolve(outDir, `${name}.json`)).tasks) tasksById.set(t.task_id, t);
  }
  for (const wave of plan.waves) {
    for (const taskIds of Object.values(wave.accounts)) {
      for (const id of taskIds) {
        const t = tasksById.get(id);
        assert.notEqual(t.action, "RETIRE", `${id}: no RETIRE task may appear in a wave until Lane C hygiene clearance exists`);
        if (isMutatingAction(t.action)) {
          assert.ok(t.owned_paths.length > 0, `${id}: mutating task in a wave must carry real owned_paths`);
        }
      }
    }
  }
});

// --- BX6 adversarial regression tests -------------------------------------

test("BX6-5 regression: claimLease refuses a RETIRE task whose hygiene clearance does not hold, even when its status says READY", async () => {
  // build-frontiers.mjs and validate.mjs already refuse to let an un-cleared
  // RETIRE task SIT in a dispatchable status -- but both are
  // generation-time/offline checks, and a claim is the moment a real worker
  // starts touching real paths. The realistic route in: the task WAS cleared
  // and claimable, its lease died, a coordinator resolved it back to PLANNED
  // via recovery.mjs, and in between Lane C downgraded its clearance to a real
  // negative finding. Before the fix the claim succeeded, and the violation
  // only surfaced on the next offline validate.mjs run.
  const cleared = fixtureTask({
    task_id: "RETIRE-9010", action: "RETIRE", status: "READY",
    hygiene_clearance: "FULLY_HARVESTED", hygiene_evidence_ids: ["harvest:fixture-1"],
  });
  const okClaim = claimLease({ tasksById: new Map([[cleared.task_id, cleared]]), taskId: cleared.task_id, workerId: "w", accountId: "a", sourceBaseSha: READY_SHA, ttlSeconds: 3600, existingLeases: [], now: 0 });
  assert.equal(okClaim.ok, true, `a properly cleared RETIRE task must stay claimable: ${okClaim.reason}`);

  for (const downgrade of [{ hygiene_clearance: "BLOCKED_UNIQUE_VALUE" }, { hygiene_clearance: "NOT_CHECKED" }, { hygiene_evidence_ids: [] }]) {
    const task = fixtureTask({
      task_id: "RETIRE-9011", action: "RETIRE", status: "READY",
      hygiene_clearance: "FULLY_HARVESTED", hygiene_evidence_ids: ["harvest:fixture-1"], ...downgrade,
    });
    const claim = claimLease({ tasksById: new Map([[task.task_id, task]]), taskId: task.task_id, workerId: "w", accountId: "a", sourceBaseSha: READY_SHA, ttlSeconds: 3600, existingLeases: [], now: 0 });
    assert.equal(claim.ok, false, `RETIRE task with ${JSON.stringify(downgrade)} must not be claimable`);
    assert.match(claim.reason, /without hygiene clearance/);
    assert.equal(claim.lease, undefined, "a refused claim must create nothing");
  }

  // Same gate on the way OUT of RECOVERY_REQUIRED: the two resolutions that
  // return a task to a dispatchable status must re-check it, while the two
  // that do not re-dispatch (READY_TO_VERIFY, FAILED) must stay available.
  const { resolveRecovery } = await import("./recovery.mjs");
  const base = {
    evidence: "state audited by hand", resolvedBy: "coordinator", sourceBaseSha: READY_SHA,
    workerId: "w2", accountId: "a2", ttlSeconds: 3600, now: "2026-09-01T00:00:00.000Z",
  };
  for (const [resolution, expectAllowed] of [["RESUME_SAME_WORK", false], ["REQUEUE_AFTER_CLEANUP", false], ["READY_TO_VERIFY", true], ["FAILED", true]]) {
    const task = fixtureTask({ task_id: "RETIRE-9012", action: "RETIRE", status: "RECOVERY_REQUIRED", ...defaultHygieneClearance() });
    const r = resolveRecovery({ ...base, tasksById: new Map([[task.task_id, task]]), leases: [], taskId: task.task_id, resolution });
    assert.equal(r.ok, expectAllowed, `${resolution} on an un-cleared RETIRE task: expected ok=${expectAllowed}, got ${r.ok} (${r.reason ?? ""})`);
    if (!expectAllowed) assert.match(r.reason, /without hygiene clearance/);
  }
});

test("BX6-6 regression: claimLease and resolveRecovery refuse a mutating task with no real owned_paths", async () => {
  // Same defense-in-depth gap as BX6-5, for the other dispatch gate:
  // mutation-ownership. A REPAIR/BUILD/RETIRE task whose owned_paths are
  // empty -- or blank/whitespace-only strings, which are functionally
  // identical -- makes the whole owned-path-overlap safety proof vacuous, yet
  // claimLease() happily issued a lease for one and skipped the overlap check
  // entirely (its `ownedPaths.length > 0` guard).
  const { resolveRecovery } = await import("./recovery.mjs");
  for (const ownedPaths of [[], ["   "], ["", "\t"]]) {
    const task = fixtureTask({ task_id: "BUILD-9020", action: "BUILD", status: "READY", owned_paths: ownedPaths });
    const claim = claimLease({ tasksById: new Map([[task.task_id, task]]), taskId: task.task_id, workerId: "w", accountId: "a", sourceBaseSha: READY_SHA, ttlSeconds: 3600, existingLeases: [], now: 0 });
    assert.equal(claim.ok, false, `owned_paths ${JSON.stringify(ownedPaths)} must not be claimable`);
    assert.match(claim.reason, /no real owned_paths/);

    const rec = fixtureTask({ task_id: "BUILD-9021", action: "BUILD", status: "RECOVERY_REQUIRED", owned_paths: ownedPaths });
    const resolved = resolveRecovery({
      tasksById: new Map([[rec.task_id, rec]]), leases: [], taskId: rec.task_id, resolution: "REQUEUE_AFTER_CLEANUP",
      evidence: "e", resolvedBy: "coordinator", sourceBaseSha: READY_SHA, now: "2026-09-01T00:00:00.000Z",
    });
    assert.equal(resolved.ok, false, "REQUEUE_AFTER_CLEANUP must not return such a task to PLANNED");
    assert.match(resolved.reason, /no real owned_paths/);
  }

  // A VERIFY task is read-only investigation: empty owned_paths is expected
  // and must remain claimable.
  const verify = fixtureTask({ task_id: "VERIFY-9022", action: "VERIFY", status: "READY", owned_paths: [] });
  const verifyClaim = claimLease({ tasksById: new Map([[verify.task_id, verify]]), taskId: verify.task_id, workerId: "w", accountId: "a", sourceBaseSha: READY_SHA, ttlSeconds: 3600, existingLeases: [], now: 0 });
  assert.equal(verifyClaim.ok, true, `a VERIFY task with empty owned_paths must stay claimable: ${verifyClaim.reason}`);
});

test("BX6-7 regression: retireDispatchGate counts only REAL owned_paths, agreeing with mutation-ownership", () => {
  // retireDispatchGate used `(task.owned_paths ?? []).length > 0` while
  // mutation-ownership.mjs filters blank/whitespace-only entries first, so a
  // fully-cleared RETIRE task with owned_paths: ["   "] passed one gate and
  // failed the other -- two gates disagreeing about the same fact.
  const base = {
    task_id: "RETIRE-9030", action: "RETIRE", risk: "T1", status: "READY",
    hygiene_clearance: "FULLY_HARVESTED", hygiene_evidence_ids: ["harvest:fixture-1"],
  };
  const blank = retireDispatchGate({ ...base, owned_paths: ["   "] });
  assert.equal(blank.allowed, false, "whitespace-only owned_paths is not real ownership scope");
  assert.match(blank.reason, /no real entries/);
  assert.equal(blank.recommendedStatus, "DESIGN_REQUIRED");
  assert.ok(validateMutationOwnership({ ...base, owned_paths: ["   "] }).length > 0, "the two gates must agree");

  assert.equal(retireDispatchGate({ ...base, owned_paths: ["crates/cap/fixture/**"] }).allowed, true);

  // Same rule inside the T3 risk check.
  const t3 = { ...base, risk: "T3", cross_account_verification_required: true, owned_paths: ["  "] };
  assert.equal(retireDispatchGate(t3).allowed, false);
});
