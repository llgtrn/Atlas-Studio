#!/usr/bin/env node
// Real lease mechanism for System Atlas tasks.
//
// A wave plan (build-dispatch-plan.mjs) is a PLAN, not a claim: it must never
// call into this module or otherwise mark a task LEASED. A lease here is the
// record of a real worker actually claiming a task -- created only via the
// CLI below (or claimLease() called directly by a future real worker
// harness). PLANNED and LEASED are different states; this file only ever
// produces the latter, and only on an explicit claim.
//
// This module implements one coherent lease lifecycle spanning two kinds of
// state that must always move together: a Lease record's own `status`
// (ACTIVE -> RELEASED | EXPIRED | COMPLETED) and the owning task's
// `status` field in whichever *-frontier.json file holds that task_id
// (READY/PLANNED -> LEASED -> IN_PROGRESS -> READY_TO_VERIFY, with
// RELEASED-while-LEASED falling back to PLANNED, and
// {RELEASED,EXPIRED}-while-IN_PROGRESS escalating to RECOVERY_REQUIRED --
// see releaseLease()/expireStaleLeases() below).
//
// B.2 hardening: every mutating CLI command below now goes through a single
// crash-safe, single-writer transaction (see runDispatchTransaction()) built
// on two sibling modules:
//   - dispatch-lock.mjs   -- exclusive filesystem lock (single writer at a
//     time; concurrent claims can no longer last-writer-wins clobber)
//   - dispatch-journal.mjs -- write-ahead journal (a crash between writing
//     leases.json and writing the affected frontier file can no longer leave
//     the pair inconsistent; an interrupted transaction is rolled forward by
//     the very next command, before it accepts any new work)
// See runDispatchTransaction()'s own comment for the exact protocol.
//
// ABORT_CLEAN was considered (per the hardening brief) and deliberately NOT
// implemented: there is no automatic way for this tooling to prove a
// partially-mutated working tree was safely reverted (that would require
// trusting a worker's own crash-time self-report, which is exactly the
// unverified claim RECOVERY_REQUIRED exists to refuse). Every IN_PROGRESS
// interruption -- whether by expiry or by voluntary release -- must go
// through the explicit, evidence-requiring RECOVERY_REQUIRED protocol in
// recovery.mjs; there is no code path that infers "clean" on its own.
//
// All state-machine logic below is implemented as PURE functions: they take
// in-memory data (a tasksById map, a leases array, a `now` timestamp) and
// return a result describing what changed. They perform NO file I/O. Only
// the CLI/orchestration code at the bottom of this file (guarded so it never
// runs on import -- the Atlas has already been bitten by exactly that bug
// once) does real file I/O.
import { existsSync, writeFileSync } from "node:fs";
import { resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { DEFAULT_JOURNAL_DIR, INVALID_JOURNAL, UNREADABLE_JOURNAL, applyTransaction, hasPendingTransaction, prepareTransaction, recoverPendingTransaction } from "./dispatch-journal.mjs";
import { DEFAULT_LOCK_PATH, currentHostId, holdsLock, releaseLock, tryAcquireOnce } from "./dispatch-lock.mjs";
import { retireDispatchGate } from "./hygiene-gate.mjs";
import { canBecomePlanned } from "./mutation-ownership.mjs";
import { REPO_ROOT, canonicalRefSha, nowIso, productSourceSha, readJson, repoRel } from "./lib.mjs";
import { RECOVERY_RESOLUTIONS, loadRecoveryLog, recoveryStatus, resolveRecovery, saveRecoveryLog } from "./recovery.mjs";

const OUT_DIR = resolve(REPO_ROOT, "docs/_machine/system-atlas/v0");
const LEASES_PATH = resolve(OUT_DIR, "leases.json");
const RECOVERY_LOG_PATH = resolve(OUT_DIR, "recovery-log.json");
const FRONTIER_FILES = ["repair-frontier.json", "verify-frontier.json", "build-frontier.json", "retire-frontier.json"];
const FRONTIER_PATHS = FRONTIER_FILES.map((f) => resolve(OUT_DIR, f));

// Same overlap semantics as pathsOverlap() in build-dispatch-plan.mjs -- do
// not diverge: strip a trailing "**" glob suffix and treat paths as
// overlapping if either (normalized) string is a prefix of the other.
function pathsOverlap(a, b) {
  const norm = (p) => p.replace(/\*\*$/, "");
  for (const pa of a) for (const pb of b) {
    const na = norm(pa);
    const nb = norm(pb);
    if (na === nb || na.startsWith(nb) || nb.startsWith(na)) return true;
  }
  return false;
}

// Task statuses that a worker may claim a fresh lease against. Anything else
// (DESIGN_REQUIRED, BLOCKED_*, LEASED, IN_PROGRESS, RECOVERY_REQUIRED, etc.)
// is not claimable -- RECOVERY_REQUIRED in particular must go through
// recovery.mjs's resolveRecovery(), never back through a plain claim.
const CLAIMABLE_STATUSES = new Set(["READY", "PLANNED"]);

const SHA_RE = /^[0-9a-f]{40}$/;

function isNonBlank(s) {
  return typeof s === "string" && s.trim().length > 0;
}

function isUnexpiredActive(lease, now) {
  return lease.status === "ACTIVE" && lease.expires_at > now;
}

function randomSuffix() {
  return Math.random().toString(36).slice(2, 8);
}

function getTask(tasksById, taskId) {
  return tasksById instanceof Map ? tasksById.get(taskId) : tasksById[taskId];
}

function findActiveLeaseForTask(leases, taskId, now) {
  return leases.find((l) => l.task_id === taskId && isUnexpiredActive(l, now));
}

// ---------------------------------------------------------------------------
// 1. claimLease -- pure. FAIL CLOSED: on any violation, returns
//    {ok:false, reason} and creates/mutates nothing.
// ---------------------------------------------------------------------------
export function claimLease({ tasksById, taskId, workerId, accountId, sourceBaseSha, ttlSeconds, existingLeases, now }) {
  const task = getTask(tasksById, taskId);
  if (!task) {
    return { ok: false, reason: `task ${taskId} not found` };
  }
  if (!CLAIMABLE_STATUSES.has(task.status)) {
    return { ok: false, reason: `task ${taskId} has status ${task.status}, not claimable (must be READY or PLANNED)` };
  }
  // Dispatch gates, re-checked at CLAIM time rather than trusted from the
  // task's status alone (BX6 adversarial finding). build-frontiers.mjs and
  // validate.mjs already refuse to let a mutating task with no real
  // owned_paths, or an un-cleared RETIRE task, SIT in a claimable status --
  // but both are generation-time/offline checks. A claim is the moment a real
  // worker starts touching real paths, and until now it was the one step in
  // the whole pipeline that re-derived nothing: a task that reached READY or
  // PLANNED by any route (a hand-edit, or -- the realistic case -- a
  // recovery.mjs REQUEUE_AFTER_CLEANUP resolution taken AFTER Lane C
  // downgraded that RETIRE task's hygiene_clearance to BLOCKED_UNIQUE_VALUE)
  // was claimable, with the violation only surfacing on the next offline
  // validate.mjs run, i.e. after the worker had already begun. These two
  // checks make the claim itself fail closed.
  if (!canBecomePlanned(task)) {
    return { ok: false, reason: `task ${taskId} is a mutating ${task.action} task with no real owned_paths -- not claimable` };
  }
  if (task.action === "RETIRE") {
    const gate = retireDispatchGate(task);
    if (!gate.allowed) {
      return { ok: false, reason: `task ${taskId} is a RETIRE task without hygiene clearance -- not claimable: ${gate.reason}` };
    }
  }
  if (!isNonBlank(workerId)) {
    return { ok: false, reason: "workerId must be a non-blank string" };
  }
  if (!isNonBlank(accountId)) {
    return { ok: false, reason: "accountId must be a non-blank string" };
  }
  if (!(typeof ttlSeconds === "number" && Number.isFinite(ttlSeconds) && ttlSeconds > 0)) {
    return { ok: false, reason: "ttlSeconds must be a positive finite number" };
  }
  if (!isNonBlank(sourceBaseSha) || !SHA_RE.test(sourceBaseSha)) {
    return { ok: false, reason: "sourceBaseSha must be a 40-hex-char string" };
  }
  if (sourceBaseSha !== task.source_base_sha) {
    return {
      ok: false,
      reason: `sourceBaseSha ${sourceBaseSha} does not match task ${taskId}'s recorded source_base_sha ${task.source_base_sha} (task derived from a different Atlas generation)`,
    };
  }

  const active = existingLeases.filter((l) => isUnexpiredActive(l, now));

  const alreadyLeased = active.find((l) => l.task_id === taskId);
  if (alreadyLeased) {
    return { ok: false, reason: `task ${taskId} already has an active lease (${alreadyLeased.lease_id}, held by ${alreadyLeased.worker_id}, expires ${alreadyLeased.expires_at})` };
  }

  const ownedPaths = task.owned_paths ?? [];
  if (ownedPaths.length > 0) {
    for (const other of active) {
      if (other.owned_paths?.length && pathsOverlap(ownedPaths, other.owned_paths)) {
        return { ok: false, reason: `owned_paths for ${taskId} overlap active lease ${other.lease_id} (task ${other.task_id}, held by ${other.worker_id})` };
      }
    }
  }

  const expiresAt = new Date(new Date(now).getTime() + ttlSeconds * 1000).toISOString();
  const lease = {
    lease_id: `lease-${taskId}-${randomSuffix()}`,
    task_id: taskId,
    worker_id: workerId,
    account_id: accountId,
    claimed_at: now,
    expires_at: expiresAt,
    source_base_sha: sourceBaseSha,
    owned_paths: ownedPaths,
    status: "ACTIVE",
  };

  return {
    ok: true,
    lease,
    leases: [...existingLeases, lease],
    taskStatusUpdate: { task_id: taskId, from: task.status, to: "LEASED" },
  };
}

// ---------------------------------------------------------------------------
// 2. beginWork -- pure. Requires task.status === "LEASED" and an ACTIVE
//    unexpired lease. Lease itself is untouched (ttl is not extended here).
// ---------------------------------------------------------------------------
export function beginWork({ tasksById, leases, taskId, now }) {
  const task = getTask(tasksById, taskId);
  if (!task) {
    return { ok: false, reason: `task ${taskId} not found` };
  }
  if (task.status !== "LEASED") {
    return { ok: false, reason: `task ${taskId} has status ${task.status}, expected LEASED` };
  }
  const lease = findActiveLeaseForTask(leases, taskId, now);
  if (!lease) {
    return { ok: false, reason: `no ACTIVE unexpired lease found for task ${taskId}` };
  }
  return { ok: true, taskStatusUpdate: { task_id: taskId, from: "LEASED", to: "IN_PROGRESS" } };
}

// ---------------------------------------------------------------------------
// 3. completeWork -- pure. Requires task.status === "IN_PROGRESS" and an
//    ACTIVE unexpired lease; flips that lease to COMPLETED.
// ---------------------------------------------------------------------------
export function completeWork({ tasksById, leases, taskId, now }) {
  const task = getTask(tasksById, taskId);
  if (!task) {
    return { ok: false, reason: `task ${taskId} not found` };
  }
  if (task.status !== "IN_PROGRESS") {
    return { ok: false, reason: `task ${taskId} has status ${task.status}, expected IN_PROGRESS` };
  }
  const lease = findActiveLeaseForTask(leases, taskId, now);
  if (!lease) {
    return { ok: false, reason: `no ACTIVE unexpired lease found for task ${taskId}` };
  }
  const updatedLeases = leases.map((l) => (l.lease_id === lease.lease_id ? { ...l, status: "COMPLETED" } : l));
  return {
    ok: true,
    leases: updatedLeases,
    taskStatusUpdate: { task_id: taskId, from: "IN_PROGRESS", to: "READY_TO_VERIFY" },
  };
}

// ---------------------------------------------------------------------------
// 4. releaseLease -- explicit, voluntary release. Pure. Requires the lease
//    exists and is currently ACTIVE.
//
//    B.2 CHANGE: releasing a LEASED (not-yet-started) task is still safe to
//    reclaim -> PLANNED. Releasing an IN_PROGRESS task is NOT: a worker
//    calling release proves only that it stopped, not that it left its
//    owned_paths in a safe, un-mutated (or cleanly-reverted) state -- a
//    second worker claiming the same paths right after could step on a
//    half-finished change. So IN_PROGRESS + release now escalates to
//    RECOVERY_REQUIRED, exactly like an IN_PROGRESS + expiry (see
//    expireStaleLeases below) -- the only way out is the explicit,
//    evidence-requiring protocol in recovery.mjs. (Previously this
//    incorrectly fell back to PLANNED, silently making an unverified
//    partial mutation reclaimable.)
// ---------------------------------------------------------------------------
export function releaseLease({ tasksById, leases, leaseId, now }) {
  const lease = leases.find((l) => l.lease_id === leaseId);
  if (!lease) {
    return { ok: false, reason: `lease ${leaseId} not found` };
  }
  if (lease.status !== "ACTIVE") {
    return { ok: false, reason: `lease ${leaseId} has status ${lease.status}, not ACTIVE` };
  }
  const updatedLeases = leases.map((l) => (l.lease_id === leaseId ? { ...l, status: "RELEASED" } : l));

  let taskStatusUpdate;
  const task = getTask(tasksById, lease.task_id);
  if (task && task.status === "LEASED") {
    taskStatusUpdate = { task_id: lease.task_id, from: "LEASED", to: "PLANNED" };
  } else if (task && task.status === "IN_PROGRESS") {
    taskStatusUpdate = { task_id: lease.task_id, from: "IN_PROGRESS", to: "RECOVERY_REQUIRED" };
  }

  return { ok: true, leases: updatedLeases, taskStatusUpdate };
}

// ---------------------------------------------------------------------------
// 5. expireStaleLeases -- pure, batch. Scans all leases; every ACTIVE lease
//    past its expires_at is flipped to EXPIRED. The owning task's CURRENT
//    status decides what happens next:
//      - LEASED (claimed but work never started)   -> safe to reclaim -> PLANNED
//      - IN_PROGRESS (may be partially mutated)     -> NOT safe to silently
//        reclaim -> RECOVERY_REQUIRED
// ---------------------------------------------------------------------------
export function expireStaleLeases({ tasksById, leases, now }) {
  const taskStatusUpdates = [];
  const updatedLeases = leases.map((l) => {
    if (l.status !== "ACTIVE" || l.expires_at > now) return l;
    const task = getTask(tasksById, l.task_id);
    if (task) {
      if (task.status === "LEASED") {
        taskStatusUpdates.push({ task_id: l.task_id, from: "LEASED", to: "PLANNED" });
      } else if (task.status === "IN_PROGRESS") {
        taskStatusUpdates.push({ task_id: l.task_id, from: "IN_PROGRESS", to: "RECOVERY_REQUIRED" });
      }
    }
    return { ...l, status: "EXPIRED" };
  });
  return { ok: true, leases: updatedLeases, taskStatusUpdates };
}

// ---------------------------------------------------------------------------
// 6. Basic load/save for leases.json.
// ---------------------------------------------------------------------------
export function loadLeases(path) {
  if (!existsSync(path)) return [];
  return readJson(path);
}

export function saveLeases(path, leases) {
  writeFileSync(path, `${JSON.stringify(leases, null, 2)}\n`);
}

// ===========================================================================
// Orchestration -- crash-safe, single-writer transactions.
//
// Every function below takes explicit paths (never the hardcoded OUT_DIR
// constants above) so it is fully testable against temp fixtures -- the CLI
// at the bottom of this file is the only place that supplies the real repo
// paths. This mirrors the parameter-injection convention already used by
// dispatch-lock.mjs and dispatch-journal.mjs.
// ===========================================================================

export const DEFAULT_LOCK_TTL_SECONDS = 60;
export const DEFAULT_MAX_LOCK_ATTEMPTS = 8;

// A synchronous, event-loop-blocking sleep (Atomics.wait on a throwaway
// SharedArrayBuffer) -- the only portable way to back off between lock
// attempts in a synchronous CLI without pulling in an async runtime split
// across this whole module. Bounded and short; never used inside a test's
// happy path (tests either avoid contention entirely or inject their own
// sleepFn).
export function sleepSync(ms) {
  if (ms <= 0) return;
  Atomics.wait(new Int32Array(new SharedArrayBuffer(4)), 0, 0, ms);
}

// Loads every frontier file at the given paths, building a tasksById Map
// (task_id -> task object) plus a record of which file (and its parsed doc)
// each task_id came from, so a status update can be written back to the
// correct file. Parametrized over frontierPaths so tests can point this at
// temp fixtures instead of the real docs/_machine/system-atlas/v0/ files.
export function loadFrontiersFrom(frontierPaths) {
  const tasksById = new Map();
  const taskLocation = new Map(); // task_id -> { path, doc }
  for (const path of frontierPaths) {
    if (!existsSync(path)) continue;
    const doc = readJson(path);
    for (const t of doc.tasks ?? []) {
      tasksById.set(t.task_id, t);
      taskLocation.set(t.task_id, { path, doc });
    }
  }
  return { tasksById, taskLocation };
}

function applyTaskStatusUpdate(taskLocation, update) {
  if (!update) return null;
  const loc = taskLocation.get(update.task_id);
  if (!loc) throw new Error(`internal error: no frontier location found for task ${update.task_id}`);
  const task = loc.doc.tasks.find((t) => t.task_id === update.task_id);
  if (!task) throw new Error(`internal error: task ${update.task_id} missing from its own frontier doc`);
  task.status = update.to;
  return loc;
}

// Given an outcome from one of the pure state-machine functions above (plus
// whatever extra fields the CLI/orchestration layer attaches -- taskStatusUpdates,
// an optional new leases array, an optional recovery-log append), computes the
// full set of {path, content} file writes this transaction must durably apply
// as one unit. Every task-status update that lands in the SAME frontier file
// is applied to that file's in-memory doc before it is serialized once --
// never serialized once per update.
export function computeIntendedFileUpdates({ leasesPath, leases, taskLocation, taskStatusUpdates, recoveryLogPath, recoveryLogAppend }) {
  const touchedFiles = new Map(); // path -> doc
  for (const update of taskStatusUpdates ?? []) {
    const loc = applyTaskStatusUpdate(taskLocation, update);
    if (loc) touchedFiles.set(loc.path, loc.doc);
  }

  const updates = [];
  if (leases) {
    updates.push({ path: leasesPath, content: `${JSON.stringify(leases, null, 2)}\n` });
  }
  for (const [path, doc] of touchedFiles) {
    updates.push({ path, content: `${JSON.stringify(doc, null, 2)}\n` });
  }
  if (recoveryLogAppend) {
    const existingLog = loadRecoveryLog(recoveryLogPath);
    const newLog = [...existingLog, recoveryLogAppend];
    updates.push({ path: recoveryLogPath, content: `${JSON.stringify(newLog, null, 2)}\n` });
  }
  return updates;
}

// Acquires the single-writer dispatch lock, rolling forward any pending
// journal FIRST -- every mutating command's contract is "recover, then
// acquire, then act", so an interrupted prior transaction is always
// completed before new work is accepted (see dispatch-journal.mjs's module
// header for the full IDLE/PREPARED/COMMITTED reasoning).
//
// Recovering a pending journal is safe to do WITHOUT holding the lock, and
// (B.2.1) this now holds across GENERATIONS, not merely within one: journal
// identity is per-transaction (journalDir/txn-<transaction_id>.json), so a
// worker that read one transaction's record, stalled, and resumes after a
// LATER transaction has been prepared and/or committed at the same
// journalDir can still only ever touch its own transaction's file --
// applyTransaction() derives that file's path from the record's own
// transaction_id, never from journalDir's current contents, and classifies
// every target file's current bytes against that transaction's own
// before_hash/after_hash before writing anything (see
// dispatch-journal.mjs's applyTransaction() doc comment for the
// already-applied / safe-to-apply / DIVERGED-fail-closed three-way split).
// So even two workers racing the exact SAME transaction converge safely (both
// write identical bytes, both get a no-op ENOENT on the redundant delete),
// AND a stale worker racing a DIFFERENT, newer transaction can neither delete
// that transaction's journal nor revert its committed state. What must NOT
// happen is a worker treating a RECOVERY_REQUIRED lock classification as
// reclaimable on its own; tryAcquireOnce() below already refuses that
// (returns needsRecovery:true, retryable:false, and does not touch the lock
// file), so this loop always resolves the journal before it can even attempt
// the lock again.
export function acquireDispatchLock({
  lockPath,
  journalDir,
  workerId,
  pid = process.pid,
  hostId = currentHostId(),
  ttlSeconds = DEFAULT_LOCK_TTL_SECONDS,
  maxAttempts = DEFAULT_MAX_LOCK_ATTEMPTS,
  sourceBaseSha,
  sleepFn = sleepSync,
  // B.2.2 defect D: recovery must re-validate every journal target path
  // against the REAL set of dispatch-state files this system is ever
  // allowed to write -- see dispatch-journal.mjs's journalRecordProblem()
  // doc comment. Optional so this module's lower-level tests (and any
  // caller with no fixed path set) can omit it; runDispatchTransaction()
  // below always supplies the real one, computed from the exact paths it
  // was itself given.
  allowedTargetPaths,
}) {
  for (let attempt = 0; attempt < maxAttempts; attempt += 1) {
    if (hasPendingTransaction(journalDir)) {
      const recovery = recoverPendingTransaction(journalDir, { allowedTargetPaths });
      // BX7 adversarial finding: a RECOVERY_CONFLICT is PERMANENT by
      // construction -- applyTransaction() leaves both the diverged target
      // file and the journal record untouched, so re-running recovery can
      // never change the outcome. The loop used to just `continue` here,
      // burning every remaining attempt and then throwing "still contended by
      // another worker" -- an actively misleading diagnosis for a state that
      // is not contention at all, has no other worker involved, and requires a
      // human to inspect the diverged file. The whole point of leaving the
      // journal record on disk is that the conflict is SURFACED, so surface it
      // here too rather than dressing it up as transient contention.
      // UNREADABLE_JOURNAL and INVALID_JOURNAL (B.2.2 defect D) are permanent
      // for the same reason (the record is deliberately never acted on and
      // never deleted), so both are surfaced identically rather than silently
      // retried into a contention message.
      const stuck = (recovery.outcomes ?? []).filter((o) => o.status === "RECOVERY_CONFLICT" || o.status === UNREADABLE_JOURNAL || o.status === INVALID_JOURNAL);
      if (stuck.length > 0) {
        const detail = stuck
          .map((c) => (c.status === "RECOVERY_CONFLICT"
            ? `${c.transactionId} (diverged: ${c.divergedPaths.map((d) => repoRel(d.path)).join(", ")})`
            : `${c.transactionId} (${c.status.toLowerCase().replace(/_/g, " ")}: ${c.reason})`))
          .join("; ");
        throw new Error(
          `refusing to acquire the dispatch lock (${repoRel(lockPath)}): ${stuck.length} pending transaction(s) cannot be recovered -- ${detail}. ` +
          `Rolling these forward would either overwrite state written by something else or act on a record this protocol did not write or does not trust. ` +
          `Nothing was written. Inspect the journal record(s) under ${repoRel(journalDir)} and the affected file(s) by hand; this state never clears on its own.`,
        );
      }
      continue; // re-check from scratch -- journal is now (or already was) clear
    }
    const now = nowIso();
    const attemptResult = tryAcquireOnce({ lockPath, workerId, pid, hostId, ttlSeconds, sourceBaseSha, now, journalPending: false });
    if (attemptResult.ok) return attemptResult.lockRecord;
    if (attemptResult.needsRecovery) continue; // loop will recover the journal above
    // B.2.2 adversarial-review finding: EXPIRED_OWNER_UNKNOWN is not
    // ordinary contention -- nothing about waiting longer makes an unknown
    // host's (or an indeterminate liveness check's) previous owner any more
    // knowable to us, so looping to exhaustion here would only produce the
    // same misleading "still contended by another worker" message BX7
    // already fixed for RECOVERY_CONFLICT/UNREADABLE_JOURNAL. Surface it
    // immediately, distinctly, and fail closed: no mutation happens, and an
    // operator must resolve it (confirm the recorded owner is genuinely
    // dead, e.g. by inspecting the foreign host or the lock's age by hand).
    if (attemptResult.staleLockRecoveryRequired) {
      throw new Error(
        `refusing to acquire the dispatch lock (${repoRel(lockPath)}): ${attemptResult.reason}. ` +
        `TTL expiry alone is never sufficient to reclaim a lock (B.2.2) -- this requires an operator to confirm the ` +
        `previous owner is genuinely gone before the lock file can be removed by hand; it will not clear on its own.`,
      );
    }
    // retryable:true -- ACTIVE_OWNED by someone else, EXPIRED_OWNER_ALIVE
    // (the previous owner may still resume and commit at any moment -- see
    // dispatch-lock.mjs's classifyOwnerLiveness()), or we lost an EEXIST race.
    sleepFn(150 * (attempt + 1));
  }
  throw new Error(`could not acquire the dispatch lock (${repoRel(lockPath)}) after ${maxAttempts} attempts -- still contended by another worker (its recorded owner may still be alive, or another worker keeps winning the race)`);
}

// One full crash-safe transaction: acquire the lock (recovering any prior
// interrupted transaction first), reload state FRESH (never trust state read
// before the lock was held -- a concurrent recovery or another worker's
// committed transaction may have changed it), run the caller's pure
// state-machine step against that fresh state, and -- only if it succeeds --
// journal and apply every resulting file write as one unit before releasing
// the lock. If buildOutcome() returns {ok:false}, nothing is written and no
// journal is ever created (a rejected operation has zero side effects, same
// as before B.2).
export function runDispatchTransaction({
  operation,
  workerId,
  pid = process.pid,
  hostId = currentHostId(),
  lockPath,
  journalDir,
  leasesPath,
  frontierPaths,
  recoveryLogPath,
  ttlSeconds = DEFAULT_LOCK_TTL_SECONDS,
  maxLockAttempts = DEFAULT_MAX_LOCK_ATTEMPTS,
  sourceBaseSha,
  canonicalRefShaValue = null,
  buildOutcome,
  sleepFn = sleepSync,
  // Test-only injection seam (same pattern as classifyOwnerLivenessFn/
  // sleepFn elsewhere in this file): applyTransaction() runs immediately
  // after prepareTransaction() with no caller-controlled hook in between,
  // so a real RECOVERY_CONFLICT at this exact call site can only be forced
  // deterministically (no timing tricks) by substituting this function --
  // see dispatch-orchestration.test.mjs's false-success regression tests.
  // Production callers never pass this.
  applyTransactionFn = applyTransaction,
}) {
  // B.2.2 defect D: the REAL allowed dispatch-state surface is exactly the
  // paths this call itself was given -- never a hardcoded list, so this
  // stays correct for any caller (the real CLI, or a test pointing at a
  // temp fixture) without needing to be told twice.
  const allowedTargetPaths = new Set([leasesPath, ...frontierPaths, recoveryLogPath]);
  const lockRecord = acquireDispatchLock({ lockPath, journalDir, workerId, pid, hostId, ttlSeconds, maxAttempts: maxLockAttempts, sourceBaseSha, sleepFn, allowedTargetPaths });
  try {
    const { tasksById, taskLocation } = loadFrontiersFrom(frontierPaths);
    const leases = loadLeases(leasesPath);
    const now = nowIso();

    const outcome = buildOutcome({ tasksById, taskLocation, leases, now });
    if (!outcome.ok) return outcome;

    const intendedFileUpdates = computeIntendedFileUpdates({
      leasesPath,
      leases: outcome.leases,
      taskLocation,
      taskStatusUpdates: outcome.taskStatusUpdates,
      recoveryLogPath,
      recoveryLogAppend: outcome.recoveryLogAppend,
    });

    if (intendedFileUpdates.length > 0) {
      // Acquiring the lock is not the same as HOLDING it: a lock has a TTL,
      // and buildOutcome() above is caller-supplied work of unbounded
      // duration. Pre-B.2.2, ANY worker could reclaim a merely-expired lock
      // on timestamp alone -- so if our own TTL lapsed while we computed
      // (routine under real contention/slow buildOutcome), another worker
      // had legitimately taken over and possibly already committed, and
      // journaling our own (now stale) `leases`/`tasksById` here would
      // silently revert its committed state while its own CLI had already
      // reported success. This was reproduced deterministically -- see the
      // B.2.2 PR history.
      //
      // B.2.2 (architect finding + fix, dispatch-lock.mjs): TTL expiry alone
      // no longer authorizes reclaim. A second worker may only take over an
      // expired lock once it machine-verifies the recorded owner is DEAD
      // (classifyOwnerLiveness: different host_id, or a genuinely
      // nonexistent/recycled pid on this host) -- while THIS worker is
      // actively executing (as it is, right here), no other worker can
      // satisfy that proof against it, so holdsLock() below checking
      // lock_id continuity (not the timestamp -- see its own doc comment
      // for why the timestamp is no longer the relevant signal) is genuine
      // proof nobody else has taken over. Re-check ownership immediately
      // before the write-ahead journal is created and fail closed if it is
      // gone: nothing is journaled and nothing is written, exactly like any
      // other rejected operation.
      //
      // RESIDUAL (still real, now far narrower -- do not overclaim it away):
      // the gap between holdsLock() returning ok:true and
      // prepareTransaction()'s first read is a couple of synchronous
      // statements with no caller code in between, unlike buildOutcome, so
      // it cannot be made arbitrarily long. Under B.2.2 the only way it
      // still matters is if THIS process genuinely crashes in that exact
      // instruction gap AND a second worker, in that same gap, both detects
      // the crash and completes an entire acquire-compute-commit cycle --
      // requiring a real crash plus a full concurrent transaction landing in
      // a machine-instruction-scale window, not merely this worker's TTL
      // lapsing (which real contention hits routinely, and which is exactly
      // what B.2.2 closes). Closing this fully needs fencing tokens (the
      // lock_id embedded in and checked atomically by the final write, or a
      // compare-and-swap primitive this filesystem-based design does not
      // have) -- a genuine design change, out of scope for a control-plane
      // system that never executes real work. Do not read this comment as
      // claiming a guarantee this code does not provide.
      const stillHeld = holdsLock({ lockPath, lockId: lockRecord.lock_id });
      if (!stillHeld.ok) {
        return {
          ok: false,
          reason: `refusing to commit ${operation}: ${stillHeld.reason}. The state this transaction was computed from may already be stale -- nothing was written; re-run the command.`,
        };
      }
      const transactionId = `${operation}-${randomSuffix()}${randomSuffix()}`;
      const journalRecord = prepareTransaction({
        journalDir,
        transactionId,
        operation,
        sourceBaseSha,
        canonicalRefSha: canonicalRefShaValue,
        intendedFileUpdates,
        now,
      });
      const applyResult = applyTransactionFn(journalRecord, journalDir);
      // B.2.2 adversarial-review finding (defect B): applyResult's status
      // used to be discarded entirely, so `return outcome` below reported
      // the pure state-machine decision (ok:true, a lease/task transition)
      // as SUCCESS even when applyTransaction() actually returned
      // RECOVERY_CONFLICT -- the intended writes were never committed (the
      // journal is still PREPARED, target bytes are unchanged per the
      // fail-closed rule), yet the caller/CLI would have printed ok:true
      // and a lease as if the claim had gone through. Only COMMITTED may
      // report success; anything else (RECOVERY_CONFLICT today, or any
      // future status this code does not explicitly recognize) fails
      // closed here, explicitly, rather than being silently accepted.
      if (applyResult.status !== "COMMITTED") {
        const detail =
          applyResult.status === "RECOVERY_CONFLICT"
            ? `diverged path(s): ${applyResult.divergedPaths.map((d) => repoRel(d.path)).join(", ")} -- state changed since this transaction was prepared`
            : `unrecognized applyTransaction() status`;
        return {
          ok: false,
          reason: `${operation} did NOT commit (applyTransaction() returned ${applyResult.status}, not COMMITTED): ${detail}. ` +
            `Nothing was actually written despite the state-machine decision having succeeded -- the journal record ` +
            `(${repoRel(journalDir)}, transaction ${transactionId}) remains for inspection; this is not reported as a successful ${operation}.`,
        };
      }
    }

    return outcome;
  } finally {
    // Best-effort: if our own lock TTL already lapsed (e.g. a very slow
    // transaction) another worker may have machine-verified this worker
    // dead and reclaimed it already (B.2.2) -- releaseLock() correctly
    // refuses to delete a lock_id that isn't ours, so this never releases
    // someone else's lock.
    releaseLock({ lockPath, lockId: lockRecord.lock_id });
  }
}

// ===========================================================================
// CLI -- the ONLY place in this file that supplies real repo paths.
// ===========================================================================

function printResult(result) {
  console.log(JSON.stringify(result, null, 2));
}

// `sourceBaseSha` is taken as a PARAMETER, never re-read here: the caller
// resolves productSourceSha() exactly once per command and passes that single
// value both to the state-machine decision and to the journal record. A prior
// version called productSourceSha() here AND again inside buildOutcome, so one
// logical command could validate a claim against one product snapshot and
// journal a different one if a product commit landed between the two `git log`
// invocations -- a small but real TOCTOU between a decision and the provenance
// recorded for it (BX6 adversarial finding, scenario 18).
function realTransactionDefaults(operation, workerId, sourceBaseSha) {
  return {
    operation,
    workerId,
    lockPath: DEFAULT_LOCK_PATH,
    journalDir: DEFAULT_JOURNAL_DIR,
    leasesPath: LEASES_PATH,
    frontierPaths: FRONTIER_PATHS,
    recoveryLogPath: RECOVERY_LOG_PATH,
    sourceBaseSha,
    canonicalRefShaValue: canonicalRefSha(),
  };
}

function cliClaim(argv) {
  const [taskId, workerId, accountId, ttlArg] = argv;
  if (!taskId || !workerId || !accountId) {
    console.error("usage: leases.mjs claim <task_id> <worker_id> <account_id> [ttlSeconds=3600]");
    process.exitCode = 1;
    return;
  }
  const ttlSeconds = ttlArg ? Number(ttlArg) : 3600;
  const sourceBaseSha = productSourceSha();

  const outcome = runDispatchTransaction({
    ...realTransactionDefaults("claim", workerId, sourceBaseSha),
    buildOutcome: ({ tasksById, leases, now }) => {
      const result = claimLease({ tasksById, taskId, workerId, accountId, sourceBaseSha, ttlSeconds, existingLeases: leases, now });
      if (!result.ok) return result;
      return { ok: true, leases: result.leases, taskStatusUpdates: [result.taskStatusUpdate], lease: result.lease, taskStatusUpdate: result.taskStatusUpdate };
    },
  });

  if (!outcome.ok) {
    printResult({ ok: false, reason: outcome.reason });
    process.exitCode = 1;
    return;
  }
  printResult({ ok: true, lease: outcome.lease, taskStatusUpdate: outcome.taskStatusUpdate });
  console.log(`wrote ${repoRel(LEASES_PATH)} and updated task ${taskId} -> LEASED`);
}

function cliBegin(argv) {
  const [taskId] = argv;
  if (!taskId) {
    console.error("usage: leases.mjs begin <task_id>");
    process.exitCode = 1;
    return;
  }

  const outcome = runDispatchTransaction({
    ...realTransactionDefaults("begin", "cli", productSourceSha()),
    buildOutcome: ({ tasksById, leases, now }) => {
      const result = beginWork({ tasksById, leases, taskId, now });
      if (!result.ok) return result;
      return { ok: true, taskStatusUpdates: [result.taskStatusUpdate], taskStatusUpdate: result.taskStatusUpdate };
    },
  });

  if (!outcome.ok) {
    printResult({ ok: false, reason: outcome.reason });
    process.exitCode = 1;
    return;
  }
  printResult({ ok: true, taskStatusUpdate: outcome.taskStatusUpdate });
  console.log(`updated task ${taskId} -> IN_PROGRESS`);
}

function cliComplete(argv) {
  const [taskId] = argv;
  if (!taskId) {
    console.error("usage: leases.mjs complete <task_id>");
    process.exitCode = 1;
    return;
  }

  const outcome = runDispatchTransaction({
    ...realTransactionDefaults("complete", "cli", productSourceSha()),
    buildOutcome: ({ tasksById, leases, now }) => {
      const result = completeWork({ tasksById, leases, taskId, now });
      if (!result.ok) return result;
      return { ok: true, leases: result.leases, taskStatusUpdates: [result.taskStatusUpdate], taskStatusUpdate: result.taskStatusUpdate };
    },
  });

  if (!outcome.ok) {
    printResult({ ok: false, reason: outcome.reason });
    process.exitCode = 1;
    return;
  }
  printResult({ ok: true, taskStatusUpdate: outcome.taskStatusUpdate });
  console.log(`wrote ${repoRel(LEASES_PATH)} and updated task ${taskId} -> READY_TO_VERIFY`);
}

function cliRelease(argv) {
  const [leaseId] = argv;
  if (!leaseId) {
    console.error("usage: leases.mjs release <lease_id>");
    process.exitCode = 1;
    return;
  }

  const outcome = runDispatchTransaction({
    ...realTransactionDefaults("release", "cli", productSourceSha()),
    buildOutcome: ({ tasksById, leases, now }) => {
      const result = releaseLease({ tasksById, leases, leaseId, now });
      if (!result.ok) return result;
      return {
        ok: true,
        leases: result.leases,
        taskStatusUpdates: result.taskStatusUpdate ? [result.taskStatusUpdate] : [],
        taskStatusUpdate: result.taskStatusUpdate ?? null,
      };
    },
  });

  if (!outcome.ok) {
    printResult({ ok: false, reason: outcome.reason });
    process.exitCode = 1;
    return;
  }
  printResult({ ok: true, lease_id: leaseId, status: "RELEASED", taskStatusUpdate: outcome.taskStatusUpdate });
  console.log(`wrote ${repoRel(LEASES_PATH)}${outcome.taskStatusUpdate ? ` and updated task ${outcome.taskStatusUpdate.task_id} -> ${outcome.taskStatusUpdate.to}` : ""}`);
  if (outcome.taskStatusUpdate?.to === "RECOVERY_REQUIRED") {
    console.log(`NOTE: task ${outcome.taskStatusUpdate.task_id} was IN_PROGRESS -- releasing it does not prove owned_paths are safe to reclaim. Use 'leases.mjs recovery-status' / 'leases.mjs resolve-recovery' to triage.`);
  }
}

function cliExpireStale() {
  const outcome = runDispatchTransaction({
    ...realTransactionDefaults("expire-stale", "cli", productSourceSha()),
    buildOutcome: ({ tasksById, leases, now }) => {
      const result = expireStaleLeases({ tasksById, leases, now });
      if (!result.ok) return result;
      return { ok: true, leases: result.leases, taskStatusUpdates: result.taskStatusUpdates };
    },
  });

  if (!outcome.ok) {
    printResult({ ok: false, reason: outcome.reason });
    process.exitCode = 1;
    return;
  }
  printResult({ ok: true, taskStatusUpdates: outcome.taskStatusUpdates });
  console.log(`wrote ${repoRel(LEASES_PATH)}; ${outcome.taskStatusUpdates.length} task status update(s) applied`);
  if (outcome.taskStatusUpdates.some((u) => u.to === "RECOVERY_REQUIRED")) {
    console.log("NOTE: one or more tasks entered RECOVERY_REQUIRED. Use 'leases.mjs recovery-status' / 'leases.mjs resolve-recovery' to triage.");
  }
}

function cliRecoveryStatus() {
  const { tasksById } = loadFrontiersFrom(FRONTIER_PATHS);
  const leases = loadLeases(LEASES_PATH);
  const status = recoveryStatus({ tasksById, leases });
  printResult({ ok: true, recovery_required: status });
  console.log(`${status.length} task(s) in RECOVERY_REQUIRED`);
}

function cliResolveRecovery(argv) {
  const [taskId, resolution, evidence, resolvedBy, workerId, accountId, ttlArg] = argv;
  if (!taskId || !resolution || !evidence || !resolvedBy) {
    console.error(`usage: leases.mjs resolve-recovery <task_id> <resolution> <evidence> <resolved_by> [worker_id] [account_id] [ttlSeconds=3600]`);
    console.error(`  resolution must be one of: ${RECOVERY_RESOLUTIONS.join(", ")}`);
    console.error(`  worker_id/account_id/ttlSeconds are required only when resolution=RESUME_SAME_WORK`);
    process.exitCode = 1;
    return;
  }
  const ttlSeconds = ttlArg ? Number(ttlArg) : 3600;
  const sourceBaseSha = productSourceSha();

  const outcome = runDispatchTransaction({
    ...realTransactionDefaults("resolve-recovery", workerId ?? "cli", sourceBaseSha),
    buildOutcome: ({ tasksById, leases, now }) => {
      const result = resolveRecovery({
        tasksById, leases, taskId, resolution, evidence, resolvedBy,
        sourceBaseSha, workerId, accountId, ttlSeconds, now,
      });
      if (!result.ok) return result;
      return {
        ok: true,
        leases: result.newLease ? [...leases, result.newLease] : undefined,
        taskStatusUpdates: [result.taskStatusUpdate],
        recoveryLogAppend: result.recoveryRecord,
        taskStatusUpdate: result.taskStatusUpdate,
        newLease: result.newLease,
        recoveryRecord: result.recoveryRecord,
      };
    },
  });

  if (!outcome.ok) {
    printResult({ ok: false, reason: outcome.reason });
    process.exitCode = 1;
    return;
  }
  printResult({ ok: true, taskStatusUpdate: outcome.taskStatusUpdate, newLease: outcome.newLease, recoveryRecord: outcome.recoveryRecord });
  console.log(`wrote ${repoRel(RECOVERY_LOG_PATH)} and updated task ${taskId} -> ${outcome.taskStatusUpdate.to}${outcome.newLease ? ` (new lease ${outcome.newLease.lease_id})` : ""}`);
}

function main() {
  const [cmd, ...rest] = process.argv.slice(2);
  if (cmd === "claim") return cliClaim(rest);
  if (cmd === "begin") return cliBegin(rest);
  if (cmd === "complete") return cliComplete(rest);
  if (cmd === "release") return cliRelease(rest);
  if (cmd === "expire-stale") return cliExpireStale();
  if (cmd === "recovery-status") return cliRecoveryStatus();
  if (cmd === "resolve-recovery") return cliResolveRecovery(rest);
  console.error("usage: leases.mjs <claim|begin|complete|release|expire-stale|recovery-status|resolve-recovery> ...");
  process.exitCode = 1;
}

// ===========================================================================
// Self-test -- exercises the pure functions above against small in-memory
// fixtures only. Never touches the real leases.json or any real frontier
// file. Run with: node tools/system-atlas/leases.mjs (no args).
//
// The crash-injection and concurrent-claim tests for the orchestration layer
// (runDispatchTransaction, acquireDispatchLock) live in the separate
// tools/system-atlas/dispatch-orchestration.test.mjs -- they need real
// temp-directory file I/O and (for the concurrency case) real child
// processes, which doesn't fit this file's "self-test with in-memory
// fixtures only" contract.
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

  function makeTask(overrides) {
    return {
      task_id: "REPAIR-0001",
      status: "READY",
      source_base_sha: SHA_A,
      owned_paths: ["crates/core/foo/**"],
      ...overrides,
    };
  }

  // (a) claim succeeds READY -> LEASED
  {
    const task = makeTask({ status: "READY" });
    const tasksById = new Map([[task.task_id, task]]);
    const r = claimLease({ tasksById, taskId: task.task_id, workerId: "w1", accountId: "acc1", sourceBaseSha: SHA_A, ttlSeconds: 3600, existingLeases: [], now: NOW });
    check("(a) claim succeeds READY->LEASED", r.ok && r.taskStatusUpdate.to === "LEASED" && r.lease.status === "ACTIVE" && r.lease.source_base_sha === SHA_A);
  }

  // (b) claim succeeds PLANNED -> LEASED
  {
    const task = makeTask({ status: "PLANNED" });
    const tasksById = new Map([[task.task_id, task]]);
    const r = claimLease({ tasksById, taskId: task.task_id, workerId: "w1", accountId: "acc1", sourceBaseSha: SHA_A, ttlSeconds: 3600, existingLeases: [], now: NOW });
    check("(b) claim succeeds PLANNED->LEASED", r.ok && r.taskStatusUpdate.from === "PLANNED" && r.taskStatusUpdate.to === "LEASED");
  }

  // (c) second claim on same task_id fails
  {
    const task = makeTask({ status: "READY" });
    const tasksById = new Map([[task.task_id, task]]);
    const first = claimLease({ tasksById, taskId: task.task_id, workerId: "w1", accountId: "acc1", sourceBaseSha: SHA_A, ttlSeconds: 3600, existingLeases: [], now: NOW });
    const second = claimLease({ tasksById, taskId: task.task_id, workerId: "w2", accountId: "acc2", sourceBaseSha: SHA_A, ttlSeconds: 3600, existingLeases: first.leases, now: NOW });
    check("(c) second claim on same task_id fails", first.ok && !second.ok);
  }

  // (d) claim fails on owned_paths overlap with another active lease
  {
    const t1 = makeTask({ task_id: "REPAIR-0001", status: "READY", owned_paths: ["crates/core/foo/**"] });
    const t2 = makeTask({ task_id: "REPAIR-0002", status: "READY", owned_paths: ["crates/core/foo/bar.rs"] });
    const tasksById = new Map([[t1.task_id, t1], [t2.task_id, t2]]);
    const first = claimLease({ tasksById, taskId: t1.task_id, workerId: "w1", accountId: "acc1", sourceBaseSha: SHA_A, ttlSeconds: 3600, existingLeases: [], now: NOW });
    const second = claimLease({ tasksById, taskId: t2.task_id, workerId: "w2", accountId: "acc2", sourceBaseSha: SHA_A, ttlSeconds: 3600, existingLeases: first.leases, now: NOW });
    check("(d) claim fails on owned_paths overlap", first.ok && !second.ok);
  }

  // (e) claim fails when sourceBaseSha is null/wrong-length/mismatched
  {
    const task = makeTask({ status: "READY" });
    const tasksById = new Map([[task.task_id, task]]);
    const rNull = claimLease({ tasksById, taskId: task.task_id, workerId: "w1", accountId: "acc1", sourceBaseSha: null, ttlSeconds: 3600, existingLeases: [], now: NOW });
    const rShort = claimLease({ tasksById, taskId: task.task_id, workerId: "w1", accountId: "acc1", sourceBaseSha: "abc123", ttlSeconds: 3600, existingLeases: [], now: NOW });
    const rMismatch = claimLease({ tasksById, taskId: task.task_id, workerId: "w1", accountId: "acc1", sourceBaseSha: SHA_B, ttlSeconds: 3600, existingLeases: [], now: NOW });
    check("(e) claim fails on null/wrong-length/mismatched sourceBaseSha", !rNull.ok && !rShort.ok && !rMismatch.ok);
  }

  // (f) claim fails on blank worker_id
  {
    const task = makeTask({ status: "READY" });
    const tasksById = new Map([[task.task_id, task]]);
    const r = claimLease({ tasksById, taskId: task.task_id, workerId: "   ", accountId: "acc1", sourceBaseSha: SHA_A, ttlSeconds: 3600, existingLeases: [], now: NOW });
    check("(f) claim fails on blank worker_id", !r.ok);
  }

  // (g) claim fails on blank account_id
  {
    const task = makeTask({ status: "READY" });
    const tasksById = new Map([[task.task_id, task]]);
    const r = claimLease({ tasksById, taskId: task.task_id, workerId: "w1", accountId: "", sourceBaseSha: SHA_A, ttlSeconds: 3600, existingLeases: [], now: NOW });
    check("(g) claim fails on blank account_id", !r.ok);
  }

  // (h) claim fails on non-positive ttlSeconds
  {
    const task = makeTask({ status: "READY" });
    const tasksById = new Map([[task.task_id, task]]);
    const rZero = claimLease({ tasksById, taskId: task.task_id, workerId: "w1", accountId: "acc1", sourceBaseSha: SHA_A, ttlSeconds: 0, existingLeases: [], now: NOW });
    const rNeg = claimLease({ tasksById, taskId: task.task_id, workerId: "w1", accountId: "acc1", sourceBaseSha: SHA_A, ttlSeconds: -5, existingLeases: [], now: NOW });
    check("(h) claim fails on non-positive ttlSeconds", !rZero.ok && !rNeg.ok);
  }

  // (i) claim fails when task.status is something else (e.g. DESIGN_REQUIRED)
  {
    const task = makeTask({ status: "DESIGN_REQUIRED" });
    const tasksById = new Map([[task.task_id, task]]);
    const r = claimLease({ tasksById, taskId: task.task_id, workerId: "w1", accountId: "acc1", sourceBaseSha: SHA_A, ttlSeconds: 3600, existingLeases: [], now: NOW });
    check("(i) claim fails when task.status is DESIGN_REQUIRED", !r.ok);
  }

  // (i2) claim fails when task.status is RECOVERY_REQUIRED -- must go
  // through recovery.mjs's resolveRecovery(), never a plain claim.
  {
    const task = makeTask({ status: "RECOVERY_REQUIRED" });
    const tasksById = new Map([[task.task_id, task]]);
    const r = claimLease({ tasksById, taskId: task.task_id, workerId: "w1", accountId: "acc1", sourceBaseSha: SHA_A, ttlSeconds: 3600, existingLeases: [], now: NOW });
    check("(i2) claim fails when task.status is RECOVERY_REQUIRED", !r.ok);
  }

  // (j) beginWork/completeWork happy path
  {
    const task = makeTask({ status: "READY" });
    const tasksById = new Map([[task.task_id, task]]);
    const claimed = claimLease({ tasksById, taskId: task.task_id, workerId: "w1", accountId: "acc1", sourceBaseSha: SHA_A, ttlSeconds: 3600, existingLeases: [], now: NOW });
    task.status = "LEASED";
    const began = beginWork({ tasksById, leases: claimed.leases, taskId: task.task_id, now: NOW });
    task.status = "IN_PROGRESS";
    const completed = completeWork({ tasksById, leases: claimed.leases, taskId: task.task_id, now: NOW });
    check(
      "(j) beginWork/completeWork happy path",
      began.ok && began.taskStatusUpdate.to === "IN_PROGRESS" &&
      completed.ok && completed.taskStatusUpdate.to === "READY_TO_VERIFY" &&
      completed.leases.find((l) => l.lease_id === claimed.lease.lease_id).status === "COMPLETED",
    );
  }

  // (k) releaseLease moves LEASED->PLANNED but IN_PROGRESS->RECOVERY_REQUIRED
  // (B.2 change: IN_PROGRESS release no longer falls back to PLANNED).
  {
    const taskLeased = makeTask({ task_id: "REPAIR-0001", status: "READY" });
    const tasksById1 = new Map([[taskLeased.task_id, taskLeased]]);
    const claimed1 = claimLease({ tasksById: tasksById1, taskId: taskLeased.task_id, workerId: "w1", accountId: "acc1", sourceBaseSha: SHA_A, ttlSeconds: 3600, existingLeases: [], now: NOW });
    taskLeased.status = "LEASED";
    const rel1 = releaseLease({ tasksById: tasksById1, leases: claimed1.leases, leaseId: claimed1.lease.lease_id, now: NOW });

    const taskInProgress = makeTask({ task_id: "REPAIR-0002", status: "READY" });
    const tasksById2 = new Map([[taskInProgress.task_id, taskInProgress]]);
    const claimed2 = claimLease({ tasksById: tasksById2, taskId: taskInProgress.task_id, workerId: "w1", accountId: "acc1", sourceBaseSha: SHA_A, ttlSeconds: 3600, existingLeases: [], now: NOW });
    taskInProgress.status = "IN_PROGRESS";
    const rel2 = releaseLease({ tasksById: tasksById2, leases: claimed2.leases, leaseId: claimed2.lease.lease_id, now: NOW });

    check(
      "(k) releaseLease: LEASED->PLANNED, IN_PROGRESS->RECOVERY_REQUIRED",
      rel1.ok && rel1.taskStatusUpdate.to === "PLANNED" && rel1.leases.find((l) => l.lease_id === claimed1.lease.lease_id).status === "RELEASED" &&
      rel2.ok && rel2.taskStatusUpdate.to === "RECOVERY_REQUIRED" && rel2.leases.find((l) => l.lease_id === claimed2.lease.lease_id).status === "RELEASED",
    );
  }

  // (l) expireStaleLeases: expired LEASED->PLANNED, expired IN_PROGRESS->RECOVERY_REQUIRED
  {
    const taskLeased = makeTask({ task_id: "REPAIR-0001", status: "LEASED" });
    const taskInProgress = makeTask({ task_id: "REPAIR-0002", status: "IN_PROGRESS" });
    const tasksById = new Map([[taskLeased.task_id, taskLeased], [taskInProgress.task_id, taskInProgress]]);
    const leases = [
      { lease_id: "lease-1", task_id: taskLeased.task_id, worker_id: "w1", account_id: "acc1", claimed_at: "2026-08-31T00:00:00.000Z", expires_at: "2026-08-31T01:00:00.000Z", source_base_sha: SHA_A, owned_paths: taskLeased.owned_paths, status: "ACTIVE" },
      { lease_id: "lease-2", task_id: taskInProgress.task_id, worker_id: "w2", account_id: "acc2", claimed_at: "2026-08-31T00:00:00.000Z", expires_at: "2026-08-31T01:00:00.000Z", source_base_sha: SHA_A, owned_paths: taskInProgress.owned_paths, status: "ACTIVE" },
    ];
    const r = expireStaleLeases({ tasksById, leases, now: NOW });
    const u1 = r.taskStatusUpdates.find((u) => u.task_id === taskLeased.task_id);
    const u2 = r.taskStatusUpdates.find((u) => u.task_id === taskInProgress.task_id);
    check(
      "(l) expireStaleLeases: LEASED->PLANNED, IN_PROGRESS->RECOVERY_REQUIRED",
      r.ok && u1?.to === "PLANNED" && u2?.to === "RECOVERY_REQUIRED" &&
      r.leases.every((l) => l.status === "EXPIRED"),
    );
  }

  // (m) computeIntendedFileUpdates: two task-status updates landing in the
  // SAME frontier doc must produce exactly one file write for that doc, with
  // both updates applied to it.
  {
    const docA = { tasks: [{ task_id: "REPAIR-1001", status: "READY" }, { task_id: "REPAIR-1002", status: "READY" }] };
    const taskLocation = new Map([
      ["REPAIR-1001", { path: "/tmp/fake-a.json", doc: docA }],
      ["REPAIR-1002", { path: "/tmp/fake-a.json", doc: docA }],
    ]);
    const updates = computeIntendedFileUpdates({
      leasesPath: "/tmp/fake-leases.json",
      leases: undefined,
      taskLocation,
      taskStatusUpdates: [{ task_id: "REPAIR-1001", to: "PLANNED" }, { task_id: "REPAIR-1002", to: "LEASED" }],
      recoveryLogPath: "/tmp/fake-recovery.json",
      recoveryLogAppend: undefined,
    });
    check(
      "(m) computeIntendedFileUpdates coalesces multiple updates to the same file into one write",
      updates.length === 1 && updates[0].path === "/tmp/fake-a.json" &&
      JSON.parse(updates[0].content).tasks[0].status === "PLANNED" &&
      JSON.parse(updates[0].content).tasks[1].status === "LEASED",
    );
  }

  console.log(`\n${pass} passed, ${fail} failed`);
  process.exitCode = fail > 0 ? 1 : 0;
}

if (process.argv[1] === fileURLToPath(import.meta.url)) {
  const argv = process.argv.slice(2);
  if (argv.length === 0) {
    runSelfTest();
  } else {
    main();
  }
}
