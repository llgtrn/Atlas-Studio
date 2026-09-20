// Integration tests for the crash-safe dispatch orchestration layer built in
// B.2 (leases.mjs's runDispatchTransaction/acquireDispatchLock, on top of
// dispatch-lock.mjs + dispatch-journal.mjs), and for B.2.1's journal
// generation-safety fix. These exercise real filesystem state under
// mkdtempSync temp directories ONLY -- never the real
// docs/_machine/system-atlas/v0/ files, and never a real committed
// leases.json/frontier file. dispatch-lock.mjs and dispatch-journal.mjs each
// already carry their own unit self-tests for their piece in isolation; this
// file is specifically about what happens when they are wired together
// through leases.mjs, including actual OS-level process crashes for the
// concurrency case.
import assert from "node:assert/strict";
import { spawn } from "node:child_process";
import { existsSync, mkdtempSync, readFileSync, readdirSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";
import test from "node:test";
import { applyTransaction, hasPendingTransaction, listPendingTransactionIds, prepareTransaction, readJournalById, recoverPendingTransaction, sha256Hex } from "./dispatch-journal.mjs";
import { currentHostId, processStartIdentity, tryAcquireOnce } from "./dispatch-lock.mjs";
import { REPO_ROOT } from "./lib.mjs";
import { acquireDispatchLock, claimLease, loadLeases, loadFrontiersFrom, runDispatchTransaction } from "./leases.mjs";

const DISPATCH_JOURNAL_MJS_PATH = resolve(REPO_ROOT, "tools/system-atlas/dispatch-journal.mjs");
const RECOVERY_MJS_PATH = resolve(REPO_ROOT, "tools/system-atlas/recovery.mjs");
const LEASES_MJS_PATH = resolve(REPO_ROOT, "tools/system-atlas/leases.mjs");

function spawnJson(scriptPath, args) {
  return new Promise((resolvePromise, rejectPromise) => {
    const child = spawn(process.execPath, [scriptPath, ...args], { stdio: ["ignore", "pipe", "pipe"] });
    let stdout = "";
    let stderr = "";
    child.stdout.on("data", (d) => { stdout += d; });
    child.stderr.on("data", (d) => { stderr += d; });
    child.on("error", rejectPromise);
    child.on("close", () => {
      try {
        resolvePromise(JSON.parse(stdout));
      } catch {
        resolvePromise({ parseError: true, stdout, stderr });
      }
    });
  });
}

const SHA = "a".repeat(40);

function makeFixture(overrides = {}) {
  const dir = mkdtempSync(join(tmpdir(), "dispatch-orch-test-"));
  const lockPath = join(dir, "dispatch.lock");
  const journalDir = join(dir, "dispatch-journal");
  const leasesPath = join(dir, "leases.json");
  const recoveryLogPath = join(dir, "recovery-log.json");
  const frontierPath = join(dir, "repair-frontier.json");
  writeFileSync(
    frontierPath,
    JSON.stringify(
      { tasks: [{ task_id: "REPAIR-0001", status: "READY", source_base_sha: SHA, owned_paths: ["crates/x/**"] }, ...(overrides.extraTasks ?? [])] },
      null,
      2,
    ) + "\n",
  );
  writeFileSync(leasesPath, "[]\n");
  return { dir, lockPath, journalDir, leasesPath, recoveryLogPath, frontierPath };
}

function claimBuildOutcome(taskId, workerId) {
  return ({ tasksById, leases, now }) => {
    const r = claimLease({ tasksById, taskId, workerId, accountId: `${workerId}-acct`, sourceBaseSha: SHA, ttlSeconds: 3600, existingLeases: leases, now });
    if (!r.ok) return r;
    return { ok: true, leases: r.leases, taskStatusUpdates: [r.taskStatusUpdate] };
  };
}

// Every leftover-scratch-file check below scans BOTH `dir` (for a
// leases.json/frontier-file write) and `journalDir` (for a journal-record
// write) -- see dispatch-journal.mjs's writeFileDurable() and B.2.1's
// per-transaction-file journal directory layout.
function leftoverTmpFiles(dir, journalDir) {
  const inDir = existsSync(dir) ? readdirSync(dir).filter((f) => f.endsWith(".tmp")) : [];
  const inJournalDir = existsSync(journalDir) ? readdirSync(journalDir).filter((f) => f.endsWith(".tmp")) : [];
  return [...inDir, ...inJournalDir];
}

test("A: a lock left behind by a crashed worker (no journal ever created) is reclaimed once expired, no journal artifact appears", () => {
  const { dir, lockPath, journalDir, leasesPath, recoveryLogPath, frontierPath } = makeFixture();
  try {
    // Simulate: worker acquires the lock, then crashes before doing any
    // journal-worthy work at all (e.g. crashed during pure computation).
    // pid 999999 does not correspond to any real running process (verified
    // ESRCH); host_id is THIS real host, so classifyOwnerLiveness() can
    // machine-verify the "crash" for real (B.2.2) -- this is genuinely
    // EXPIRED_OWNER_DEAD, not just an expired timestamp.
    const crashedLock = {
      lock_id: "lock-crashed-a",
      worker_id: "w-crashed",
      pid: 999999,
      host_id: currentHostId(),
      created_at: "2020-01-01T00:00:00.000Z",
      expires_at: "2020-01-01T00:01:00.000Z", // long expired
      source_base_sha: SHA,
    };
    writeFileSync(lockPath, `${JSON.stringify(crashedLock, null, 2)}\n`);
    assert.equal(hasPendingTransaction(journalDir), false, "no journal should exist for this scenario");

    const outcome = runDispatchTransaction({
      operation: "claim", workerId: "w-new", lockPath, journalDir, leasesPath, frontierPaths: [frontierPath], recoveryLogPath,
      sourceBaseSha: SHA, buildOutcome: claimBuildOutcome("REPAIR-0001", "w-new"),
    });

    assert.equal(outcome.ok, true, outcome.reason);
    assert.equal(hasPendingTransaction(journalDir), false, "no journal artifact should be left behind");
    assert.equal(existsSync(lockPath), false, "lock is released after the transaction completes");
    const frontier = JSON.parse(readFileSync(frontierPath, "utf8"));
    assert.equal(frontier.tasks[0].status, "LEASED");
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
});

test("B: crash after journal PREPARED but before any target file is written -- next worker recovers fully before acquiring", () => {
  // This is also B.2.2's dead-owner-recovery scenario B: an EXPIRED lock
  // whose owner is machine-verifiably DEAD (pid 999999 does not exist) AND
  // a pending PREPARED journal. The correct order is journal-recovery FIRST
  // (safe regardless of owner liveness -- it only ever completes an
  // already-decided write), THEN lock reclaim (gated on the now-confirmed
  // dead owner) -- acquireDispatchLock()'s loop enforces exactly that order.
  const { dir, lockPath, journalDir, leasesPath, recoveryLogPath, frontierPath } = makeFixture();
  try {
    // A crashed worker's lock, already expired by the time we get here.
    const crashedLock = {
      lock_id: "lock-crashed-b", worker_id: "w-crashed", pid: 999999, host_id: currentHostId(),
      created_at: "2020-01-01T00:00:00.000Z", expires_at: "2020-01-01T00:01:00.000Z", source_base_sha: SHA,
    };
    writeFileSync(lockPath, `${JSON.stringify(crashedLock, null, 2)}\n`);

    const beforeLeases = readFileSync(leasesPath, "utf8");
    const beforeFrontier = JSON.parse(readFileSync(frontierPath, "utf8"));
    beforeFrontier.tasks[0].status = "LEASED";
    const afterFrontierContent = `${JSON.stringify(beforeFrontier, null, 2)}\n`;
    const afterLeasesContent = `[${JSON.stringify({ lease_id: "lease-fixture", task_id: "REPAIR-0001", status: "ACTIVE" })}]\n`;

    // prepareTransaction durably writes the journal but touches NO target
    // file -- this IS "crashed right after prepare, before any apply".
    prepareTransaction({
      journalDir, transactionId: "crash-b", operation: "claim", sourceBaseSha: SHA, canonicalRefSha: null,
      intendedFileUpdates: [
        { path: frontierPath, content: afterFrontierContent },
        { path: leasesPath, content: afterLeasesContent },
      ],
      now: "2020-01-01T00:00:30.000Z",
    });
    assert.equal(hasPendingTransaction(journalDir), true);
    assert.equal(readFileSync(frontierPath, "utf8"), JSON.stringify({ tasks: [{ task_id: "REPAIR-0001", status: "READY", source_base_sha: SHA, owned_paths: ["crates/x/**"] }] }, null, 2) + "\n", "target file must be untouched while only PREPARED");
    assert.equal(readFileSync(leasesPath, "utf8"), beforeLeases);

    const lockRecord = acquireDispatchLock({ lockPath, journalDir, workerId: "w-new", sourceBaseSha: SHA });

    assert.equal(hasPendingTransaction(journalDir), false, "pending journal must be fully recovered before a fresh lock is granted");
    assert.equal(readFileSync(frontierPath, "utf8"), afterFrontierContent, "recovery must apply the journaled content, not discard it");
    assert.equal(readFileSync(leasesPath, "utf8"), afterLeasesContent);
    assert.ok(lockRecord.lock_id !== "lock-crashed-b");
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
});

test("B2.2-B2: an ALIVE owner's pending journal still recovers (safe regardless of liveness), but the lock itself remains refused afterward -- journal recovery never bypasses the owner-liveness gate", () => {
  const { dir, lockPath, journalDir, leasesPath, recoveryLogPath, frontierPath } = makeFixture();
  try {
    // Same real, genuinely-alive pid AND real process_start_identity as the
    // test process itself (no explicit pid override below either) -- this
    // worker is NOT dead, and is machine-verifiable as such (OWNER_ALIVE,
    // not merely OWNER_UNKNOWN).
    const aliveLock = {
      lock_id: "lock-alive-with-journal", worker_id: "w-alive", pid: process.pid, host_id: currentHostId(),
      process_start_identity: processStartIdentity(process.pid),
      created_at: "2020-01-01T00:00:00.000Z", expires_at: "2020-01-01T00:01:00.000Z", source_base_sha: SHA,
    };
    writeFileSync(lockPath, `${JSON.stringify(aliveLock, null, 2)}\n`);

    const afterFrontierContent = `${JSON.stringify({ tasks: [{ task_id: "REPAIR-0001", status: "LEASED", source_base_sha: SHA, owned_paths: ["crates/x/**"] }] }, null, 2)}\n`;
    prepareTransaction({
      journalDir, transactionId: "alive-owner-journal", operation: "claim", sourceBaseSha: SHA, canonicalRefSha: null,
      intendedFileUpdates: [{ path: frontierPath, content: afterFrontierContent }],
      now: "2020-01-01T00:00:30.000Z",
    });
    assert.equal(hasPendingTransaction(journalDir), true);

    let threw = null;
    try {
      acquireDispatchLock({ lockPath, journalDir, workerId: "w-new", maxAttempts: 2, sleepFn: () => {}, sourceBaseSha: SHA });
    } catch (err) {
      threw = err.message;
    }

    // The journal MUST still have been rolled forward -- that part is
    // unconditionally safe (idempotent replay of an already-decided write)
    // and must not be held hostage by the separate liveness question.
    assert.equal(hasPendingTransaction(journalDir), false, "the pending journal must still be recovered even though its owner is alive");
    assert.equal(readFileSync(frontierPath, "utf8"), afterFrontierContent, "the journaled content must have landed");
    // But the LOCK itself must still be refused: the alive owner might
    // resume and acquire again at any moment; nothing about recovering an
    // unrelated pending journal proves it is safe to take its lock.
    assert.ok(threw, "acquiring the lock must still fail -- the recorded owner is alive");
    assert.match(threw, /still contended|EXPIRED_OWNER|alive/i);
    const onDisk = JSON.parse(readFileSync(lockPath, "utf8"));
    assert.equal(onDisk.lock_id, "lock-alive-with-journal", "the alive owner's lock must be completely untouched");
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
});

test("C: crash after SOME target files were written but before the journal was cleared -- recovery finishes only the remainder, idempotently", () => {
  const { dir, lockPath, journalDir, leasesPath, recoveryLogPath, frontierPath } = makeFixture();
  try {
    const crashedLock = {
      lock_id: "lock-crashed-c", worker_id: "w-crashed", pid: 999999, host_id: currentHostId(),
      created_at: "2020-01-01T00:00:00.000Z", expires_at: "2020-01-01T00:01:00.000Z", source_base_sha: SHA,
    };
    writeFileSync(lockPath, `${JSON.stringify(crashedLock, null, 2)}\n`);

    const afterFrontierContent = `${JSON.stringify({ tasks: [{ task_id: "REPAIR-0001", status: "LEASED", source_base_sha: SHA, owned_paths: ["crates/x/**"] }] }, null, 2)}\n`;
    const afterLeasesContent = `[${JSON.stringify({ lease_id: "lease-fixture", task_id: "REPAIR-0001", status: "ACTIVE" })}]\n`;

    prepareTransaction({
      journalDir, transactionId: "crash-c", operation: "claim", sourceBaseSha: SHA, canonicalRefSha: null,
      intendedFileUpdates: [
        { path: frontierPath, content: afterFrontierContent },
        { path: leasesPath, content: afterLeasesContent },
      ],
      now: "2020-01-01T00:00:30.000Z",
    });

    // Manually apply ONLY the frontier file, as if the crash happened right
    // after that rename but before leases.json's write.
    writeFileSync(frontierPath, afterFrontierContent);
    // leasesPath deliberately left at its pre-transaction "[]\n" content.

    const lockRecord = acquireDispatchLock({ lockPath, journalDir, workerId: "w-new", sourceBaseSha: SHA });

    assert.equal(hasPendingTransaction(journalDir), false);
    assert.equal(readFileSync(frontierPath, "utf8"), afterFrontierContent, "already-applied file must be left exactly as-is (idempotent skip)");
    assert.equal(readFileSync(leasesPath, "utf8"), afterLeasesContent, "the remaining un-applied file must now be completed");
    assert.ok(lockRecord.lock_id !== "lock-crashed-c");
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
});

test("D: crash after the journal is committed but before the lock is released -- lock stays contended until it naturally expires, then reclaims WITHOUT re-running recovery", () => {
  const { dir, lockPath, journalDir, leasesPath, recoveryLogPath, frontierPath } = makeFixture();
  try {
    const outcome = runDispatchTransaction({
      operation: "claim", workerId: "w-crashing", lockPath, journalDir, leasesPath, frontierPaths: [frontierPath], recoveryLogPath,
      sourceBaseSha: SHA, buildOutcome: claimBuildOutcome("REPAIR-0001", "w-crashing"),
    });
    assert.equal(outcome.ok, true);
    // runDispatchTransaction's `finally` always releases the lock -- simulate
    // a crash strictly between commit and release by re-acquiring it
    // out-of-band and never releasing THIS one, so the on-disk state matches
    // "committed transaction, still-held lock".
    const reacquired = tryAcquireOnce({ lockPath, workerId: "w-crashing", pid: 12345, ttlSeconds: 60, sourceBaseSha: SHA, now: new Date().toISOString(), journalPending: false });
    assert.equal(reacquired.ok, true, "lock was released by the prior transaction, as expected; re-acquiring simulates the still-held state");
    assert.equal(hasPendingTransaction(journalDir), false, "journal is already committed in this scenario");

    // A second worker must NOT be able to acquire immediately (still ACTIVE_OWNED).
    assert.throws(
      () => acquireDispatchLock({ lockPath, journalDir, workerId: "w-second", sourceBaseSha: SHA, maxAttempts: 2, sleepFn: () => {} }),
      /could not acquire the dispatch lock/,
    );

    // Now simulate the crashed lock actually expiring.
    const stale = JSON.parse(readFileSync(lockPath, "utf8"));
    stale.expires_at = "2020-01-01T00:00:00.000Z";
    writeFileSync(lockPath, `${JSON.stringify(stale, null, 2)}\n`);

    const lockRecord = acquireDispatchLock({ lockPath, journalDir, workerId: "w-second", sourceBaseSha: SHA });
    assert.ok(lockRecord.lock_id !== stale.lock_id, "second worker reclaims a genuinely stale lock");
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
});

test("E: recovering the same pending journal twice in a row is a safe no-op the second time", () => {
  const { dir, lockPath, journalDir, leasesPath, recoveryLogPath, frontierPath } = makeFixture();
  try {
    const crashedLock = {
      lock_id: "lock-crashed-e", worker_id: "w-crashed", pid: 999999, host_id: currentHostId(),
      created_at: "2020-01-01T00:00:00.000Z", expires_at: "2020-01-01T00:01:00.000Z", source_base_sha: SHA,
    };
    writeFileSync(lockPath, `${JSON.stringify(crashedLock, null, 2)}\n`);
    const afterFrontierContent = `${JSON.stringify({ tasks: [{ task_id: "REPAIR-0001", status: "LEASED", source_base_sha: SHA, owned_paths: ["crates/x/**"] }] }, null, 2)}\n`;
    prepareTransaction({
      journalDir, transactionId: "crash-e", operation: "claim", sourceBaseSha: SHA, canonicalRefSha: null,
      intendedFileUpdates: [{ path: frontierPath, content: afterFrontierContent }],
      now: "2020-01-01T00:00:30.000Z",
    });

    const first = acquireDispatchLock({ lockPath, journalDir, workerId: "w-first", sourceBaseSha: SHA });
    assert.equal(readFileSync(frontierPath, "utf8"), afterFrontierContent);

    // Re-run recovery machinery directly a second time: nothing is pending
    // any more, so this must be a pure no-op, not an error and not a
    // re-write of already-correct content.
    assert.equal(hasPendingTransaction(journalDir), false);
    const beforeSecond = readFileSync(frontierPath, "utf8");
    // Calling acquireDispatchLock again (different worker) while the first
    // still holds it should simply contend (proves recovery isn't
    // re-triggered spuriously when nothing is pending).
    assert.throws(
      () => acquireDispatchLock({ lockPath, journalDir, workerId: "w-second", sourceBaseSha: SHA, maxAttempts: 2, sleepFn: () => {} }),
    );
    assert.equal(readFileSync(frontierPath, "utf8"), beforeSecond, "no spurious re-write happened");
    assert.equal(first.lock_id !== undefined, true);
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
});

test("concurrent claim: N real child processes racing the SAME task_id -- exactly one wins, the rest fail cleanly, no corruption", async () => {
  const { dir, lockPath, journalDir, leasesPath, recoveryLogPath, frontierPath } = makeFixture();
  try {
    const childScriptPath = join(dir, "concurrent-claim-same.mjs");
    writeFileSync(
      childScriptPath,
      [
        `import { claimLease, runDispatchTransaction } from ${JSON.stringify(LEASES_MJS_PATH)};`,
        `const [lockPath, journalDir, leasesPath, frontierPath, recoveryLogPath, taskId, workerId] = process.argv.slice(2);`,
        `const outcome = runDispatchTransaction({`,
        `  operation: "claim", workerId, lockPath, journalDir, leasesPath, frontierPaths: [frontierPath], recoveryLogPath,`,
        `  sourceBaseSha: ${JSON.stringify(SHA)},`,
        `  buildOutcome: ({ tasksById, leases, now }) => {`,
        `    const r = claimLease({ tasksById, taskId, workerId, accountId: workerId + "-acct", sourceBaseSha: ${JSON.stringify(SHA)}, ttlSeconds: 3600, existingLeases: leases, now });`,
        `    if (!r.ok) return r;`,
        `    return { ok: true, leases: r.leases, taskStatusUpdates: [r.taskStatusUpdate] };`,
        `  },`,
        `});`,
        `process.stdout.write(JSON.stringify(outcome));`,
      ].join("\n"),
    );

    function runChild(workerId) {
      return new Promise((resolvePromise, rejectPromise) => {
        const child = spawn(process.execPath, [childScriptPath, lockPath, journalDir, leasesPath, frontierPath, recoveryLogPath, "REPAIR-0001", workerId], { stdio: ["ignore", "pipe", "pipe"] });
        let stdout = "";
        let stderr = "";
        child.stdout.on("data", (d) => { stdout += d; });
        child.stderr.on("data", (d) => { stderr += d; });
        child.on("error", rejectPromise);
        child.on("close", () => resolvePromise({ stdout, stderr }));
      });
    }

    const N = 8;
    const promises = [];
    for (let i = 0; i < N; i++) promises.push(runChild(`cworker-${i}`));
    const results = await Promise.all(promises);

    const parsed = results.map((r) => {
      try {
        return JSON.parse(r.stdout);
      } catch {
        return { ok: false, parseError: true, raw: r.stdout, stderr: r.stderr };
      }
    });
    const successes = parsed.filter((p) => p.ok === true);
    const noParseErrors = parsed.every((p) => !p.parseError);

    assert.ok(noParseErrors, `every child must produce valid JSON: ${JSON.stringify(parsed)}`);
    assert.equal(successes.length, 1, `exactly one of ${N} concurrent claims on the same task must win`);
    assert.equal(existsSync(lockPath), false, "lock must be fully released after all children finish");
    assert.equal(hasPendingTransaction(journalDir), false, "no journal artifact left behind");

    const finalLeases = loadLeases(leasesPath);
    const activeLeases = finalLeases.filter((l) => l.status === "ACTIVE");
    assert.equal(activeLeases.length, 1, "exactly one ACTIVE lease must exist for the contended task");
    const { tasksById } = loadFrontiersFrom([frontierPath]);
    assert.equal(tasksById.get("REPAIR-0001").status, "LEASED");
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
});

test("concurrent claim: N real child processes claiming DIFFERENT non-overlapping tasks all succeed, none corrupts the shared leases.json", async () => {
  const N = 6;
  const extraTasks = Array.from({ length: N - 1 }, (_, i) => ({
    task_id: `REPAIR-000${i + 2}`, status: "READY", source_base_sha: SHA, owned_paths: [`crates/y${i}/**`],
  }));
  const { dir, lockPath, journalDir, leasesPath, recoveryLogPath, frontierPath } = makeFixture({ extraTasks });
  try {
    const childScriptPath = join(dir, "concurrent-claim-distinct.mjs");
    writeFileSync(
      childScriptPath,
      [
        `import { claimLease, runDispatchTransaction } from ${JSON.stringify(LEASES_MJS_PATH)};`,
        `const [lockPath, journalDir, leasesPath, frontierPath, recoveryLogPath, taskId, workerId] = process.argv.slice(2);`,
        `const outcome = runDispatchTransaction({`,
        `  operation: "claim", workerId, lockPath, journalDir, leasesPath, frontierPaths: [frontierPath], recoveryLogPath,`,
        `  sourceBaseSha: ${JSON.stringify(SHA)},`,
        `  buildOutcome: ({ tasksById, leases, now }) => {`,
        `    const r = claimLease({ tasksById, taskId, workerId, accountId: workerId + "-acct", sourceBaseSha: ${JSON.stringify(SHA)}, ttlSeconds: 3600, existingLeases: leases, now });`,
        `    if (!r.ok) return r;`,
        `    return { ok: true, leases: r.leases, taskStatusUpdates: [r.taskStatusUpdate] };`,
        `  },`,
        `});`,
        `process.stdout.write(JSON.stringify(outcome));`,
      ].join("\n"),
    );

    function runChild(taskId, workerId) {
      return new Promise((resolvePromise, rejectPromise) => {
        const child = spawn(process.execPath, [childScriptPath, lockPath, journalDir, leasesPath, frontierPath, recoveryLogPath, taskId, workerId], { stdio: ["ignore", "pipe", "pipe"] });
        let stdout = "";
        let stderr = "";
        child.stdout.on("data", (d) => { stdout += d; });
        child.stderr.on("data", (d) => { stderr += d; });
        child.on("error", rejectPromise);
        child.on("close", () => resolvePromise({ stdout, stderr }));
      });
    }

    const taskIds = ["REPAIR-0001", ...extraTasks.map((t) => t.task_id)];
    const promises = taskIds.map((taskId, i) => runChild(taskId, `dworker-${i}`));
    const results = await Promise.all(promises);
    const parsed = results.map((r) => {
      try {
        return JSON.parse(r.stdout);
      } catch {
        return { ok: false, parseError: true, raw: r.stdout, stderr: r.stderr };
      }
    });

    assert.ok(parsed.every((p) => !p.parseError), `every child must produce valid JSON: ${JSON.stringify(parsed)}`);
    assert.ok(parsed.every((p) => p.ok === true), `every non-overlapping claim should succeed: ${JSON.stringify(parsed)}`);

    const finalLeases = loadLeases(leasesPath);
    assert.equal(finalLeases.length, taskIds.length, "every claim must have produced exactly one lease, none lost to a lost update");
    assert.equal(new Set(finalLeases.map((l) => l.lease_id)).size, taskIds.length, "lease_ids must be unique -- no lease clobbered another's slot");

    const { tasksById } = loadFrontiersFrom([frontierPath]);
    for (const taskId of taskIds) {
      assert.equal(tasksById.get(taskId).status, "LEASED", `${taskId} must be LEASED`);
    }
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
});

// --- BX6 adversarial regression tests -------------------------------------

test("BX6-1 regression: several workers recovering the SAME pending journal at the same time never crash each other (unique durable-write scratch files)", async () => {
  // acquireDispatchLock() deliberately rolls a pending journal forward WITHOUT
  // holding the dispatch lock, so a stuck journal can never deadlock the
  // system -- which means genuinely concurrent recovery of one journal is a
  // designed-for state, not an exotic one. It was NOT safe: writeFileDurable()
  // used a shared `<path>.tmp` scratch name, so two appliers raced on one
  // scratch file and whichever renamed first made the other's renameSync()
  // fail ENOENT, escaping as an unhandled exception out of
  // acquireDispatchLock(). Measured before the fix: 3 of 4 concurrent
  // recoverers crashed, in 10 of 10 rounds, including for a realistic
  // two-file (leases.json + one frontier) claim journal. This is also the
  // B.2.1 brief's "GEN-A: two recoverers / same J1 / no new tx" case.
  const { dir, journalDir, leasesPath, frontierPath } = makeFixture();
  try {
    const afterFrontierContent = `${JSON.stringify({ tasks: [{ task_id: "REPAIR-0001", status: "LEASED", source_base_sha: SHA, owned_paths: ["crates/x/**"] }] }, null, 2)}\n`;
    const afterLeasesContent = `[${JSON.stringify({ lease_id: "lease-fixture", task_id: "REPAIR-0001", status: "ACTIVE" })}]\n`;
    // Extra padding entries widen the apply window enough to make the race
    // deterministic rather than probabilistic; the two real files above are
    // the ones whose final content is asserted.
    const padding = Array.from({ length: 40 }, (_, i) => ({ path: join(dir, `pad-${i}.json`), content: `{"i":${i},"filler":"${"x".repeat(2048)}"}\n` }));
    prepareTransaction({
      journalDir, transactionId: "bx6-concurrent-recovery", operation: "claim", sourceBaseSha: SHA, canonicalRefSha: null,
      intendedFileUpdates: [...padding, { path: frontierPath, content: afterFrontierContent }, { path: leasesPath, content: afterLeasesContent }],
      now: "2020-01-01T00:00:30.000Z",
    });

    const childScriptPath = join(dir, "recover-child.mjs");
    writeFileSync(
      childScriptPath,
      [
        `import { recoverPendingTransaction } from ${JSON.stringify(DISPATCH_JOURNAL_MJS_PATH)};`,
        `const [journalDir] = process.argv.slice(2);`,
        `try { const r = recoverPendingTransaction(journalDir); process.stdout.write(JSON.stringify({ ok: true, recovered: r.recovered })); }`,
        `catch (err) { process.stdout.write(JSON.stringify({ ok: false, threw: String(err && err.message) })); }`,
      ].join("\n"),
    );

    const parsed = await Promise.all(Array.from({ length: 4 }, () => spawnJson(childScriptPath, [journalDir])));

    assert.ok(parsed.every((p) => !p.parseError), `every recoverer must produce valid JSON: ${JSON.stringify(parsed)}`);
    const threw = parsed.filter((p) => p.threw);
    assert.equal(threw.length, 0, `no concurrent recoverer may crash; got: ${JSON.stringify(threw)}`);
    assert.equal(hasPendingTransaction(journalDir), false, "the journal must be cleared exactly once, by whoever finished");
    assert.equal(readFileSync(frontierPath, "utf8"), afterFrontierContent, "the journaled frontier content must have landed intact");
    assert.equal(readFileSync(leasesPath, "utf8"), afterLeasesContent, "the journaled leases content must have landed intact");
    assert.deepEqual(leftoverTmpFiles(dir, journalDir), [], "no durable-write scratch file may be left behind");
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
});

test("BX6-2 / B.2.2 regression: a transaction whose own lock lapsed mid-buildOutcome, while the process is still genuinely alive, can never be reclaimed by a second worker in the first place", () => {
  // ORIGINAL (B.2) finding: a lock has a TTL; buildOutcome() is
  // caller-supplied work of unbounded duration. If the lock's timestamp
  // lapses while a worker computes, a second worker used to be able to
  // reclaim it on TTL alone and commit its own transaction; the first worker
  // then journaled and applied state it had read BEFORE losing the lock,
  // silently erasing the second worker's committed lease. B.2's own fix
  // (holdsLock() re-checked immediately before commit) closed the "the first
  // worker journals stale state" half of that.
  //
  // B.2.2 finding (architect review): that still left a real, deterministically
  // reproducible interleaving -- see leases.mjs's runDispatchTransaction() doc
  // comment and this PR's own reproduction -- where the SECOND worker's
  // reclaim itself was the problem: TTL expiry alone authorized it, even
  // though the first worker's real OS process was never actually gone and
  // could resume and commit at any moment. B.2.2 closes this at the SOURCE:
  // tryAcquireOnce() now refuses to reclaim an expired lock unless the
  // recorded owner is machine-verified dead (dispatch-lock.mjs's
  // classifyOwnerLiveness()). This test proves that end-to-end: both
  // "workers" in this process share the same real, genuinely-alive pid
  // (process.pid, since neither passes an explicit pid override), so once
  // W1's lock timestamp is forced into the past, W2's attempt to acquire
  // must fail outright -- it can never even reach the point where it would
  // commit a conflicting transaction. W1, still the legitimate holder from
  // the OS's perspective (nobody ever reclaimed its lock_id), then finishes
  // normally.
  const { dir, lockPath, journalDir, leasesPath, recoveryLogPath, frontierPath } = makeFixture({
    extraTasks: [{ task_id: "REPAIR-0002", status: "READY", source_base_sha: SHA, owned_paths: ["crates/y/**"] }],
  });
  try {
    const common = { lockPath, journalDir, leasesPath, frontierPaths: [frontierPath], recoveryLogPath, sourceBaseSha: SHA };
    let secondThrew = null;
    const first = runDispatchTransaction({
      ...common,
      operation: "claim",
      workerId: "w-slow",
      buildOutcome: ({ tasksById, leases, now }) => {
        // Simulate our own lock's TTL lapsing during this (slow)
        // computation -- but the process itself (pid, host, start identity)
        // is completely real and unchanged.
        const ourLock = JSON.parse(readFileSync(lockPath, "utf8"));
        ourLock.expires_at = "2020-01-01T00:00:00.000Z";
        writeFileSync(lockPath, `${JSON.stringify(ourLock, null, 2)}\n`);
        // A second worker attempts to reclaim and commit. Under B.2.2 this
        // must fail closed -- our real process is still alive.
        try {
          runDispatchTransaction({
            ...common,
            operation: "claim",
            workerId: "w-fast",
            maxLockAttempts: 2,
            sleepFn: () => {},
            buildOutcome: claimBuildOutcome("REPAIR-0002", "w-fast"),
          });
        } catch (err) {
          secondThrew = err.message;
        }
        // Finish with our own (still legitimately held) view of leases/tasks.
        const r = claimLease({ tasksById, taskId: "REPAIR-0001", workerId: "w-slow", accountId: "w-slow-acct", sourceBaseSha: SHA, ttlSeconds: 3600, existingLeases: leases, now });
        if (!r.ok) return r;
        return { ok: true, leases: r.leases, taskStatusUpdates: [r.taskStatusUpdate] };
      },
    });

    assert.ok(secondThrew, "the second worker must fail to even ACQUIRE the lock -- its recorded owner (this real process) is alive");
    assert.match(secondThrew, /still contended|EXPIRED_OWNER|alive/i);
    assert.equal(first.ok, true, `W1, never actually dispossessed, must still complete normally: ${first.reason}`);

    // Final state: ONLY W1's claim exists. REPAIR-0002 was never touched --
    // there is no S2 to have been reverted from, because W2 never got the
    // chance to commit it in the first place.
    const finalLeases = loadLeases(leasesPath);
    assert.equal(finalLeases.length, 1, "only W1's lease may exist -- W2 never committed anything");
    assert.equal(finalLeases[0].task_id, "REPAIR-0001");
    assert.equal(finalLeases[0].worker_id, "w-slow");
    const { tasksById } = loadFrontiersFrom([frontierPath]);
    assert.equal(tasksById.get("REPAIR-0001").status, "LEASED");
    assert.equal(tasksById.get("REPAIR-0002").status, "READY", "REPAIR-0002 was never claimed by anyone");
    assert.equal(hasPendingTransaction(journalDir), false);
    assert.equal(existsSync(lockPath), false, "W1's lock is released normally at the end of its own transaction");
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
});

test("BX6-3: concurrent resolve-recovery on one RECOVERY_REQUIRED task -- exactly one wins, exactly one new lease, exactly one audit entry", async () => {
  const dir = mkdtempSync(join(tmpdir(), "dispatch-orch-test-"));
  try {
    const lockPath = join(dir, "dispatch.lock");
    const journalDir = join(dir, "dispatch-journal");
    const leasesPath = join(dir, "leases.json");
    const recoveryLogPath = join(dir, "recovery-log.json");
    const frontierPath = join(dir, "repair-frontier.json");
    writeFileSync(frontierPath, `${JSON.stringify({ tasks: [{ task_id: "REPAIR-0001", action: "REPAIR", status: "RECOVERY_REQUIRED", source_base_sha: SHA, owned_paths: ["crates/x/**"] }] }, null, 2)}\n`);
    writeFileSync(leasesPath, `${JSON.stringify([{ lease_id: "lease-old", task_id: "REPAIR-0001", worker_id: "w-old", account_id: "a", claimed_at: "2020-01-01T00:00:00.000Z", expires_at: "2020-01-01T01:00:00.000Z", source_base_sha: SHA, owned_paths: ["crates/x/**"], status: "EXPIRED" }], null, 2)}\n`);

    const childScriptPath = join(dir, "resolve-child.mjs");
    writeFileSync(
      childScriptPath,
      [
        `import { runDispatchTransaction } from ${JSON.stringify(LEASES_MJS_PATH)};`,
        `import { resolveRecovery } from ${JSON.stringify(RECOVERY_MJS_PATH)};`,
        `const [lockPath, journalDir, leasesPath, frontierPath, recoveryLogPath, workerId] = process.argv.slice(2);`,
        `const o = runDispatchTransaction({ operation: "resolve-recovery", workerId, lockPath, journalDir, leasesPath, frontierPaths: [frontierPath], recoveryLogPath, sourceBaseSha: ${JSON.stringify(SHA)},`,
        `  buildOutcome: ({ tasksById, leases, now }) => {`,
        `    const r = resolveRecovery({ tasksById, leases, taskId: "REPAIR-0001", resolution: "RESUME_SAME_WORK", evidence: "audited by fixture", resolvedBy: workerId, sourceBaseSha: ${JSON.stringify(SHA)}, workerId, accountId: workerId + "-acct", ttlSeconds: 3600, now });`,
        `    if (!r.ok) return r;`,
        `    return { ok: true, leases: [...leases, r.newLease], taskStatusUpdates: [r.taskStatusUpdate], recoveryLogAppend: r.recoveryRecord };`,
        `  } });`,
        `process.stdout.write(JSON.stringify(o));`,
      ].join("\n"),
    );

    const N = 6;
    const parsed = await Promise.all(
      Array.from({ length: N }, (_, i) => spawnJson(childScriptPath, [lockPath, journalDir, leasesPath, frontierPath, recoveryLogPath, `rworker-${i}`])),
    );

    assert.ok(parsed.every((p) => !p.parseError), `every child must produce valid JSON: ${JSON.stringify(parsed)}`);
    assert.equal(parsed.filter((p) => p.ok === true).length, 1, "exactly one concurrent resolve-recovery may win");
    for (const loser of parsed.filter((p) => p.ok === false)) {
      assert.match(loser.reason, /expected RECOVERY_REQUIRED|could not acquire the dispatch lock/);
    }
    const finalLeases = loadLeases(leasesPath);
    assert.equal(finalLeases.filter((l) => l.status === "ACTIVE").length, 1, "exactly one ACTIVE lease may exist after recovery");
    assert.equal(JSON.parse(readFileSync(recoveryLogPath, "utf8")).length, 1, "exactly one recovery-log entry may be appended");
    const { tasksById } = loadFrontiersFrom([frontierPath]);
    assert.equal(tasksById.get("REPAIR-0001").status, "IN_PROGRESS");
    assert.equal(existsSync(lockPath), false);
    assert.equal(hasPendingTransaction(journalDir), false);
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
});

test("BX6-4 regression: a torn/unparseable lock file makes a worker retry, never crash with a raw SyntaxError", () => {
  // A lock is published in two observable steps unless it is published
  // atomically: the file becomes visible, then its content lands. Any
  // concurrent reader in that window used to hit JSON.parse("") inside
  // readLock(), and the SyntaxError escaped uncaught out of tryAcquireOnce()
  // -> acquireDispatchLock() -> runDispatchTransaction(). Measured before the
  // fix: ~1 in 5 runs of this file's own contended-claim tests had a child die
  // with `SyntaxError: Unexpected end of JSON input` and no result at all.
  const { dir, lockPath, journalDir, leasesPath, recoveryLogPath, frontierPath } = makeFixture();
  try {
    for (const torn of ["", '{\n  "lock_id": "lock-abc",\n  "worker']) {
      writeFileSync(lockPath, torn);
      const attempt = tryAcquireOnce({ lockPath, workerId: "w", pid: 1, ttlSeconds: 60, sourceBaseSha: SHA, now: new Date().toISOString(), journalPending: false });
      assert.equal(attempt.ok, false);
      assert.equal(attempt.retryable, true, "a torn lock read is retryable contention, not a fatal error");
      assert.match(attempt.reason, /does not parse/);
      assert.equal(readFileSync(lockPath, "utf8"), torn, "a lock we cannot read must never be deleted or overwritten");

      // And it must surface through the orchestration layer as a clean,
      // diagnosable acquisition failure rather than a SyntaxError.
      assert.throws(
        () => runDispatchTransaction({
          operation: "claim", workerId: "w2", lockPath, journalDir, leasesPath, frontierPaths: [frontierPath], recoveryLogPath,
          sourceBaseSha: SHA, maxLockAttempts: 2, sleepFn: () => {}, buildOutcome: claimBuildOutcome("REPAIR-0001", "w2"),
        }),
        (err) => err instanceof Error && !(err instanceof SyntaxError) && /could not acquire the dispatch lock/.test(err.message),
      );
    }

    // Once the torn file is gone, acquisition works normally and leaves no
    // scratch file behind.
    rmSync(lockPath, { force: true });
    const outcome = runDispatchTransaction({
      operation: "claim", workerId: "w3", lockPath, journalDir, leasesPath, frontierPaths: [frontierPath], recoveryLogPath,
      sourceBaseSha: SHA, buildOutcome: claimBuildOutcome("REPAIR-0001", "w3"),
    });
    assert.equal(outcome.ok, true, outcome.reason);
    assert.deepEqual(leftoverTmpFiles(dir, journalDir), [], "lock publishing must leave no scratch file behind");
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
});

// --- B.2.1 journal generation-safety test matrix (architect finding) ------
//
// Architect-reported ABA race: the pre-B.2.1 design used ONE shared
// well-known journal path. A recoverer that read transaction J1's record
// into memory, then paused (a slow process, GC, or plain OS scheduling
// preemption -- deterministically reproduced here via direct API calls
// rather than probabilistic sleeps, exactly as required), could resume
// after a newer transaction J2 had been prepared and/or committed at that
// SAME path, and its unconditional apply-then-delete would (a) delete J2's
// still-PREPARED journal, making it unrecoverable, or (b) overwrite J2's
// already-committed target-file state with J1's stale bytes. Both are
// reproduced and documented in this PR's history (a throwaway script, not
// committed, demonstrated both before this fix landed). The fix is two
// complementary, independently-tested changes:
//   (1) journal identity is now per-transaction and immutable
//       (journalDir/txn-<transaction_id>.json) -- applyTransaction() derives
//       the file it reads/deletes from journalRecord.transaction_id itself,
//       never from journalDir's current contents, so a stale recoverer
//       structurally cannot discover or touch a different transaction's file;
//   (2) every target-file write is classified against ITS OWN
//       before_hash/after_hash before being applied -- current==after_hash
//       is a no-op, current==before_hash is safe to apply, anything else is
//       DIVERGED and is NEVER overwritten (RECOVERY_CONFLICT).
// dispatch-journal.mjs's own self-test cases (f) and (g) prove this at the
// pure-module level; the tests below prove it holds through the full
// orchestration stack (acquireDispatchLock/runDispatchTransaction), per the
// B.2.1 brief's required GEN-A..GEN-H matrix (GEN-A is BX6-1 above; GEN-H is
// the two "concurrent claim" tests above, both already exercising real
// concurrent dispatch following normal recovery).

test("GEN-B: R1+R2 load J1; R1 commits J1; J2 is PREPARED (not yet applied); stale R2 resumes with J1 -- must NOT delete J2, must NOT revert any J2-era state", () => {
  const { dir, journalDir, frontierPath } = makeFixture();
  try {
    const j1 = prepareTransaction({
      journalDir, transactionId: "gen-b-j1", operation: "claim", sourceBaseSha: SHA, canonicalRefSha: null,
      intendedFileUpdates: [{ path: frontierPath, content: '{"state":"J1"}\n' }],
      now: "2020-01-01T00:00:00.000Z",
    });
    // R2 "loads" J1 into memory right here (holding the `j1` record is the
    // simulation of that read), then pauses -- it does not call
    // applyTransaction yet.

    // J1 is committed (by R1, or by R2 itself before it paused -- either way,
    // it's done, and R2's in-memory copy is now stale relative to disk).
    applyTransaction(j1, journalDir);
    assert.equal(readFileSync(frontierPath, "utf8"), '{"state":"J1"}\n');

    // A fresh worker prepares J2 at the same journalDir. It has NOT applied
    // it yet -- this is the "J2 PREPARED" half of the interleaving.
    const j2 = prepareTransaction({
      journalDir, transactionId: "gen-b-j2", operation: "begin", sourceBaseSha: SHA, canonicalRefSha: null,
      intendedFileUpdates: [{ path: frontierPath, content: '{"state":"J2"}\n' }],
      now: "2020-01-01T00:00:01.000Z",
    });
    assert.ok(listPendingTransactionIds(journalDir).includes("gen-b-j2"));

    // R2 now resumes and applies its STALE j1 record.
    const staleResult = applyTransaction(j1, journalDir);

    assert.equal(staleResult.status, "COMMITTED", "J1's own file is already gone -- an ENOENT-caught no-op delete IS success for J1's own generation");
    assert.ok(listPendingTransactionIds(journalDir).includes("gen-b-j2"), "J2's PREPARED journal file must still exist -- untouched by the stale J1 recoverer");
    assert.deepEqual(readJournalById(journalDir, "gen-b-j2"), j2, "J2's record content must be byte-identical to what was prepared -- never disturbed");
    assert.equal(readFileSync(frontierPath, "utf8"), '{"state":"J1"}\n', "target file must still read J1's content -- J2 has not applied yet, and the stale J1 recoverer correctly no-ops (current already matches its own after_hash)");
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
});

test("GEN-C: R1+R2 load J1; R1 commits J1; J2 FULLY COMMITS; stale R2 resumes with J1 -- final state remains exactly J2, stale R2 fails closed (never reverts)", () => {
  const { dir, journalDir, frontierPath } = makeFixture();
  try {
    const j1 = prepareTransaction({
      journalDir, transactionId: "gen-c-j1", operation: "claim", sourceBaseSha: SHA, canonicalRefSha: null,
      intendedFileUpdates: [{ path: frontierPath, content: '{"state":"J1"}\n' }],
      now: "2020-01-01T00:00:00.000Z",
    });
    applyTransaction(j1, journalDir); // J1 committed.

    const j2 = prepareTransaction({
      journalDir, transactionId: "gen-c-j2", operation: "begin", sourceBaseSha: SHA, canonicalRefSha: null,
      intendedFileUpdates: [{ path: frontierPath, content: '{"state":"J2"}\n' }],
      now: "2020-01-01T00:00:01.000Z",
    });
    applyTransaction(j2, journalDir); // J2 FULLY committed too, before R2 ever resumes.
    assert.equal(readFileSync(frontierPath, "utf8"), '{"state":"J2"}\n');

    const staleResult = applyTransaction(j1, journalDir); // R2 finally resumes with stale J1.

    assert.equal(staleResult.status, "RECOVERY_CONFLICT", "a stale recoverer whose target diverged must fail closed, not silently succeed");
    assert.equal(staleResult.divergedPaths.length, 1);
    assert.equal(staleResult.divergedPaths[0].path, frontierPath);
    assert.equal(readFileSync(frontierPath, "utf8"), '{"state":"J2"}\n', "final state must remain EXACTLY J2 -- never reverted to J1");
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
});

test("GEN-D: after a stale-J1 recovery attempt safely no-ops against a PREPARED J2, the process actually owning J2 can crash and a later, fresh recovery pass still discovers and completes J2", () => {
  const { dir, journalDir, frontierPath, leasesPath } = makeFixture();
  try {
    const j1 = prepareTransaction({
      journalDir, transactionId: "gen-d-j1", operation: "claim", sourceBaseSha: SHA, canonicalRefSha: null,
      intendedFileUpdates: [{ path: frontierPath, content: '{"state":"J1"}\n' }],
      now: "2020-01-01T00:00:00.000Z",
    });
    applyTransaction(j1, journalDir);

    prepareTransaction({
      journalDir, transactionId: "gen-d-j2", operation: "begin", sourceBaseSha: SHA, canonicalRefSha: null,
      intendedFileUpdates: [{ path: frontierPath, content: '{"state":"J2"}\n' }, { path: leasesPath, content: '{"leaseState":"J2"}\n' }],
      now: "2020-01-01T00:00:01.000Z",
    });
    // J2's owning worker has now crashed -- it never called applyTransaction.

    // Stale R2 resumes with J1 (a no-op, per GEN-B) sometime in between.
    applyTransaction(j1, journalDir);
    assert.ok(listPendingTransactionIds(journalDir).includes("gen-d-j2"), "J2 must still be discoverable after the stale-J1 no-op");

    // A later, completely fresh recovery pass (simulating the next worker
    // to run any mutating command) must still discover and complete J2.
    const result = recoverPendingTransaction(journalDir);
    const j2Outcome = result.outcomes.find((o) => o.transactionId === "gen-d-j2");

    assert.ok(j2Outcome, "J2 must be discovered by a fresh recovery pass");
    assert.equal(j2Outcome.recovered, true);
    assert.equal(readFileSync(frontierPath, "utf8"), '{"state":"J2"}\n');
    assert.equal(readFileSync(leasesPath, "utf8"), '{"leaseState":"J2"}\n');
    assert.equal(hasPendingTransaction(journalDir), false);
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
});

test("GEN-E: a target file diverged for ANY reason (not just a newer transaction) since prepare -- recovery fails closed, target bytes unchanged", () => {
  const { dir, journalDir, frontierPath } = makeFixture();
  try {
    const j1 = prepareTransaction({
      journalDir, transactionId: "gen-e-j1", operation: "claim", sourceBaseSha: SHA, canonicalRefSha: null,
      intendedFileUpdates: [{ path: frontierPath, content: '{"state":"J1"}\n' }],
      now: "2020-01-01T00:00:00.000Z",
    });

    // Something OUTSIDE this protocol entirely mutated the target -- not a
    // J2, just arbitrary drift (a hand-edit, an unrelated tool). Neither
    // before_hash (the pre-prepare content) nor after_hash (J1's intended
    // content) matches this.
    writeFileSync(frontierPath, '{"state":"hand-edited-by-something-else"}\n');

    const result = applyTransaction(j1, journalDir);

    assert.equal(result.status, "RECOVERY_CONFLICT");
    assert.equal(result.divergedPaths[0].actual_hash, sha256Hex('{"state":"hand-edited-by-something-else"}\n'));
    assert.notEqual(result.divergedPaths[0].actual_hash, result.divergedPaths[0].expected_before_hash);
    assert.notEqual(result.divergedPaths[0].actual_hash, result.divergedPaths[0].expected_after_hash);
    assert.equal(readFileSync(frontierPath, "utf8"), '{"state":"hand-edited-by-something-else"}\n', "diverged bytes must be left completely untouched");
    assert.ok(listPendingTransactionIds(journalDir).includes("gen-e-j1"), "a RECOVERY_CONFLICT must not silently clear the journal record -- it is the evidence of the conflict");
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
});

test("GEN-F: a stale process finalizing its OWN already-gone generation never affects a newer transaction's journal, even called repeatedly", () => {
  const { dir, journalDir, frontierPath } = makeFixture();
  try {
    const j1 = prepareTransaction({
      journalDir, transactionId: "gen-f-j1", operation: "claim", sourceBaseSha: SHA, canonicalRefSha: null,
      intendedFileUpdates: [{ path: frontierPath, content: '{"state":"J1"}\n' }],
      now: "2020-01-01T00:00:00.000Z",
    });
    applyTransaction(j1, journalDir); // J1's own file is now gone.

    const j2 = prepareTransaction({
      journalDir, transactionId: "gen-f-j2", operation: "begin", sourceBaseSha: SHA, canonicalRefSha: null,
      intendedFileUpdates: [{ path: frontierPath, content: '{"state":"J2"}\n' }],
      now: "2020-01-01T00:00:01.000Z",
    });

    // The stale process calls applyTransaction(j1, ...) three more times --
    // "repeatedly" per the requirement -- each must be a harmless no-op that
    // never touches J2.
    for (let i = 0; i < 3; i++) {
      const result = applyTransaction(j1, journalDir);
      assert.equal(result.status, "COMMITTED");
      assert.ok(listPendingTransactionIds(journalDir).includes("gen-f-j2"), `J2 must survive stale-finalize attempt #${i + 1}`);
      assert.deepEqual(readJournalById(journalDir, "gen-f-j2"), j2);
    }
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
});

test("GEN-G: repeated recovery of the SAME generation via recoverPendingTransaction (not just applyTransaction directly) is idempotent", () => {
  const { dir, journalDir, frontierPath } = makeFixture();
  try {
    prepareTransaction({
      journalDir, transactionId: "gen-g", operation: "claim", sourceBaseSha: SHA, canonicalRefSha: null,
      intendedFileUpdates: [{ path: frontierPath, content: '{"state":"applied"}\n' }],
      now: "2020-01-01T00:00:00.000Z",
    });

    const first = recoverPendingTransaction(journalDir);
    assert.equal(first.recovered, true);
    assert.equal(readFileSync(frontierPath, "utf8"), '{"state":"applied"}\n');

    const second = recoverPendingTransaction(journalDir);
    assert.equal(second.recovered, false, "nothing pending any more -- a safe no-op, not an error");
    assert.equal(second.reason, "no pending journal");
    assert.equal(readFileSync(frontierPath, "utf8"), '{"state":"applied"}\n', "content must be unchanged by the redundant call");
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
});

test("regression: a transaction listed as pending that vanishes (its own owner finishes independently) before it is read must never crash a recoverer", () => {
  // Found by soak-testing this suite after the B.2.1 refactor, not by the
  // architect's own report: recoverPendingTransaction() now lists every
  // pending transaction_id via a directory scan, then reads each one
  // individually -- and because journal identity is per-transaction, one of
  // those listed ids can legitimately belong to a DIFFERENT, independently
  // running worker that finishes (applies + deletes its own file) in the gap
  // between the listing and this read. readIfExists() used to be
  // existsSync()-then-readFileSync(), a TOCTOU: measured concretely via the
  // "concurrent claim: ... DIFFERENT non-overlapping tasks" test above, which
  // failed with an uncaught ENOENT from readFileSync() roughly 1 run in 15-50
  // under real multi-process contention. Fixed by making readIfExists() a
  // single attempt-and-catch (see its own doc comment). This test reproduces
  // the exact failing code path deterministically, with no dependency on
  // timing luck: list a transaction, delete it out from under the read
  // (simulating its owner finishing independently), then read it.
  const { dir, journalDir, frontierPath } = makeFixture();
  try {
    const record = prepareTransaction({
      journalDir, transactionId: "vanishing", operation: "claim", sourceBaseSha: SHA, canonicalRefSha: null,
      intendedFileUpdates: [{ path: frontierPath, content: '{"state":"applied-by-someone-else"}\n' }],
      now: "2020-01-01T00:00:00.000Z",
    });

    const listedIds = listPendingTransactionIds(journalDir);
    assert.ok(listedIds.includes("vanishing"));

    // Simulate: between our listing and our read, this transaction's actual
    // owner independently finished it.
    applyTransaction(record, journalDir);
    assert.equal(listPendingTransactionIds(journalDir).includes("vanishing"), false);

    // The read that used to throw ENOENT here.
    assert.doesNotThrow(() => readJournalById(journalDir, "vanishing"));
    assert.equal(readJournalById(journalDir, "vanishing"), null, "a transaction that vanished between listing and reading must read back as null, not throw");

    // And the full public entry point, given a stale listedIds snapshot that
    // includes an id no longer present, must not throw either -- it should
    // simply be unable to recover something already gone.
    const outcome = recoverPendingTransaction(journalDir);
    assert.equal(outcome.recovered, false);
    assert.equal(outcome.reason, "no pending journal");
    assert.equal(readFileSync(frontierPath, "utf8"), '{"state":"applied-by-someone-else"}\n', "the independently-finished owner's content must be intact");
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
});

// --- BX7 adversarial regression tests --------------------------------------

test("BX7-1 regression: a multi-file transaction with a diverged LATER file writes NOTHING -- it must never leave leases.json updated while the frontier file it belongs with is not", () => {
  // The DIVERGED classification was applied inside the same pass that WROTE,
  // so a transaction whose leases.json update was still safe-to-apply and
  // whose frontier update had diverged wrote leases.json and only then
  // returned RECOVERY_CONFLICT -- leaving exactly the torn pair (an ACTIVE
  // lease for a task whose frontier status was never moved to LEASED) that
  // the write-ahead journal exists to make impossible, and leaving it
  // PERMANENTLY: the retained journal record can never complete, because the
  // diverged path stays diverged on every later retry. Reachable in practice
  // via an out-of-protocol edit to one of the targets (the drift GEN-E is
  // about) or via the multiple-pending-journal anomaly the module tolerates.
  const { dir, journalDir, leasesPath, frontierPath } = makeFixture();
  try {
    const beforeLeases = readFileSync(leasesPath, "utf8");
    const afterLeasesContent = `[${JSON.stringify({ lease_id: "lease-fixture", task_id: "REPAIR-0001", status: "ACTIVE" })}]\n`;
    const afterFrontierContent = `${JSON.stringify({ tasks: [{ task_id: "REPAIR-0001", status: "LEASED", source_base_sha: SHA, owned_paths: ["crates/x/**"] }] }, null, 2)}\n`;

    // leases.json is always the FIRST intended update (see
    // computeIntendedFileUpdates in leases.mjs), the frontier file follows.
    const record = prepareTransaction({
      journalDir, transactionId: "bx7-torn", operation: "claim", sourceBaseSha: SHA, canonicalRefSha: null,
      intendedFileUpdates: [
        { path: leasesPath, content: afterLeasesContent },
        { path: frontierPath, content: afterFrontierContent },
      ],
      now: "2020-01-01T00:00:00.000Z",
    });

    // Crash before ANY write, then the frontier file (only) diverges.
    writeFileSync(frontierPath, `${JSON.stringify({ tasks: [{ task_id: "REPAIR-0001", status: "DESIGN_REQUIRED", source_base_sha: SHA, owned_paths: ["crates/x/**"] }] }, null, 2)}\n`);
    const divergedFrontier = readFileSync(frontierPath, "utf8");

    const result = applyTransaction(record, journalDir);

    assert.equal(result.status, "RECOVERY_CONFLICT");
    assert.deepEqual(result.appliedPaths, [], "a conflicted transaction must apply nothing at all");
    assert.equal(result.divergedPaths.length, 1);
    assert.equal(result.divergedPaths[0].path, frontierPath);
    assert.equal(readFileSync(leasesPath, "utf8"), beforeLeases, "leases.json must NOT be half-written when the frontier file it belongs with diverged");
    assert.equal(readFileSync(frontierPath, "utf8"), divergedFrontier, "diverged bytes must be left completely untouched");
    assert.ok(listPendingTransactionIds(journalDir).includes("bx7-torn"), "the conflict evidence must remain on disk");

    // And the transaction is left genuinely recoverable, not stuck half-done:
    // clear the divergence and the SAME record still applies as one unit.
    writeFileSync(frontierPath, JSON.stringify({ tasks: [{ task_id: "REPAIR-0001", status: "READY", source_base_sha: SHA, owned_paths: ["crates/x/**"] }] }, null, 2) + "\n");
    const retry = applyTransaction(record, journalDir);
    assert.equal(retry.status, "COMMITTED");
    assert.equal(readFileSync(leasesPath, "utf8"), afterLeasesContent);
    assert.equal(readFileSync(frontierPath, "utf8"), afterFrontierContent);
    assert.equal(hasPendingTransaction(journalDir), false);
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
});

test("BX7-2 regression: an unrecoverable (permanently diverged) pending journal surfaces AS a recovery conflict, not as phantom lock contention -- and still writes nothing", () => {
  // acquireDispatchLock() recovers before it even attempts the lock. A
  // RECOVERY_CONFLICT is permanent by construction (the diverged file and the
  // journal record are both left untouched, so re-running recovery can never
  // change the outcome), but the loop simply `continue`d -- burning every
  // remaining attempt and then throwing "still contended by another worker".
  // That is an actively misleading diagnosis: there is no other worker, and
  // the state requires a human to inspect the diverged file. The whole point
  // of retaining the journal record is that the conflict is surfaced.
  const { dir, lockPath, journalDir, leasesPath, recoveryLogPath, frontierPath } = makeFixture();
  try {
    prepareTransaction({
      journalDir, transactionId: "bx7-wedge", operation: "claim", sourceBaseSha: SHA, canonicalRefSha: null,
      intendedFileUpdates: [{ path: frontierPath, content: '{"state":"never-lands"}\n' }],
      now: "2020-01-01T00:00:00.000Z",
    });
    writeFileSync(frontierPath, '{"state":"diverged-by-something-outside-the-protocol"}\n');
    const beforeFrontier = readFileSync(frontierPath, "utf8");
    const beforeLeases = readFileSync(leasesPath, "utf8");

    assert.throws(
      () => runDispatchTransaction({
        operation: "claim", workerId: "w1", lockPath, journalDir, leasesPath, frontierPaths: [frontierPath], recoveryLogPath,
        sourceBaseSha: SHA, maxLockAttempts: 3, sleepFn: () => {}, buildOutcome: claimBuildOutcome("REPAIR-0001", "w1"),
      }),
      (err) =>
        err instanceof Error &&
        /cannot be recovered/.test(err.message) &&
        /bx7-wedge/.test(err.message) &&
        /diverged/.test(err.message) &&
        !/still contended by another worker/.test(err.message),
    );

    assert.equal(readFileSync(frontierPath, "utf8"), beforeFrontier, "the diverged file must be untouched");
    assert.equal(readFileSync(leasesPath, "utf8"), beforeLeases, "nothing may be written while the journal is unrecoverable");
    assert.equal(existsSync(lockPath), false, "no lock may be taken or left behind");
    assert.ok(listPendingTransactionIds(journalDir).includes("bx7-wedge"), "the conflict evidence must remain for inspection");

    // Once a human clears the divergence, normal dispatch resumes.
    writeFileSync(frontierPath, JSON.stringify({ tasks: [{ task_id: "REPAIR-0001", status: "READY", source_base_sha: SHA, owned_paths: ["crates/x/**"] }] }, null, 2) + "\n");
    // (the journal's own recorded content is what recovery lands, then the claim proceeds)
    const outcome = runDispatchTransaction({
      operation: "claim", workerId: "w1", lockPath, journalDir, leasesPath, frontierPaths: [frontierPath], recoveryLogPath,
      sourceBaseSha: SHA, maxLockAttempts: 3, sleepFn: () => {}, buildOutcome: claimBuildOutcome("REPAIR-0001", "w1"),
    });
    assert.equal(hasPendingTransaction(journalDir), false, "the previously-stuck journal must now be recoverable");
    assert.equal(existsSync(lockPath), false);
    assert.ok(outcome.ok === false || outcome.ok === true, "the command now runs to a normal decision instead of throwing");
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
});

test("BX7-3 regression: a journal FILENAME dispatch-journal.mjs could never have written is skipped rather than crashing every mutating command", () => {
  // listPendingTransactionIds() feeds every id it returns straight back into
  // journalFilePath(), which THROWS on an id that is not a safe path
  // component. `txn-.json` yields the empty id, so a single stray file made
  // recoverPendingTransaction() -> acquireDispatchLock() die with a raw
  // `unsafe transaction_id for a journal filename: ""` Error, wedging every
  // mutating dispatch command. Unrecognizable on-disk state is a STATE to
  // classify, never a crash (same principle as dispatch-lock.mjs's TORN lock).
  const { dir, lockPath, journalDir, leasesPath, recoveryLogPath, frontierPath } = makeFixture();
  try {
    // Force the journal directory to exist, then drop impossible names in it.
    prepareTransaction({
      journalDir, transactionId: "bx7-real", operation: "claim", sourceBaseSha: SHA, canonicalRefSha: null,
      intendedFileUpdates: [{ path: join(dir, "side-file.json"), content: '{"side":true}\n' }],
      now: "2020-01-01T00:00:00.000Z",
    });
    writeFileSync(join(journalDir, "txn-.json"), "{}\n");
    writeFileSync(join(journalDir, "txn-not a safe id.json"), "{}\n");

    const listed = listPendingTransactionIds(journalDir);
    assert.ok(listed.includes("bx7-real"), "real pending transactions must still be listed");
    assert.ok(!listed.includes(""), "the empty id must never be listed");
    assert.ok(listed.every((id) => /^[A-Za-z0-9._-]+$/.test(id)), `every listed id must be a safe path component: ${JSON.stringify(listed)}`);

    assert.doesNotThrow(() => recoverPendingTransaction(journalDir));
    assert.equal(readFileSync(join(dir, "side-file.json"), "utf8"), '{"side":true}\n', "the real transaction must still have been recovered");

    // And a full mutating command must run normally rather than dying.
    const outcome = runDispatchTransaction({
      operation: "claim", workerId: "w1", lockPath, journalDir, leasesPath, frontierPaths: [frontierPath], recoveryLogPath,
      sourceBaseSha: SHA, maxLockAttempts: 3, sleepFn: () => {}, buildOutcome: claimBuildOutcome("REPAIR-0001", "w1"),
    });
    assert.equal(outcome.ok, true, outcome.reason);
    assert.equal(existsSync(lockPath), false);
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
});

test("BX7-4 regression: a journal file with a well-formed NAME but unusable CONTENT is classified UNREADABLE_JOURNAL, never acted on and never thrown out of recovery", () => {
  // A file can sit at a perfectly safe txn-<id>.json path and still not be a
  // usable record: unparseable JSON, no transaction_id, or -- the dangerous
  // one -- a transaction_id that disagrees with its own filename. That last
  // case would have been APPLIED, and applyTransaction() derives the journal
  // file it DELETES from the record's transaction_id, so a foreign record
  // naming another transaction would have deleted that other transaction's
  // still-pending journal: the exact cross-generation hazard B.2.1 exists to
  // close, re-opened through the recovery path. The others escaped as raw
  // SyntaxError / `unsafe transaction_id ... undefined` Errors out of
  // acquireDispatchLock(), wedging every mutating dispatch command.
  const { dir, lockPath, journalDir, leasesPath, recoveryLogPath, frontierPath } = makeFixture();
  try {
    // A real, legitimately pending transaction that must survive untouched.
    const victim = prepareTransaction({
      journalDir, transactionId: "bx7-victim", operation: "claim", sourceBaseSha: SHA, canonicalRefSha: null,
      intendedFileUpdates: [{ path: join(dir, "victim.json"), content: '{"victim":"intact"}\n' }],
      now: "2020-01-01T00:00:00.000Z",
    });

    // (1) a record whose transaction_id names the VICTIM, not itself.
    writeFileSync(join(journalDir, "txn-bx7-impostor.json"), `${JSON.stringify({ ...victim, transaction_id: "bx7-victim", intended_file_updates: [] }, null, 2)}\n`);
    // (2) unparseable JSON.
    writeFileSync(join(journalDir, "txn-bx7-torn-record.json"), '{"schema": "chron');
    // (3) parseable but shapeless.
    writeFileSync(join(journalDir, "txn-bx7-empty-record.json"), "{}\n");

    let result;
    assert.doesNotThrow(() => { result = recoverPendingTransaction(journalDir); });

    const byId = Object.fromEntries(result.outcomes.map((o) => [o.transactionId, o]));
    assert.equal(byId["bx7-impostor"].status, "UNREADABLE_JOURNAL", "a record whose transaction_id disagrees with its filename must never be acted on");
    assert.match(byId["bx7-impostor"].reason, /does not match its own filename/);
    assert.equal(byId["bx7-torn-record"].status, "UNREADABLE_JOURNAL");
    assert.match(byId["bx7-torn-record"].reason, /does not parse/);
    assert.equal(byId["bx7-empty-record"].status, "UNREADABLE_JOURNAL");

    // The legitimate transaction was recovered, and its journal was cleared by
    // ITS OWN record -- never by the impostor.
    assert.equal(byId["bx7-victim"].recovered, true);
    assert.equal(readFileSync(join(dir, "victim.json"), "utf8"), '{"victim":"intact"}\n');

    // The unusable files are left on disk as evidence, not silently removed.
    for (const f of ["txn-bx7-impostor.json", "txn-bx7-torn-record.json", "txn-bx7-empty-record.json"]) {
      assert.ok(existsSync(join(journalDir, f)), `${f} must be left on disk as evidence`);
    }

    // And the orchestration layer surfaces it as an unrecoverable journal, not
    // as phantom lock contention and not as a raw crash.
    assert.throws(
      () => runDispatchTransaction({
        operation: "claim", workerId: "w1", lockPath, journalDir, leasesPath, frontierPaths: [frontierPath], recoveryLogPath,
        sourceBaseSha: SHA, maxLockAttempts: 3, sleepFn: () => {}, buildOutcome: claimBuildOutcome("REPAIR-0001", "w1"),
      }),
      (err) =>
        err instanceof Error && !(err instanceof SyntaxError) &&
        /cannot be recovered/.test(err.message) &&
        /unreadable journal/.test(err.message) &&
        !/still contended by another worker/.test(err.message),
    );
    assert.equal(existsSync(lockPath), false, "no lock may be taken while the journal directory is unusable");
    assert.equal(readFileSync(leasesPath, "utf8"), "[]\n", "nothing may be written");
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
});

// --- B.2.2 defect B: false-success regression tests -----------------------
//
// applyTransaction()'s return status used to be discarded entirely inside
// runDispatchTransaction() -- `return outcome` always reported the pure
// business/state-machine decision as success, even when the actual file
// writes never committed (RECOVERY_CONFLICT: the journal stays PREPARED,
// target bytes are unchanged per the fail-closed rule). A caller/CLI would
// have printed `ok:true` and a lease as though the claim went through.
//
// Forcing a REAL RECOVERY_CONFLICT at this EXACT call site (immediately
// after runDispatchTransaction's own prepareTransaction(), before its own
// applyTransaction()) has no caller-controlled hook to inject through via
// buildOutcome() -- that gap is a couple of synchronous statements with
// nothing in between (see leases.mjs's own doc comment on this). So these
// tests use runDispatchTransaction's applyTransactionFn injection seam
// (same pattern as sleepFn/classifyOwnerLivenessFn elsewhere in this
// codebase) to substitute a stand-in that returns a canned RECOVERY_CONFLICT
// without touching any file -- which is exactly what a REAL RECOVERY_CONFLICT
// also does (see dispatch-journal.mjs's applyTransaction(): on divergence it
// writes nothing and leaves the journal record in place), so asserting on
// real on-disk state afterward is a faithful test of the real behavior.

test("B.2.2 false-success: RECOVERY_CONFLICT from applyTransaction() must make runDispatchTransaction report failure, not the pure state-machine's success", () => {
  const { dir, lockPath, journalDir, leasesPath, recoveryLogPath, frontierPath } = makeFixture();
  try {
    const fakeDivergedPath = join(dir, "leases.json");
    const forcedConflict = (journalRecord) => ({
      appliedPaths: [],
      alreadyAppliedPaths: [],
      divergedPaths: [{ path: fakeDivergedPath, expected_before_hash: "before", expected_after_hash: "after", actual_hash: "something-else" }],
      status: "RECOVERY_CONFLICT",
    });

    const outcome = runDispatchTransaction({
      operation: "claim", workerId: "w1", lockPath, journalDir, leasesPath, frontierPaths: [frontierPath], recoveryLogPath,
      sourceBaseSha: SHA, applyTransactionFn: forcedConflict,
      buildOutcome: claimBuildOutcome("REPAIR-0001", "w1"),
    });

    assert.equal(outcome.ok, false, "a RECOVERY_CONFLICT must never be reported as success");
    assert.match(outcome.reason, /did NOT commit/);
    assert.match(outcome.reason, /RECOVERY_CONFLICT/);
    assert.doesNotMatch(outcome.reason ?? "", /^\s*$/); // a real, non-blank diagnostic, not a silent failure

    // The journal our own prepareTransaction() wrote is genuinely still
    // pending (the fake applyTransactionFn never touched it, matching what
    // a real RECOVERY_CONFLICT also leaves behind).
    assert.equal(hasPendingTransaction(journalDir), true, "the journal must remain -- nothing committed");
    // The target file (leases.json) is completely unchanged.
    assert.equal(readFileSync(leasesPath, "utf8"), "[]\n", "no partial state may appear committed");
    const { tasksById } = loadFrontiersFrom([frontierPath]);
    assert.equal(tasksById.get("REPAIR-0001").status, "READY", "the frontier file must be unchanged too");
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
});

test("B.2.2 false-success: an unrecognized applyTransaction() status also fails closed, not just the known RECOVERY_CONFLICT case", () => {
  const { dir, lockPath, journalDir, leasesPath, recoveryLogPath, frontierPath } = makeFixture();
  try {
    const forcedUnknownStatus = () => ({ appliedPaths: [], alreadyAppliedPaths: [], divergedPaths: [], status: "SOME_FUTURE_STATUS_THIS_CODE_DOES_NOT_KNOW" });

    const outcome = runDispatchTransaction({
      operation: "claim", workerId: "w1", lockPath, journalDir, leasesPath, frontierPaths: [frontierPath], recoveryLogPath,
      sourceBaseSha: SHA, applyTransactionFn: forcedUnknownStatus,
      buildOutcome: claimBuildOutcome("REPAIR-0001", "w1"),
    });

    assert.equal(outcome.ok, false, "only an EXACT status of COMMITTED may report success -- an unrecognized status must never be silently treated as one");
    assert.match(outcome.reason, /did NOT commit/);
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
});

test("B.2.2 false-success: a normal COMMITTED transaction (the real applyTransaction, no injection) still reports success as before", () => {
  const { dir, lockPath, journalDir, leasesPath, recoveryLogPath, frontierPath } = makeFixture();
  try {
    const outcome = runDispatchTransaction({
      operation: "claim", workerId: "w1", lockPath, journalDir, leasesPath, frontierPaths: [frontierPath], recoveryLogPath,
      sourceBaseSha: SHA, buildOutcome: claimBuildOutcome("REPAIR-0001", "w1"),
    });

    assert.equal(outcome.ok, true, outcome.reason);
    assert.equal(outcome.taskStatusUpdates[0].to, "LEASED");
    assert.equal(hasPendingTransaction(journalDir), false, "a real COMMITTED transaction must clear its own journal");
    const { tasksById } = loadFrontiersFrom([frontierPath]);
    assert.equal(tasksById.get("REPAIR-0001").status, "LEASED");
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
});

test("B.2.2 defect C: two REAL child processes racing prepareTransaction() with the SAME transaction_id -- exactly one publication wins, the other fails closed", async () => {
  const dir = mkdtempSync(join(tmpdir(), "dispatch-orch-test-"));
  try {
    const journalDir = join(dir, "dispatch-journal");
    const targetPath = join(dir, "target.json");
    writeFileSync(targetPath, '{"state":"original"}\n');

    const childScriptPath = join(dir, "collide-child.mjs");
    writeFileSync(
      childScriptPath,
      [
        `import { prepareTransaction } from ${JSON.stringify(resolve(REPO_ROOT, "tools/system-atlas/dispatch-journal.mjs"))};`,
        `const [journalDir, targetPath, content] = process.argv.slice(2);`,
        `try {`,
        `  const record = prepareTransaction({ journalDir, transactionId: "shared-race-id", operation: "claim", sourceBaseSha: ${JSON.stringify(SHA)}, intendedFileUpdates: [{ path: targetPath, content }], now: new Date().toISOString() });`,
        `  process.stdout.write(JSON.stringify({ ok: true, transactionId: record.transaction_id }));`,
        `} catch (err) {`,
        `  process.stdout.write(JSON.stringify({ ok: false, code: err.code, message: err.message }));`,
        `}`,
      ].join("\n"),
    );

    function runChild(content) {
      return new Promise((resolvePromise, rejectPromise) => {
        const child = spawn(process.execPath, [childScriptPath, journalDir, targetPath, content], { stdio: ["ignore", "pipe", "pipe"] });
        let stdout = "";
        let stderr = "";
        child.stdout.on("data", (d) => { stdout += d; });
        child.stderr.on("data", (d) => { stderr += d; });
        child.on("error", rejectPromise);
        child.on("close", () => resolvePromise({ stdout, stderr }));
      });
    }

    const N = 8;
    const results = await Promise.all(Array.from({ length: N }, (_, i) => runChild(`{"state":"from-child-${i}"}\n`)));
    const parsed = results.map((r) => {
      try {
        return JSON.parse(r.stdout);
      } catch {
        return { ok: false, parseError: true, raw: r.stdout, stderr: r.stderr };
      }
    });

    assert.ok(parsed.every((p) => !p.parseError), `every child must produce valid JSON: ${JSON.stringify(parsed)}`);
    const winners = parsed.filter((p) => p.ok === true);
    const losers = parsed.filter((p) => p.ok === false);
    assert.equal(winners.length, 1, `exactly one of ${N} concurrent prepareTransaction() calls with the same transaction_id may publish`);
    assert.equal(losers.length, N - 1);
    assert.ok(losers.every((l) => l.code === "TRANSACTION_ID_ALREADY_EXISTS"), `every loser must fail with TRANSACTION_ID_ALREADY_EXISTS: ${JSON.stringify(losers)}`);

    // Exactly one journal file exists, and no scratch artifacts remain from
    // any of the N-1 losers.
    const journalFiles = readdirSync(journalDir).filter((f) => f.startsWith("txn-"));
    assert.equal(journalFiles.length, 1);
    const scratchFiles = readdirSync(journalDir).filter((f) => f.endsWith(".tmp"));
    assert.deepEqual(scratchFiles, [], "no loser may leave a scratch file behind");
    assert.equal(readFileSync(targetPath, "utf8"), '{"state":"original"}\n', "prepareTransaction never touches target files regardless of who won");
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
});
