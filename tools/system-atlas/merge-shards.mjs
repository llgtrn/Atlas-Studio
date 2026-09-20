#!/usr/bin/env node
// Coordinator-only: merges docs/_machine/system-atlas/v0/shards/*.json into the
// top-level aggregates (snapshot, current-map, target-map, dependency-graph,
// system-index). Does not invent facts -- purely reshapes what shards already
// contain. Missing shards are reported, not fatal, so this can run against a
// partial Atlas (status becomes SYSTEM_ATLAS_V0_PARTIAL).
import { existsSync, writeFileSync } from "node:fs";
import { resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { REPO_ROOT, atlasGenerationSha, canonicalRefSha, nowIso, productSourceSha, readJson, repoRel } from "./lib.mjs";

const SHARD_DIR = resolve(REPO_ROOT, "docs/_machine/system-atlas/v0/shards");
const OUT_DIR = resolve(REPO_ROOT, "docs/_machine/system-atlas/v0");

const EXPECTED_LANES = [
  ["B1_topology", "topology.json"],
  ["B2_capabilities", "capabilities.json"],
  ["B3_runtime", "runtime.json"],
  ["B4_state_event", "state-event.json"],
  ["B5_authority_security", "authority-security.json"],
  ["B6_systems", "systems.json"],
  ["B7_legacy", "legacy.json"],
  ["B8_target", "target.json"],
];

function loadShards() {
  const present = [];
  const missing = [];
  for (const [lane, file] of EXPECTED_LANES) {
    const p = resolve(SHARD_DIR, file);
    if (existsSync(p)) present.push({ lane, file, shard: readJson(p) });
    else missing.push(lane);
  }
  return { present, missing };
}

function main() {
  const { present, missing } = loadShards();
  const sourceBaseSha = productSourceSha();
  const atlasSha = atlasGenerationSha();

  // Item 2: every shard must share one source_base_sha representing the
  // exact product-code snapshot -- refuse to merge a mismatched set rather
  // than silently blending facts derived from different code states.
  const shaMismatches = present.filter((p) => p.shard.source_base_sha !== sourceBaseSha);
  if (shaMismatches.length > 0) {
    console.error(`FATAL: source_base_sha mismatch -- expected ${sourceBaseSha} (current product snapshot), but found:`);
    for (const p of shaMismatches) console.error(`  ${p.lane}: ${p.shard.source_base_sha ?? "(missing field)"}`);
    console.error("Regenerate or re-patch the mismatched shard(s) before merging.");
    process.exit(1);
  }

  const allNodes = [];
  const allEdges = [];
  const nodeIds = new Set();
  const duplicateNodeIds = [];
  const debtFamilies = [];
  const findings = { high: [], low: [], anomalies: [] };

  for (const { lane, shard } of present) {
    for (const n of shard.nodes ?? []) {
      if (nodeIds.has(n.id)) duplicateNodeIds.push({ id: n.id, lane });
      else nodeIds.add(n.id);
      allNodes.push(n); // schema-conformant: no internal bookkeeping fields leak into aggregates
    }
    for (const e of shard.edges ?? []) allEdges.push(e);
    for (const f of shard.debt_families ?? []) debtFamilies.push(f);
    for (const f of shard.high_confidence_findings ?? []) findings.high.push(`[${lane}] ${f}`);
    for (const f of shard.low_confidence_findings ?? []) findings.low.push(`[${lane}] ${f}`);
    for (const f of shard.major_anomalies ?? []) findings.anomalies.push(`[${lane}] ${f}`);
  }

  // current-map.json -- current truth only (source_class != TARGET_PROPOSAL)
  const currentNodes = allNodes.filter((n) => n.source_class !== "TARGET_PROPOSAL");
  const currentEdges = allEdges.filter((e) => e.source_class !== "TARGET_PROPOSAL");
  writeFileSync(
    resolve(OUT_DIR, "current-map.json"),
    `${JSON.stringify(
      {
        schema: "chronica.system-atlas.current-map.v0",
        source_base_sha: sourceBaseSha,
        atlas_generation_sha: atlasSha,
        generated_at: nowIso(),
        node_count: currentNodes.length,
        edge_count: currentEdges.length,
        by_kind: countBy(currentNodes, (n) => n.kind),
        by_reachability: countBy(currentNodes, (n) => n.reachability ?? "UNKNOWN"),
        by_existence: countBy(currentNodes, (n) => n.existence),
        nodes: currentNodes,
        edges: currentEdges,
      },
      null,
      2,
    )}\n`,
  );

  // target-map.json -- TARGET_PROPOSAL only, never mixed with current truth
  const targetNodes = allNodes.filter((n) => n.source_class === "TARGET_PROPOSAL" || n.kind === "PLANNED_SYSTEM");
  const targetEdges = allEdges.filter((e) => e.source_class === "TARGET_PROPOSAL" || e.type === "TARGET_DEPENDS_ON");
  writeFileSync(
    resolve(OUT_DIR, "target-map.json"),
    `${JSON.stringify(
      {
        schema: "chronica.system-atlas.target-map.v0",
        source_base_sha: sourceBaseSha,
        atlas_generation_sha: atlasSha,
        generated_at: nowIso(),
        warning: "Every entry here is source_class=TARGET_PROPOSAL. Nothing in this file may be cited as currently implemented.",
        node_count: targetNodes.length,
        nodes: targetNodes,
        edges: targetEdges,
      },
      null,
      2,
    )}\n`,
  );

  // dependency-graph.json -- full merged graph (current truth), for tooling
  writeFileSync(
    resolve(OUT_DIR, "dependency-graph.json"),
    `${JSON.stringify(
      {
        schema: "chronica.system-atlas.dependency-graph.v0",
        source_base_sha: sourceBaseSha,
        atlas_generation_sha: atlasSha,
        generated_at: nowIso(),
        node_count: currentNodes.length,
        edge_count: currentEdges.length,
        forbidden_edge_count: currentEdges.filter((e) => e.forbidden).length,
        debt_families: debtFamilies,
        nodes: currentNodes,
        edges: currentEdges,
      },
      null,
      2,
    )}\n`,
  );

  // system-index.json -- SYSTEM-kind nodes + their member edges
  const systemNodes = currentNodes.filter((n) => n.kind === "SYSTEM" || n.kind === "CELL_CANDIDATE");
  writeFileSync(
    resolve(OUT_DIR, "system-index.json"),
    `${JSON.stringify(
      {
        schema: "chronica.system-atlas.system-index.v0",
        source_base_sha: sourceBaseSha,
        atlas_generation_sha: atlasSha,
        generated_at: nowIso(),
        system_count: systemNodes.filter((n) => n.kind === "SYSTEM").length,
        cell_candidate_count: systemNodes.filter((n) => n.kind === "CELL_CANDIDATE").length,
        systems: systemNodes,
        membership_edges: currentEdges.filter((e) => e.type === "BELONGS_TO_SYSTEM" || e.type === "BELONGS_TO_CELL_CANDIDATE"),
      },
      null,
      2,
    )}\n`,
  );

  // snapshot.json -- top-level pointer + status
  const status = missing.length === 0 && duplicateNodeIds.length === 0 ? "SYSTEM_ATLAS_V0_BOOTSTRAPPED" : "SYSTEM_ATLAS_V0_PARTIAL";
  const snapshot = {
    schema: "chronica.system-atlas.snapshot.v0",
    source_base_sha: sourceBaseSha,
    atlas_generation_sha: atlasSha,
    canonical_ref_sha: canonicalRefSha("origin/main"),
    generated_at: nowIso(),
    status,
    missing_shards: missing,
    duplicate_node_ids: duplicateNodeIds,
    shards: present.map((p) => `v0/shards/${p.file}`),
    counts: {
      total_nodes_merged: allNodes.length,
      total_edges_merged: allEdges.length,
      current_nodes: currentNodes.length,
      target_proposal_nodes: targetNodes.length,
      debt_families: debtFamilies.length,
      high_confidence_findings: findings.high.length,
      major_anomalies: findings.anomalies.length,
    },
  };
  writeFileSync(resolve(OUT_DIR, "snapshot.json"), `${JSON.stringify(snapshot, null, 2)}\n`);

  console.log(`status=${status}`);
  console.log(`present lanes: ${present.map((p) => p.lane).join(", ") || "(none)"}`);
  if (missing.length) console.log(`missing lanes: ${missing.join(", ")}`);
  if (duplicateNodeIds.length) console.log(`DUPLICATE NODE IDS: ${JSON.stringify(duplicateNodeIds)}`);
  console.log(`merged ${allNodes.length} nodes / ${allEdges.length} edges -> current=${currentNodes.length} nodes, target_proposal=${targetNodes.length} nodes`);
  console.log(`wrote snapshot.json, current-map.json, target-map.json, dependency-graph.json, system-index.json under ${repoRel(OUT_DIR)}`);
}

function countBy(arr, fn) {
  const out = {};
  for (const x of arr) {
    const k = fn(x) ?? "UNKNOWN";
    out[k] = (out[k] ?? 0) + 1;
  }
  return out;
}

// Guarded so this module can be imported (e.g. by tests) without the side
// effect of regenerating and overwriting every aggregate file -- the Atlas
// got bitten by exactly this bug once already (legacy-debt-census.mjs).
if (process.argv[1] === fileURLToPath(import.meta.url)) main();
