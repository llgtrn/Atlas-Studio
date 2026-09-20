#!/usr/bin/env node
// Coordinator-only: builds repair/build/retire/verify frontiers from the merged
// shards. REPAIR groups the known reverse-edge debt into one task per root-cause
// family (not 164 unrelated tasks). Every other task is derived from a node that
// itself carries an `action` field in its owning shard -- this guarantees
// node_ids is never empty (v0 generated tasks with node_ids: [] and no evidence
// trail; v0.1 fixes that at the source instead of validating around it).
import { readdirSync, writeFileSync } from "node:fs";
import { resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { defaultHygieneClearance, retireDispatchGate } from "./hygiene-gate.mjs";
import { REPO_ROOT, atlasGenerationSha, nowIso, productSourceSha, readJson, repoRel } from "./lib.mjs";
import { isMutatingAction, validateMutationOwnership } from "./mutation-ownership.mjs";

const SHARD_DIR = resolve(REPO_ROOT, "docs/_machine/system-atlas/v0/shards");
const OUT_DIR = resolve(REPO_ROOT, "docs/_machine/system-atlas/v0");

// Node `action` -> task `action` (task.schema.json's action enum is coarser
// than node.schema.json's: MOVE/SPLIT/MERGE are architecture-repair work).
const NODE_ACTION_TO_TASK_ACTION = {
  REPAIR: "REPAIR",
  MOVE: "REPAIR",
  SPLIT: "REPAIR",
  MERGE: "REPAIR",
  BUILD: "BUILD",
  RETIRE: "RETIRE",
  VERIFY: "VERIFY",
  INVESTIGATE: "VERIFY",
};

function loadShards() {
  return readdirSync(SHARD_DIR)
    .filter((f) => f.endsWith(".json"))
    .map((f) => ({ file: f, shard: readJson(resolve(SHARD_DIR, f)) }));
}

let taskCounter = { REPAIR: 0, BUILD: 0, RETIRE: 0, VERIFY: 0 };
function nextTaskId(action) {
  taskCounter[action] += 1;
  return `${action}-${String(taskCounter[action]).padStart(4, "0")}`;
}

// Best-effort owned_paths derivation from a node's own evidence citations --
// never fabricated, only extracted from crate paths the node's evidence
// already names. Falls back to [] (vacuously safe, not a lie) when a node
// isn't crate-scoped.
function ownedPathsFromNode(node) {
  const paths = new Set();
  for (const ev of node.evidence ?? []) {
    const m = /crates\/(?:core|cap|adapter|runtime)\/[a-zA-Z0-9_-]+/g;
    for (const match of ev.match(m) ?? []) paths.add(`${match}/**`);
  }
  if (paths.size === 0 && node.id.startsWith("crate:")) {
    paths.add(`crates/**/${node.id.slice("crate:".length)}/**`);
  }
  if (paths.size === 0 && (node.evidence ?? []).some((e) => e.includes("docs/capabilities-canonical"))) {
    paths.add("docs/capabilities-canonical/domains/**");
  }
  return [...paths];
}

// Item 10: a T3 architecture task must stay DESIGN_REQUIRED until an explicit
// canonical_ownership_note documents target ownership/migration semantics.
// v0.1 has none authorized yet, so every T3 REPAIR/BUILD task is DESIGN_REQUIRED.
function statusFor(taskAction, risk, canonicalOwnershipNote) {
  if (risk === "T3" && (taskAction === "REPAIR" || taskAction === "BUILD")) {
    return canonicalOwnershipNote ? "READY" : "DESIGN_REQUIRED";
  }
  return "READY";
}

// Item 2 of the hardening brief: refuse to build frontiers from shards that
// don't all agree on the same product-code snapshot as each other AND as the
// currently-computed snapshot. Exported and pure so this exact rule is
// independently testable -- a previous inline version of this check had an
// operator-precedence bug (`![...set][0] === x` instead of
// `[...set][0] !== x`) that silently made the mismatch branch dead code;
// BF5's adversarial review caught it because nothing exercised it directly.
export function shardsAgreeWithSource(shardShas, sourceBaseSha) {
  if (shardShas.size > 1) return { ok: false, reason: `shards disagree on source_base_sha: ${[...shardShas].join(", ")}` };
  if (shardShas.size === 1 && [...shardShas][0] !== sourceBaseSha) {
    return { ok: false, reason: `shard source_base_sha (${[...shardShas][0]}) != computed productSourceSha() (${sourceBaseSha})` };
  }
  return { ok: true, reason: null };
}

function main() {
  const sourceBaseSha = productSourceSha();
  const atlasSha = atlasGenerationSha();
  const shards = loadShards();
  const byLane = Object.fromEntries(shards.map((s) => [s.shard.lane, s.shard]));
  const topology = byLane.B1_topology;
  const legacy = byLane.B7_legacy;
  const target = byLane.B8_target;

  const shardShas = new Set(shards.map((s) => s.shard.source_base_sha).filter(Boolean));
  const shaCheck = shardsAgreeWithSource(shardShas, sourceBaseSha);
  if (!shaCheck.ok) {
    console.error(`FATAL: ${shaCheck.reason}`);
    process.exit(1);
  }

  const crateDirById = new Map();
  for (const n of topology?.nodes ?? []) crateDirById.set(n.id, n.evidence?.[0]?.replace(/\/Cargo\.toml$/, "") ?? null);

  const repair = [];
  const build = [];
  const retire = [];
  const verify = [];
  const byAction = { REPAIR: repair, BUILD: build, RETIRE: retire, VERIFY: verify };

  // REPAIR: one task per debt family (root-cause clustered, not per-edge).
  for (const family of legacy?.debt_families ?? []) {
    const sourceCrates = new Set();
    for (const e of legacy.edges) {
      if (e.notes === `debt_family=${family.family_id}`) sourceCrates.add(e.from);
    }
    const ownedPaths = [...sourceCrates].map((id) => crateDirById.get(id)).filter(Boolean).map((p) => `${p}/**`);
    const moneyOrAuthority = /POLICY|MONEY/.test(family.family_id);
    const risk = moneyOrAuthority ? "T3" : family.family_id.includes("API_STATE") ? "T2" : "T1";
    repair.push({
      task_id: nextTaskId("REPAIR"),
      action: "REPAIR",
      title: family.family_id,
      node_ids: [...sourceCrates],
      priority: Math.round(family.edge_count * 2 + family.distinct_source_crates * 1.5 + (moneyOrAuthority ? 15 : 0)),
      risk,
      source_base_sha: sourceBaseSha,
      owned_paths: ownedPaths,
      forbidden_paths: ["Cargo.lock", "tools/refoundation/refoundation-layer-debt.json"],
      dependencies: [],
      blocked_by: [],
      verification_required: moneyOrAuthority ? ["focused_compile", "architecture_gate", "cross_account_review"] : ["focused_compile", "architecture_gate"],
      parallel_conflict_group: family.family_id,
      cross_account_verification_required: moneyOrAuthority,
      status: statusFor("REPAIR", risk, null),
      root_cause_family: family.family_id,
      notes: family.root_cause,
    });
  }

  // REPAIR/BUILD/RETIRE/VERIFY: one task per node that itself carries an
  // `action` field in its owning shard. node_ids is always [node.id] --
  // never empty. This replaces v0's free-text-derived tasks (which produced
  // node_ids: [] for anything that wasn't a debt-family edge).
  for (const { shard } of shards) {
    for (const node of shard.nodes ?? []) {
      if (!node.action || node.action === "KEEP") continue;
      const taskAction = NODE_ACTION_TO_TASK_ACTION[node.action];
      if (!taskAction) continue; // unmapped node action (shouldn't happen; skip rather than guess)
      const risk = node.risk && node.risk !== "UNKNOWN" ? node.risk : "T1";
      const ownedPaths = taskAction === "VERIFY" ? [] : ownedPathsFromNode(node);
      const list = byAction[taskAction];
      list.push({
        task_id: nextTaskId(taskAction),
        action: taskAction,
        title: `${node.action}: ${node.label ?? node.id}`,
        node_ids: [node.id],
        priority: taskAction === "VERIFY" ? 25 : node.action === "RETIRE" ? 20 : 35,
        risk,
        source_base_sha: sourceBaseSha,
        owned_paths: ownedPaths,
        forbidden_paths: ["Cargo.lock"],
        dependencies: [],
        blocked_by: [],
        verification_required: taskAction === "VERIFY" ? ["independent_review"] : ["investigate", "focused_compile"],
        parallel_conflict_group: `${shard.lane}-${node.action.toLowerCase()}`,
        cross_account_verification_required: risk === "T3",
        status: statusFor(taskAction, risk, null),
        root_cause_family: shard.lane,
        notes: `${node.notes ?? ""} evidence: ${JSON.stringify(node.evidence ?? [])}`.trim(),
      });
    }
  }

  // BUILD: one task per B8 target system not already fully present, ordered
  // by the build_order_chain (used only to seed priority, never to hardcode
  // dependency safety -- that's derived later by dependency-safety.mjs).
  const chainOrder = (target?.build_order_chain ?? []).map((line) => {
    const m = /^\d+\.\s*(target:[\w-]+(?:\s*\/\s*target:[\w-]+)*)/.exec(line);
    return m ? m[1].split("/").map((s) => s.trim()) : [];
  }).flat().filter(Boolean);

  const targetIdToTaskId = new Map();
  for (const n of target?.nodes ?? []) {
    if (n.existence === "ABSENT" || n.existence === "PARTIAL") {
      targetIdToTaskId.set(n.id, nextTaskId("BUILD"));
    }
  }
  for (const n of target?.nodes ?? []) {
    const taskId = targetIdToTaskId.get(n.id);
    if (!taskId) continue;
    const chainIdx = chainOrder.indexOf(n.id);
    const deps = (target.edges ?? [])
      .filter((e) => e.from === n.id && e.type === "TARGET_DEPENDS_ON")
      .map((e) => targetIdToTaskId.get(e.to))
      .filter(Boolean);
    build.push({
      task_id: taskId,
      action: "BUILD",
      title: n.label,
      node_ids: [n.id],
      priority: Math.round((n.existence === "PARTIAL" ? 30 : 15) - (chainIdx >= 0 ? chainIdx : 20)),
      risk: "T2",
      source_base_sha: sourceBaseSha,
      owned_paths: [],
      forbidden_paths: ["Cargo.lock"],
      dependencies: deps,
      blocked_by: deps,
      verification_required: ["design_review", "architecture_gate"],
      parallel_conflict_group: "target-architecture",
      cross_account_verification_required: false,
      status: deps.length ? "BLOCKED_DEPENDENCY" : "DESIGN_REQUIRED",
      notes: `${n.existence === "PARTIAL" ? "Promote existing partial implementation" : "Design and build"} -- see docs/_machine/system-atlas/v0/shards/target.json for evidence. TARGET_PROPOSAL: not yet authorized, design-readiness mapping only.`,
    });
  }

  // Also retire the OSINT probe-bin cluster explicitly (already covered by
  // the action-node loop above via runtime.json's entrypoint:osint-probe-bins
  // node, kept here as a no-op comment for traceability with the v0 frontier).

  // B.1.1 hardening gates, applied uniformly to every generated mutating task
  // regardless of which loop produced it -- a task never reaches a dispatchable
  // status (READY/PLANNED) by construction if it fails either gate.
  for (const t of [...repair, ...build, ...retire]) {
    if (t.action === "RETIRE") {
      if (!t.hygiene_clearance) Object.assign(t, defaultHygieneClearance());
      const gate = retireDispatchGate(t);
      if (!gate.allowed && (t.status === "READY" || t.status === "PLANNED")) {
        t.status = gate.recommendedStatus;
        t.notes = `${t.notes ?? ""} HYGIENE_GATE_BLOCKED: ${gate.reason}`.trim();
      }
    }
    if (isMutatingAction(t.action) && t.owned_paths.length === 0 && (t.status === "READY" || t.status === "PLANNED")) {
      t.status = "DESIGN_REQUIRED";
      t.notes = `${t.notes ?? ""} MUTATION_OWNERSHIP_GATE: no owned_paths known; broad/imprecise scope must not be a runnable mutation -- decompose into bounded owner-specific tasks first.`.trim();
    }
  }

  // Defense in depth: hard-fail the generator itself if any mutating task
  // somehow still reaches a dispatchable status with no owned_paths.
  const mutationViolations = [...repair, ...build, ...retire, ...verify].flatMap((t) => validateMutationOwnership(t));
  if (mutationViolations.length > 0) {
    console.error(`FATAL: ${mutationViolations.length} mutation-ownership violation(s) survived gating:`);
    for (const v of mutationViolations) console.error(`  - ${v}`);
    process.exit(1);
  }

  for (const [name, tasks] of [["repair-frontier", repair], ["build-frontier", build], ["retire-frontier", retire], ["verify-frontier", verify]]) {
    tasks.sort((a, b) => b.priority - a.priority);
    writeFileSync(
      resolve(OUT_DIR, `${name}.json`),
      `${JSON.stringify({ schema: `chronica.system-atlas.${name}.v0`, source_base_sha: sourceBaseSha, atlas_generation_sha: atlasSha, generated_at: nowIso(), task_count: tasks.length, tasks }, null, 2)}\n`,
    );
  }

  const emptyNodeIds = [...repair, ...build, ...retire, ...verify].filter((t) => t.node_ids.length === 0);
  if (emptyNodeIds.length > 0) {
    console.error(`FATAL: ${emptyNodeIds.length} task(s) generated with empty node_ids: ${emptyNodeIds.map((t) => t.task_id).join(", ")}`);
    process.exit(1);
  }

  console.log(`REPAIR: ${repair.length} tasks (${legacy?.debt_families?.length ?? 0} debt families + ${repair.length - (legacy?.debt_families?.length ?? 0)} action-node)`);
  console.log(`BUILD: ${build.length} tasks`);
  console.log(`RETIRE: ${retire.length} tasks`);
  console.log(`VERIFY: ${verify.length} tasks`);
  const designRequired = [...repair, ...build].filter((t) => t.status === "DESIGN_REQUIRED").length;
  console.log(`DESIGN_REQUIRED (T3 without canonical_ownership_note, or mutating task without owned_paths): ${designRequired}`);
  const hygieneBlocked = retire.filter((t) => t.status === "BLOCKED_SEMANTICS" || t.status === "BLOCKED_DEPENDENCY").length;
  console.log(`RETIRE hygiene-gate blocked: ${hygieneBlocked}/${retire.length} (Lane C integration required before any RETIRE task can dispatch)`);
  console.log(`wrote *-frontier.json under ${repoRel(OUT_DIR)}`);
}

// Guarded so this module can be imported (e.g. by tests, to reuse
// shardsAgreeWithSource()) without the side effect of regenerating every
// frontier file -- the Atlas got bitten by exactly this bug once already.
if (process.argv[1] === fileURLToPath(import.meta.url)) main();
