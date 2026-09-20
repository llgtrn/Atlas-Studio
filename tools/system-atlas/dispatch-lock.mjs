#!/usr/bin/env node
// Exclusive filesystem lock for System Atlas dispatch, built on an atomic
// exclusive-publish primitive: fs.linkSync(scratch, lockPath) fails with
// EEXIST if the target already exists. That single syscall is the only thing
// standing between "two workers claim the same task" and "one worker claims
// it" -- everything else here is bookkeeping around it.
//
// It is link() rather than the more obvious writeFileSync(..., {flag:"wx"})
// for one reason: `wx` makes the file VISIBLE atomically but writes its
// CONTENT in a second syscall, so a concurrent reader can observe (and fail
// to parse) an empty lock file -- see readLock()'s TORN READS note.
// linkSync() publishes a file whose content is already complete, with exactly
// the same all-or-nothing EEXIST semantics.
//
// Deliberately NOT a check-then-write pattern: readLock() is used only to
// decide *how* to react to an existing lock (classifyLock), never to decide
// *whether* it is safe to write. The actual "am I allowed to write" decision
// is made by the OS at the moment of the link(), which is why concurrent
// callers racing on the same lockPath can never both "win".
//
// B.2.2 STALE-OWNER FENCING (architect finding): a TTL expiry is NOT proof
// the previous owner has stopped running -- it only proves the owner has not
// RENEWED its lock. A worker whose own long computation merely outlasts its
// TTL is still very much alive and can resume at any moment; treating
// "expired" as "safe to reclaim" (this module's behavior before B.2.2) lets
// a second worker commit a transaction, and then the FIRST worker resumes
// and unknowingly overwrites it with content it computed before losing the
// lock (reproduced deterministically: see the B.2.2 PR history and
// leases.mjs's runDispatchTransaction() doc comment). TTL expiry now only
// means STALE_CANDIDATE-for-liveness-check, never SAFE_TO_RECLAIM on its
// own. Reclaiming an expired lock additionally requires machine-verifiable
// proof the previous owner cannot resume execution -- see
// classifyOwnerLiveness() below. Availability loss (a worker that must wait,
// or a human that must intervene) is an accepted cost; a silent stale
// overwrite is not.
//
// This module is a pure library plus a self-test. It intentionally has NO
// CLI of its own -- the coordinator's leases.mjs orchestration is expected
// to import tryAcquireOnce/releaseLock and wrap tryAcquireOnce in its own
// bounded retry loop for real contention.
import { linkSync, readFileSync, unlinkSync, writeFileSync } from "node:fs";
import { randomUUID } from "node:crypto";
import { hostname } from "node:os";
import { resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { REPO_ROOT } from "./lib.mjs";

// Thin default only -- never referenced by any function below. Core logic
// always takes an explicit lockPath parameter; this exists purely so a
// future CLI wrapper (built by the coordinator) has somewhere to import a
// sane default from instead of hardcoding the path itself.
export const DEFAULT_LOCK_PATH = resolve(REPO_ROOT, "docs/_machine/system-atlas/v0/dispatch.lock");

// LOCK_CONTRACT_VERSION -- a supplementary, module-local counter for this
// record's own shape/semantics, for anyone reading this file in isolation.
// It is NOT a second authoritative version number: the Lock and Journal
// interfaces are both covered by the single `dispatch_contract_version`
// field in dispatch-plan.json (see docs/_machine/system-atlas/README.md's
// "Dispatch contract freeze" section) -- THAT is the one number a
// downstream consumer (Lane C, Lane D) actually pins against. This constant
// just documents, at the point of definition, which revision the record
// shape below is on: v1 was B.2 (lock_id/worker_id/pid/created_at/
// expires_at/source_base_sha); v2 is B.2.2, which added host_id/
// process_start_identity (both now required) and changed reclamation
// semantics from "TTL expiry alone is sufficient" to "TTL expiry plus
// machine-verified owner death" -- both real breaks for any consumer that
// assumed the old fields/semantics, which is exactly why
// dispatch_contract_version was bumped (1 -> 2 in B.2.1, for the journal's
// on-disk location change; 2 -> 3 in B.2.2, for this).
export const LOCK_CONTRACT_VERSION = 2;

function newLockId() {
  return `lock-${randomUUID()}`;
}

// ---------------------------------------------------------------------------
// currentHostId -- this process's identity for the `host_id` lock field.
// os.hostname() is what's available portably in Node; it identifies "this
// machine" well enough for a repo-local, single-host control plane (the only
// kind this module is designed for -- see classifyOwnerLiveness()'s
// docs/_machine/system-atlas/README.md cross-reference for the explicit
// multi-host non-goal).
// ---------------------------------------------------------------------------
export function currentHostId() {
  return hostname();
}

// ---------------------------------------------------------------------------
// processStartIdentity -- a value that is stable for the lifetime of process
// `pid` and DIFFERENT for any other process that the OS later assigns the
// same pid number to (defends against PID REUSE: two unrelated processes,
// possibly years apart, sharing a numeric pid is routine on any long-lived
// host). Returns null if this cannot be determined -- callers must treat
// null as "unknown", never as "matches" or "differs".
//
// PORTABILITY (documented, not silently assumed): this reads
// /proc/<pid>/stat's starttime field (field 22 per `man proc`; the comm
// field can itself contain spaces/parens, so this splits on the LAST `)`
// rather than the first). This ONLY works on Linux with /proc mounted --
// exactly this repo's execution environment. On any other platform, or if
// /proc/<pid> cannot be read for any reason (already gone, permissions,
// no /proc at all), this returns null, and every caller below (see
// classifyOwnerLiveness) treats null as unable-to-prove rather than
// guessing.
// ---------------------------------------------------------------------------
export function processStartIdentity(pid) {
  try {
    const stat = readFileSync(`/proc/${pid}/stat`, "utf8");
    const afterComm = stat.slice(stat.lastIndexOf(")") + 1).trim().split(/\s+/);
    // afterComm[0] is field 3 (state); starttime is field 22 -> index 19.
    const startTime = afterComm[19];
    return startTime && /^\d+$/.test(startTime) ? startTime : null;
  } catch {
    return null;
  }
}

// ---------------------------------------------------------------------------
// isProcessAlive -- true/false/null (null = genuinely indeterminate, kept
// distinct from false so callers never conflate "we couldn't check" with
// "confirmed gone"). process.kill(pid, 0) sends no signal -- it only asks
// the OS "does this pid exist and may I signal it" -- and is POSIX-portable
// (Node also implements it on Windows).
// ---------------------------------------------------------------------------
function isProcessAlive(pid) {
  try {
    process.kill(pid, 0);
    return true;
  } catch (err) {
    if (err.code === "ESRCH") return false; // no such process: definitely gone
    if (err.code === "EPERM") return true; // exists, just not ours to signal
    return null; // some other, genuinely indeterminate error
  }
}

export const OWNER_ALIVE = "OWNER_ALIVE";
export const OWNER_DEAD = "OWNER_DEAD";
export const OWNER_UNKNOWN = "OWNER_UNKNOWN";

// ---------------------------------------------------------------------------
// classifyOwnerLiveness -- the ONLY place that does actual host/pid liveness
// probing (impure; everything else in this module that reasons about
// liveness takes its result as a plain value, same pattern as
// `journalPending`). Called only for an EXPIRED lock -- an unexpired lock is
// ACTIVE_OWNED regardless of liveness, so liveness is irrelevant until TTL
// expiry makes reclamation even a question.
//
// - A lock recorded by a DIFFERENT (or missing/pre-B.2.2) host_id: this
//   module has no way to inspect a foreign host's process table. Per the
//   B.2.2 brief: "do not guess from TTL" -- OWNER_UNKNOWN, never a guess.
// - Same host, pid does not exist (ESRCH): OWNER_DEAD -- the process that
//   held this lock cannot possibly resume.
// - Same host, pid exists, but its current process_start_identity differs
//   from what the lock recorded: the pid number has been RECYCLED by an
//   unrelated process since the lock was written. The ORIGINAL owner is
//   still dead -- OWNER_DEAD, not OWNER_ALIVE, even though something with
//   that pid number is currently running.
// - Same host, pid exists, start identity matches (or cannot be compared on
//   either side, e.g. a non-Linux host or a lock written before B.2.2):
//   OWNER_ALIVE if identities agree, else OWNER_UNKNOWN if either side's
//   start identity is unavailable -- never OWNER_ALIVE on a guess, but also
//   never OWNER_DEAD without the disagreement actually being observed.
// ---------------------------------------------------------------------------
export function classifyOwnerLiveness({ lockRecord, hostId = currentHostId(), processStartIdentityFn = processStartIdentity, isProcessAliveFn = isProcessAlive }) {
  if (!lockRecord.host_id || lockRecord.host_id !== hostId) {
    return OWNER_UNKNOWN;
  }
  const alive = isProcessAliveFn(lockRecord.pid);
  if (alive === null) return OWNER_UNKNOWN;
  if (!alive) return OWNER_DEAD;
  const recordedStart = lockRecord.process_start_identity ?? null;
  const currentStart = processStartIdentityFn(lockRecord.pid);
  if (recordedStart === null || currentStart === null) {
    // The pid is alive, but we cannot confirm it is the SAME process --
    // exactly the ambiguity PID reuse creates. Treating "alive" as "the
    // recorded owner" here would be the bug this whole mechanism exists to
    // close.
    return OWNER_UNKNOWN;
  }
  return currentStart === recordedStart ? OWNER_ALIVE : OWNER_DEAD;
}

// Marker returned by readLock() for a lock file that exists but does not
// parse. See readLock()/classifyLock() below for why this is a state rather
// than an exception.
export const TORN_LOCK = "TORN";

// ---------------------------------------------------------------------------
// readLock -- parsed lock record, null if no lock file exists, or a
// {[TORN_LOCK]: true} marker if the file exists but does not parse. Does not
// itself decide anything about staleness/validity; classifyLock does that.
//
// TORN READS (BX6 adversarial finding): a lock file is published in two steps
// -- the exclusive create makes it VISIBLE, and its content lands immediately
// afterwards -- so a concurrent reader can observe a zero-byte (or partially
// written) lock file. `JSON.parse("")` throws a SyntaxError, which used to
// escape uncaught all the way out of tryAcquireOnce() ->
// acquireDispatchLock(): under real contention roughly 1 in 5 test runs had a
// worker die with a raw `SyntaxError: Unexpected end of JSON input` stack
// trace instead of either acquiring or cleanly reporting contention. An
// unparseable lock is a STATE the coordinator must classify, never a crash.
// (tryAcquireOnce() below now also publishes lock content atomically, so this
// path should be unreachable for locks this module writes -- it remains as
// the defence for a lock left by an older writer, a hand-edit, or a crash
// mid-write.)
// ---------------------------------------------------------------------------
export function readLock(lockPath) {
  let raw;
  try {
    raw = readFileSync(lockPath, "utf8");
  } catch (err) {
    if (err.code === "ENOENT") return null;
    throw err;
  }
  try {
    return JSON.parse(raw);
  } catch {
    return { [TORN_LOCK]: true, byte_length: raw.length };
  }
}

// ---------------------------------------------------------------------------
// classifyLock -- PURE given already-read state. Never reads the filesystem
// itself; the caller reads the lock (readLock), separately determines
// journalPending (from the sibling dispatch-journal.mjs module), and --
// B.2.2 -- separately determines ownerLiveness (via classifyOwnerLiveness(),
// itself impure) for an EXPIRED lock, so this function stays fully decoupled
// and trivially testable with fabricated inputs.
//
// TTL expiry is NEVER by itself sufficient to reclaim (B.2.2): it only ever
// produces one of the EXPIRED_OWNER_* outcomes below, gated on
// ownerLiveness. There is no "STALE_CANDIDATE" outcome any more -- that name
// implied "probably safe to take", which is exactly the false assumption
// that produced the B.2.2 lost-update.
// ---------------------------------------------------------------------------
export function classifyLock({ lockRecord, now, journalPending, ownerLiveness }) {
  if (!lockRecord) return "ABSENT";
  // A lock file that exists but does not parse carries no expires_at, so it
  // can never be judged stale -- it must NOT be treated as ABSENT (that would
  // invite deleting/overwriting a lock another worker is in the middle of
  // publishing). Almost always this is a momentary torn read that the next
  // attempt resolves; a genuinely corrupt lock file will exhaust the caller's
  // retry budget and surface as a loud, diagnosable acquisition failure --
  // fail-closed, and never a silent takeover.
  if (lockRecord[TORN_LOCK]) return TORN_LOCK;
  const expired = !(lockRecord.expires_at > now);
  if (!expired) return "ACTIVE_OWNED";
  // A pending journal always takes priority over owner-liveness: rolling a
  // journal forward only ever completes an ALREADY-DECIDED set of writes
  // (idempotent by construction, see dispatch-journal.mjs), so it is safe
  // regardless of whether the transaction's own lock holder is alive, dead,
  // or unknown.
  if (journalPending) return "RECOVERY_REQUIRED";
  if (ownerLiveness === OWNER_DEAD) return "EXPIRED_OWNER_DEAD";
  if (ownerLiveness === OWNER_ALIVE) return "EXPIRED_OWNER_ALIVE";
  return "EXPIRED_OWNER_UNKNOWN"; // OWNER_UNKNOWN, or any unrecognized value -- fail closed by default
}

// ---------------------------------------------------------------------------
// tryAcquireOnce -- ONE attempt, no retry/waiting. The CLI layer built by the
// coordinator is expected to wrap this in a bounded retry loop for real
// contention; this function never loops or sleeps on its own.
//
// `hostId` and `classifyOwnerLivenessFn` are injectable (default to the real
// implementations) purely so tests can fabricate liveness outcomes without
// needing a real second process for every scenario; production callers never
// need to pass either.
// ---------------------------------------------------------------------------
export function tryAcquireOnce({ lockPath, workerId, pid, hostId = currentHostId(), ttlSeconds, sourceBaseSha, now, journalPending, classifyOwnerLivenessFn = classifyOwnerLiveness }) {
  const lockRecord = readLock(lockPath);
  // Owner liveness is only ever relevant for an expired, parseable,
  // journal-clear lock -- computing it unconditionally would probe a pid
  // that classifyLock is about to ignore anyway (ABSENT/ACTIVE_OWNED/TORN)
  // or that RECOVERY_REQUIRED will handle first regardless.
  const ownerLiveness =
    lockRecord && !lockRecord[TORN_LOCK] && !(lockRecord.expires_at > now) && !journalPending
      ? classifyOwnerLivenessFn({ lockRecord, hostId })
      : undefined;
  const classification = classifyLock({ lockRecord, now, journalPending, ownerLiveness });

  if (classification === "ACTIVE_OWNED") {
    return { ok: false, reason: `lock held by ${lockRecord.worker_id} until ${lockRecord.expires_at}`, retryable: true };
  }

  if (classification === TORN_LOCK) {
    // Retryable, and the lock file is deliberately left untouched: another
    // worker is most likely mid-publish, and we must never delete or
    // overwrite a record we cannot read.
    return { ok: false, reason: `lock file exists but does not parse (${lockRecord.byte_length} byte(s)) -- another worker may be publishing it right now; if this persists the lock file is corrupt and must be inspected manually`, retryable: true };
  }

  if (classification === "RECOVERY_REQUIRED") {
    // Deliberately retryable:false -- retrying the lock attempt alone won't
    // help. The caller (coordinator's leases.mjs orchestration) must run
    // journal recovery first, then retry. We must NOT touch the existing
    // lock file in this branch.
    return {
      ok: false,
      reason: "pending transaction must be recovered before this lock can be reclaimed",
      retryable: false,
      needsRecovery: true,
    };
  }

  if (classification === "EXPIRED_OWNER_ALIVE") {
    // The lock has lapsed, but its recorded owner is machine-verifiably
    // still running on this host -- it may resume and commit at any moment.
    // Reclaiming here is exactly the B.2.2 lost-update; refuse, and let the
    // caller's retry loop wait for either a real release or a genuine crash.
    return {
      ok: false,
      reason: `lock is expired but its owner (worker ${lockRecord.worker_id}, pid ${lockRecord.pid} on host ${lockRecord.host_id}) is still alive -- refusing to reclaim; it may resume and commit at any moment`,
      retryable: true,
    };
  }

  if (classification === "EXPIRED_OWNER_UNKNOWN") {
    // The lock has lapsed and we cannot machine-verify the previous owner is
    // gone (a foreign/unrecorded host, or an indeterminate liveness check on
    // our own host). Per the B.2.2 brief: do not guess from TTL. This is
    // NOT retryable in the ordinary sense -- nothing about waiting longer
    // makes an unknown host's process table any more knowable to us -- so it
    // is surfaced distinctly (staleLockRecoveryRequired) for the caller to
    // report as an explicit, human-actionable state rather than silently
    // retried into a generic contention message.
    return {
      ok: false,
      reason: `lock is expired but its owner's liveness cannot be machine-verified (host_id ${lockRecord.host_id ?? "(not recorded)"} vs this host ${hostId}, or process-start identity unavailable) -- refusing to auto-reclaim`,
      retryable: false,
      staleLockRecoveryRequired: true,
    };
  }

  // ABSENT or EXPIRED_OWNER_DEAD: attempt to become the new lock holder.
  if (classification === "EXPIRED_OWNER_DEAD") {
    // BX8 adversarial finding: this delete used to be UNCONDITIONAL ("best
    // effort; if someone else already replaced it, the exclusive create below
    // will observe the race"). That reasoning is wrong, because the delete
    // itself is the race: the decision to delete was made from the readLock()
    // at the top of this function, and classifyOwnerLiveness() in between does
    // real syscalls (process.kill + reading /proc/<pid>/stat), so the gap
    // between "we read the stale record" and "we delete it" is many syscalls
    // wide, not a machine instruction. Two workers finding the SAME crashed
    // lock -- the normal, expected situation after a real crash -- therefore
    // interleaved as: A unlinks the stale lock and links its OWN fresh,
    // UNEXPIRED lock; B (still holding its stale classification) then unlinks
    // A's brand-new lock and links its own. Both then believe they hold the
    // dispatch lock, which is the one invariant this whole module exists to
    // provide -- and unlike the documented instruction-gap residual, it needs
    // no crash and no TTL lapse at all. (Reproduced deterministically; see the
    // (d4) self-test below.)
    //
    // COMPARE-AND-DELETE: re-read immediately before deleting and only remove
    // the file if it is STILL the very record we classified as dead
    // (lock_id equality -- randomUUID-derived, so never reused across
    // acquisitions). Anything else means the reclaim was won or the lock was
    // re-taken by somebody else while we were classifying, so we must not
    // touch it: report a lost race and let the caller's retry loop re-observe
    // reality from scratch. This does not make the delete atomic with the
    // read -- nothing in POSIX unlink() can -- but it narrows the window from
    // "readLock + a full liveness probe" down to two adjacent statements, the
    // same machine-instruction scale as the residual documented in holdsLock()
    // and leases.mjs's runDispatchTransaction(), and it removes the case
    // entirely for any interleaving in which the competing reclaim has already
    // published its lock.
    const recheck = readLock(lockPath);
    if (recheck !== null) {
      if (recheck[TORN_LOCK] || recheck.lock_id !== lockRecord.lock_id) {
        return {
          ok: false,
          reason: `the expired lock we classified as dead (${lockRecord.lock_id}) was replaced by another worker before we could reclaim it -- refusing to delete a lock record we did not classify`,
          retryable: true,
        };
      }
      try {
        unlinkSync(lockPath);
      } catch (err) {
        if (err.code !== "ENOENT") throw err;
      }
    }
  }

  const lockRecord2 = {
    lock_id: newLockId(),
    worker_id: workerId,
    pid,
    host_id: hostId,
    process_start_identity: processStartIdentity(pid),
    created_at: now,
    expires_at: new Date(new Date(now).getTime() + ttlSeconds * 1000).toISOString(),
    source_base_sha: sourceBaseSha,
  };

  // The atomicity primitive: publish the lock as ONE indivisible step, so the
  // file is never observable in a half-written state.
  //
  // This used to be writeFileSync(lockPath, ..., {flag:"wx"}). That create is
  // atomic, but the WRITE that follows it is a second syscall: between them
  // the lock file exists and is empty, and any concurrent reader parsing it
  // crashed (BX6 adversarial finding -- see readLock()'s TORN READS note).
  // link() gives the same all-or-nothing "I won / EEXIST, someone else won"
  // semantics as O_EXCL, but publishes a file whose CONTENT is already
  // complete: write the record to a private scratch file first, then link it
  // into place. Either the link succeeds (we are the holder, and every reader
  // sees the whole record) or it throws EEXIST (someone else is the holder).
  // As before, the "am I allowed to write" decision is made by the OS at this
  // one syscall, never by the readLock() above.
  const scratchPath = `${lockPath}.${pid}-${newLockId().slice(5, 13)}.tmp`;
  try {
    writeFileSync(scratchPath, `${JSON.stringify(lockRecord2, null, 2)}\n`);
    try {
      linkSync(scratchPath, lockPath);
    } catch (err) {
      if (err.code === "EEXIST") {
        return { ok: false, reason: "lost race to another worker", retryable: true };
      }
      throw err;
    }
  } finally {
    // Always drop our scratch file -- on success the lock path is now a
    // second link to the same inode, so removing this one changes nothing.
    try {
      unlinkSync(scratchPath);
    } catch {
      // never created, or already gone; nothing to clean up
    }
  }

  return { ok: true, lockRecord: lockRecord2 };
}

// ---------------------------------------------------------------------------
// holdsLock -- "do I STILL hold the lock I acquired?" Answers the question
// tryAcquireOnce() cannot: acquiring a lock is not the same as holding it for
// the whole transaction. A lock has a TTL, so a caller whose own work
// outlives that TTL can no longer point to its own timestamp as proof of
// ownership -- but B.2.2 changed what that actually implies. Pre-B.2.2,
// TTL expiry alone authorized ANY other worker to reclaim, so an expired
// timestamp here had to be treated as "may already be lost" defensively.
// Since B.2.2, reclaiming an expired lock additionally requires
// machine-verified proof the recorded owner is dead (classifyOwnerLiveness);
// this function IS that owner, actively running (it is the one asking the
// question), so no other worker can have satisfied that proof against it --
// no other worker can legitimately have reclaimed this lock_id while this
// call is executing, REGARDLESS of what the timestamp says. The only thing
// that changes what's on disk is releaseLock() (deletes it) or a successful
// reclaim by someone else (replaces it with a NEW, different lock_id, which
// lock_id values are randomUUID-derived and therefore never collide across
// acquisitions) -- so `lock_id` continuity is by itself sufficient proof of
// continued ownership. Two ways to stop holding a lock, both reported
// distinctly: the file is gone (someone reclaimed AND released it), or the
// file now holds a different lock_id (someone reclaimed it and still holds
// it) -- both are only reachable via a machine-verified-dead reclaim, which
// this call proves did not happen to IT specifically.
//
// RESIDUAL (do not overclaim): this closes the "TTL alone" hazard, not every
// conceivable race. If this process were to crash in the literal instruction
// gap between holdsLock() returning ok:true and prepareTransaction()'s first
// write -- and a second worker, in that same gap, both detects the crash AND
// completes an entire acquire-compute-commit cycle -- the window discussed in
// leases.mjs's runDispatchTransaction() doc comment still applies. B.2.2
// makes that window require an actual crash plus a full concurrent
// transaction landing in a machine-instruction-scale gap, rather than merely
// requiring this worker's TTL to lapse (which real contention hits
// routinely) -- narrower by construction, not eliminated. Closing it fully
// needs fencing tokens, a genuine design change, out of scope here.
// ---------------------------------------------------------------------------
export function holdsLock({ lockPath, lockId }) {
  const lockRecord = readLock(lockPath);
  if (!lockRecord) {
    return { ok: false, reason: `dispatch lock ${lockId} is no longer present -- it was reclaimed (after being machine-verified dead) and released by another worker` };
  }
  if (lockRecord.lock_id !== lockId) {
    return { ok: false, reason: `dispatch lock is now held by ${lockRecord.worker_id} (lock_id ${lockRecord.lock_id}), not by us (${lockId}) -- it was reclaimed after being machine-verified dead` };
  }
  return { ok: true, lockRecord };
}

// ---------------------------------------------------------------------------
// releaseLock -- safe to call twice (already-absent lock is not an error).
// Refuses to release a lock whose lock_id does not match the caller's --
// a worker must never release a lock it does not actually hold (e.g. its
// own lease expired and someone else already reclaimed it).
// ---------------------------------------------------------------------------
export function releaseLock({ lockPath, lockId }) {
  const lockRecord = readLock(lockPath);
  if (!lockRecord) {
    return { ok: true, alreadyReleased: true };
  }
  if (lockRecord.lock_id !== lockId) {
    return { ok: false, reason: "lock is held by a different lock_id -- refusing to release someone else's lock" };
  }
  try {
    unlinkSync(lockPath);
  } catch (err) {
    if (err.code === "ENOENT") return { ok: true, alreadyReleased: true };
    throw err;
  }
  return { ok: true };
}

// ===========================================================================
// Self-test -- guarded so it never runs on import. Uses a real OS temp
// directory for every file operation; never the real
// docs/_machine/system-atlas/v0/ path.
// ===========================================================================
async function runSelfTest() {
  const { mkdtempSync, writeFileSync: fsWriteFileSync, readFileSync: fsReadFileSync, existsSync } = await import("node:fs");
  const { tmpdir } = await import("node:os");
  const { join } = await import("node:path");
  const { spawn } = await import("node:child_process");

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

  const tmpRoot = mkdtempSync(join(tmpdir(), "dispatch-lock-test-"));
  const SHA_A = "a".repeat(40);
  const NOW = "2026-09-01T00:00:00.000Z";
  const LATER = "2026-09-01T01:00:00.000Z"; // > NOW + default ttl below
  const PAST = "2026-08-31T23:00:00.000Z"; // < NOW: an already-expired expires_at

  // (a) tryAcquireOnce succeeds when no lock exists
  let firstResult;
  {
    const lockPath = join(tmpRoot, "a-absent.lock");
    firstResult = tryAcquireOnce({ lockPath, workerId: "w1", pid: 111, ttlSeconds: 60, sourceBaseSha: SHA_A, now: NOW, journalPending: false });
    check(
      "(a) tryAcquireOnce succeeds when no lock exists, and records host_id/process_start_identity",
      firstResult.ok && firstResult.lockRecord.worker_id === "w1" && existsSync(lockPath) &&
      firstResult.lockRecord.host_id === currentHostId() &&
      Object.hasOwn(firstResult.lockRecord, "process_start_identity"),
    );

    // (b) a second tryAcquireOnce immediately after (still ACTIVE_OWNED) fails, retryable
    const second = tryAcquireOnce({ lockPath, workerId: "w2", pid: 222, ttlSeconds: 60, sourceBaseSha: SHA_A, now: NOW, journalPending: false });
    check("(b) second tryAcquireOnce against ACTIVE_OWNED fails with retryable:true", !second.ok && second.retryable === true);
  }

  // (c) classifyLock: B.2.2 -- EXPIRED_OWNER_DEAD/ALIVE/UNKNOWN, gated by
  // ownerLiveness, plus RECOVERY_REQUIRED (always takes priority) and the
  // unaffected ABSENT/ACTIVE_OWNED/TORN cases.
  {
    const expiredRecord = { lock_id: "lock-x", worker_id: "wX", pid: 1, host_id: "some-host", process_start_identity: "123", created_at: PAST, expires_at: PAST, source_base_sha: SHA_A };
    check(
      "(c) classifyLock: EXPIRED_OWNER_DEAD/ALIVE/UNKNOWN gated by ownerLiveness",
      classifyLock({ lockRecord: expiredRecord, now: NOW, journalPending: false, ownerLiveness: OWNER_DEAD }) === "EXPIRED_OWNER_DEAD" &&
      classifyLock({ lockRecord: expiredRecord, now: NOW, journalPending: false, ownerLiveness: OWNER_ALIVE }) === "EXPIRED_OWNER_ALIVE" &&
      classifyLock({ lockRecord: expiredRecord, now: NOW, journalPending: false, ownerLiveness: OWNER_UNKNOWN }) === "EXPIRED_OWNER_UNKNOWN" &&
      classifyLock({ lockRecord: expiredRecord, now: NOW, journalPending: false, ownerLiveness: undefined }) === "EXPIRED_OWNER_UNKNOWN",
    );
    check(
      "(c1) classifyLock: RECOVERY_REQUIRED always wins over ownerLiveness (journal recovery is safe regardless of owner state)",
      classifyLock({ lockRecord: expiredRecord, now: NOW, journalPending: true, ownerLiveness: OWNER_DEAD }) === "RECOVERY_REQUIRED" &&
      classifyLock({ lockRecord: expiredRecord, now: NOW, journalPending: true, ownerLiveness: OWNER_ALIVE }) === "RECOVERY_REQUIRED",
    );
    check("(c2) classifyLock ABSENT for null record", classifyLock({ lockRecord: null, now: NOW, journalPending: false }) === "ABSENT");
    const activeRecord = { ...expiredRecord, expires_at: LATER };
    check(
      "(c3) classifyLock ACTIVE_OWNED for unexpired record regardless of ownerLiveness",
      classifyLock({ lockRecord: activeRecord, now: NOW, journalPending: false, ownerLiveness: OWNER_DEAD }) === "ACTIVE_OWNED",
    );
  }

  // (c4) classifyOwnerLiveness -- the actual liveness-probing function, via
  // dependency-injected probes so this is deterministic (no real process
  // spawning needed here; a REAL end-to-end version is test (j) below).
  {
    const HOST = "this-host";
    const base = { host_id: HOST, pid: 4242, process_start_identity: "999" };
    const deadProbe = { processStartIdentityFn: () => "999", isProcessAliveFn: () => false };
    const aliveMatchingProbe = { processStartIdentityFn: () => "999", isProcessAliveFn: () => true };
    const aliveButRecycledPidProbe = { processStartIdentityFn: () => "DIFFERENT-START-TIME", isProcessAliveFn: () => true };
    const indeterminateAliveProbe = { processStartIdentityFn: () => "999", isProcessAliveFn: () => null };
    const noStartIdentityAvailableProbe = { processStartIdentityFn: () => null, isProcessAliveFn: () => true };

    check(
      "(c4) classifyOwnerLiveness: foreign/missing host_id -> OWNER_UNKNOWN, never a guess",
      classifyOwnerLiveness({ lockRecord: base, hostId: "a-different-host", ...deadProbe }) === OWNER_UNKNOWN &&
      classifyOwnerLiveness({ lockRecord: { ...base, host_id: undefined }, hostId: HOST, ...deadProbe }) === OWNER_UNKNOWN,
    );
    check(
      "(c5) classifyOwnerLiveness: same host, pid does not exist -> OWNER_DEAD",
      classifyOwnerLiveness({ lockRecord: base, hostId: HOST, ...deadProbe }) === OWNER_DEAD,
    );
    check(
      "(c6) classifyOwnerLiveness: same host, pid alive, start identity MATCHES -> OWNER_ALIVE",
      classifyOwnerLiveness({ lockRecord: base, hostId: HOST, ...aliveMatchingProbe }) === OWNER_ALIVE,
    );
    check(
      "(c7) classifyOwnerLiveness: same host, pid alive, start identity DIFFERS (PID reuse) -> OWNER_DEAD, never OWNER_ALIVE",
      classifyOwnerLiveness({ lockRecord: base, hostId: HOST, ...aliveButRecycledPidProbe }) === OWNER_DEAD,
    );
    check(
      "(c8) classifyOwnerLiveness: liveness check itself indeterminate -> OWNER_UNKNOWN",
      classifyOwnerLiveness({ lockRecord: base, hostId: HOST, ...indeterminateAliveProbe }) === OWNER_UNKNOWN,
    );
    check(
      "(c9) classifyOwnerLiveness: pid alive but start identity unavailable (either side) -> OWNER_UNKNOWN, not a guess of ALIVE",
      classifyOwnerLiveness({ lockRecord: base, hostId: HOST, ...noStartIdentityAvailableProbe }) === OWNER_UNKNOWN &&
      classifyOwnerLiveness({ lockRecord: { ...base, process_start_identity: null }, hostId: HOST, processStartIdentityFn: () => "999", isProcessAliveFn: () => true }) === OWNER_UNKNOWN,
    );
  }

  // (d) tryAcquireOnce reclaims an expired lock ONLY when ownerLiveness
  // (via injected classifyOwnerLivenessFn) proves OWNER_DEAD.
  {
    const lockPath = join(tmpRoot, "d-dead-owner.lock");
    const staleRecord = { lock_id: "lock-stale", worker_id: "wOld", pid: 999, host_id: "h", process_start_identity: "1", created_at: PAST, expires_at: PAST, source_base_sha: SHA_A };
    fsWriteFileSync(lockPath, `${JSON.stringify(staleRecord, null, 2)}\n`);
    const r = tryAcquireOnce({ lockPath, workerId: "wNew", pid: 333, ttlSeconds: 60, sourceBaseSha: SHA_A, now: NOW, journalPending: false, classifyOwnerLivenessFn: () => OWNER_DEAD });
    const onDisk = JSON.parse(fsReadFileSync(lockPath, "utf8"));
    check("(d) tryAcquireOnce reclaims EXPIRED_OWNER_DEAD lock", r.ok && r.lockRecord.worker_id === "wNew" && onDisk.lock_id === r.lockRecord.lock_id);
  }

  // (d2) tryAcquireOnce REFUSES an expired lock whose owner is provably
  // still alive -- THIS is the fix for the B.2.2 lost-update: TTL expiry
  // alone must never authorize reclaim.
  {
    const lockPath = join(tmpRoot, "d2-alive-owner.lock");
    const aliveRecord = { lock_id: "lock-alive", worker_id: "wStillRunning", pid: 999, host_id: "h", process_start_identity: "1", created_at: PAST, expires_at: PAST, source_base_sha: SHA_A };
    const before = `${JSON.stringify(aliveRecord, null, 2)}\n`;
    fsWriteFileSync(lockPath, before);
    const r = tryAcquireOnce({ lockPath, workerId: "wNew", pid: 333, ttlSeconds: 60, sourceBaseSha: SHA_A, now: NOW, journalPending: false, classifyOwnerLivenessFn: () => OWNER_ALIVE });
    const after = fsReadFileSync(lockPath, "utf8");
    check(
      "(d2) tryAcquireOnce refuses EXPIRED_OWNER_ALIVE: retryable, lock file COMPLETELY untouched",
      !r.ok && r.retryable === true && !r.staleLockRecoveryRequired && after === before,
    );
  }

  // (d3) tryAcquireOnce REFUSES an expired lock whose owner liveness cannot
  // be machine-verified -- fail closed, never a guess, and distinctly NOT
  // retryable in the ordinary sense (surfaced via staleLockRecoveryRequired).
  {
    const lockPath = join(tmpRoot, "d3-unknown-owner.lock");
    const unknownRecord = { lock_id: "lock-unknown", worker_id: "wForeign", pid: 999, host_id: "some-other-machine", created_at: PAST, expires_at: PAST, source_base_sha: SHA_A };
    const before = `${JSON.stringify(unknownRecord, null, 2)}\n`;
    fsWriteFileSync(lockPath, before);
    const r = tryAcquireOnce({ lockPath, workerId: "wNew", pid: 333, hostId: "this-machine", ttlSeconds: 60, sourceBaseSha: SHA_A, now: NOW, journalPending: false });
    const after = fsReadFileSync(lockPath, "utf8");
    check(
      "(d3) tryAcquireOnce refuses EXPIRED_OWNER_UNKNOWN (foreign host, no DI stub needed -- real classifyOwnerLiveness): staleLockRecoveryRequired, lock file untouched",
      !r.ok && r.retryable === false && r.staleLockRecoveryRequired === true && after === before,
    );
  }

  // (d4) BX8 REGRESSION -- reclaiming an EXPIRED_OWNER_DEAD lock must never
  // delete a lock record other than the exact one it classified. Two workers
  // finding the same crashed lock is the normal post-crash situation, and the
  // reclaim's delete used to be unconditional, so the second worker's stale
  // decision deleted the FIRST worker's freshly-published, UNEXPIRED lock and
  // both ended up "holding" it -- a direct mutual-exclusion break needing
  // neither a crash nor a TTL lapse.
  //
  // The interleaving is forced deterministically (no sleeps/timing) through
  // the existing classifyOwnerLivenessFn injection seam, which runs at exactly
  // the point where B has read the stale record but has not yet acted on it:
  // worker A's entire reclaim is published from inside that call.
  {
    const lockPath = join(tmpRoot, "d4-double-reclaim.lock");
    const crashed = { lock_id: "lock-CRASHED", worker_id: "wCrashed", pid: 999999, host_id: currentHostId(), process_start_identity: "1", created_at: PAST, expires_at: PAST, source_base_sha: SHA_A };
    fsWriteFileSync(lockPath, `${JSON.stringify(crashed, null, 2)}\n`);

    // A reclaims for real (the OS-level winner).
    const a = tryAcquireOnce({ lockPath, workerId: "wA", pid: process.pid, ttlSeconds: 3600, sourceBaseSha: SHA_A, now: NOW, journalPending: false });

    // B's readLock() must observe the stale record, so restore it; A's fresh
    // lock is then published inside B's liveness probe -- i.e. after B read,
    // before B acts.
    fsWriteFileSync(lockPath, `${JSON.stringify(crashed, null, 2)}\n`);
    const b = tryAcquireOnce({
      lockPath, workerId: "wB", pid: process.pid, ttlSeconds: 3600, sourceBaseSha: SHA_A, now: NOW, journalPending: false,
      classifyOwnerLivenessFn: () => {
        fsWriteFileSync(lockPath, `${JSON.stringify(a.lockRecord, null, 2)}\n`);
        return OWNER_DEAD;
      },
    });

    const onDisk = JSON.parse(fsReadFileSync(lockPath, "utf8"));
    check(
      "(d4) BX8: reclaiming a dead owner's lock never deletes a DIFFERENT lock record -- A's fresh unexpired lock survives, B loses the race retryably, holdsLock stays true for A",
      a.ok === true &&
      b.ok === false && b.retryable === true &&
      onDisk.lock_id === a.lockRecord.lock_id &&
      holdsLock({ lockPath, lockId: a.lockRecord.lock_id }).ok === true,
    );
  }

  // (d5) BX8 -- the compare-and-delete above must NOT cost availability: a
  // genuinely dead owner's lock that nobody else is competing for is still
  // reclaimed on the first attempt (the recheck sees the same record).
  {
    const lockPath = join(tmpRoot, "d5-uncontended-dead.lock");
    const crashed = { lock_id: "lock-CRASHED-2", worker_id: "wCrashed", pid: 999999, host_id: currentHostId(), process_start_identity: "1", created_at: PAST, expires_at: PAST, source_base_sha: SHA_A };
    fsWriteFileSync(lockPath, `${JSON.stringify(crashed, null, 2)}\n`);
    const r = tryAcquireOnce({ lockPath, workerId: "wNew", pid: process.pid, ttlSeconds: 60, sourceBaseSha: SHA_A, now: NOW, journalPending: false });
    check(
      "(d5) BX8: an uncontended, genuinely dead owner's lock is still reclaimed first try -- no availability regression from compare-and-delete",
      r.ok === true && JSON.parse(fsReadFileSync(lockPath, "utf8")).lock_id === r.lockRecord.lock_id,
    );
  }

  // (e) tryAcquireOnce against RECOVERY_REQUIRED fails retryable:false, needsRecovery:true, file untouched
  {
    const lockPath = join(tmpRoot, "e-recovery.lock");
    const stuckRecord = { lock_id: "lock-stuck", worker_id: "wCrashed", pid: 777, created_at: PAST, expires_at: PAST, source_base_sha: SHA_A };
    const before = `${JSON.stringify(stuckRecord, null, 2)}\n`;
    fsWriteFileSync(lockPath, before);
    const r = tryAcquireOnce({ lockPath, workerId: "wNew", pid: 444, ttlSeconds: 60, sourceBaseSha: SHA_A, now: NOW, journalPending: true });
    const after = fsReadFileSync(lockPath, "utf8");
    check(
      "(e) tryAcquireOnce on RECOVERY_REQUIRED fails without mutating lock file (takes priority over owner liveness)",
      !r.ok && r.retryable === false && r.needsRecovery === true && after === before,
    );
  }

  // (j) REAL end-to-end dead-owner reclaim: spawn a real child process,
  // capture its process_start_identity while it is genuinely alive, let it
  // actually exit, then confirm classifyOwnerLiveness (no DI stubs -- the
  // real isProcessAlive/processStartIdentity) reports OWNER_DEAD, and that a
  // lock referencing it is reclaimable.
  {
    const child = spawn(process.execPath, ["-e", "setTimeout(() => {}, 300)"], { stdio: "ignore" });
    // spawn() only returns once the OS has actually created the child
    // process, so /proc/<pid> already exists -- no sleep/spin needed here.
    const capturedStart = processStartIdentity(child.pid);
    const childPid = child.pid;

    await new Promise((resolvePromise) => child.on("exit", resolvePromise));
    // The child has now genuinely exited.
    const lockPath = join(tmpRoot, "j-real-dead.lock");
    const record = { lock_id: "lock-real-dead", worker_id: "wRealChild", pid: childPid, host_id: currentHostId(), process_start_identity: capturedStart, created_at: PAST, expires_at: PAST, source_base_sha: SHA_A };
    fsWriteFileSync(lockPath, `${JSON.stringify(record, null, 2)}\n`);
    const liveness = classifyOwnerLiveness({ lockRecord: record, hostId: currentHostId() });
    const r = tryAcquireOnce({ lockPath, workerId: "wNew", pid: process.pid, ttlSeconds: 60, sourceBaseSha: SHA_A, now: NOW, journalPending: false });
    check(
      "(j) REAL exited child process is classified OWNER_DEAD (not a DI stub) and its lock is reclaimable",
      capturedStart !== null && liveness === OWNER_DEAD && r.ok === true,
    );
  }

  // (k) REAL end-to-end alive-owner refusal: use THIS process's own real
  // pid/host/start-identity (genuinely alive, no stub) as the lock's
  // recorded owner, with an EXPIRED timestamp -- proving the fix actually
  // closes the B.2.2 architect scenario against real process state, not
  // just a mocked classifyOwnerLivenessFn.
  {
    const lockPath = join(tmpRoot, "k-real-alive.lock");
    const ownStart = processStartIdentity(process.pid);
    const record = { lock_id: "lock-real-alive", worker_id: "wSelf", pid: process.pid, host_id: currentHostId(), process_start_identity: ownStart, created_at: PAST, expires_at: PAST, source_base_sha: SHA_A };
    const before = `${JSON.stringify(record, null, 2)}\n`;
    fsWriteFileSync(lockPath, before);
    const liveness = classifyOwnerLiveness({ lockRecord: record, hostId: currentHostId() });
    const r = tryAcquireOnce({ lockPath, workerId: "wOther", pid: 88888, ttlSeconds: 60, sourceBaseSha: SHA_A, now: NOW, journalPending: false });
    const after = fsReadFileSync(lockPath, "utf8");
    check(
      "(k) REAL still-running process (this test itself) is classified OWNER_ALIVE and its expired lock is NOT reclaimed -- the B.2.2 architect scenario, closed",
      ownStart !== null && liveness === OWNER_ALIVE && !r.ok && r.retryable === true && after === before,
    );
  }

  // (f) releaseLock with correct lock_id succeeds and file is gone
  {
    const lockPath = join(tmpRoot, "f-release-ok.lock");
    const acquired = tryAcquireOnce({ lockPath, workerId: "w1", pid: 555, ttlSeconds: 60, sourceBaseSha: SHA_A, now: NOW, journalPending: false });
    const rel = releaseLock({ lockPath, lockId: acquired.lockRecord.lock_id });
    check("(f) releaseLock with correct lock_id succeeds and deletes file", rel.ok && !existsSync(lockPath));
  }

  // (g) releaseLock with WRONG lock_id fails and file is NOT deleted
  {
    const lockPath = join(tmpRoot, "g-release-wrong.lock");
    const acquired = tryAcquireOnce({ lockPath, workerId: "w1", pid: 666, ttlSeconds: 60, sourceBaseSha: SHA_A, now: NOW, journalPending: false });
    const record = JSON.parse(fsReadFileSync(lockPath, "utf8"));
    record.lock_id = "some-other-lock-id";
    fsWriteFileSync(lockPath, `${JSON.stringify(record, null, 2)}\n`);
    const rel = releaseLock({ lockPath, lockId: acquired.lockRecord.lock_id }); // original id, now stale
    check("(g) releaseLock with wrong lock_id fails and file remains", !rel.ok && existsSync(lockPath));
  }

  // (h) releaseLock on already-absent lock file returns {ok:true, alreadyReleased:true}
  {
    const lockPath = join(tmpRoot, "h-never-existed.lock");
    let threw = false;
    let rel;
    try {
      rel = releaseLock({ lockPath, lockId: "whatever" });
    } catch {
      threw = true;
    }
    check("(h) releaseLock on absent file returns alreadyReleased, no throw", !threw && rel.ok && rel.alreadyReleased === true);
  }

  // (i) REAL concurrency test: multiple actual child processes race for the
  // same lock path. Exactly one must succeed; the rest must fail retryable.
  // This exercises the actual OS-level EEXIST race -- not a fake sequential
  // simulation in this same process.
  {
    const selfPath = fileURLToPath(import.meta.url);
    const childScriptPath = join(tmpRoot, "concurrent-attempt.mjs");
    fsWriteFileSync(
      childScriptPath,
      [
        `import { tryAcquireOnce } from ${JSON.stringify(selfPath)};`,
        `const [lockPath, workerId] = process.argv.slice(2);`,
        `const result = tryAcquireOnce({ lockPath, workerId, pid: process.pid, ttlSeconds: 60, sourceBaseSha: ${JSON.stringify(SHA_A)}, now: new Date().toISOString(), journalPending: false });`,
        `process.stdout.write(JSON.stringify(result));`,
      ].join("\n"),
    );

    const concurrentLockPath = join(tmpRoot, "i-concurrent.lock");
    const N = 12;

    function runChild(workerId) {
      return new Promise((resolvePromise, rejectPromise) => {
        const child = spawn(process.execPath, [childScriptPath, concurrentLockPath, workerId], { stdio: ["ignore", "pipe", "pipe"] });
        let stdout = "";
        let stderr = "";
        child.stdout.on("data", (d) => { stdout += d; });
        child.stderr.on("data", (d) => { stderr += d; });
        child.on("error", rejectPromise);
        child.on("close", () => resolvePromise({ stdout, stderr }));
      });
    }

    // Launch all N children before awaiting any of them, so their attempts
    // genuinely overlap in time rather than running one-at-a-time.
    const childPromises = [];
    for (let i = 0; i < N; i++) {
      childPromises.push(runChild(`cworker-${i}`));
    }
    const results = await Promise.all(childPromises);

    const parsed = results.map((r) => {
      try {
        return JSON.parse(r.stdout);
      } catch {
        return { ok: false, parseError: true, raw: r.stdout, stderr: r.stderr };
      }
    });
    const successes = parsed.filter((p) => p.ok === true);
    const failures = parsed.filter((p) => p.ok === false);
    const allFailuresRetryable = failures.every((p) => p.retryable === true);
    const noParseErrors = parsed.every((p) => !p.parseError);

    let finalLockParses = false;
    let finalLockMatchesWinner = false;
    try {
      const onDisk = JSON.parse(fsReadFileSync(concurrentLockPath, "utf8"));
      finalLockParses = true;
      finalLockMatchesWinner = successes.length === 1 && onDisk.lock_id === successes[0].lockRecord.lock_id;
    } catch {
      finalLockParses = false;
    }

    console.log(`concurrency test: ${N} processes attempted, ${successes.length} succeeded, ${failures.length} failed (all retryable: ${allFailuresRetryable})`);
    check(
      "(i) exactly one concurrent process wins the lock; rest fail retryable; final lock file uncorrupted",
      noParseErrors && successes.length === 1 && failures.length === N - 1 && allFailuresRetryable && finalLockParses && finalLockMatchesWinner,
    );
  }

  console.log(`\n${pass} passed, ${fail} failed`);
  process.exitCode = fail > 0 ? 1 : 0;
}

if (process.argv[1] === fileURLToPath(import.meta.url)) {
  runSelfTest();
}
