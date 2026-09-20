#!/usr/bin/env node
// Recovery path out of RECOVERY_REQUIRED for System Atlas tasks.
//
// expireStaleLeases() in leases.mjs already introduces RECOVERY_REQUIRED: an
// ACTIVE lease that expires while its task is IN_PROGRESS moves the task to
// RECOVERY_REQUIRED instead of silently reclaiming it, because a worker may
// have partially mutated its owned paths and a second worker stepping on the
// same files unsupervised is unsafe. Until this module, RECOVERY_REQUIRED was
// a dead end: this file is the human/coordinator-mediated way out.
//
// Every function below is PURE: in-memory data in (tasksById, leases, a
// `now`/`resolvedAt` timestamp), a result describing what changed out. No
// file I/O except loadRecoveryLog/saveRecoveryLog (trivial JSON helpers) and
// the self-test's own use of them against temp paths. There is no CLI here --
// leases.mjs is expected to grow `recovery-status` / `resolve-recovery`
// commands that call into recoveryStatus()/resolveRecovery() and do the
// actual persistence (append to leases.json / a frontier file / the recovery
// log), the same way its own CLI persists claimLease()/releaseLease() etc.
import { existsSync, mkdirSync, mkdtempSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { retireDispatchGate } from "./hygiene-gate.mjs";
import { readJson } from "./lib.mjs";
import { canBecomePlanned } from "./mutation-ownership.mjs";

// Deliberately narrow -- per the hardening brief, allowed resolutions must be
// explicit and narrow, not an open string.
export const RECOVERY_RESOLUTIONS = ["RESUME_SAME_WORK", "REQUEUE_AFTER_CLEANUP", "READY_TO_VERIFY", "FAILED"];

function isNonBlank(s) {
  return typeof s === "string" && s.trim().length > 0;
}

function getTask(tasksById, taskId) {
  return tasksById instanceof Map ? tasksById.get(taskId) : tasksById[taskId];
}

function isUnexpiredActive(lease, now) {
  return lease.status === "ACTIVE" && lease.expires_at > now;
}

// The most recent lease on record for a task that could have driven it into
// RECOVERY_REQUIRED. Two different paths land a task in RECOVERY_REQUIRED
// (see leases.mjs): expireStaleLeases() flips the lease to EXPIRED, and a
// voluntary releaseLease() call on an IN_PROGRESS task flips it to RELEASED
// instead (a worker calling release does not prove its owned paths are
// clean, so this is treated exactly as cautiously as an expiry) -- either
// status is "the lease that was active when this task last entered
// RECOVERY_REQUIRED". A task can accumulate several such terminal leases
// across repeated claim/recover cycles; since only one lease may be ACTIVE
// for a task at a time (see claimLease's overlap check), the lease with the
// latest claimed_at is unambiguously the most recent one -- unlike
// expires_at, claimed_at is set once at claim time and never means
// something different between the EXPIRED and RELEASED cases.
function findLastRelevantLeaseForTask(leases, taskId) {
  let best = null;
  for (const l of leases) {
    if (l.task_id !== taskId || (l.status !== "EXPIRED" && l.status !== "RELEASED")) continue;
    if (!best || l.claimed_at > best.claimed_at) best = l;
  }
  return best;
}

function randomSuffix() {
  return Math.random().toString(36).slice(2, 8);
}

// ---------------------------------------------------------------------------
// recoveryStatus -- pure, read-only. Lists every RECOVERY_REQUIRED task along
// with its last relevant (EXPIRED) lease, for a human/coordinator to triage
// via a `leases.mjs recovery-status` CLI command.
// ---------------------------------------------------------------------------
export function recoveryStatus({ tasksById, leases }) {
  const tasks = tasksById instanceof Map ? [...tasksById.values()] : Object.values(tasksById);
  const out = [];
  for (const task of tasks) {
    if (task.status !== "RECOVERY_REQUIRED") continue;
    const lastLease = findLastRelevantLeaseForTask(leases, task.task_id);
    out.push({
      task_id: task.task_id,
      task_status: task.status,
      last_lease_id: lastLease?.lease_id ?? null,
      last_worker_id: lastLease?.worker_id ?? null,
      last_account_id: lastLease?.account_id ?? null,
      lease_expired_at: lastLease?.expires_at ?? null,
    });
  }
  return out;
}

// ---------------------------------------------------------------------------
// resolveRecovery -- pure. FAIL CLOSED: on any violation, returns
// {ok:false, reason} and mutates/creates nothing.
// ---------------------------------------------------------------------------
export function resolveRecovery({
  tasksById,
  leases,
  taskId,
  resolution,
  evidence,
  resolvedBy,
  sourceBaseSha,
  workerId,
  accountId,
  ttlSeconds,
  now,
}) {
  const task = getTask(tasksById, taskId);
  if (!task) {
    return { ok: false, reason: `task ${taskId} not found` };
  }
  if (task.status !== "RECOVERY_REQUIRED") {
    return { ok: false, reason: `task ${taskId} has status ${task.status}, expected RECOVERY_REQUIRED` };
  }
  if (!RECOVERY_RESOLUTIONS.includes(resolution)) {
    return { ok: false, reason: `resolution ${resolution} is not one of ${RECOVERY_RESOLUTIONS.join(", ")}` };
  }
  // "do not auto-requeue": every resolution, including terminal ones,
  // requires a real non-blank coordinator note. Never a silent default.
  if (!isNonBlank(evidence)) {
    return { ok: false, reason: "evidence must be a non-blank string" };
  }
  if (!isNonBlank(resolvedBy)) {
    return { ok: false, reason: "resolvedBy must be a non-blank string" };
  }
  if (sourceBaseSha !== task.source_base_sha) {
    return {
      ok: false,
      reason: `sourceBaseSha ${sourceBaseSha} does not match task ${taskId}'s recorded source_base_sha ${task.source_base_sha} (task derived from a different Atlas generation)`,
    };
  }
  // There should not be an ACTIVE-and-unexpired lease for a RECOVERY_REQUIRED
  // task, but check defensively -- fail closed if there somehow is.
  const activeLease = leases.find((l) => l.task_id === taskId && isUnexpiredActive(l, now));
  if (activeLease) {
    return { ok: false, reason: `task ${taskId} already has an active lease (${activeLease.lease_id}, held by ${activeLease.worker_id}, expires ${activeLease.expires_at})` };
  }

  // Two of the four resolutions put the task back into a DISPATCHABLE state
  // (RESUME_SAME_WORK -> IN_PROGRESS, REQUEUE_AFTER_CLEANUP -> PLANNED), so
  // both must re-check the same dispatch gates a fresh claim does -- the task
  // may have been claimable when its first lease was taken and NOT claimable
  // now (BX6 adversarial finding: Lane C re-runs and downgrades a RETIRE
  // task's hygiene_clearance to BLOCKED_UNIQUE_VALUE while that task sits in
  // RECOVERY_REQUIRED; without this, REQUEUE_AFTER_CLEANUP would hand it
  // straight back out for retirement work Lane C had just forbidden).
  // READY_TO_VERIFY and FAILED are deliberately NOT gated: neither
  // re-dispatches the task, and a coordinator must always be able to record
  // "this finished" or "this is dead" for a task that can no longer run.
  const REDISPATCHING_RESOLUTIONS = new Set(["RESUME_SAME_WORK", "REQUEUE_AFTER_CLEANUP"]);
  if (REDISPATCHING_RESOLUTIONS.has(resolution)) {
    if (!canBecomePlanned(task)) {
      return { ok: false, reason: `task ${taskId} is a mutating ${task.action} task with no real owned_paths -- ${resolution} would return it to a dispatchable status it must not hold (use READY_TO_VERIFY or FAILED, or restore real owned_paths first)` };
    }
    if (task.action === "RETIRE") {
      const gate = retireDispatchGate(task);
      if (!gate.allowed) {
        return { ok: false, reason: `task ${taskId} is a RETIRE task without hygiene clearance -- ${resolution} would return it to a dispatchable status: ${gate.reason}` };
      }
    }
  }

  const lastLease = findLastRelevantLeaseForTask(leases, taskId);
  const recoveryRecord = {
    recovery_id: `recovery-${taskId}-${randomSuffix()}`,
    task_id: taskId,
    previous_worker: lastLease?.worker_id ?? null,
    previous_lease_id: lastLease?.lease_id ?? null,
    reason:
      lastLease?.status === "RELEASED"
        ? "lease voluntarily released while task was IN_PROGRESS"
        : "lease expired while task was IN_PROGRESS",
    resolution,
    evidence,
    resolved_at: now,
    resolved_by: resolvedBy,
  };

  if (resolution === "RESUME_SAME_WORK") {
    if (!isNonBlank(workerId)) {
      return { ok: false, reason: "workerId must be a non-blank string" };
    }
    if (!isNonBlank(accountId)) {
      return { ok: false, reason: "accountId must be a non-blank string" };
    }
    if (!(typeof ttlSeconds === "number" && Number.isFinite(ttlSeconds) && ttlSeconds > 0)) {
      return { ok: false, reason: "ttlSeconds must be a positive finite number" };
    }
    const expiresAt = new Date(new Date(now).getTime() + ttlSeconds * 1000).toISOString();
    const newLease = {
      lease_id: `lease-${taskId}-${randomSuffix()}`,
      task_id: taskId,
      worker_id: workerId,
      account_id: accountId,
      claimed_at: now,
      expires_at: expiresAt,
      source_base_sha: sourceBaseSha,
      owned_paths: task.owned_paths ?? [],
      status: "ACTIVE",
    };
    return {
      ok: true,
      taskStatusUpdate: { task_id: taskId, from: "RECOVERY_REQUIRED", to: "IN_PROGRESS" },
      newLease,
      recoveryRecord,
    };
  }

  if (resolution === "REQUEUE_AFTER_CLEANUP") {
    return {
      ok: true,
      taskStatusUpdate: { task_id: taskId, from: "RECOVERY_REQUIRED", to: "PLANNED" },
      newLease: null,
      recoveryRecord,
    };
  }

  if (resolution === "READY_TO_VERIFY") {
    return {
      ok: true,
      taskStatusUpdate: { task_id: taskId, from: "RECOVERY_REQUIRED", to: "READY_TO_VERIFY" },
      newLease: null,
      recoveryRecord,
    };
  }

  // resolution === "FAILED"
  return {
    ok: true,
    taskStatusUpdate: { task_id: taskId, from: "RECOVERY_REQUIRED", to: "FAILED" },
    newLease: null,
    recoveryRecord,
  };
}

// ---------------------------------------------------------------------------
// Basic load/save for recovery-log.json (array of recoveryRecord entries),
// matching the loadLeases/saveLeases pattern in leases.mjs.
// ---------------------------------------------------------------------------
export function loadRecoveryLog(path) {
  if (!existsSync(path)) return [];
  return readJson(path);
}

export function saveRecoveryLog(path, records) {
  mkdirSync(dirname(path), { recursive: true });
  writeFileSync(path, `${JSON.stringify(records, null, 2)}\n`);
}

// ===========================================================================
// Self-test -- exercises the pure functions above against small in-memory
// fixtures and temp paths only. Never touches the real recovery-log.json or
// any real frontier/leases file. Run with:
//   node tools/system-atlas/recovery.mjs
// ===========================================================================
function runSelfTest() {
  let pass = 0;
  let fail = 0;
  const check = (name, cond) => {
    if (cond) {
      pass++;
      console.log(`PASS: ${name}`);
    } else {
      fail++;
      console.log(`FAIL: ${name}`);
    }
  };

  const SHA_A = "a".repeat(40);
  const SHA_B = "b".repeat(40);
  const NOW = "2026-09-01T00:00:00.000Z";
  const PAST = "2026-08-31T00:00:00.000Z";
  const EXPIRED_AT = "2026-08-31T01:00:00.000Z";
  const FUTURE = "2026-09-02T00:00:00.000Z";

  function makeRecoveryTask(overrides) {
    return {
      task_id: "REPAIR-0001",
      status: "RECOVERY_REQUIRED",
      source_base_sha: SHA_A,
      owned_paths: ["crates/core/foo/**"],
      ...overrides,
    };
  }

  function makeExpiredLease(taskId, overrides) {
    return {
      lease_id: `lease-${taskId}-x1`,
      task_id: taskId,
      worker_id: "w1",
      account_id: "acc1",
      claimed_at: PAST,
      expires_at: EXPIRED_AT,
      source_base_sha: SHA_A,
      owned_paths: ["crates/core/foo/**"],
      status: "EXPIRED",
      ...overrides,
    };
  }

  // (a) each of the 4 resolutions succeeds on a valid fixture, right taskStatusUpdate.to
  {
    const expected = {
      RESUME_SAME_WORK: "IN_PROGRESS",
      REQUEUE_AFTER_CLEANUP: "PLANNED",
      READY_TO_VERIFY: "READY_TO_VERIFY",
      FAILED: "FAILED",
    };
    let allOk = true;
    for (const resolution of RECOVERY_RESOLUTIONS) {
      const task = makeRecoveryTask({ task_id: `T-${resolution}` });
      const tasksById = new Map([[task.task_id, task]]);
      const leases = [makeExpiredLease(task.task_id)];
      const r = resolveRecovery({
        tasksById,
        leases,
        taskId: task.task_id,
        resolution,
        evidence: "cleaned up partial state; verified no dangling writes",
        resolvedBy: "coordinator",
        sourceBaseSha: SHA_A,
        workerId: "w2",
        accountId: "acc2",
        ttlSeconds: 3600,
        now: NOW,
      });
      const okShape =
        r.ok &&
        r.taskStatusUpdate.from === "RECOVERY_REQUIRED" &&
        r.taskStatusUpdate.to === expected[resolution] &&
        r.recoveryRecord.resolution === resolution &&
        r.recoveryRecord.previous_lease_id === leases[0].lease_id &&
        r.recoveryRecord.previous_worker === "w1" &&
        (resolution === "RESUME_SAME_WORK" ? r.newLease?.status === "ACTIVE" && r.newLease.worker_id === "w2" : r.newLease === null);
      allOk = allOk && okShape;
      if (!okShape) console.log(`  detail: ${resolution} ->`, JSON.stringify(r));
    }
    check("(a) all 4 resolutions succeed with correct taskStatusUpdate.to", allOk);
  }

  // (b) fails when task.status !== RECOVERY_REQUIRED
  {
    const task = makeRecoveryTask({ status: "READY" });
    const tasksById = new Map([[task.task_id, task]]);
    const r = resolveRecovery({
      tasksById, leases: [], taskId: task.task_id, resolution: "FAILED",
      evidence: "e", resolvedBy: "coord", sourceBaseSha: SHA_A, now: NOW,
    });
    check("(b) fails when task.status is not RECOVERY_REQUIRED", !r.ok);
  }

  // (c) fails when evidence is blank/whitespace-only
  {
    const task = makeRecoveryTask();
    const tasksById = new Map([[task.task_id, task]]);
    const r1 = resolveRecovery({
      tasksById, leases: [], taskId: task.task_id, resolution: "FAILED",
      evidence: "   ", resolvedBy: "coord", sourceBaseSha: SHA_A, now: NOW,
    });
    const r2 = resolveRecovery({
      tasksById, leases: [], taskId: task.task_id, resolution: "FAILED",
      evidence: undefined, resolvedBy: "coord", sourceBaseSha: SHA_A, now: NOW,
    });
    check("(c) fails when evidence is blank/missing", !r1.ok && !r2.ok);
  }

  // (d) fails when resolvedBy is blank
  {
    const task = makeRecoveryTask();
    const tasksById = new Map([[task.task_id, task]]);
    const r = resolveRecovery({
      tasksById, leases: [], taskId: task.task_id, resolution: "FAILED",
      evidence: "e", resolvedBy: "  ", sourceBaseSha: SHA_A, now: NOW,
    });
    check("(d) fails when resolvedBy is blank", !r.ok);
  }

  // (e) fails when resolution is not one of the 4 allowed values
  {
    const task = makeRecoveryTask();
    const tasksById = new Map([[task.task_id, task]]);
    const r1 = resolveRecovery({
      tasksById, leases: [], taskId: task.task_id, resolution: "REQUEUE",
      evidence: "e", resolvedBy: "coord", sourceBaseSha: SHA_A, now: NOW,
    });
    const r2 = resolveRecovery({
      tasksById, leases: [], taskId: task.task_id, resolution: "RESUME",
      evidence: "e", resolvedBy: "coord", sourceBaseSha: SHA_A, now: NOW,
    });
    check("(e) fails when resolution is not one of the 4 allowed values", !r1.ok && !r2.ok);
  }

  // (f) fails when sourceBaseSha doesn't match the task's own
  {
    const task = makeRecoveryTask({ source_base_sha: SHA_A });
    const tasksById = new Map([[task.task_id, task]]);
    const r = resolveRecovery({
      tasksById, leases: [], taskId: task.task_id, resolution: "FAILED",
      evidence: "e", resolvedBy: "coord", sourceBaseSha: SHA_B, now: NOW,
    });
    check("(f) fails when sourceBaseSha mismatches task.source_base_sha", !r.ok);
  }

  // (g) RESUME_SAME_WORK additionally fails on blank workerId/accountId or non-positive ttlSeconds
  {
    const base = { evidence: "e", resolvedBy: "coord", sourceBaseSha: SHA_A, resolution: "RESUME_SAME_WORK", now: NOW };
    const t1 = makeRecoveryTask({ task_id: "T-g1" });
    const t2 = makeRecoveryTask({ task_id: "T-g2" });
    const t3 = makeRecoveryTask({ task_id: "T-g3" });
    const tasksById = new Map([[t1.task_id, t1], [t2.task_id, t2], [t3.task_id, t3]]);
    const rWorker = resolveRecovery({ ...base, tasksById, leases: [], taskId: t1.task_id, workerId: "  ", accountId: "acc2", ttlSeconds: 3600 });
    const rAccount = resolveRecovery({ ...base, tasksById, leases: [], taskId: t2.task_id, workerId: "w2", accountId: "", ttlSeconds: 3600 });
    const rTtl = resolveRecovery({ ...base, tasksById, leases: [], taskId: t3.task_id, workerId: "w2", accountId: "acc2", ttlSeconds: 0 });
    check("(g) RESUME_SAME_WORK fails on blank workerId/accountId or bad ttlSeconds", !rWorker.ok && !rAccount.ok && !rTtl.ok);
  }

  // (h) RESUME_SAME_WORK fails if an ACTIVE unexpired lease already exists for the task
  {
    const task = makeRecoveryTask();
    const tasksById = new Map([[task.task_id, task]]);
    const activeLease = {
      lease_id: "lease-stray", task_id: task.task_id, worker_id: "w9", account_id: "acc9",
      claimed_at: NOW, expires_at: FUTURE, source_base_sha: SHA_A, owned_paths: task.owned_paths, status: "ACTIVE",
    };
    const r = resolveRecovery({
      tasksById, leases: [activeLease], taskId: task.task_id, resolution: "RESUME_SAME_WORK",
      evidence: "e", resolvedBy: "coord", sourceBaseSha: SHA_A, workerId: "w2", accountId: "acc2", ttlSeconds: 3600, now: NOW,
    });
    check("(h) RESUME_SAME_WORK fails if an ACTIVE unexpired lease already exists", !r.ok);
  }

  // (i) recoveryStatus lists RECOVERY_REQUIRED tasks, excludes others, picks
  // the lease with the latest claimed_at (not the latest expires_at -- those
  // two orderings can disagree, e.g. a short-TTL later claim vs a long-TTL
  // earlier one, and claimed_at is the one that is unambiguous across both
  // the EXPIRED and RELEASED cases).
  {
    const rr = makeRecoveryTask({ task_id: "REPAIR-0001" });
    const ready = { task_id: "REPAIR-0002", status: "READY", source_base_sha: SHA_A, owned_paths: [] };
    const inProgress = { task_id: "REPAIR-0003", status: "IN_PROGRESS", source_base_sha: SHA_A, owned_paths: [] };
    const tasksById = new Map([[rr.task_id, rr], [ready.task_id, ready], [inProgress.task_id, inProgress]]);
    const olderExpired = makeExpiredLease(rr.task_id, {
      lease_id: "lease-old", claimed_at: "2026-08-29T00:00:00.000Z", expires_at: "2026-08-31T12:00:00.000Z", worker_id: "w-old",
    });
    const newerExpired = makeExpiredLease(rr.task_id, {
      lease_id: "lease-new", claimed_at: "2026-08-30T00:00:00.000Z", expires_at: "2026-08-30T01:00:00.000Z", worker_id: "w-new",
    });
    const leases = [olderExpired, newerExpired];
    const status = recoveryStatus({ tasksById, leases });
    check(
      "(i) recoveryStatus lists RECOVERY_REQUIRED only, picks latest-claimed (not latest-expiring) lease",
      status.length === 1 &&
      status[0].task_id === rr.task_id &&
      status[0].last_lease_id === "lease-new" &&
      status[0].last_worker_id === "w-new" &&
      status[0].lease_expired_at === "2026-08-30T01:00:00.000Z",
    );
  }

  // (i2) recoveryStatus/resolveRecovery also find a RELEASED (not just
  // EXPIRED) terminal lease -- this is the case produced by leases.mjs's
  // releaseLease() when a worker voluntarily releases an IN_PROGRESS task:
  // the lease is marked RELEASED, never EXPIRED, but the task still lands in
  // RECOVERY_REQUIRED and must remain triageable the same way.
  {
    const rr = makeRecoveryTask({ task_id: "REPAIR-0099" });
    const tasksById = new Map([[rr.task_id, rr]]);
    const releasedLease = makeExpiredLease(rr.task_id, {
      lease_id: "lease-released", status: "RELEASED", worker_id: "w-released", claimed_at: "2026-08-31T00:00:00.000Z",
    });
    const status = recoveryStatus({ tasksById, leases: [releasedLease] });
    const resolved = resolveRecovery({
      tasksById, leases: [releasedLease], taskId: rr.task_id, resolution: "FAILED",
      evidence: "worker released mid-task; state audited manually", resolvedBy: "coordinator", sourceBaseSha: SHA_A, now: NOW,
    });
    check(
      "(i2) recoveryStatus/resolveRecovery find a RELEASED terminal lease, not just EXPIRED",
      status.length === 1 && status[0].last_lease_id === "lease-released" && status[0].last_worker_id === "w-released" &&
      resolved.ok && resolved.recoveryRecord.previous_lease_id === "lease-released" &&
      resolved.recoveryRecord.reason === "lease voluntarily released while task was IN_PROGRESS",
    );
  }

  // (j) loadRecoveryLog/saveRecoveryLog round-trip against a temp path only
  {
    const dir = mkdtempSync(join(tmpdir(), "recovery-log-test-"));
    const path = join(dir, "recovery-log.json");
    const empty = loadRecoveryLog(path);
    const records = [
      { recovery_id: "recovery-1", task_id: "REPAIR-0001", previous_worker: "w1", previous_lease_id: "lease-1", reason: "r", resolution: "FAILED", evidence: "e", resolved_at: NOW, resolved_by: "coord" },
    ];
    saveRecoveryLog(path, records);
    const reloaded = loadRecoveryLog(path);
    check(
      "(j) loadRecoveryLog/saveRecoveryLog round-trip via temp path",
      Array.isArray(empty) && empty.length === 0 &&
      Array.isArray(reloaded) && reloaded.length === 1 && reloaded[0].recovery_id === "recovery-1",
    );
  }

  console.log(`\n${pass} passed, ${fail} failed`);
  process.exitCode = fail > 0 ? 1 : 0;
}

if (process.argv[1] === fileURLToPath(import.meta.url)) {
  runSelfTest();
}
