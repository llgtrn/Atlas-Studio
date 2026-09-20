#!/usr/bin/env node
// Write-ahead journal for multi-file dispatch transactions.
//
// PROBLEM THIS SOLVES: a lease transition (see leases.mjs) can touch TWO
// files -- docs/_machine/system-atlas/v0/leases.json and one *-frontier.json
// -- via independent write-tmp-then-rename operations. A crash between the
// two renames leaves an inconsistent pair (e.g. an ACTIVE lease whose task
// still shows its pre-transition status, or vice versa). This module makes
// that impossible to leave inconsistent, by making the *set* of file writes
// recoverable as a unit:
//
//   IDLE (no journal file for this transaction) --prepareTransaction()-->
//   PREPARED (a journal file exists, on disk, listing every {path, content}
//   the transaction intends to write) --applyTransaction()--> every target
//   file written, then the journal file itself is deleted, which IS the
//   commit signal.
//
// B.2.1 GENERATION SAFETY (architect finding, was previously unenforced):
// journal identity is now PER-TRANSACTION and IMMUTABLE, not one shared
// well-known path. Every transaction gets its own file,
// txn-<transaction_id>.json, inside a journal DIRECTORY
// (DEFAULT_JOURNAL_DIR). applyTransaction() derives the file it reads/
// deletes from journalRecord.transaction_id itself -- never from a
// caller-supplied path that could have drifted out from under an
// in-memory-held stale record. This makes an ABA race structurally
// impossible for journal IDENTITY: a recoverer holding a stale in-memory
// copy of transaction J1 (read long ago, then paused -- by a slow process,
// GC, or OS scheduling) can only ever act on txn-J1's own file. It cannot
// discover, delete, or otherwise disturb a newer transaction J2's file, even
// though the two may exist side-by-side or J2 may have been prepared and/or
// committed in the meantime. (Before this fix, a single shared journalPath
// meant a stale recoverer's unconditional `unlinkSync(journalPath)` deleted
// WHATEVER was currently there -- possibly a completely different,
// newer transaction's PREPARED journal, silently destroying its only
// record. See docs/_machine/system-atlas/README.md's "Crash recovery"
// section and the B.2.1 PR history for the concrete reproduction.)
//
// Journal identity safety alone is NOT enough, though: a stale J1 recoverer
// can still legitimately be asked to apply J1's OWN recorded target-file
// writes, and those target files (e.g. leases.json) may since have been
// moved on by J2. applyTransaction() therefore classifies each target file's
// CURRENT content against J1's own before_hash/after_hash before touching
// it -- see applyTransaction()'s own comment for the three-way
// already-applied / safe-to-apply / DIVERGED classification. Only the
// DIVERGED case is new to B.2.1; the other two already existed.
//
// The caller's contract: every mutating command should call
// recoverPendingTransaction(journalDir) BEFORE doing anything else, so any
// interrupted prior transaction(s) -- there can be more than one pending
// file only as a diagnosable anomaly; normal operation (the dispatch lock
// serializes prepareTransaction calls) produces at most one at a time -- are
// always rolled forward first, before new work is accepted. Each pending
// transaction is recovered independently and safely regardless of how many
// there are, thanks to the per-file hash classification above.
//
// dispatch_contract_version: schema/dispatch-journal.schema.json's persisted
// RECORD shape (the JSON fields) is unchanged by this fix -- but WHERE a
// record lives on disk changed, from one shared well-known path to one
// immutable file per transaction_id inside a directory, which is a real
// behavioral/operational break for any consumer that hardcoded the old
// path. Per the B.2 contract-freeze section, that is honestly a breaking
// change to the frozen interface, not a silent one: dispatch_contract_version
// is bumped 1 -> 2 (see build-dispatch-plan.mjs and
// docs/_machine/system-atlas/README.md).
//
// All functions here are pure EXCEPT where file I/O is explicitly the
// point -- this module's whole purpose is durable multi-file writes, so
// unlike the pure state-machine modules elsewhere in system-atlas/, real fs
// calls belong here. Each function still does exactly one clear thing.
import { closeSync, existsSync, fsyncSync, linkSync, mkdirSync, openSync, readFileSync, readdirSync, renameSync, unlinkSync, writeFileSync } from "node:fs";
import { createHash, randomUUID } from "node:crypto";
import { resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { mkdtempSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";

export const JOURNAL_SCHEMA = "chronica.system-atlas.dispatch-journal.v0";

// JOURNAL_CONTRACT_VERSION -- a supplementary, module-local counter (see
// dispatch-lock.mjs's LOCK_CONTRACT_VERSION for the same pattern and why
// this is not a second authoritative version -- dispatch_contract_version
// in dispatch-plan.json is that). v1 was B.2 (one shared well-known path).
// v2 was B.2.1: per-transaction immutable identity (journalDir/txn-
// <transaction_id>.json) plus diverged-hash fail-closed classification --
// this closed the ABA generation race. v3 is B.2.2: publication is now
// EXCLUSIVE (a transaction_id collision throws TRANSACTION_ID_ALREADY_EXISTS
// rather than silently overwriting), and recovery now validates full
// record integrity (INVALID_JOURNAL: unknown operation, malformed/mismatched
// hashes, duplicate or out-of-allowlist target paths) before acting on
// anything read from disk -- both real breaks for any consumer that assumed
// the old publish-by-rename behavior or trusted a journal record's shape
// without independently verifying its content.
export const JOURNAL_CONTRACT_VERSION = 3;

// Thin CLI-facing default -- the core API below never hardcodes this; every
// function that touches journal files takes journalDir explicitly so it is
// trivially testable against a temp path. This is now a DIRECTORY (B.2.1),
// not a single file -- see module header.
export const DEFAULT_JOURNAL_DIR = resolve(
  fileURLToPath(new URL(".", import.meta.url)),
  "..",
  "..",
  "docs/_machine/system-atlas/v0/dispatch-journal",
);

// Filename-safety guard: transaction_id is always generated by this
// codebase (leases.mjs's randomSuffix()-based ids, or a test's own literal
// strings), but a journal filename is derived from it, so defensively
// reject anything that isn't a safe path component rather than trust the
// caller. This can never fire in normal operation.
const SAFE_ID_RE = /^[A-Za-z0-9._-]+$/;

function isSafeTransactionId(transactionId) {
  return typeof transactionId === "string" && SAFE_ID_RE.test(transactionId);
}

function journalFilePath(journalDir, transactionId) {
  if (!isSafeTransactionId(transactionId)) {
    throw new Error(`unsafe transaction_id for a journal filename: ${JSON.stringify(transactionId)}`);
  }
  return resolve(journalDir, `txn-${transactionId}.json`);
}

// ---------------------------------------------------------------------------
// sha256Hex -- content hash used for before/after comparisons.
// ---------------------------------------------------------------------------
export function sha256Hex(content) {
  return createHash("sha256").update(content, "utf8").digest("hex");
}

// A single attempt-and-catch, NOT existsSync()-then-readFileSync(): the
// latter is a TOCTOU that became reachable in B.2.1's design.
// recoverPendingTransaction() lists every currently-pending transaction_id
// (a directory scan), then reads each one individually -- and since journal
// identity is now per-transaction, more than one of those listed
// transactions can legitimately belong to a DIFFERENT, independently
// running worker that finishes (applies + deletes its own file) between the
// listing and this read. A pre-check-then-read here would throw ENOENT on
// exactly that (harmless, expected) case; a bare read with ENOENT caught as
// "not there" treats "never existed" and "existed a moment ago, now
// committed by its owner" identically -- both correctly mean "nothing left
// to recover for this id" to this caller.
function readIfExists(path) {
  try {
    return readFileSync(path, "utf8");
  } catch (err) {
    if (err.code === "ENOENT") return null;
    throw err;
  }
}

// Writes `contents` to a private, per-call scratch file next to `path` and
// fsyncs it, WITHOUT publishing it anywhere -- the two publish strategies
// below (writeFileDurable's rename, and publishJournalRecordExclusive's
// exclusive link) both build on this, differing only in the final syscall.
//
// DURABILITY NOTE (do not overclaim): fsyncSync on the scratch file's fd
// flushes that file's data (and, depending on OS/filesystem, its metadata)
// to storage before publication, which is the guarantee that matters for
// "the content that will become `path` is durable before we make it visible
// as `path`". What this does NOT guarantee, and what Node's synchronous
// node:fs API gives no portable way to guarantee, is durability of the
// publish syscall itself or of the containing directory entry -- that would
// require an additional fsync on an open fd for the directory, which
// node:fs's sync API does not expose a cross-platform way to do (open a
// directory with O_RDONLY and fsync its fd works on Linux but is not
// portable, e.g. to Windows). So: the bytes that end up at `path` are
// durable once the caller's publish step returns; the fact that `path`
// (rather than a leftover scratch file, on a crash between fsync and
// publish) is what's on disk is only as durable as a single syscall, which
// is atomic but not separately fsynced here.
//
// CONCURRENCY NOTE (BX6 adversarial finding, still true under B.2.1/B.2.2's
// per-transaction journal files): the scratch file name MUST be unique per
// call, never a shared `<path>.tmp`. Two processes can legitimately be
// recovering the SAME transaction's journal at the same moment (see
// recoverPendingTransaction()'s doc comment for why lock-free concurrent
// recovery of one generation is still intentional and safe), and each
// target-file write they perform is byte-identical (both replaying the same
// journal record), so racing on a shared scratch name is purely a
// self-inflicted crash, not a real data hazard -- fixed by giving every
// call its own scratch file.
function writeScratchFileDurable(path, contents) {
  const tmpPath = `${path}.${process.pid}-${randomUUID().slice(0, 8)}.tmp`;
  writeFileSync(tmpPath, contents);
  try {
    const fd = openSync(tmpPath, "r+");
    try {
      fsyncSync(fd);
    } finally {
      closeSync(fd);
    }
  } catch {
    // Best-effort: if fsync isn't available/permitted in this environment,
    // fall through -- the write itself already happened via writeFileSync.
  }
  return tmpPath;
}

function unlinkIfExists(path) {
  try {
    unlinkSync(path);
  } catch {
    // already gone; nothing to clean up
  }
}

// Durable write-then-OVERWRITE of `contents` to `path`. Used for TARGET file
// writes (leases.json, a frontier file, recovery-log.json) during
// applyTransaction() -- those are meant to be updated in place every time; a
// rename over an existing file is the correct, intended behavior there.
// NEVER used for journal record creation -- see publishJournalRecordExclusive.
function writeFileDurable(path, contents) {
  const tmpPath = writeScratchFileDurable(path, contents);
  try {
    renameSync(tmpPath, path);
  } catch (err) {
    // Never leave our private scratch file behind on a failed rename --
    // validate.mjs treats any leftover *.tmp under v0/ as evidence of an
    // interrupted non-journaled write.
    unlinkIfExists(tmpPath);
    throw err;
  }
}

// TRANSACTION_ID_ALREADY_EXISTS -- thrown by publishJournalRecordExclusive()
// (and so by prepareTransaction()) when a journal file for this
// transaction_id is already present. See publishJournalRecordExclusive()'s
// own doc comment for why this must be exclusive-create, not rename-over.
export const TRANSACTION_ID_ALREADY_EXISTS = "TRANSACTION_ID_ALREADY_EXISTS";

// Durable write-then-EXCLUSIVE-PUBLISH of `contents` to `path`. Used ONLY
// for journal record CREATION (prepareTransaction()).
//
// B.2.2 (architect review, defect C): journal identity is supposed to be
// IMMUTABLE per transaction_id -- but the module previously published a
// freshly-prepared record via the same rename-over-existing helper used for
// target files, so a transaction_id collision (or reuse) would silently
// REPLACE whatever journal record already existed at that path. Random
// transaction_id collision probability is negligible (leases.mjs generates
// them via randomSuffix()), but the invariant this module's whole design
// depends on -- "a transaction's own file, once it exists, is that
// transaction's and only that transaction's, until ITS OWN apply deletes
// it" -- must be structural, not merely improbable-to-violate.
//
// linkSync(scratch, path) gives the SAME all-or-nothing semantics
// dispatch-lock.mjs's tryAcquireOnce() already relies on for the lock file:
// either the link succeeds (nothing was there, our content is now visible
// whole) or it throws EEXIST (something already IS there -- we do not
// touch it). This is NOT existsSync()-then-rename(): that pattern has a
// TOCTOU (another process could publish between the check and the rename);
// linkSync's exclusivity is enforced by the OS at the one syscall that
// matters, with no window for a second writer to observe.
function publishJournalRecordExclusive(path, contents) {
  const tmpPath = writeScratchFileDurable(path, contents);
  try {
    try {
      linkSync(tmpPath, path);
    } catch (err) {
      if (err.code === "EEXIST") {
        throw Object.assign(
          new Error(`${TRANSACTION_ID_ALREADY_EXISTS}: a journal record already exists at ${path} -- refusing to overwrite it (original bytes are untouched)`),
          { code: TRANSACTION_ID_ALREADY_EXISTS },
        );
      }
      throw err;
    }
  } finally {
    // Always drop our scratch file -- on success `path` is now a second
    // link to the same inode, so removing this one changes nothing; on
    // EEXIST or any other failure, never leave it behind (validate.mjs
    // treats any leftover *.tmp as evidence of an interrupted write).
    unlinkIfExists(tmpPath);
  }
}

// ---------------------------------------------------------------------------
// readJournal -- returns the parsed journal record at an EXACT file path, or
// null if that file doesn't exist. This is a low-level primitive; most
// callers want readJournalById() or recoverPendingTransaction() instead,
// which operate in terms of journalDir + transaction_id rather than a raw
// path.
// ---------------------------------------------------------------------------
export function readJournal(path) {
  const raw = readIfExists(path);
  if (raw === null) return null;
  return JSON.parse(raw);
}

// ---------------------------------------------------------------------------
// readJournalById -- the parsed record for one specific transaction, or null
// if it doesn't exist (already committed/cleared, or never existed).
// ---------------------------------------------------------------------------
export function readJournalById(journalDir, transactionId) {
  return readJournal(journalFilePath(journalDir, transactionId));
}

// ---------------------------------------------------------------------------
// listPendingTransactionIds -- every transaction_id with a PREPARED (not yet
// committed) journal file currently in journalDir. Normally at most one (the
// dispatch lock serializes prepareTransaction calls -- see leases.mjs's
// acquireDispatchLock, which never prepares a new transaction while any are
// pending); more than one is a diagnosable anomaly, not a crash -- each is
// still independently safe to recover (see applyTransaction()'s all-or-nothing
// hash classification).
//
// BX7 adversarial finding: every id returned here is fed straight back into
// journalFilePath() by readJournalById()/recoverPendingTransaction(), which
// THROWS on an id that is not a safe path component. A file in this directory
// whose name this module could never have produced -- the degenerate
// `txn-.json` (which yields the empty id) is the concrete case, but any
// hand-placed or foreign `txn-*.json` will do -- therefore used to escape as a
// raw `unsafe transaction_id ...` Error out of recoverPendingTransaction() ->
// acquireDispatchLock(), wedging EVERY mutating dispatch command on
// unexpected directory content. Same principle as dispatch-lock.mjs's TORN
// lock handling: unreadable/unrecognizable on-disk state is a STATE to
// classify, never a crash. A name journalFilePath() could not have written is
// by definition not one of this module's journal files, so it is not listed as
// pending -- and validate.mjs separately warns about ANY unrecognized file
// under dispatch-journal/, so skipping it here hides nothing.
// ---------------------------------------------------------------------------
export function listPendingTransactionIds(journalDir) {
  if (!existsSync(journalDir)) return [];
  return readdirSync(journalDir)
    .filter((f) => f.startsWith("txn-") && f.endsWith(".json"))
    .map((f) => f.slice("txn-".length, -".json".length))
    .filter(isSafeTransactionId);
}

// ---------------------------------------------------------------------------
// UNREADABLE_JOURNAL / INVALID_JOURNAL / journalRecordProblem -- a journal
// FILE can exist at a well-formed path and still not be a usable journal
// RECORD. Two distinct failure classes (B.2.2, defect D):
//
//   UNREADABLE_JOURNAL -- the record's basic SHAPE is broken: unparseable
//   JSON, not an object, a transaction_id disagreeing with its own filename,
//   a missing intended_file_updates array, or an update missing a string
//   path/content. None of this can be meaningfully validated further.
//
//   INVALID_JOURNAL -- the shape parses fine, but its CONTENT fails
//   integrity or authority checks this module can actually verify: an
//   unrecognized operation, a malformed source_base_sha/canonical_ref_sha,
//   a malformed before_hash/after_hash, content whose sha256 does not match
//   its own recorded after_hash, a duplicate target path within one
//   transaction, or a target path outside the caller-supplied allowlist of
//   real dispatch-state surfaces (leases.json, the frontier files,
//   recovery-log.json). A corrupted or hand-edited journal must never
//   become an arbitrary file-write primitive just because it happens to
//   parse as JSON.
//
// Both are impossible for a record this module actually wrote
// (prepareTransaction() builds the shape+hashes and
// publishJournalRecordExclusive() publishes it atomically), so either one
// means a hand-edit, a foreign file, or a truncated/corrupted write by
// something outside this protocol -- a STATE to classify and surface, never
// a crash and never silently repaired (same principle as dispatch-lock.mjs's
// TORN lock handling).
//
// BX7 adversarial finding (pre-B.2.2): recoverPendingTransaction() used to
// hand whatever it read straight to applyTransaction(), so a broken record
// escaped as a raw TypeError/SyntaxError/`unsafe transaction_id ...
// undefined` Error out of acquireDispatchLock(), wedging EVERY mutating
// dispatch command. Worse, a record whose transaction_id disagreed with its
// own filename would have been applied and would have deleted a DIFFERENT
// transaction's journal file -- re-opening the cross-generation hazard
// B.2.1 exists to close.
//
// `allowedTargetPaths` is optional (a Set of absolute paths) -- when
// provided, every update's path must be a member; when omitted, that
// specific check is skipped (used by this module's own unit self-test,
// which has no real "dispatch state surface" to check against). The real
// orchestration (leases.mjs) always supplies the real one -- see
// runDispatchTransaction()'s ALLOWED_TARGET_PATHS.
export const UNREADABLE_JOURNAL = "UNREADABLE_JOURNAL";
export const INVALID_JOURNAL = "INVALID_JOURNAL";

const GIT_SHA_RE = /^[0-9a-f]{40}$/;
const SHA256_HEX_RE = /^[0-9a-f]{64}$/;

// Mirrors schema/dispatch-journal.schema.json's `operation` enum -- kept in
// sync manually (this module has no schema-validation dependency of its
// own; see validate.mjs for the schema-driven check on committed files).
const KNOWN_OPERATIONS = new Set(["claim", "begin", "complete", "release", "expire-stale", "resolve-recovery"]);

function journalRecordProblem(record, expectedTransactionId, allowedTargetPaths) {
  if (record === null || typeof record !== "object" || Array.isArray(record)) {
    return { status: UNREADABLE_JOURNAL, reason: "journal record is not a JSON object" };
  }
  if (record.transaction_id !== expectedTransactionId) {
    return {
      status: UNREADABLE_JOURNAL,
      reason: `journal record's transaction_id ${JSON.stringify(record.transaction_id)} does not match its own filename (txn-${expectedTransactionId}.json) -- refusing to act on it, since applying it would touch a different transaction's journal file`,
    };
  }
  if (!Array.isArray(record.intended_file_updates)) {
    return { status: UNREADABLE_JOURNAL, reason: "journal record has no intended_file_updates array" };
  }
  for (const update of record.intended_file_updates) {
    if (!update || typeof update.path !== "string" || typeof update.content !== "string") {
      return { status: UNREADABLE_JOURNAL, reason: "journal record has an intended_file_update with no string path/content" };
    }
  }

  // Shape is sound; validate content/integrity/authority.
  if (typeof record.operation !== "string" || !KNOWN_OPERATIONS.has(record.operation)) {
    return { status: INVALID_JOURNAL, reason: `operation ${JSON.stringify(record.operation)} is not one of ${[...KNOWN_OPERATIONS].join(", ")}` };
  }
  if (typeof record.source_base_sha !== "string" || !GIT_SHA_RE.test(record.source_base_sha)) {
    return { status: INVALID_JOURNAL, reason: `source_base_sha ${JSON.stringify(record.source_base_sha)} is not a 40-hex-char string` };
  }
  if (record.canonical_ref_sha !== null && !(typeof record.canonical_ref_sha === "string" && GIT_SHA_RE.test(record.canonical_ref_sha))) {
    return { status: INVALID_JOURNAL, reason: `canonical_ref_sha ${JSON.stringify(record.canonical_ref_sha)} must be null or a 40-hex-char string` };
  }

  const seenPaths = new Set();
  for (const update of record.intended_file_updates) {
    if (seenPaths.has(update.path)) {
      return { status: INVALID_JOURNAL, reason: `duplicate target path within one transaction: ${update.path}` };
    }
    seenPaths.add(update.path);

    if (allowedTargetPaths && !allowedTargetPaths.has(update.path)) {
      return { status: INVALID_JOURNAL, reason: `target path is outside the allowed dispatch-state surface: ${update.path} -- a recovery journal must never become an arbitrary file-write primitive` };
    }
    if (update.before_hash !== null && !(typeof update.before_hash === "string" && SHA256_HEX_RE.test(update.before_hash))) {
      return { status: INVALID_JOURNAL, reason: `before_hash for ${update.path} is not null or a valid sha256 hex string` };
    }
    if (typeof update.after_hash !== "string" || !SHA256_HEX_RE.test(update.after_hash)) {
      return { status: INVALID_JOURNAL, reason: `after_hash for ${update.path} is not a valid sha256 hex string` };
    }
    if (sha256Hex(update.content) !== update.after_hash) {
      return { status: INVALID_JOURNAL, reason: `content for ${update.path} does not hash to its own recorded after_hash -- record integrity violated` };
    }
  }
  return null;
}

// ---------------------------------------------------------------------------
// hasPendingTransaction -- true if any transaction has a PREPARED journal
// file in journalDir.
// ---------------------------------------------------------------------------
export function hasPendingTransaction(journalDir) {
  return listPendingTransactionIds(journalDir).length > 0;
}

// ---------------------------------------------------------------------------
// prepareTransaction -- computes before/after hashes for every target path,
// builds the journal record, and durably, EXCLUSIVELY publishes it to this
// transaction's OWN file (journalDir/txn-<transaction_id>.json) -- see
// publishJournalRecordExclusive() above. THROWS (does not return a
// {ok:false,...} result) with a TRANSACTION_ID_ALREADY_EXISTS-coded Error if
// a journal already exists at that path; the existing file's bytes are left
// completely untouched. Does NOT touch any of the target files themselves --
// only the journal.
// ---------------------------------------------------------------------------
export function prepareTransaction({
  journalDir,
  transactionId,
  operation,
  sourceBaseSha,
  canonicalRefSha = null,
  intendedFileUpdates,
  now,
}) {
  const resolvedUpdates = intendedFileUpdates.map(({ path, content }) => {
    const before = readIfExists(path);
    return {
      path,
      before_hash: before === null ? null : sha256Hex(before),
      after_hash: sha256Hex(content),
      content,
    };
  });

  const record = {
    schema: JOURNAL_SCHEMA,
    transaction_id: transactionId,
    operation,
    source_base_sha: sourceBaseSha,
    canonical_ref_sha: canonicalRefSha,
    started_at: now,
    intended_file_updates: resolvedUpdates,
  };

  mkdirSync(journalDir, { recursive: true });
  publishJournalRecordExclusive(journalFilePath(journalDir, transactionId), `${JSON.stringify(record, null, 2)}\n`);
  return record;
}

// ---------------------------------------------------------------------------
// applyTransaction -- applies journalRecord's intended_file_updates, then
// deletes THIS TRANSACTION'S OWN journal file (derived from
// journalRecord.transaction_id, never from whatever else might be in
// journalDir) as the commit signal.
//
// Per-file classification (B.2.1 -- the DIVERGED case is new; the other two
// already existed):
//   current content hash == after_hash  -> already landed, safe no-op
//   current content hash == before_hash -> this transaction's own
//                                           not-yet-applied step, safe to
//                                           write after_hash's content
//   neither                             -> DIVERGED: some other transaction
//                                           (necessarily a NEWER one, since
//                                           this transaction's own prepare
//                                           recorded before_hash as truth at
//                                           prepare time) has since written
//                                           this path. NEVER overwrite with
//                                           this (now-stale) transaction's
//                                           bytes -- a stale recoverer must
//                                           never roll a newer committed
//                                           state backward. The file is left
//                                           completely untouched.
//
// ALL-OR-NOTHING (BX7 adversarial finding): every update is CLASSIFIED first,
// and target files are only written once the whole set is known to be
// conflict-free. This used to be a single pass that classified and wrote each
// update as it went, so a transaction whose FIRST file was safe-to-apply and
// whose SECOND file had diverged wrote the first file and then returned
// RECOVERY_CONFLICT -- leaving exactly the torn, half-applied multi-file state
// (e.g. a lease recorded in leases.json whose task's frontier status was never
// updated) that this whole module exists to make impossible, and leaving it
// permanently: the retained journal record can never complete, because the
// diverged path stays diverged on every later retry. Reachable in practice
// two ways -- an out-of-protocol edit to one of a multi-file transaction's
// targets (exactly the drift the DIVERGED case is designed for, see GEN-E),
// and the "more than one pending journal" anomaly this module explicitly
// tolerates, where recovering an older pending transaction can find a file a
// newer one already moved. Classifying the whole set before touching anything
// makes the DIVERGED case a true no-op, which is what its doc and
// docs/_machine/system-atlas/README.md already claim.
//
// If ANY path diverged, this transaction's own journal file is intentionally
// NOT deleted either (status: RECOVERY_CONFLICT) -- this should be
// structurally unreachable in normal operation (per-transaction journal
// identity means a stale recoverer never even discovers a newer
// transaction's file, and the newer transaction's OWN before_hash would have
// been read from this transaction's already-applied after_hash, so a
// correctly-operating newer transaction's writes always classify as
// already-landed for a still-live older recoverer, never as diverged -- see
// module header). If it ever happens anyway (e.g. a file was mutated by
// something outside this protocol entirely), silently discarding the
// journal record would erase the only evidence of the conflict; leaving it
// on disk makes `validate.mjs`'s leftover-artifact check surface it for a
// human to inspect, per this pass's "fail closed, not silently converged"
// requirement.
// ---------------------------------------------------------------------------
export function applyTransaction(journalRecord, journalDir) {
  // Resolve (and validate) our own journal file path BEFORE any target file is
  // touched, so an unusable transaction_id can never leave target files
  // written by a transaction that then cannot be committed.
  const ownJournalFilePath = journalFilePath(journalDir, journalRecord.transaction_id);

  // PHASE 1 -- classify every update. Writes nothing.
  const toWrite = [];
  const alreadyAppliedPaths = [];
  const divergedPaths = [];

  for (const update of journalRecord.intended_file_updates) {
    const current = readIfExists(update.path);
    const currentHash = current === null ? null : sha256Hex(current);

    if (currentHash === update.after_hash) {
      alreadyAppliedPaths.push(update.path);
      continue;
    }
    if (currentHash === update.before_hash) {
      toWrite.push(update);
      continue;
    }
    divergedPaths.push({ path: update.path, expected_before_hash: update.before_hash, expected_after_hash: update.after_hash, actual_hash: currentHash });
  }

  if (divergedPaths.length > 0) {
    return { appliedPaths: [], alreadyAppliedPaths, divergedPaths, status: "RECOVERY_CONFLICT" };
  }

  // PHASE 2 -- the whole set is conflict-free; apply it.
  const appliedPaths = [];
  for (const update of toWrite) {
    writeFileDurable(update.path, update.content);
    appliedPaths.push(update.path);
  }

  // Deleting this transaction's own journal file IS the commit signal, so
  // this must be safe against a concurrent recoverer doing the exact same
  // thing, for the exact same transaction, at the exact same moment (see
  // recoverPendingTransaction()'s doc comment: recovering a pending
  // transaction is deliberately allowed without holding the dispatch lock,
  // because -- for the SAME generation -- every write above is idempotent).
  // A bare unlinkSync() with no existence pre-check avoids the
  // existsSync()-then-unlinkSync() TOCTOU: two concurrent same-generation
  // recoverers can both reach this line, and whichever runs second simply
  // gets ENOENT, which is caught here as a successful no-op (the file is
  // gone either way, which is exactly what both callers wanted).
  try {
    unlinkSync(ownJournalFilePath);
  } catch (err) {
    if (err.code !== "ENOENT") throw err;
  }

  return { appliedPaths, alreadyAppliedPaths, divergedPaths: [], status: "COMMITTED" };
}

// ---------------------------------------------------------------------------
// recoverPendingTransaction -- convenience wrapper a caller runs at the
// START of every mutating command, before accepting new work. Discovers
// EVERY currently-pending transaction (normally at most one) and recovers
// each independently and safely -- per-transaction journal identity means
// none of them can interfere with another, even if more than one somehow
// exists at once. Safe to run repeatedly: a call with nothing pending is a
// no-op.
//
// LOCK-FREE BY DESIGN, and this is still safe post-B.2.1: two workers may
// legitimately call this concurrently for the SAME pending transaction
// (acquireDispatchLock() in leases.mjs deliberately recovers before even
// attempting to acquire the lock, so a stuck journal can never deadlock the
// system on a crashed lock holder). What must NEVER happen -- and is now
// structurally prevented rather than merely "usually fine" -- is one
// recoverer's WORK or DELETE reaching a DIFFERENT, newer transaction's
// journal file or target-file state; see applyTransaction()'s doc comment.
// ---------------------------------------------------------------------------
export function recoverPendingTransaction(journalDir, { allowedTargetPaths } = {}) {
  const transactionIds = listPendingTransactionIds(journalDir);
  if (transactionIds.length === 0) {
    return { recovered: false, reason: "no pending journal" };
  }

  const outcomes = transactionIds.map((transactionId) => {
    let record;
    try {
      record = readJournalById(journalDir, transactionId);
    } catch (err) {
      // Unparseable JSON (a hand-edit, a foreign file, a truncated write by
      // something that is not writeFileDurable). See UNREADABLE_JOURNAL below.
      return { transactionId, recovered: false, status: UNREADABLE_JOURNAL, reason: `journal record does not parse: ${err.message}` };
    }
    if (record === null) {
      // `null` is ambiguous and the two meanings must NOT be conflated (BX8
      // adversarial finding): readJournal() returns null both when the file is
      // GONE (a concurrent recoverer already committed this exact transaction
      // between our listPendingTransactionIds() scan and this read -- benign,
      // nothing left to do) and when the file EXISTS and its content is the
      // literal JSON `null`, which parses cleanly. The latter used to be
      // reported as "already cleared", i.e. silently misread as "no pending
      // transaction" -- precisely the failure mode UNREADABLE_JOURNAL exists to
      // prevent. The file stays on disk, so hasPendingTransaction() keeps
      // returning true and acquireDispatchLock() burned every attempt and then
      // blamed phantom worker contention (same misdiagnosis BX7 fixed for
      // RECOVERY_CONFLICT). existsSync() here is not a TOCTOU hazard: both
      // branches are purely diagnostic and neither writes or deletes anything.
      if (existsSync(journalFilePath(journalDir, transactionId))) {
        return {
          transactionId,
          recovered: false,
          status: UNREADABLE_JOURNAL,
          reason: "journal record parses as the literal `null`, not a JSON object -- refusing to act on it or to treat it as an already-committed transaction",
        };
      }
      return { transactionId, recovered: false, reason: "already cleared (raced with another recoverer)" };
    }
    const problem = journalRecordProblem(record, transactionId, allowedTargetPaths);
    if (problem) {
      return { transactionId, recovered: false, status: problem.status, reason: problem.reason };
    }
    const result = applyTransaction(record, journalDir);
    return { transactionId, recovered: result.status === "COMMITTED", ...result };
  });

  return {
    recovered: outcomes.some((o) => o.recovered),
    multiplePending: transactionIds.length > 1,
    outcomes,
  };
}

// ===========================================================================
// Self-test -- uses a real temporary directory for ALL file operations.
// Never touches the real docs/_machine/system-atlas/v0/ files.
// Run with: node tools/system-atlas/dispatch-journal.mjs
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

  const dir = mkdtempSync(join(tmpdir(), "dispatch-journal-selftest-"));
  try {
    const journalDir = join(dir, "journal");
    const NOW = "2026-09-01T00:00:00.000Z";
    const SHA_A = "a".repeat(40);

    // (a) prepare then apply a 2-file transaction from scratch.
    {
      const flatA = join(dir, "a-leases.json");
      const flatB = join(dir, "b-frontier.json");

      const record = prepareTransaction({
        journalDir,
        transactionId: "test-a",
        operation: "claim",
        sourceBaseSha: SHA_A,
        canonicalRefSha: null,
        intendedFileUpdates: [
          { path: flatA, content: '{"leases":[]}\n' },
          { path: flatB, content: '{"tasks":[]}\n' },
        ],
        now: NOW,
      });

      const beforeApplyJournalExists = hasPendingTransaction(journalDir);
      const { appliedPaths, alreadyAppliedPaths, status } = applyTransaction(record, journalDir);
      const afterApplyJournalExists = hasPendingTransaction(journalDir);

      check(
        "(a) prepare+apply from scratch writes both files and clears journal",
        beforeApplyJournalExists === true &&
        afterApplyJournalExists === false &&
        status === "COMMITTED" &&
        appliedPaths.length === 2 &&
        alreadyAppliedPaths.length === 0 &&
        readFileSync(flatA, "utf8") === '{"leases":[]}\n' &&
        readFileSync(flatB, "utf8") === '{"tasks":[]}\n' &&
        record.intended_file_updates[0].before_hash === null &&
        record.intended_file_updates[0].after_hash === sha256Hex('{"leases":[]}\n'),
      );
    }

    // (b) simulate a crash: apply only the first of two updates, then
    // recover the rest via recoverPendingTransaction.
    {
      const flatA = join(dir, "crash-a.json");
      const flatB = join(dir, "crash-b.json");
      writeFileSync(flatA, '{"v":0}\n');
      writeFileSync(flatB, '{"v":0}\n');

      prepareTransaction({
        journalDir,
        transactionId: "test-b",
        operation: "release",
        sourceBaseSha: SHA_A,
        canonicalRefSha: null,
        intendedFileUpdates: [
          { path: flatA, content: '{"v":1}\n' },
          { path: flatB, content: '{"v":1}\n' },
        ],
        now: NOW,
      });

      // Manually apply only the FIRST update, as if a crash happened right
      // after that file's rename but before the second's write.
      writeFileSync(flatA, '{"v":1}\n');
      // flatB stays at its pre-transaction content -- journal is still
      // present (we never called applyTransaction), simulating the crash.

      const journalPresentBeforeRecover = hasPendingTransaction(journalDir);
      const result = recoverPendingTransaction(journalDir);
      const journalPresentAfterRecover = hasPendingTransaction(journalDir);
      const outcome = result.outcomes.find((o) => o.transactionId === "test-b");

      check(
        "(b) recoverPendingTransaction finishes remaining update, clears journal, leaves first file untouched",
        journalPresentBeforeRecover === true &&
        result.recovered === true &&
        outcome.recovered === true &&
        outcome.appliedPaths.includes(flatB) &&
        outcome.alreadyAppliedPaths.includes(flatA) &&
        !outcome.appliedPaths.includes(flatA) &&
        readFileSync(flatA, "utf8") === '{"v":1}\n' &&
        readFileSync(flatB, "utf8") === '{"v":1}\n' &&
        journalPresentAfterRecover === false,
      );
    }

    // (c) running recoverPendingTransaction again immediately (nothing
    // pending) is a safe no-op.
    {
      const result = recoverPendingTransaction(journalDir);
      check(
        "(c) second recoverPendingTransaction with nothing pending is a safe no-op",
        result.recovered === false && result.reason === "no pending journal" && !hasPendingTransaction(journalDir),
      );
    }

    // (d) hasPendingTransaction reports true while journal exists, false
    // once cleared.
    {
      const flatC = join(dir, "d-file.json");
      const record = prepareTransaction({
        journalDir,
        transactionId: "test-d",
        operation: "resolve-recovery",
        sourceBaseSha: SHA_A,
        canonicalRefSha: "b".repeat(40),
        intendedFileUpdates: [{ path: flatC, content: '{"ok":true}\n' }],
        now: NOW,
      });
      const duringPrepared = hasPendingTransaction(journalDir);
      applyTransaction(record, journalDir);
      const afterApplied = hasPendingTransaction(journalDir);
      check("(d) hasPendingTransaction true while prepared, false once cleared", duringPrepared === true && afterApplied === false);
    }

    // (e) readJournalById returns null for a missing/never-existed
    // transaction, and the parsed record for an existing one.
    {
      const missing = readJournalById(journalDir, "does-not-exist");
      const flatE = join(dir, "e-file.json");
      const record = prepareTransaction({
        journalDir,
        transactionId: "test-e",
        operation: "claim",
        sourceBaseSha: SHA_A,
        intendedFileUpdates: [{ path: flatE, content: "x\n" }],
        now: NOW,
      });
      const readBack = readJournalById(journalDir, "test-e");
      applyTransaction(record, journalDir); // cleanup
      check(
        "(e) readJournalById: null for missing transaction, parsed record otherwise",
        missing === null && readBack !== null && readBack.transaction_id === "test-e" && readBack.schema === JOURNAL_SCHEMA,
      );
    }

    // (f) B.2.1 GENERATION SAFETY -- this is the architect-reported ABA
    // regression: a recoverer holding a stale in-memory J1 record must
    // NEVER delete a newer, still-PREPARED J2's own journal file.
    {
      const target = join(dir, "f-target.json");
      writeFileSync(target, '{"state":"pre-J1"}\n');

      const j1 = prepareTransaction({
        journalDir, transactionId: "f-J1", operation: "claim", sourceBaseSha: SHA_A,
        intendedFileUpdates: [{ path: target, content: '{"state":"J1"}\n' }], now: NOW,
      });
      // R2 "loads" J1 into memory here, then pauses (does not call
      // applyTransaction yet) -- simulated by simply holding onto `j1`.
      applyTransaction(j1, journalDir); // J1 committed by someone else (or R2 itself, earlier).

      const j2 = prepareTransaction({
        journalDir, transactionId: "f-J2", operation: "begin", sourceBaseSha: SHA_A,
        intendedFileUpdates: [{ path: target, content: '{"state":"J2"}\n' }], now: NOW,
      });
      const j2PendingBefore = hasPendingTransaction(join(journalDir)) && listPendingTransactionIds(journalDir).includes("f-J2");

      // R2 now resumes with its STALE j1 record.
      const staleResult = applyTransaction(j1, journalDir);

      const j2StillPending = listPendingTransactionIds(journalDir).includes("f-J2");
      const j2RecordIntact = readJournalById(journalDir, "f-J2")?.transaction_id === "f-J2";

      check(
        "(f) stale recoverer with an old in-memory record NEVER deletes a newer, still-PREPARED transaction's journal file",
        j2PendingBefore === true &&
        staleResult.status === "COMMITTED" && // J1's own file: already gone (ENOENT-caught no-op), which IS success for J1
        j2StillPending === true &&
        j2RecordIntact === true,
      );

      applyTransaction(j2, journalDir); // cleanup
    }

    // (g) B.2.1 GENERATION SAFETY -- a stale recoverer must never revert
    // state that a newer transaction has since fully committed.
    {
      const target = join(dir, "g-target.json");
      writeFileSync(target, '{"state":"pre-J1"}\n');

      const j1 = prepareTransaction({
        journalDir, transactionId: "g-J1", operation: "claim", sourceBaseSha: SHA_A,
        intendedFileUpdates: [{ path: target, content: '{"state":"J1"}\n' }], now: NOW,
      });
      applyTransaction(j1, journalDir);

      const j2 = prepareTransaction({
        journalDir, transactionId: "g-J2", operation: "begin", sourceBaseSha: SHA_A,
        intendedFileUpdates: [{ path: target, content: '{"state":"J2"}\n' }], now: NOW,
      });
      applyTransaction(j2, journalDir); // J2 fully commits.

      const staleResult = applyTransaction(j1, journalDir); // R2 resumes with stale J1.
      const finalState = readFileSync(target, "utf8");

      check(
        "(g) stale recoverer never reverts state a newer transaction already committed -- fails closed with RECOVERY_CONFLICT instead",
        staleResult.status === "RECOVERY_CONFLICT" &&
        staleResult.divergedPaths.length === 1 &&
        staleResult.divergedPaths[0].path === target &&
        finalState === '{"state":"J2"}\n',
      );
    }

    // (h) BX7 REGRESSION -- a multi-file transaction where an EARLIER update is
    // safe-to-apply and a LATER one has DIVERGED must write NOTHING. Before the
    // all-or-nothing classify-then-apply split, applyTransaction() wrote the
    // earlier file and only then discovered the divergence, leaving a
    // permanently torn half-applied transaction (the retained journal record can
    // never complete, because the diverged path stays diverged forever) -- the
    // exact multi-file inconsistency this module exists to prevent.
    {
      const first = join(dir, "h-first.json"); // stands in for leases.json
      const second = join(dir, "h-second.json"); // stands in for a frontier file
      writeFileSync(first, '{"v":0}\n');
      writeFileSync(second, '{"v":0}\n');

      const record = prepareTransaction({
        journalDir, transactionId: "h-torn", operation: "claim", sourceBaseSha: SHA_A,
        intendedFileUpdates: [
          { path: first, content: '{"v":1}\n' },
          { path: second, content: '{"v":1}\n' },
        ],
        now: NOW,
      });

      // Crash before ANY write, then the SECOND target diverges (an
      // out-of-protocol edit, or a newer transaction that touched only it).
      writeFileSync(second, '{"v":"moved-on-by-something-else"}\n');

      const result = applyTransaction(record, journalDir);

      check(
        "(h) BX7: a diverged LATER update makes the WHOLE transaction a no-op -- the earlier, still-appliable file is never half-written",
        result.status === "RECOVERY_CONFLICT" &&
        result.appliedPaths.length === 0 &&
        result.divergedPaths.length === 1 &&
        result.divergedPaths[0].path === second &&
        readFileSync(first, "utf8") === '{"v":0}\n' &&
        readFileSync(second, "utf8") === '{"v":"moved-on-by-something-else"}\n' &&
        listPendingTransactionIds(journalDir).includes("h-torn"),
      );

      // Clearing the divergence lets the SAME journal record still complete in
      // full -- proving the no-op left the transaction recoverable, not stuck.
      writeFileSync(second, '{"v":0}\n');
      const retry = applyTransaction(record, journalDir);
      check(
        "(h2) BX7: once the divergence clears, the untouched transaction still applies in full",
        retry.status === "COMMITTED" && retry.appliedPaths.length === 2 &&
        readFileSync(first, "utf8") === '{"v":1}\n' && readFileSync(second, "utf8") === '{"v":1}\n' &&
        !listPendingTransactionIds(journalDir).includes("h-torn"),
      );
    }

    // (i) BX7 REGRESSION -- a file under journalDir whose name this module
    // could never have written (the degenerate `txn-.json`, yielding an empty
    // transaction_id) must not be listed as pending and must not throw a raw
    // `unsafe transaction_id` Error out of recovery. It used to escape all the
    // way out of recoverPendingTransaction() -> acquireDispatchLock(), wedging
    // EVERY mutating dispatch command on unexpected directory content.
    {
      const flatI = join(dir, "i-file.json");
      const record = prepareTransaction({
        journalDir, transactionId: "i-real", operation: "claim", sourceBaseSha: SHA_A,
        intendedFileUpdates: [{ path: flatI, content: '{"i":1}\n' }],
        now: NOW,
      });
      writeFileSync(join(journalDir, "txn-.json"), "{}\n");
      writeFileSync(join(journalDir, "txn-not a safe id.json"), "{}\n");

      const listed = listPendingTransactionIds(journalDir);
      let threw = null;
      let result;
      try {
        result = recoverPendingTransaction(journalDir);
      } catch (err) {
        threw = err.message;
      }
      if (threw) console.log(`  detail: recoverPendingTransaction threw ${threw}`);

      check(
        "(i) BX7: an unrecognizable txn-*.json filename is skipped, never fed back as a transaction_id and never thrown on",
        threw === null &&
        listed.includes("i-real") &&
        !listed.includes("") &&
        !listed.includes("not a safe id") &&
        result.outcomes.some((o) => o.transactionId === "i-real" && o.recovered === true) &&
        readFileSync(flatI, "utf8") === '{"i":1}\n',
      );
      unlinkSync(join(journalDir, "txn-.json"));
      unlinkSync(join(journalDir, "txn-not a safe id.json"));
      if (record && listPendingTransactionIds(journalDir).includes("i-real")) applyTransaction(record, journalDir);
    }

    // (j) B.2.2 GEN-IDENTITY -- journal publication is EXCLUSIVE: a second
    // prepareTransaction() call reusing an existing transaction_id must
    // throw TRANSACTION_ID_ALREADY_EXISTS, never silently overwrite the
    // first record. This is the structural fix for defect C (previously
    // publication used rename-over-existing, the same TOCTOU-prone pattern
    // this codebase already fixed for the lock file in dispatch-lock.mjs).
    {
      const targetJ = join(dir, "j-target.json");
      writeFileSync(targetJ, '{"state":"original"}\n');
      const first = prepareTransaction({
        journalDir, transactionId: "collision-id", operation: "claim", sourceBaseSha: SHA_A,
        intendedFileUpdates: [{ path: targetJ, content: '{"state":"FIRST"}\n' }], now: NOW,
      });
      const firstBytesOnDisk = readFileSync(join(journalDir, "txn-collision-id.json"), "utf8");

      let threw = null;
      try {
        prepareTransaction({
          journalDir, transactionId: "collision-id", operation: "release", sourceBaseSha: SHA_A,
          intendedFileUpdates: [{ path: targetJ, content: '{"state":"SECOND-SHOULD-NEVER-LAND"}\n' }], now: NOW,
        });
      } catch (err) {
        threw = err;
      }

      const afterBytesOnDisk = readFileSync(join(journalDir, "txn-collision-id.json"), "utf8");
      const scratchLeftBehind = readdirSync(journalDir).filter((f) => f.endsWith(".tmp"));

      check(
        "(j) prepareTransaction() with a colliding transaction_id throws TRANSACTION_ID_ALREADY_EXISTS, original journal bytes untouched, no scratch artifact left, no target file changed",
        threw !== null &&
        threw.code === TRANSACTION_ID_ALREADY_EXISTS &&
        /TRANSACTION_ID_ALREADY_EXISTS/.test(threw.message) &&
        afterBytesOnDisk === firstBytesOnDisk &&
        JSON.parse(afterBytesOnDisk).operation === "claim" && // still the FIRST record, not overwritten by "release"
        scratchLeftBehind.length === 0 &&
        readFileSync(targetJ, "utf8") === '{"state":"original"}\n', // prepare never touches target files at all
      );

      applyTransaction(first, journalDir); // cleanup
    }

    // (k) B.2.2 defect D -- full journal-record INTEGRITY validation matrix.
    // Each sub-case hand-writes a record this module could never itself
    // have produced (prepareTransaction() always builds a valid one), then
    // proves: recoverPendingTransaction() never throws a raw error, always
    // classifies it (INVALID_JOURNAL for content-level problems), never
    // writes anything, and never touches the malformed file's own bytes.
    // A single legitimate "victim" transaction proves recovery of a REAL
    // pending transaction still proceeds unaffected by the bad ones sitting
    // alongside it.
    {
      const allowedA = join(dir, "k-allowed-a.json");
      const allowedB = join(dir, "k-allowed-b.json");
      writeFileSync(allowedA, '{"v":0}\n');
      writeFileSync(allowedB, '{"v":0}\n');
      const allowedTargetPaths = new Set([allowedA, allowedB]);

      const validContentA = '{"v":1}\n';
      const validAfterHashA = sha256Hex(validContentA);
      const validBeforeHashA = sha256Hex('{"v":0}\n');

      function writeRawJournal(transactionId, record) {
        writeFileSync(join(journalDir, `txn-${transactionId}.json`), `${JSON.stringify(record, null, 2)}\n`);
      }

      // A: content does not hash to its own recorded after_hash.
      writeRawJournal("k-bad-hash", {
        schema: JOURNAL_SCHEMA, transaction_id: "k-bad-hash", operation: "claim", source_base_sha: SHA_A, canonical_ref_sha: null, started_at: NOW,
        intended_file_updates: [{ path: allowedA, before_hash: validBeforeHashA, after_hash: "0".repeat(64), content: validContentA }],
      });
      // B: malformed before_hash (not null, not a valid sha256 hex string).
      writeRawJournal("k-bad-before-hash", {
        schema: JOURNAL_SCHEMA, transaction_id: "k-bad-before-hash", operation: "claim", source_base_sha: SHA_A, canonical_ref_sha: null, started_at: NOW,
        intended_file_updates: [{ path: allowedA, before_hash: "not-a-real-hash", after_hash: validAfterHashA, content: validContentA }],
      });
      // C: duplicate target path within one transaction.
      writeRawJournal("k-duplicate-target", {
        schema: JOURNAL_SCHEMA, transaction_id: "k-duplicate-target", operation: "claim", source_base_sha: SHA_A, canonical_ref_sha: null, started_at: NOW,
        intended_file_updates: [
          { path: allowedA, before_hash: validBeforeHashA, after_hash: validAfterHashA, content: validContentA },
          { path: allowedA, before_hash: validBeforeHashA, after_hash: validAfterHashA, content: validContentA },
        ],
      });
      // D: target outside the allowed dispatch-state surface (a plausible-
      // looking but never-authorized repo-relative-ish path).
      writeRawJournal("k-target-outside-allowlist", {
        schema: JOURNAL_SCHEMA, transaction_id: "k-target-outside-allowlist", operation: "claim", source_base_sha: SHA_A, canonical_ref_sha: null, started_at: NOW,
        intended_file_updates: [{ path: join(dir, "crates-fixture", "Cargo.toml"), before_hash: null, after_hash: sha256Hex("[package]\n"), content: "[package]\n" }],
      });
      // E: absolute, completely unrelated target (the classic "arbitrary
      // file write" attempt this validation exists to prevent).
      writeRawJournal("k-absolute-unrelated-target", {
        schema: JOURNAL_SCHEMA, transaction_id: "k-absolute-unrelated-target", operation: "claim", source_base_sha: SHA_A, canonical_ref_sha: null, started_at: NOW,
        intended_file_updates: [{ path: "/etc/passwd", before_hash: null, after_hash: sha256Hex("pwned\n"), content: "pwned\n" }],
      });
      // A legitimate, real pending transaction that must be unaffected.
      const victim = prepareTransaction({
        journalDir, transactionId: "k-victim", operation: "claim", sourceBaseSha: SHA_A, canonicalRefSha: null,
        intendedFileUpdates: [{ path: allowedB, content: '{"v":"victim-applied"}\n' }], now: NOW,
      });

      const bytesBefore = Object.fromEntries(
        ["k-bad-hash", "k-bad-before-hash", "k-duplicate-target", "k-target-outside-allowlist", "k-absolute-unrelated-target"]
          .map((id) => [id, readFileSync(join(journalDir, `txn-${id}.json`), "utf8")]),
      );

      let threw = null;
      let result;
      try {
        result = recoverPendingTransaction(journalDir, { allowedTargetPaths });
      } catch (err) {
        threw = err; // (G) must never throw a raw error
      }

      const byId = threw ? {} : Object.fromEntries(result.outcomes.map((o) => [o.transactionId, o]));
      const bytesAfter = Object.fromEntries(
        Object.keys(bytesBefore).map((id) => [id, readFileSync(join(journalDir, `txn-${id}.json`), "utf8")]),
      );

      check(
        "(k-A) content hash mismatch -> INVALID_JOURNAL, no writes",
        threw === null && byId["k-bad-hash"]?.status === INVALID_JOURNAL && /after_hash/.test(byId["k-bad-hash"].reason),
      );
      check(
        "(k-B) malformed before_hash -> INVALID_JOURNAL",
        byId["k-bad-before-hash"]?.status === INVALID_JOURNAL && /before_hash/.test(byId["k-bad-before-hash"].reason),
      );
      check(
        "(k-C) duplicate target path -> INVALID_JOURNAL",
        byId["k-duplicate-target"]?.status === INVALID_JOURNAL && /duplicate target path/.test(byId["k-duplicate-target"].reason),
      );
      check(
        "(k-D) target outside the allowed dispatch-state surface -> INVALID_JOURNAL",
        byId["k-target-outside-allowlist"]?.status === INVALID_JOURNAL && /outside the allowed/.test(byId["k-target-outside-allowlist"].reason),
      );
      check(
        "(k-E) absolute, unrelated target (/etc/passwd) -> INVALID_JOURNAL, never attempted as a write",
        byId["k-absolute-unrelated-target"]?.status === INVALID_JOURNAL && /outside the allowed/.test(byId["k-absolute-unrelated-target"].reason),
      );
      check(
        "(k-G) recoverPendingTransaction() never throws a raw error for any of the above -- always an explicit, classified diagnostic",
        threw === null,
      );
      check(
        "(k-H) none of the malformed journal files' own bytes were deleted or rewritten by validation failure",
        Object.keys(bytesBefore).every((id) => bytesAfter[id] === bytesBefore[id]),
      );
      check(
        "(k-victim) a REAL legitimate pending transaction alongside 5 malformed ones still recovers normally",
        byId["k-victim"]?.recovered === true && readFileSync(allowedB, "utf8") === '{"v":"victim-applied"}\n',
      );
      check(
        "(k-untouched) the allowed target files that the bad records claimed to write are completely unchanged",
        readFileSync(allowedA, "utf8") === '{"v":0}\n',
      );

      // cleanup: leave only what fixtures normally leave (nothing) -- delete
      // the 5 malformed records directly (recovery deliberately never does).
      for (const id of Object.keys(bytesBefore)) unlinkSync(join(journalDir, `txn-${id}.json`));
    }

    // (l) BX8 REGRESSION -- a journal file whose content is the literal JSON
    // `null` parses cleanly, so readJournalById() returns null, which
    // recoverPendingTransaction() used to interpret as "already cleared (raced
    // with another recoverer)" -- silently misreading an unusable record as a
    // committed transaction, exactly what UNREADABLE_JOURNAL exists to prevent.
    // The file is still there afterwards, so the misclassification also wedged
    // acquireDispatchLock() into blaming phantom worker contention. It must be
    // classified UNREADABLE_JOURNAL, and a genuinely VANISHED file must still
    // be reported as the benign "already cleared" race it is.
    {
      writeFileSync(join(journalDir, "txn-l-nulled.json"), "null\n");
      const nulled = recoverPendingTransaction(journalDir);
      const outcome = nulled.outcomes.find((o) => o.transactionId === "l-nulled");
      check(
        "(l) BX8: a journal file containing the literal `null` is UNREADABLE_JOURNAL, never mistaken for an already-committed transaction",
        outcome?.status === UNREADABLE_JOURNAL &&
        outcome.recovered === false &&
        /literal `null`/.test(outcome.reason) &&
        readFileSync(join(journalDir, "txn-l-nulled.json"), "utf8") === "null\n", // never deleted or rewritten
      );
      unlinkSync(join(journalDir, "txn-l-nulled.json"));

      // (l2) the OTHER meaning of a null read is preserved -- a transaction
      // whose file genuinely vanished (its own owner committed it between a
      // recoverer's directory scan and its read) must still read back as a
      // plain null, so that branch stays the benign already-cleared race
      // rather than an unreadable journal. (The full concurrent version runs
      // for real in dispatch-orchestration.test.mjs's DIFFERENT-non-overlapping
      // -tasks child-process test; here we only pin the primitive it rests on.)
      const flatL = join(dir, "l-file.json");
      const record = prepareTransaction({
        journalDir, transactionId: "l-vanishes", operation: "claim", sourceBaseSha: SHA_A,
        intendedFileUpdates: [{ path: flatL, content: '{"l":1}\n' }], now: NOW,
      });
      const listedWhilePending = listPendingTransactionIds(journalDir).includes("l-vanishes");
      applyTransaction(record, journalDir); // its owner finishes it independently
      check(
        "(l2) BX8: a vanished journal file still reads back as a plain null (benign already-cleared race), distinct from a file that CONTAINS null",
        listedWhilePending === true &&
        readJournalById(journalDir, "l-vanishes") === null &&
        !existsSync(join(journalDir, "txn-l-vanishes.json")),
      );
    }

    console.log(`\n${pass} passed, ${fail} failed`);
    process.exitCode = fail > 0 ? 1 : 0;
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
}

if (process.argv[1] === fileURLToPath(import.meta.url)) {
  runSelfTest();
}
