#!/usr/bin/env node
// Coordinator-only: deterministic, complete Atlas validation. Exits non-zero
// on any violation. Every instance is validated against its real JSON Schema
// (schema-validate.mjs) -- no more hand-rolled partial enum checks -- plus
// the cross-cutting invariants (dependency/blocked_by closure, owned/
// forbidden-path overlap, shared-file conflicts, risk requirements,
// cross-account verification, wave proves_* re-derivation, lease
// consistency, source_base_sha agreement) that schema validation alone
// can't express.
import { existsSync, readdirSync } from "node:fs";
import { resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { analyzeWave, riskRequirements } from "./dependency-safety.mjs";
import { DEFAULT_JOURNAL_DIR, listPendingTransactionIds } from "./dispatch-journal.mjs";
import { DEFAULT_LOCK_PATH } from "./dispatch-lock.mjs";
import { retireDispatchGate } from "./hygiene-gate.mjs";
import { REPO_ROOT, productSourceSha, readJson, repoRel } from "./lib.mjs";
import { validateMutationOwnership } from "./mutation-ownership.mjs";
import { RECOVERY_RESOLUTIONS } from "./recovery.mjs";
import { compileSchemas, formatErrors, validate as schemaValidate } from "./schema-validate.mjs";

// 2026-09-16: schemas are hand-authored source, not generated state -- they must be a permanent,
// committed part of the repo, but docs/_machine/ is a forbidden documentation root (never
// committed; see tools/docs/architecture-registry.mjs's forbiddenDocumentationRoots) for exactly
// the reason the repo owner already wiped the entire docs/_machine/system-atlas/ tree (schemas
// included) in commit 308114899b. Moved to tools/system-atlas/schema/, alongside the rest of this
// tool's own source, instead of restoring them to the forbidden path.
const SCHEMA_DIR = resolve(REPO_ROOT, "tools/system-atlas/schema");
const SHARD_DIR = resolve(REPO_ROOT, "docs/_machine/system-atlas/v0/shards");
const OUT_DIR = resolve(REPO_ROOT, "docs/_machine/system-atlas/v0");
const RECOVERY_LOG_PATH = resolve(OUT_DIR, "recovery-log.json");

// Tasks that must never be wave-eligible under any circumstance -- defense
// in depth alongside build-dispatch-plan.mjs's own eligibility filter (which
// already only selects status==="READY"). A wave referencing any of these is
// evidence the planner (or a hand-edited dispatch-plan.json) drifted from
// the real state machine, not something a re-derivation of proves_* alone
// would necessarily catch.
const NEVER_WAVE_ELIGIBLE_STATUSES = new Set(["LEASED", "IN_PROGRESS", "RECOVERY_REQUIRED", "DESIGN_REQUIRED", "BLOCKED_DEPENDENCY", "BLOCKED_ENV", "BLOCKED_SEMANTICS", "FAILED"]);

const errors = [];
const warn = (msg) => errors.push(msg);

function checkInstance(label, instance, schemaId, schemas) {
  const { valid, errors: schemaErrors } = schemaValidate(instance, schemaId, schemas);
  if (!valid) warn(`${label} failed schema validation (${schemaId}):\n${formatErrors(schemaErrors).split("\n").map((l) => `    ${l}`).join("\n")}`);
}

function checkShards(schemas, expectedSourceBaseSha) {
  const idsByNode = new Map(); // id -> lane
  const shardFiles = readdirSync(SHARD_DIR).filter((f) => f.endsWith(".json"));
  for (const file of shardFiles) {
    const shard = readJson(resolve(SHARD_DIR, file));
    checkInstance(`shards/${file}`, shard, "chronica.system-atlas.shard.v0", schemas);
    if (shard.source_base_sha !== expectedSourceBaseSha) {
      warn(`shards/${file}: source_base_sha (${shard.source_base_sha}) != current product snapshot (${expectedSourceBaseSha})`);
    }
    for (const n of shard.nodes ?? []) {
      checkInstance(`shards/${file} node ${n.id}`, n, "chronica.system-atlas.node.v0", schemas);
      if (idsByNode.has(n.id) && idsByNode.get(n.id) !== shard.lane) {
        warn(`DUPLICATE NODE ID across shards: ${n.id} in both ${idsByNode.get(n.id)} and ${shard.lane}`);
      }
      idsByNode.set(n.id, shard.lane);
      if (n.kind === "PLANNED_SYSTEM" && n.source_class !== "TARGET_PROPOSAL" && n.existence === "ABSENT") {
        warn(`shards/${file}: node ${n.id} is PLANNED_SYSTEM/ABSENT but source_class is ${n.source_class}, not TARGET_PROPOSAL`);
      }
    }
    for (const e of shard.edges ?? []) {
      checkInstance(`shards/${file} edge ${e.from}->${e.to}`, e, "chronica.system-atlas.edge.v0", schemas);
    }
  }
  return idsByNode;
}

function checkAggregates(schemas, allNodeIds) {
  const currentMap = readJson(resolve(OUT_DIR, "current-map.json"));
  for (const n of currentMap.nodes) {
    checkInstance(`current-map.json node ${n.id}`, n, "chronica.system-atlas.node.v0", schemas);
    if (n.source_class === "TARGET_PROPOSAL") warn(`current-map.json contains a TARGET_PROPOSAL node represented as current truth: ${n.id}`);
  }
  for (const e of currentMap.edges) {
    if (!allNodeIds.has(e.from)) warn(`current-map.json: edge references nonexistent node ${e.from}`);
    if (!allNodeIds.has(e.to) && !e.to.startsWith("runtime:")) warn(`current-map.json: edge references nonexistent node ${e.to}`);
  }

  const targetMap = readJson(resolve(OUT_DIR, "target-map.json"));
  for (const n of targetMap.nodes) {
    if (n.source_class !== "TARGET_PROPOSAL" && n.kind !== "PLANNED_SYSTEM") {
      warn(`target-map.json contains a node without source_class=TARGET_PROPOSAL: ${n.id}`);
    }
  }

  const snapshot = readJson(resolve(OUT_DIR, "snapshot.json"));
  checkInstance("snapshot.json", snapshot, "chronica.system-atlas.snapshot.v0", schemas);
  if (snapshot.duplicate_node_ids?.length) warn(`snapshot.json reports duplicate node ids: ${JSON.stringify(snapshot.duplicate_node_ids)}`);
  return snapshot;
}

function checkFrontiers(schemas, allNodeIds, expectedSourceBaseSha) {
  const seenTaskIds = new Set();
  const allTasks = [];
  const tasksById = new Map();
  for (const name of ["repair-frontier", "build-frontier", "retire-frontier", "verify-frontier"]) {
    const frontier = readJson(resolve(OUT_DIR, `${name}.json`));
    if (frontier.source_base_sha !== expectedSourceBaseSha) {
      warn(`${name}.json: source_base_sha (${frontier.source_base_sha}) != current product snapshot (${expectedSourceBaseSha})`);
    }
    for (const t of frontier.tasks) {
      checkInstance(`${name}.json task ${t.task_id}`, t, "chronica.system-atlas.task.v0", schemas);
      if (seenTaskIds.has(t.task_id)) warn(`DUPLICATE TASK ID: ${t.task_id} (in ${name}.json)`);
      seenTaskIds.add(t.task_id);
      for (const nid of t.node_ids) {
        if (!allNodeIds.has(nid)) warn(`${name}.json: task ${t.task_id} references nonexistent node_id ${nid}`);
      }
      for (const violation of riskRequirements(t)) warn(`${name}.json: task ${t.task_id} risk requirement violated: ${violation}`);
      for (const violation of validateMutationOwnership(t)) warn(`${name}.json: ${violation}`);
      if (t.action === "RETIRE" && (t.status === "READY" || t.status === "PLANNED" || t.status === "LEASED" || t.status === "IN_PROGRESS")) {
        const gate = retireDispatchGate(t);
        if (!gate.allowed) warn(`${name}.json: task ${t.task_id} is RETIRE in dispatchable status ${t.status} without hygiene clearance: ${gate.reason}`);
      }
      allTasks.push(t);
      tasksById.set(t.task_id, t);
    }
  }
  return { allTasks, tasksById };
}

// Item 8/9: re-derive every wave's proves_* independently and fail if the
// stored value doesn't match what analyzeWave() computes right now -- this
// is what actually prevents a hardcoded `true` (or a stale one after the
// frontiers changed) from silently passing validation.
function checkDispatchPlan(schemas, tasksById) {
  const plan = readJson(resolve(OUT_DIR, "dispatch-plan.json"));
  checkInstance("dispatch-plan.json", plan, "chronica.system-atlas.dispatch.v0", schemas);
  for (const wave of plan.waves) {
    const recomputed = analyzeWave(wave, tasksById);
    if (recomputed.provesDependencySafe !== wave.proves_dependency_safe) {
      warn(`dispatch-plan.json: wave ${wave.wave_id} stores proves_dependency_safe=${wave.proves_dependency_safe} but re-derivation gives ${recomputed.provesDependencySafe}`);
    }
    if (recomputed.provesNoOwnedPathOverlap !== wave.proves_no_owned_path_overlap) {
      warn(`dispatch-plan.json: wave ${wave.wave_id} stores proves_no_owned_path_overlap=${wave.proves_no_owned_path_overlap} but re-derivation gives ${recomputed.provesNoOwnedPathOverlap}`);
    }
    if (wave.proves_dependency_safe === true && wave.proves_no_owned_path_overlap === true) {
      // A wave claiming full safety must have zero unsafe entries in its own stored report.
      const r = wave.safety_report ?? {};
      if ((r.cycles?.length ?? 0) || (r.blockedByCycles?.length ?? 0) || (r.ownedPathOverlaps?.length ?? 0) || (r.forbiddenPathOverlaps?.length ?? 0) || (r.sharedFileConflicts?.length ?? 0)) {
        warn(`dispatch-plan.json: wave ${wave.wave_id} claims full safety but its stored safety_report still lists violations`);
      }
    }
    // Wave inclusion must never itself imply LEASED, and no task in any
    // never-wave-eligible status (LEASED/IN_PROGRESS/RECOVERY_REQUIRED/
    // DESIGN_REQUIRED/BLOCKED_*/FAILED) may appear in a wave at all -- a
    // RECOVERY_REQUIRED task in particular must never be silently
    // re-dispatched; it can only leave that status via recovery.mjs's
    // resolveRecovery(), never via wave inclusion.
    for (const taskIds of Object.values(wave.accounts)) {
      for (const taskId of taskIds) {
        const t = tasksById.get(taskId);
        if (!t) {
          warn(`dispatch-plan.json: wave ${wave.wave_id} references unknown task ${taskId}`);
        } else if (NEVER_WAVE_ELIGIBLE_STATUSES.has(t.status)) {
          warn(`dispatch-plan.json: wave ${wave.wave_id} task ${taskId} has status ${t.status} -- never wave-eligible (only a real claim via leases.mjs, or -- for RECOVERY_REQUIRED -- recovery.mjs's resolveRecovery(), may move a task out of this status)`);
        }
      }
    }
  }
}

// Item 24: dispatch.lock and any file under dispatch-journal/ are RUNTIME
// artifacts only -- a real worker always clears both before it exits
// cleanly (see leases.mjs's runDispatchTransaction()/acquireDispatchLock()).
// Either one present in the committed tree means a transaction crashed and
// was never rolled forward/reconciled, or the artifact was mistakenly
// committed; both must fail the validator closed rather than silently pass.
// Also flags any leftover `*.tmp` scratch file (B.2.1: these can appear
// either directly under OUT_DIR, for a leases.json/frontier-file write, or
// inside dispatch-journal/ itself, for a journal-record write -- see
// dispatch-journal.mjs's writeFileDurable()) -- evidence of an interrupted
// durable write.
function checkNoLeftoverRuntimeArtifacts() {
  if (existsSync(DEFAULT_LOCK_PATH)) {
    warn(`${repoRel(DEFAULT_LOCK_PATH)} exists in the working tree -- a dispatch lock must never be committed or left behind; this means a transaction did not clean up (crashed worker whose lock has not yet been reclaimed, or an accidental commit)`);
  }
  const pendingTransactionIds = listPendingTransactionIds(DEFAULT_JOURNAL_DIR);
  if (pendingTransactionIds.length > 0) {
    warn(`${repoRel(DEFAULT_JOURNAL_DIR)} contains ${pendingTransactionIds.length} PREPARED (uncommitted) dispatch journal file(s) [${pendingTransactionIds.join(", ")}] -- these must never be committed or left behind unrecovered; run any leases.mjs mutating command (it recovers every pending transaction before doing anything else) or recover them directly`);
  }
  for (const dir of [OUT_DIR, DEFAULT_JOURNAL_DIR]) {
    if (!existsSync(dir)) continue;
    for (const file of readdirSync(dir)) {
      if (file.endsWith(".tmp")) {
        warn(`${repoRel(resolve(dir, file))}: leftover .tmp file in ${repoRel(dir)} -- evidence of an interrupted durable write`);
      }
    }
  }
  // BX7 adversarial finding: listPendingTransactionIds() deliberately ignores
  // any file under dispatch-journal/ whose name dispatch-journal.mjs could not
  // itself have produced (feeding such a name back through journalFilePath()
  // used to throw a raw Error out of recovery and wedge every mutating dispatch
  // command). Ignoring it there must not mean ignoring it everywhere: a clean
  // tree has ZERO files under dispatch-journal/, so anything not already
  // reported above is unexplained and must be surfaced here rather than sit
  // there invisibly.
  if (existsSync(DEFAULT_JOURNAL_DIR)) {
    const explained = new Set(pendingTransactionIds.map((id) => `txn-${id}.json`));
    for (const file of readdirSync(DEFAULT_JOURNAL_DIR)) {
      if (explained.has(file) || file.endsWith(".tmp")) continue;
      warn(`${repoRel(resolve(DEFAULT_JOURNAL_DIR, file))}: unrecognized file under ${repoRel(DEFAULT_JOURNAL_DIR)} -- this directory holds only txn-<transaction_id>.json records written by dispatch-journal.mjs, and a clean tree has none; it is NOT treated as a recoverable transaction, so inspect and remove it by hand`);
    }
  }
}

// Item 24: recovery-log.json (if present) must validate against its schema,
// every entry must reference a real task_id, and resolution must be one of
// recovery.mjs's own narrow RECOVERY_RESOLUTIONS -- never a hand-typed
// string that happens to look right.
function checkRecoveryLog(schemas, tasksById) {
  if (!existsSync(RECOVERY_LOG_PATH)) return;
  const log = readJson(RECOVERY_LOG_PATH);
  if (!Array.isArray(log)) {
    warn("recovery-log.json is not an array");
    return;
  }
  for (const entry of log) {
    checkInstance(`recovery-log.json entry ${entry.recovery_id ?? "?"}`, entry, "chronica.system-atlas.recovery-record.v0", schemas);
    if (entry.task_id && !tasksById.has(entry.task_id)) {
      warn(`recovery-log.json: entry ${entry.recovery_id ?? "?"} references unknown task ${entry.task_id}`);
    }
    if (entry.resolution && !RECOVERY_RESOLUTIONS.includes(entry.resolution)) {
      warn(`recovery-log.json: entry ${entry.recovery_id ?? "?"} has resolution ${entry.resolution}, not one of ${RECOVERY_RESOLUTIONS.join(", ")}`);
    }
  }
}

// Item 6/7: leases.json (if present) must validate, and every LEASED task
// must have a corresponding ACTIVE lease -- and vice versa, no orphan lease
// claiming a task that isn't actually marked LEASED.
function checkLeases(schemas, tasksById) {
  const path = resolve(OUT_DIR, "leases.json");
  if (!existsSync(path)) return; // v0.1: leases.json is optional until a worker claims something
  const leases = readJson(path);
  if (!Array.isArray(leases)) {
    warn("leases.json is not an array");
    return;
  }
  const activeByTask = new Map();
  const seenLeaseIds = new Set();
  for (const lease of leases) {
    checkInstance(`leases.json lease ${lease.lease_id ?? "?"}`, lease, "chronica.system-atlas.lease.v0", schemas);
    if (lease.lease_id) {
      if (seenLeaseIds.has(lease.lease_id)) warn(`leases.json: duplicate lease_id ${lease.lease_id}`);
      seenLeaseIds.add(lease.lease_id);
    }
    if (lease.status === "ACTIVE") {
      if (activeByTask.has(lease.task_id)) warn(`leases.json: task ${lease.task_id} has more than one ACTIVE lease`);
      activeByTask.set(lease.task_id, lease);
      const task = tasksById.get(lease.task_id);
      if (task && lease.source_base_sha !== task.source_base_sha) {
        warn(`leases.json: lease ${lease.lease_id} source_base_sha (${lease.source_base_sha}) != task ${lease.task_id}'s own source_base_sha (${task.source_base_sha})`);
      }
    }
  }
  for (const [taskId, task] of tasksById) {
    const hasActiveLease = activeByTask.has(taskId);
    if (task.status === "LEASED" && !hasActiveLease) warn(`task ${taskId} has status LEASED but no ACTIVE lease exists in leases.json`);
    if (hasActiveLease && task.status !== "LEASED" && task.status !== "IN_PROGRESS") {
      warn(`task ${taskId} has an ACTIVE lease but frontier status is ${task.status}, not LEASED/IN_PROGRESS`);
    }
    // A RECOVERY_REQUIRED task must never have a currently-ACTIVE lease --
    // resolveRecovery() already refuses to create one over an existing
    // unexpired ACTIVE lease, but the validator checks this independently,
    // as a system-wide invariant rather than trusting one call site's logic.
    if (task.status === "RECOVERY_REQUIRED" && hasActiveLease) {
      warn(`task ${taskId} is RECOVERY_REQUIRED but still has an ACTIVE lease (${activeByTask.get(taskId).lease_id}) -- RECOVERY_REQUIRED must mean no live claim exists`);
    }
  }
}

function main() {
  const schemas = compileSchemas(SCHEMA_DIR);
  const expectedSourceBaseSha = productSourceSha();

  const idsByShardLane = checkShards(schemas, expectedSourceBaseSha);
  const allNodeIds = new Set(idsByShardLane.keys());
  checkAggregates(schemas, allNodeIds);
  const { tasksById } = checkFrontiers(schemas, allNodeIds, expectedSourceBaseSha);
  checkDispatchPlan(schemas, tasksById);
  checkLeases(schemas, tasksById);
  checkRecoveryLog(schemas, tasksById);
  checkNoLeftoverRuntimeArtifacts();

  if (errors.length > 0) {
    console.error(`Atlas validation FAILED with ${errors.length} error(s):`);
    for (const e of errors) console.error(`  - ${e}`);
    process.exit(1);
  }
  console.log(`Atlas validation passed. ${allNodeIds.size} node ids across shards, ${tasksById.size} tasks across frontiers, source_base_sha=${expectedSourceBaseSha} agreed everywhere.`);
}

// Guarded so this module can be imported without the side effect of running
// the full validator (including process.exit(1) on failure) as an import.
if (process.argv[1] === fileURLToPath(import.meta.url)) main();
